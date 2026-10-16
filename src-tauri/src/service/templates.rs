//! SPEC-22：文件模板的应用服务。
//!
//! 所有写操作遵循“先文件、后数据库；数据库失败则撤销文件操作”。
//! 撤销失败返回 `TEMPLATE_RECONCILE_PENDING`，由下一次目录扫描以磁盘为准收敛。
//! 日志只经由调用方记录错误码；本文件不记录文件名、路径或分组名。

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, MutexGuard,
    },
};

use super::SnippetService;
use crate::{
    error::AppError,
    model::{
        Group, GroupCreateInput, GroupDeleteInput, GroupDeleteOutcome, GroupUpdateInput,
        OpenWithCandidate, ScanResult, Snippet, TemplateCreateBlankInput, TemplateImportInput,
        TemplateImportOutcome, TemplateImportRejected, TemplateOpenInput, TemplateUpdateInput,
    },
    platform::files as platform_files,
    storage::{FileMove, FileRow, GroupDeleteFs, NewFile, UpdateFile},
    templates::{
        fsops::{self, fs_error},
        names::{
            dedupe_stem, file_name, key_from_stem, sanitize_stem, split_name, title_from_stem,
            unique_key,
        },
        paths::{
            base_name, dir_join, group_dirs, join_rel, parent_dir, plan_group_delete_names,
            safe_join, verify_under,
        },
        scan, MAX_ENTRIES, MAX_FILE_BYTES, SCAN_BATCH, TRASH_DIR,
    },
    validation::{normalize_key, normalize_list, validate_group_name},
};

/// 模板目录的运行时状态。
pub struct TemplateFs {
    root: PathBuf,
    ready: AtomicBool,
    /// 串行化所有会改动模板目录的操作（含扫描）。
    lock: Mutex<()>,
    /// 合并并发扫描。
    scan_gate: Mutex<()>,
    last_scan: Mutex<Option<ScanResult>>,
}

impl TemplateFs {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            ready: AtomicBool::new(false),
            lock: Mutex::new(()),
            scan_gate: Mutex::new(()),
            last_scan: Mutex::new(None),
        }
    }

    fn guard(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn missing_error() -> AppError {
    AppError::new(
        "TEMPLATE_FILE_MISSING",
        "模板文件已不在模板目录中。已重新扫描目录，请刷新列表。",
    )
}

fn reconcile_pending() -> AppError {
    AppError::new(
        "TEMPLATE_RECONCILE_PENDING",
        "操作失败，且文件未能还原。下一次目录扫描会以磁盘为准修正记录。",
    )
}

fn path_exists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

fn trash_rel(id: &str, ext: &str) -> String {
    if ext.is_empty() {
        format!("{TRASH_DIR}/{id}")
    } else {
        format!("{TRASH_DIR}/{id}.{ext}")
    }
}

impl SnippetService {
    /// 绑定模板目录并立即初始化（补建分组目录）。失败不阻止启动：之后的文件操作会重试并报告 `TEMPLATE_FS_FAILED`。
    pub fn with_templates(mut self, root: PathBuf) -> Self {
        let fs = TemplateFs::new(root);
        self.templates = Some(fs);
        let _ = self.tfs();
        self
    }

    fn tfs(&self) -> Result<&TemplateFs, AppError> {
        let fs = self
            .templates
            .as_ref()
            .ok_or_else(|| fs_error("模板目录未启用。"))?;
        if !fs.ready.load(Ordering::SeqCst) {
            let groups = self.repository.group_list()?;
            let dirs: Vec<String> = group_dirs(&groups).into_values().collect();
            fsops::init_dirs(&fs.root, &dirs)?;
            fs.ready.store(true, Ordering::SeqCst);
        }
        Ok(fs)
    }

    fn tfs_opt(&self) -> Result<Option<&TemplateFs>, AppError> {
        if self.templates.is_none() {
            return Ok(None);
        }
        self.tfs().map(Some)
    }

    /// 分组 id → 相对目录；`None` 表示模板目录根。
    fn dir_of_group(&self, group_id: Option<&str>) -> Result<String, AppError> {
        let Some(id) = group_id else {
            return Ok(String::new());
        };
        let groups = self.repository.group_list()?;
        group_dirs(&groups)
            .remove(id)
            .ok_or_else(|| AppError::validation("groupId", "分组不存在。请重新载入后再试。"))
    }

    fn require_file(&self, id: &str) -> Result<FileRow, AppError> {
        self.repository
            .file_row(id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "模板不存在。请重新载入列表。"))
    }

    // ------------------------------------------------------------ 分组目录同步

    pub(super) fn fs_group_create(
        &self,
        input: GroupCreateInput,
        name: &str,
    ) -> Result<Group, AppError> {
        let Some(fs) = self.tfs_opt()? else {
            return self.repository.group_create(
                name,
                input.parent_id.as_deref(),
                &input.request_id,
            );
        };
        let _guard = fs.guard();
        let dir = join_rel(&self.dir_of_group(input.parent_id.as_deref())?, name);
        let path = safe_join(&fs.root, &dir)?;
        let existed = path_exists(&path);
        fsops::ensure_dir_in(&fs.root, &path)?;
        match self
            .repository
            .group_create(name, input.parent_id.as_deref(), &input.request_id)
        {
            Ok(group) => Ok(group),
            Err(error) => {
                if !existed {
                    // 只会删除空目录，绝不删除文件。
                    let _ = fs::remove_dir(&path);
                }
                Err(error)
            }
        }
    }

    pub(super) fn fs_group_update(
        &self,
        input: GroupUpdateInput,
        name: &str,
    ) -> Result<Group, AppError> {
        let Some(fs) = self.tfs_opt()? else {
            return self.repository.group_update(
                &input.id,
                input.revision,
                name,
                input.parent_id.as_deref(),
            );
        };
        let _guard = fs.guard();
        let groups = self.repository.group_list()?;
        let dirs = group_dirs(&groups);
        let current = groups
            .iter()
            .find(|g| g.id == input.id)
            .ok_or_else(|| AppError::new("GROUP_NOT_FOUND", "分组不存在。请重新载入。"))?;
        if current.revision != input.revision {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "分组已在其他窗口修改。请重新载入后再保存。",
            ));
        }
        let old_dir = dirs.get(&input.id).cloned().unwrap_or_default();
        let new_parent = match input.parent_id.as_deref() {
            Some(parent) => dirs
                .get(parent)
                .cloned()
                .ok_or_else(|| AppError::new("GROUP_NOT_FOUND", "上级分组不存在。请重新载入。"))?,
            None => String::new(),
        };
        let new_dir = join_rel(&new_parent, name);
        if new_dir.starts_with(&format!("{old_dir}/")) {
            return Err(AppError::validation(
                "parentId",
                "不能把分组移动到自身或其子分组下。",
            ));
        }
        if old_dir == new_dir {
            return self.repository.group_update(
                &input.id,
                input.revision,
                name,
                input.parent_id.as_deref(),
            );
        }
        let old_path = safe_join(&fs.root, &old_dir)?;
        let new_path = safe_join(&fs.root, &new_dir)?;
        let renamed = path_exists(&old_path);
        if renamed {
            if path_exists(&new_path) {
                // 仅允许替换空目录。
                fs::remove_dir(&new_path).map_err(|_| fs_error("目标位置已存在同名目录。"))?;
            }
            if let Some(parent) = new_path.parent() {
                fsops::ensure_dir_in(&fs.root, parent)?;
            }
            fs::rename(&old_path, &new_path).map_err(|_| fs_error("无法重命名模板目录。"))?;
        } else {
            fsops::ensure_dir_in(&fs.root, &new_path)?;
        }
        match self.repository.group_update_with_rewrite(
            &input.id,
            input.revision,
            name,
            input.parent_id.as_deref(),
            Some((&old_dir, &new_dir)),
        ) {
            Ok(group) => Ok(group),
            Err(error) => {
                if renamed {
                    if fs::rename(&new_path, &old_path).is_err() {
                        return Err(reconcile_pending());
                    }
                } else {
                    let _ = fs::remove_dir(&new_path);
                }
                Err(error)
            }
        }
    }

    pub(super) fn fs_group_delete(
        &self,
        input: GroupDeleteInput,
    ) -> Result<GroupDeleteOutcome, AppError> {
        let Some(fs) = self.tfs_opt()? else {
            return self.repository.group_delete(&input.id, input.revision);
        };
        let _guard = fs.guard();
        let groups = self.repository.group_list()?;
        let Some(current) = groups.iter().find(|g| g.id == input.id) else {
            return self.repository.group_delete(&input.id, input.revision);
        };
        if current.revision != input.revision {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "分组已在其他窗口修改。请重新载入后再删除。",
            ));
        }
        let dirs = group_dirs(&groups);
        let dir = dirs.get(&input.id).cloned().unwrap_or_default();
        let parent_dir_rel = parent_dir(&dir).to_owned();
        let dir_path = safe_join(&fs.root, &dir)?;
        let tracked: HashMap<String, (String, String)> = self
            .repository
            .file_rows_for_scan()?
            .into_iter()
            .map(|row| (row.rel_path.clone(), (row.id, row.title)))
            .collect();

        let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut plan = GroupDeleteFs::default();
        let fs_result = (|| -> Result<(), AppError> {
            if !path_exists(&dir_path) {
                return Ok(());
            }
            let target_parent = dir_join(&fs.root, &parent_dir_rel)?;
            verify_under(&fs.root, &dir_path)?;
            // 1. 子分组目录整体上移（名称与存储层的 (2) 规则一致）。
            for (child_id, final_name) in plan_group_delete_names(&groups, &input.id) {
                let Some(child_name) = groups
                    .iter()
                    .find(|g| g.id == child_id)
                    .map(|g| g.name.clone())
                else {
                    continue;
                };
                let old_rel = join_rel(&dir, &child_name);
                let old = safe_join(&fs.root, &old_rel)?;
                if !path_exists(&old) {
                    continue;
                }
                let new_rel = join_rel(&parent_dir_rel, &final_name);
                let new = safe_join(&fs.root, &new_rel)?;
                if path_exists(&new) {
                    return Err(fs_error("目标位置已存在同名目录。"));
                }
                fs::rename(&old, &new).map_err(|_| fs_error("无法移动子分组目录。"))?;
                moves.push((old, new));
                plan.prefix_rewrites.push((old_rel, new_rel));
            }
            // 2. 目录里剩余的内容（文件模板、未被扫描的文件、杂项目录）逐个上移，重名加 (2)。
            let mut entries: Vec<String> = fs::read_dir(&dir_path)
                .map_err(|_| fs_error("无法读取模板目录。"))?
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
                .collect();
            entries.sort();
            for name in entries {
                let old = dir_path.join(&name);
                let is_dir = fs::symlink_metadata(&old)
                    .map(|m| m.is_dir())
                    .unwrap_or(false);
                let (stem, ext) = if is_dir {
                    (name.clone(), String::new())
                } else {
                    split_name(&name)
                };
                let final_stem = dedupe_stem(&stem, &ext, |n| path_exists(&target_parent.join(n)));
                let final_name = file_name(&final_stem, &ext);
                let new = target_parent.join(&final_name);
                if is_dir {
                    fs::rename(&old, &new).map_err(|_| fs_error("无法移动模板目录。"))?;
                } else {
                    fsops::move_file(&old, &new)?;
                }
                moves.push((old, new));
                if let Some((id, title)) = tracked.get(&join_rel(&dir, &name)) {
                    plan.file_moves.push(FileMove {
                        id: id.clone(),
                        new_rel_path: join_rel(&parent_dir_rel, &final_name),
                        new_title: if final_stem == stem {
                            title.clone()
                        } else {
                            title_from_stem(&final_stem)
                        },
                    });
                }
            }
            fs::remove_dir(&dir_path).map_err(|_| fs_error("无法删除空的模板目录。"))?;
            Ok(())
        })();
        let undo = |moves: &[(PathBuf, PathBuf)], dir_path: &Path| -> Result<(), AppError> {
            fsops::ensure_dir_in(&fs.root, dir_path)?;
            for (old, new) in moves.iter().rev() {
                fs::rename(new, old).map_err(|_| reconcile_pending())?;
            }
            Ok(())
        };
        if let Err(error) = fs_result {
            return Err(undo(&moves, &dir_path).err().unwrap_or(error));
        }
        match self
            .repository
            .group_delete_with_fs(&input.id, input.revision, &plan)
        {
            Ok(outcome) => Ok(outcome),
            Err(error) => Err(undo(&moves, &dir_path).err().unwrap_or(error)),
        }
    }

    // ------------------------------------------------------------ 导入与新建

    pub fn template_import(
        &self,
        input: TemplateImportInput,
    ) -> Result<TemplateImportOutcome, AppError> {
        if uuid::Uuid::parse_str(&input.request_id).is_err() {
            return Err(AppError::validation("requestId", "requestId 必须是 UUID。"));
        }
        let fs = self.tfs()?;
        let _guard = fs.guard();
        let dir = self.dir_of_group(input.group_id.as_deref())?;
        let dir_path = dir_join(&fs.root, &dir)?;
        fsops::ensure_dir_in(&fs.root, &dir_path)?;
        let mut active = self.repository.active_entry_count()?;
        let mut created = Vec::new();
        let mut rejected = Vec::new();
        let mut first_hard_error: Option<AppError> = None;

        for (index, source) in input.source_paths.iter().enumerate() {
            let request_id = format!("{}#{index}", input.request_id);
            if let Some(existing) = self.repository.snippet_by_request(&request_id)? {
                created.push(existing);
                continue;
            }
            let source_path = Path::new(source);
            let display = source_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let reject = |rejected: &mut Vec<TemplateImportRejected>, code: &str| {
                rejected.push(TemplateImportRejected {
                    name: display.clone(),
                    code: code.to_owned(),
                });
            };
            let Ok(meta) = fs::metadata(source_path) else {
                reject(&mut rejected, "TEMPLATE_FS_FAILED");
                first_hard_error.get_or_insert_with(|| fs_error("无法读取要导入的文件。"));
                continue;
            };
            if !meta.is_file() {
                reject(&mut rejected, "TEMPLATE_FS_FAILED");
                first_hard_error
                    .get_or_insert_with(|| fs_error("只能导入普通文件，不能导入文件夹。"));
                continue;
            }
            if meta.len() > MAX_FILE_BYTES {
                reject(&mut rejected, "TEMPLATE_TOO_LARGE");
                continue;
            }
            if active >= MAX_ENTRIES {
                reject(&mut rejected, "TEMPLATE_TOO_LARGE");
                continue;
            }
            // 用户主动选择的根外来源：解析链接后复制目标内容，命名沿用所选路径。
            let Ok(resolved) = fs::canonicalize(source_path) else {
                reject(&mut rejected, "TEMPLATE_FS_FAILED");
                first_hard_error.get_or_insert_with(|| fs_error("无法读取要导入的文件。"));
                continue;
            };
            match self.import_one(fs, &dir, &dir_path, source_path, &resolved, &request_id) {
                Ok(snippet) => {
                    active += 1;
                    created.push(snippet);
                }
                Err(error) => {
                    reject(&mut rejected, error.code);
                    first_hard_error.get_or_insert(error);
                }
            }
        }
        if created.is_empty() {
            if let Some(error) = first_hard_error {
                return Err(error);
            }
        }
        Ok(TemplateImportOutcome { created, rejected })
    }

    fn import_one(
        &self,
        fs: &TemplateFs,
        dir: &str,
        dir_path: &Path,
        source: &Path,
        resolved: &Path,
        request_id: &str,
    ) -> Result<Snippet, AppError> {
        let original = source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| fs_error("文件名不是有效的 UTF-8。"))?;
        let (stem, ext) = split_name(original);
        let final_stem = dedupe_stem(&sanitize_stem(&stem), &ext, |n| {
            path_exists(&dir_path.join(n))
        });
        let final_name = file_name(&final_stem, &ext);
        let final_path = dir_path.join(&final_name);
        let rel = join_rel(dir, &final_name);
        // 创建临时文件与 rename 之前再校验一次最终目标目录。
        verify_under(&fs.root, dir_path)?;

        let tmp = dir_path.join(format!(".import-{}.tmp", uuid::Uuid::new_v4()));
        let (sha256, size) = match fsops::copy_with_hash(resolved, &tmp) {
            Ok(value) => value,
            Err(_) => {
                let _ = fs::remove_file(&tmp);
                return Err(fs_error("复制文件失败。"));
            }
        };
        if size > MAX_FILE_BYTES {
            let _ = fs::remove_file(&tmp);
            return Err(AppError::new("TEMPLATE_TOO_LARGE", "文件超过 20 MB 上限。"));
        }
        if path_exists(&final_path) || fs::rename(&tmp, &final_path).is_err() {
            let _ = fs::remove_file(&tmp);
            return Err(fs_error("无法写入模板目录。"));
        }
        let stat = match fsops::stat_regular(&final_path) {
            Ok(stat) => stat,
            Err(error) => {
                let _ = fs::remove_file(&final_path);
                return Err(error);
            }
        };
        let key = unique_key(&key_from_stem(&final_stem), |k| {
            self.repository.key_taken(k).unwrap_or(true)
        });
        let new = NewFile {
            key,
            title: title_from_stem(&final_stem),
            group_id: if dir.is_empty() {
                None
            } else {
                self.group_id_of_dir(dir)?
            },
            rel_path: rel,
            ext,
            size: stat.size,
            inode: Some(stat.inode),
            sha256,
            mtime: stat.mtime,
            auto_imported: true,
        };
        let _ = fs;
        self.repository
            .file_insert(new, Some(request_id))
            .map_err(|error| {
                // 数据库失败：撤销刚复制的副本（只删除我们自己创建的那个文件）。
                if fs::remove_file(&final_path).is_err() {
                    return reconcile_pending();
                }
                error
            })
    }

    fn group_id_of_dir(&self, dir: &str) -> Result<Option<String>, AppError> {
        let groups = self.repository.group_list()?;
        Ok(group_dirs(&groups)
            .into_iter()
            .find(|(_, d)| d == dir)
            .map(|(id, _)| id))
    }

    pub fn template_create_blank(
        &self,
        input: TemplateCreateBlankInput,
    ) -> Result<Snippet, AppError> {
        if uuid::Uuid::parse_str(&input.request_id).is_err() {
            return Err(AppError::validation("requestId", "requestId 必须是 UUID。"));
        }
        let ext = match input.kind.as_str() {
            "md" => "md",
            "xlsx" => "xlsx",
            _ => return Err(AppError::validation("kind", "只能新建 md 或 xlsx。")),
        };
        let title = input.title.trim();
        if !(1..=100).contains(&title.chars().count()) {
            return Err(AppError::validation(
                "title",
                "标题长度必须为 1–100 个字符。",
            ));
        }
        let key = normalize_key(&input.key)?;
        if let Some(existing) = self.repository.snippet_by_request(&input.request_id)? {
            return Ok(existing);
        }
        if self.repository.key_taken(&key)? {
            return Err(AppError::key_conflict(key));
        }
        let fs = self.tfs()?;
        let _guard = fs.guard();
        if self.repository.active_entry_count()? >= MAX_ENTRIES {
            return Err(AppError::new(
                "TEMPLATE_TOO_LARGE",
                "条目数量已达 10,000 上限。",
            ));
        }
        let dir = self.dir_of_group(input.group_id.as_deref())?;
        let dir_path = dir_join(&fs.root, &dir)?;
        fsops::ensure_dir_in(&fs.root, &dir_path)?;
        let final_stem = dedupe_stem(&sanitize_stem(title), ext, |n| {
            path_exists(&dir_path.join(n))
        });
        let final_name = file_name(&final_stem, ext);
        let final_path = dir_path.join(&final_name);
        verify_under(&fs.root, &dir_path)?;
        match ext {
            "md" => fsops::write_new_file(&final_path, format!("# {title}\n").as_bytes())?,
            _ => write_blank_xlsx(&final_path)?,
        }
        let outcome = (|| {
            let stat = fsops::stat_regular(&final_path)?;
            let sha256 = fsops::sha256_file(&final_path)
                .map_err(|_| fs_error("无法读取新建的模板文件。"))?;
            self.repository.file_insert(
                NewFile {
                    key: key.clone(),
                    title: title_from_stem(&final_stem),
                    group_id: input.group_id.clone(),
                    rel_path: join_rel(&dir, &final_name),
                    ext: ext.to_owned(),
                    size: stat.size,
                    inode: Some(stat.inode),
                    sha256,
                    mtime: stat.mtime,
                    auto_imported: false,
                },
                Some(&input.request_id),
            )
        })();
        outcome.map_err(|error| {
            if fs::remove_file(&final_path).is_err() {
                return reconcile_pending();
            }
            error
        })
    }

    // ------------------------------------------------------------ 更新

    pub fn template_update(&self, input: TemplateUpdateInput) -> Result<Snippet, AppError> {
        let fs = self.tfs()?;
        let _guard = fs.guard();
        let row = self.require_file(&input.id)?;
        if row.deleted_at.is_some() {
            return Err(AppError::new(
                "NOT_FOUND",
                "模板在回收站中，请先还原再编辑。",
            ));
        }
        if row.revision != input.revision {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "模板已在其他窗口修改。请重新载入后再保存。",
            ));
        }
        let key = normalize_key(&input.key)?;
        let title = input.title.trim().to_owned();
        if !(1..=100).contains(&title.chars().count()) {
            return Err(AppError::validation(
                "title",
                "标题长度必须为 1–100 个字符。",
            ));
        }
        let aliases = normalize_list(&input.aliases, "aliases", 64)?;
        let tags = normalize_list(&input.tags, "tags", 32)?;

        let new_dir = self.dir_of_group(input.group_id.as_deref())?;
        let old_dir = parent_dir(&row.rel_path).to_owned();
        let current_name = base_name(&row.rel_path).to_owned();
        let (current_stem, _) = split_name(&current_name);
        let title_changed = title != row.title;
        let wanted_stem = if title_changed {
            sanitize_stem(&title)
        } else {
            current_stem.clone()
        };
        let new_dir_path = dir_join(&fs.root, &new_dir)?;
        let same_dir = new_dir == old_dir;
        let final_stem = dedupe_stem(&wanted_stem, &row.ext, |name| {
            !(same_dir && name == current_name) && path_exists(&new_dir_path.join(name))
        });
        let new_name = file_name(&final_stem, &row.ext);
        let new_rel = join_rel(&new_dir, &new_name);
        let final_title = if title_changed || final_stem != current_stem {
            title_from_stem(&final_stem)
        } else {
            row.title.clone()
        };

        let src = safe_join(&fs.root, &row.rel_path)?;
        let dst = safe_join(&fs.root, &new_rel)?;
        let moving = new_rel != row.rel_path;
        if moving {
            fsops::stat_regular(&src).map_err(|_| missing_error())?;
            fsops::move_file(&src, &dst)?;
        }
        let new_path = if moving {
            match fsops::stat_regular(&dst) {
                Ok(stat) => Some((new_rel.clone(), stat.inode, stat.mtime)),
                Err(error) => {
                    return Err(if fs::rename(&dst, &src).is_err() {
                        reconcile_pending()
                    } else {
                        error
                    });
                }
            }
        } else {
            None
        };
        let result = self.repository.file_update(
            &input.id,
            input.revision,
            UpdateFile {
                key: key.clone(),
                normalized_key: key,
                title: final_title,
                group_id: input.group_id.clone(),
                aliases,
                tags,
                pinned: input.pinned,
                new_path,
            },
        );
        match result {
            Ok(snippet) => Ok(snippet),
            Err(error) => {
                if moving && fsops::move_file(&dst, &src).is_err() {
                    return Err(reconcile_pending());
                }
                Err(error)
            }
        }
    }

    // ------------------------------------------------------------ 打开

    fn open_target(&self, id: &str) -> Result<(FileRow, PathBuf), AppError> {
        let fs = self.tfs()?;
        let row = self.require_file(id)?;
        if row.deleted_at.is_some() {
            return Err(missing_error());
        }
        let path = safe_join(&fs.root, &row.rel_path)?;
        fsops::stat_regular(&path).map_err(|_| missing_error())?;
        Ok((row, path))
    }

    pub fn template_open(&self, input: TemplateOpenInput) -> Result<(), AppError> {
        let (row, path) = self.open_target(&input.id)?;
        match input.desktop_id {
            None => platform_files::xdg_open(&path),
            Some(desktop_id) => {
                let mime = platform_files::mime_of(&path);
                let entry =
                    platform_files::find_desktop_entry(&desktop_id, &mime).ok_or_else(|| {
                        AppError::new("TEMPLATE_OPEN_FAILED", "所选应用已不可用。请重新选择。")
                    })?;
                platform_files::launch_desktop(&entry, &path)?;
                self.remember_open_with(&row.ext, &desktop_id)
            }
        }
    }

    fn remember_open_with(&self, ext: &str, desktop_id: &str) -> Result<(), AppError> {
        let (mut settings, revision) = self.repository.get_settings()?;
        if settings.open_with_by_extension.get(ext).map(String::as_str) == Some(desktop_id) {
            return Ok(());
        }
        settings
            .open_with_by_extension
            .insert(ext.to_owned(), desktop_id.to_owned());
        self.repository
            .update_settings(&settings, revision)
            .map(|_| ())
    }

    pub fn template_open_with_candidates(
        &self,
        id: &str,
    ) -> Result<Vec<OpenWithCandidate>, AppError> {
        let (_, path) = self.open_target(id)?;
        let mime = platform_files::mime_of(&path);
        Ok(platform_files::desktop_entries_for(&mime)
            .into_iter()
            .map(|entry| OpenWithCandidate {
                desktop_id: entry.id,
                name: entry.name,
                icon: entry.icon,
            })
            .collect())
    }

    // ------------------------------------------------------------ 回收站

    pub(super) fn file_trash_move(&self, id: &str) -> Result<Snippet, AppError> {
        let fs = self.tfs()?;
        let _guard = fs.guard();
        let row = self.require_file(id)?;
        if row.deleted_at.is_some() {
            return Err(AppError::new("ALREADY_TRASHED", "模板已在回收站中。"));
        }
        let src = safe_join(&fs.root, &row.rel_path)?;
        fsops::stat_regular(&src).map_err(|_| missing_error())?;
        let trash = trash_rel(id, &row.ext);
        let dst = safe_join(&fs.root, &trash)?;
        fsops::move_file(&src, &dst)?;
        self.repository.file_trash(id, &trash).map_err(|error| {
            if fsops::move_file(&dst, &src).is_err() {
                return reconcile_pending();
            }
            error
        })
    }

    pub(super) fn file_trash_restore(&self, id: &str) -> Result<Snippet, AppError> {
        let fs = self.tfs()?;
        let _guard = fs.guard();
        let row = self.require_file(id)?;
        if row.deleted_at.is_none() {
            return Err(AppError::new("NOT_TRASHED", "模板不在回收站中。"));
        }
        if row.missing {
            return Err(AppError::new(
                "TEMPLATE_FILE_MISSING",
                "文件已缺失，无法还原。请把文件放回模板目录后重新扫描。",
            ));
        }
        let groups = self.repository.group_list()?;
        let dirs = group_dirs(&groups);
        let (group_id, dir) = match row
            .original_group_id
            .as_deref()
            .and_then(|g| dirs.get(g).map(|d| (g, d)))
        {
            Some((group, dir)) => (Some(group.to_owned()), dir.clone()),
            None => (None, String::new()),
        };
        let dir_path = dir_join(&fs.root, &dir)?;
        fsops::ensure_dir_in(&fs.root, &dir_path)?;
        let src = safe_join(&fs.root, &trash_rel(id, &row.ext))?;
        fsops::stat_regular(&src).map_err(|_| missing_error())?;
        let wanted = sanitize_stem(&row.title);
        let final_stem = dedupe_stem(&wanted, &row.ext, |n| path_exists(&dir_path.join(n)));
        let new_name = file_name(&final_stem, &row.ext);
        let dst = dir_path.join(&new_name);
        fsops::move_file(&src, &dst)?;
        let title = if final_stem == wanted {
            row.title.clone()
        } else {
            title_from_stem(&final_stem)
        };
        self.repository
            .file_restore(id, &join_rel(&dir, &new_name), group_id.as_deref(), &title)
            .map_err(|error| {
                if fsops::move_file(&dst, &src).is_err() {
                    return reconcile_pending();
                }
                error
            })
    }

    /// 删除回收站里的文件。先改数据库、后删文件：文件删除失败只会留下孤立的回收站文件，不会丢记录。
    fn remove_trashed_files(&self, rows: &[FileRow]) {
        let Some(fs) = self.templates.as_ref() else {
            return;
        };
        for row in rows {
            // 只删除 `.trash/<id>.<ext>`，绝不按记录里的原路径删除。
            if let Ok(path) = safe_join(&fs.root, &trash_rel(&row.id, &row.ext)) {
                let _ = fs::remove_file(path);
            }
        }
    }

    pub(super) fn file_trash_purge_one(&self, id: &str) -> Result<(), AppError> {
        let fs = self.tfs()?;
        let _guard = fs.guard();
        let row = self.require_file(id)?;
        self.repository.trash_purge_one(id)?;
        if !row.missing {
            self.remove_trashed_files(&[row]);
        }
        Ok(())
    }

    pub(super) fn files_trash_empty(&self) -> Result<i64, AppError> {
        let _guard = self.templates.as_ref().map(|fs| fs.guard());
        let rows = self.repository.trashed_file_rows(None)?;
        let count = self.repository.trash_empty()?;
        self.remove_trashed_files(&rows);
        Ok(count)
    }

    pub(super) fn files_auto_purge(&self, threshold: &str) -> Result<i64, AppError> {
        let _guard = self.templates.as_ref().map(|fs| fs.guard());
        let rows = self.repository.trashed_file_rows(Some(threshold))?;
        let count = self.repository.auto_purge(threshold)?;
        self.remove_trashed_files(&rows);
        Ok(count)
    }

    // ------------------------------------------------------------ 扫描

    pub fn template_scan(&self) -> Result<ScanResult, AppError> {
        let fs = self.tfs()?;
        // 并发调用合并：已有扫描在跑时等它结束并返回其结果。
        let _gate = match fs.scan_gate.try_lock() {
            Ok(gate) => gate,
            Err(std::sync::TryLockError::WouldBlock) => {
                let _wait = fs.scan_gate.lock().unwrap_or_else(|p| p.into_inner());
                return Ok(fs
                    .last_scan
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .clone()
                    .unwrap_or_default());
            }
            Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
        };
        let _guard = fs.guard();
        let result = self.scan_locked(fs)?;
        *fs.last_scan.lock().unwrap_or_else(|p| p.into_inner()) = Some(result.clone());
        Ok(result)
    }

    fn scan_locked(&self, fs: &TemplateFs) -> Result<ScanResult, AppError> {
        // 目录被删除时重建，并补建分组目录。
        fs.ready.store(false, Ordering::SeqCst);
        self.tfs()?;
        let groups = self.repository.group_list()?;
        let mut dir_ids: HashMap<String, String> = group_dirs(&groups)
            .into_iter()
            .map(|(id, dir)| (dir, id))
            .collect();
        let rows = self.repository.file_rows_for_scan()?;
        let snapshot = fsops::snapshot(&fs.root)?;
        let capacity = MAX_ENTRIES - self.repository.active_entry_count()?;
        let hasher = |rel: &str| -> Option<String> {
            let path = safe_join(&fs.root, rel).ok()?;
            fsops::sha256_file(&path).ok()
        };
        let plan = scan::plan(&rows, &dir_ids, &snapshot, capacity, &hasher);
        let mut result = ScanResult {
            skipped: plan.skipped.clone(),
            ..ScanResult::default()
        };
        // 新子目录 → 分组（按深度升序，父级先建）。
        for dir in &plan.new_dirs {
            let parent = parent_dir(dir);
            let parent_id = if parent.is_empty() {
                None
            } else {
                dir_ids.get(parent).cloned()
            };
            if !parent.is_empty() && parent_id.is_none() {
                continue;
            }
            let name = base_name(dir);
            if validate_group_name(name).is_err() {
                continue;
            }
            match self.repository.group_create(
                name,
                parent_id.as_deref(),
                &uuid::Uuid::new_v4().to_string(),
            ) {
                Ok(group) => {
                    dir_ids.insert(dir.clone(), group.id);
                }
                Err(_) => result.failed_batches += 1,
            }
        }
        let ops: Vec<scan::ScanOp> = plan
            .ops
            .into_iter()
            .filter(|op| {
                let dir = match op {
                    scan::ScanOp::Move { dir, .. }
                    | scan::ScanOp::Restore { dir, .. }
                    | scan::ScanOp::Import { dir, .. } => dir.as_str(),
                    _ => "",
                };
                dir.is_empty() || dir_ids.contains_key(dir)
            })
            .collect();
        for batch in ops.chunks(SCAN_BATCH) {
            match self.repository.apply_scan_batch(batch, &dir_ids) {
                Ok(()) => {
                    let (added, updated, missing, restored) = scan::count_ops(batch);
                    result.added += added;
                    result.updated += updated;
                    result.missing += missing;
                    result.restored += restored;
                }
                Err(_) => result.failed_batches += 1,
            }
        }
        Ok(result)
    }
}

fn write_blank_xlsx(path: &Path) -> Result<(), AppError> {
    let tmp = path.with_file_name(format!(".new-{}.tmp", uuid::Uuid::new_v4()));
    let book = umya_spreadsheet::new_file();
    let outcome = umya_spreadsheet::writer::xlsx::write(&book, &tmp)
        .map_err(|_| fs_error("无法生成空白 xlsx。"))
        .and_then(|()| {
            if path_exists(path) {
                return Err(fs_error("目标文件已存在。"));
            }
            fs::rename(&tmp, path).map_err(|_| fs_error("无法写入模板文件。"))
        });
    if outcome.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::{model::SearchInput, storage::Repository};

    struct Env {
        _dir: tempfile::TempDir,
        service: SnippetService,
        root: PathBuf,
        scratch: PathBuf,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let root = dir.path().join("templates");
        let scratch = dir.path().join("scratch");
        fs::create_dir_all(&scratch).unwrap();
        let service = SnippetService::new(repo).with_templates(root.clone());
        Env {
            _dir: dir,
            service,
            root,
            scratch,
        }
    }

    impl Env {
        fn source(&self, name: &str, bytes: &[u8]) -> String {
            let path = self.scratch.join(name);
            fs::write(&path, bytes).unwrap();
            path.to_string_lossy().into_owned()
        }

        fn import(&self, paths: &[String], group: Option<&str>) -> TemplateImportOutcome {
            self.service
                .template_import(TemplateImportInput {
                    source_paths: paths.to_vec(),
                    group_id: group.map(String::from),
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap()
        }

        fn group(&self, name: &str, parent: Option<&str>) -> Group {
            self.service
                .group_create(GroupCreateInput {
                    name: name.into(),
                    parent_id: parent.map(String::from),
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap()
        }

        fn row(&self, id: &str) -> FileRow {
            self.service.repository.file_row(id).unwrap().unwrap()
        }

        fn on_disk(&self, rel: &str) -> bool {
            self.root.join(rel).exists()
        }

        fn update(
            &self,
            snippet: &Snippet,
            title: &str,
            group: Option<&str>,
        ) -> Result<Snippet, AppError> {
            self.service.template_update(TemplateUpdateInput {
                id: snippet.id.clone(),
                revision: snippet.revision,
                title: title.into(),
                key: snippet.key.clone(),
                group_id: group.map(String::from),
                tags: vec![],
                aliases: vec![],
                pinned: false,
            })
        }

        fn search(&self, query: &str) -> Vec<String> {
            self.service
                .search(SearchInput {
                    query: query.into(),
                    limit: 20,
                })
                .unwrap()
                .items
                .into_iter()
                .map(|i| i.id)
                .collect()
        }
    }

    #[test]
    fn startup_creates_private_template_and_trash_dirs_and_group_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        repo.group_create("运维", None, &uuid::Uuid::new_v4().to_string())
            .unwrap();
        let root = dir.path().join("templates");
        let _service = SnippetService::new(repo).with_templates(root.clone());
        use std::os::unix::fs::PermissionsExt;
        for path in [root.clone(), root.join(".trash"), root.join("运维")] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn import_copies_file_generates_key_and_marks_auto_imported() {
        let env = env();
        let group = env.group("行政", None);
        let source = env.source("报销单.xlsx", b"PK-fake-xlsx");
        let first = env.import(std::slice::from_ref(&source), Some(&group.id));
        let created = &first.created[0];
        assert_eq!(created.kind, "file");
        assert_eq!(created.key, "报销单");
        assert_eq!(created.title, "报销单");
        assert_eq!(created.file_ext.as_deref(), Some("xlsx"));
        assert!(created.file_auto_imported && !created.sensitive && created.content.is_empty());
        assert!(env.on_disk("行政/报销单.xlsx"));
        assert_eq!(
            fs::read(&source).unwrap(),
            b"PK-fake-xlsx",
            "source untouched"
        );

        let second = env.import(&[source], Some(&group.id));
        let dup = &second.created[0];
        assert_eq!(dup.key, "报销单-2");
        assert_eq!(dup.title, "报销单 (2)");
        assert!(env.on_disk("行政/报销单 (2).xlsx"));
        assert_eq!(
            fs::read_dir(env.root.join("行政")).unwrap().count(),
            2,
            "no temp files left"
        );
    }

    #[test]
    fn import_resolves_symlink_source_outside_root_and_copies_target_content() {
        let env = env();
        let target = env.source("真实.md", b"# hello");
        let link = env.scratch.join("链接.md");
        std::os::unix::fs::symlink(Path::new(&target), &link).unwrap();
        let out = env.import(&[link.to_string_lossy().into_owned()], None);
        assert_eq!(out.created.len(), 1);
        let created = &out.created[0];
        assert_eq!(created.title, "链接");
        assert_eq!(fs::read(env.root.join("链接.md")).unwrap(), b"# hello");
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), b"# hello");
        let dump = format!("{created:?}");
        assert!(!dump.contains(env.scratch.to_str().unwrap()));
    }

    #[test]
    fn import_rejects_oversize_but_keeps_other_files_and_is_idempotent() {
        let env = env();
        let big = env.scratch.join("big.bin");
        fs::File::create(&big)
            .unwrap()
            .set_len(MAX_FILE_BYTES + 1)
            .unwrap();
        let ok = env.source("ok.md", b"# ok\n");
        let request_id = uuid::Uuid::new_v4().to_string();
        let input = TemplateImportInput {
            source_paths: vec![big.to_string_lossy().into_owned(), ok],
            group_id: None,
            request_id,
        };
        let first = env.service.template_import(input.clone()).unwrap();
        assert_eq!(first.created.len(), 1);
        assert_eq!(first.rejected.len(), 1);
        assert_eq!(first.rejected[0].code, "TEMPLATE_TOO_LARGE");
        assert!(!env.on_disk("big.bin"));
        let again = env.service.template_import(input).unwrap();
        assert_eq!(
            again.created[0].id, first.created[0].id,
            "same requestId does not re-import"
        );
        assert_eq!(env.service.repository.list().unwrap().len(), 1);
    }

    #[test]
    fn import_database_failure_removes_the_copy_and_leaves_source() {
        let env = env();
        let source = env.source("a.md", b"hello");
        env.service.repository.inject_file_write_failure(true);
        let error = env
            .service
            .template_import(TemplateImportInput {
                source_paths: vec![source.clone()],
                group_id: None,
                request_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap_err();
        env.service.repository.inject_file_write_failure(false);
        assert_eq!(error.code, "DB_WRITE_FAILED");
        assert_eq!(
            fs::read_dir(&env.root)
                .unwrap()
                .filter(|e| e.as_ref().unwrap().file_name() != ".trash")
                .count(),
            0
        );
        assert!(Path::new(&source).exists());
        assert!(env.service.repository.list().unwrap().is_empty());
    }

    #[test]
    fn create_blank_md_and_xlsx_write_valid_files() {
        let env = env();
        let md = env
            .service
            .template_create_blank(TemplateCreateBlankInput {
                kind: "md".into(),
                title: "周报".into(),
                key: "zhoubao".into(),
                group_id: None,
                request_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        assert_eq!(
            fs::read_to_string(env.root.join("周报.md")).unwrap(),
            "# 周报\n"
        );
        assert!(!md.file_auto_imported);
        let xlsx = env
            .service
            .template_create_blank(TemplateCreateBlankInput {
                kind: "xlsx".into(),
                title: "预算".into(),
                key: "yusuan".into(),
                group_id: None,
                request_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        let book = umya_spreadsheet::reader::xlsx::read(env.root.join("预算.xlsx")).unwrap();
        assert_eq!(book.sheet_count(), 1);
        assert_eq!(book.sheet(0).unwrap().name(), "Sheet1");
        assert_eq!(xlsx.file_ext.as_deref(), Some("xlsx"));
        let conflict = env.service.template_create_blank(TemplateCreateBlankInput {
            kind: "md".into(),
            title: "另一个".into(),
            key: "ZHOUBAO".into(),
            group_id: None,
            request_id: uuid::Uuid::new_v4().to_string(),
        });
        assert_eq!(conflict.unwrap_err().code, "KEY_CONFLICT");
        assert!(!env.on_disk("另一个.md"));
    }

    #[test]
    fn rename_title_syncs_file_name_with_numeric_suffix() {
        let env = env();
        let group = env.group("行政", None);
        let a = env
            .import(&[env.source("周报.md", b"a")], Some(&group.id))
            .created
            .remove(0);
        env.import(&[env.source("月报.md", b"b")], Some(&group.id));
        let updated = env.update(&a, "月报", Some(&group.id)).unwrap();
        assert_eq!(updated.title, "月报 (2)");
        assert!(env.on_disk("行政/月报 (2).md") && !env.on_disk("行政/周报.md"));
        assert!(
            !updated.file_auto_imported,
            "saving metadata clears the auto-import flag"
        );
        assert_eq!(env.row(&a.id).rel_path, "行政/月报 (2).md");
    }

    #[test]
    fn update_moves_file_between_groups_and_restores_it_when_database_fails() {
        let env = env();
        let ops = env.group("运维", None);
        let admin = env.group("行政", None);
        let item = env
            .import(&[env.source("值班表.xlsx", b"x")], Some(&ops.id))
            .created
            .remove(0);
        env.service.repository.inject_file_write_failure(true);
        let failed = env.update(&item, "值班表", Some(&admin.id));
        env.service.repository.inject_file_write_failure(false);
        assert_eq!(failed.unwrap_err().code, "DB_WRITE_FAILED");
        assert!(env.on_disk("运维/值班表.xlsx") && !env.on_disk("行政/值班表.xlsx"));
        assert_eq!(env.row(&item.id).rel_path, "运维/值班表.xlsx");

        let moved = env.update(&item, "值班表", Some(&admin.id)).unwrap();
        assert_eq!(moved.group_id.as_deref(), Some(admin.id.as_str()));
        assert!(env.on_disk("行政/值班表.xlsx") && !env.on_disk("运维/值班表.xlsx"));
        let stale = env.update(&item, "x", None).unwrap_err();
        assert_eq!(stale.code, "REVISION_CONFLICT");
        assert!(
            env.on_disk("行政/值班表.xlsx"),
            "revision conflict must not touch the file"
        );
    }

    #[test]
    fn generic_snippet_update_and_copy_reject_file_templates() {
        let env = env();
        let item = env
            .import(&[env.source("a.md", b"a")], None)
            .created
            .remove(0);
        let error = env
            .service
            .update(crate::model::UpdateSnippetInput {
                id: item.id.clone(),
                revision: item.revision,
                key: "a".into(),
                title: "a".into(),
                content: "text".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                group_id: None,
                language: None,
            })
            .unwrap_err();
        assert_eq!(error.code, "VALIDATION_FAILED");
    }

    #[test]
    fn trash_lifecycle_moves_files_and_keeps_search_index_consistent() {
        let env = env();
        let group = env.group("行政", None);
        let item = env
            .import(&[env.source("报销单.xlsx", b"x")], Some(&group.id))
            .created
            .remove(0);
        assert_eq!(env.search("报销单"), vec![item.id.clone()]);

        env.service.trash_move(&item.id).unwrap();
        let trash_name = format!(".trash/{}.xlsx", item.id);
        assert!(env.on_disk(&trash_name) && !env.on_disk("行政/报销单.xlsx"));
        assert!(env.search("报销单").is_empty());

        env.service.trash_restore(&item.id).unwrap();
        assert!(env.on_disk("行政/报销单.xlsx") && !env.on_disk(&trash_name));
        assert_eq!(env.search("报销单"), vec![item.id.clone()]);

        env.service.trash_move(&item.id).unwrap();
        env.service.trash_purge_one(&item.id).unwrap();
        assert!(!env.on_disk(&trash_name) && !env.on_disk("行政/报销单.xlsx"));
        assert!(env.service.repository.file_row(&item.id).unwrap().is_none());
        assert!(env.search("报销单").is_empty());
    }

    #[test]
    fn restore_falls_back_to_ungrouped_and_suffixes_conflicts() {
        let env = env();
        let group = env.group("临时", None);
        let item = env
            .import(&[env.source("a.md", b"1")], Some(&group.id))
            .created
            .remove(0);
        env.service.trash_move(&item.id).unwrap();
        let current = env.service.group_list().unwrap().remove(0);
        env.service
            .group_delete(GroupDeleteInput {
                id: current.id,
                revision: current.revision,
            })
            .unwrap();
        env.import(&[env.source("a.md", b"2")], None);
        let restored = env.service.trash_restore(&item.id).unwrap();
        assert!(restored.group_id.is_none());
        assert_eq!(restored.title, "a (2)");
        assert!(env.on_disk("a (2).md"));
    }

    #[test]
    fn empty_trash_and_auto_purge_delete_only_trashed_files() {
        let env = env();
        let keep = env
            .import(&[env.source("keep.md", b"k")], None)
            .created
            .remove(0);
        let gone = env
            .import(&[env.source("gone.md", b"g")], None)
            .created
            .remove(0);
        env.service.trash_move(&gone.id).unwrap();
        assert_eq!(env.service.trash_empty().unwrap(), 1);
        assert!(env.on_disk("keep.md") && !env.on_disk(&format!(".trash/{}.md", gone.id)));
        assert!(env.service.repository.file_row(&keep.id).unwrap().is_some());

        let old = env
            .import(&[env.source("old.md", b"o")], None)
            .created
            .remove(0);
        env.service.trash_move(&old.id).unwrap();
        assert_eq!(env.service.auto_purge(Some(7)).unwrap(), 0);
        env.service
            .repository
            .raw_connection()
            .execute(
                "UPDATE snippets SET deleted_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
                params![old.id],
            )
            .unwrap();
        assert_eq!(env.service.auto_purge(Some(7)).unwrap(), 1);
        assert!(!env.on_disk(&format!(".trash/{}.md", old.id)));
    }

    #[test]
    fn open_reports_missing_file_without_launching_anything() {
        let env = env();
        let item = env
            .import(&[env.source("a.md", b"a")], None)
            .created
            .remove(0);
        fs::remove_file(env.root.join("a.md")).unwrap();
        let error = env
            .service
            .template_open(TemplateOpenInput {
                id: item.id.clone(),
                desktop_id: None,
            })
            .unwrap_err();
        assert_eq!(error.code, "TEMPLATE_FILE_MISSING");
        let candidates = env
            .service
            .template_open_with_candidates(&item.id)
            .unwrap_err();
        assert_eq!(candidates.code, "TEMPLATE_FILE_MISSING");
    }

    #[test]
    fn scan_follows_external_rename_move_and_atomic_save_keeping_identity() {
        let env = env();
        let ops = env.group("运维", None);
        env.group("行政", None);
        let item = env
            .import(&[env.source("值班表.xlsx", b"v1")], Some(&ops.id))
            .created
            .remove(0);
        env.service
            .repository
            .raw_connection()
            .execute(
                "UPDATE snippets SET usage_count = 5 WHERE id = ?1",
                params![item.id],
            )
            .unwrap();
        let original_key = item.key.clone();

        // AC-26：外部改名并移动到另一个分组
        fs::rename(
            env.root.join("运维/值班表.xlsx"),
            env.root.join("行政/排班表.xlsx"),
        )
        .unwrap();
        let result = env.service.template_scan().unwrap();
        assert_eq!(
            (
                result.added,
                result.updated,
                result.missing,
                result.restored
            ),
            (0, 1, 0, 0)
        );
        let moved = env.service.get(&item.id).unwrap();
        assert_eq!(moved.key, original_key);
        assert_eq!(moved.usage_count, 5);
        assert_eq!(moved.title, "排班表");
        let admin = env
            .service
            .group_list()
            .unwrap()
            .into_iter()
            .find(|g| g.name == "行政")
            .unwrap();
        assert_eq!(moved.group_id.as_deref(), Some(admin.id.as_str()));
        assert_eq!(env.service.repository.list().unwrap().len(), 1);

        // AC-27：写临时文件再改名（inode 变化）
        fs::write(env.root.join("行政/.排班表.tmp"), b"v2-longer").unwrap();
        fs::rename(
            env.root.join("行政/.排班表.tmp"),
            env.root.join("行政/排班表.xlsx"),
        )
        .unwrap();
        let result = env.service.template_scan().unwrap();
        assert_eq!((result.added, result.updated), (0, 1));
        let saved = env.service.get(&item.id).unwrap();
        assert_eq!(saved.file_size, Some(9));
        assert_eq!(env.service.repository.list().unwrap().len(), 1);

        // 无变化的再次扫描什么都不做
        let quiet = env.service.template_scan().unwrap();
        assert_eq!(
            (quiet.added, quiet.updated, quiet.missing, quiet.restored),
            (0, 0, 0, 0)
        );
        assert_eq!(env.service.get(&item.id).unwrap().revision, saved.revision);
    }

    #[test]
    fn scan_marks_deleted_files_missing_and_restores_them_when_they_return() {
        let env = env();
        let group = env.group("行政", None);
        let item = env
            .import(&[env.source("机房规范.pdf", b"pdf")], Some(&group.id))
            .created
            .remove(0);
        let path = env.root.join("行政/机房规范.pdf");
        let bytes = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let gone = env.service.template_scan().unwrap();
        assert_eq!(gone.missing, 1);
        let missing = env.service.get(&item.id).unwrap();
        assert!(missing.file_missing && missing.deleted_at.is_some());
        assert!(env.search("机房规范").is_empty());
        assert_eq!(
            env.service.trash_restore(&item.id).unwrap_err().code,
            "TEMPLATE_FILE_MISSING"
        );
        assert_eq!(
            env.service.template_scan().unwrap().missing,
            0,
            "already missing stays quiet"
        );

        fs::write(&path, bytes).unwrap();
        let back = env.service.template_scan().unwrap();
        assert_eq!(back.restored, 1);
        let restored = env.service.get(&item.id).unwrap();
        assert!(!restored.file_missing && restored.deleted_at.is_none());
        assert_eq!(restored.group_id.as_deref(), Some(group.id.as_str()));
        assert_eq!(env.search("机房规范"), vec![item.id]);
    }

    #[test]
    fn purging_a_missing_entry_never_deletes_a_file_at_the_original_path() {
        let env = env();
        let item = env
            .import(&[env.source("a.md", b"a")], None)
            .created
            .remove(0);
        fs::remove_file(env.root.join("a.md")).unwrap();
        env.service.template_scan().unwrap();
        fs::write(env.root.join("a.md"), b"someone else's file").unwrap();
        env.service.trash_purge_one(&item.id).unwrap();
        assert_eq!(
            fs::read(env.root.join("a.md")).unwrap(),
            b"someone else's file"
        );
    }

    #[test]
    fn scan_imports_new_files_and_directories_and_skips_symlinks_ignored_and_deep_paths() {
        let env = env();
        fs::create_dir_all(env.root.join("新目录/子目录")).unwrap();
        fs::create_dir_all(env.root.join("a/b/c/d")).unwrap();
        fs::write(env.root.join("新目录/子目录/说明.md"), "# hi").unwrap();
        fs::write(env.root.join("root.txt"), "r").unwrap();
        fs::write(env.root.join("a/b/c/d/deep.md"), "deep").unwrap();
        fs::write(env.root.join("~$lock.docx"), "l").unwrap();
        fs::write(env.root.join("scratch.tmp"), "t").unwrap();
        let outside = env.scratch.join("secret.txt");
        fs::write(&outside, "secret").unwrap();
        std::os::unix::fs::symlink(&outside, env.root.join("link.txt")).unwrap();
        let big = env.root.join("big.bin");
        fs::File::create(&big)
            .unwrap()
            .set_len(MAX_FILE_BYTES + 1)
            .unwrap();

        let result = env.service.template_scan().unwrap();
        assert_eq!(result.added, 2, "说明.md and root.txt");
        let reasons: Vec<&str> = result.skipped.iter().map(|s| s.reason.as_str()).collect();
        assert!(
            reasons.contains(&"symlink")
                && reasons.contains(&"depth_exceeded")
                && reasons.contains(&"too_large")
        );
        let names: Vec<String> = env
            .service
            .group_list()
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        for expected in ["新目录", "子目录", "a", "b", "c"] {
            assert!(names.contains(&expected.to_owned()), "{expected}");
        }
        assert!(!names.contains(&"d".to_owned()));
        let all = env.service.repository.list().unwrap();
        let doc = all.iter().find(|s| s.title == "说明").unwrap();
        assert!(doc.file_auto_imported);
        let sub = env
            .service
            .group_list()
            .unwrap()
            .into_iter()
            .find(|g| g.name == "子目录")
            .unwrap();
        assert_eq!(doc.group_id.as_deref(), Some(sub.id.as_str()));
        assert!(!all.iter().any(|s| s.title == "secret" || s.title == "link"));
    }

    #[test]
    fn scan_recreates_deleted_template_root_and_marks_entries_missing() {
        let env = env();
        let group = env.group("行政", None);
        env.import(&[env.source("a.md", b"a")], Some(&group.id));
        fs::remove_dir_all(&env.root).unwrap();
        let result = env.service.template_scan().unwrap();
        assert_eq!(result.missing, 1);
        assert!(env.root.join(".trash").is_dir() && env.root.join("行政").is_dir());
    }

    #[test]
    fn group_directories_follow_create_rename_move_and_delete() {
        let env = env();
        let ops = env.group("运维", None);
        let db = env.group("数据库", Some(&ops.id));
        env.group("慢查询", Some(&db.id));
        assert!(env.on_disk("运维/数据库/慢查询"));

        let item = env
            .import(&[env.source("a.md", b"a")], Some(&db.id))
            .created
            .remove(0);
        let renamed = env
            .service
            .group_update(GroupUpdateInput {
                id: db.id.clone(),
                revision: db.revision,
                name: "DB".into(),
                parent_id: Some(ops.id.clone()),
            })
            .unwrap();
        assert!(
            env.on_disk("运维/DB/a.md")
                && env.on_disk("运维/DB/慢查询")
                && !env.on_disk("运维/数据库")
        );
        assert_eq!(env.row(&item.id).rel_path, "运维/DB/a.md");

        // 移到顶层
        env.service
            .group_update(GroupUpdateInput {
                id: renamed.id.clone(),
                revision: renamed.revision,
                name: "DB".into(),
                parent_id: None,
            })
            .unwrap();
        assert!(env.on_disk("DB/a.md") && env.on_disk("DB/慢查询"));
        assert_eq!(env.row(&item.id).rel_path, "DB/a.md");
    }

    #[test]
    fn deleting_a_group_moves_directories_and_files_up_with_numeric_suffixes() {
        let env = env();
        let admin = env.group("行政", None);
        let sub = env.group("报销", Some(&admin.id));
        env.group("报销", None);
        let inside = env
            .import(&[env.source("单.md", b"1")], Some(&admin.id))
            .created
            .remove(0);
        let nested = env
            .import(&[env.source("嵌套.md", b"2")], Some(&sub.id))
            .created
            .remove(0);
        let top = env
            .import(&[env.source("单.md", b"3")], None)
            .created
            .remove(0);
        assert_eq!(top.title, "单");

        let current = env
            .service
            .group_list()
            .unwrap()
            .into_iter()
            .find(|g| g.id == admin.id)
            .unwrap();
        let outcome = env
            .service
            .group_delete(GroupDeleteInput {
                id: current.id,
                revision: current.revision,
            })
            .unwrap();
        assert_eq!((outcome.moved_snippets, outcome.moved_groups), (1, 1));
        assert!(!env.on_disk("行政"));
        assert!(
            env.on_disk("报销 (2)/嵌套.md"),
            "child directory renamed like the group"
        );
        assert!(env.on_disk("单 (2).md") && env.on_disk("单.md"));
        assert_eq!(env.row(&inside.id).rel_path, "单 (2).md");
        assert_eq!(env.service.get(&inside.id).unwrap().title, "单 (2)");
        assert_eq!(env.row(&nested.id).rel_path, "报销 (2)/嵌套.md");
        assert_eq!(env.row(&top.id).rel_path, "单.md");
        // 磁盘与数据库一致：随后扫描不产生任何变化
        let quiet = env.service.template_scan().unwrap();
        assert_eq!(
            (quiet.added, quiet.updated, quiet.missing, quiet.restored),
            (0, 0, 0, 0)
        );
    }

    #[test]
    fn failed_group_delete_puts_directories_and_files_back() {
        let env = env();
        let admin = env.group("行政", None);
        let item = env
            .import(&[env.source("单.md", b"1")], Some(&admin.id))
            .created
            .remove(0);
        env.service.repository.inject_file_write_failure(true);
        let error = env
            .service
            .group_delete(GroupDeleteInput {
                id: admin.id.clone(),
                revision: admin.revision,
            })
            .unwrap_err();
        env.service.repository.inject_file_write_failure(false);
        assert_eq!(error.code, "DB_WRITE_FAILED");
        assert!(env.on_disk("行政/单.md") && !env.on_disk("单.md"));
        assert_eq!(env.row(&item.id).rel_path, "行政/单.md");
        assert!(env
            .service
            .group_list()
            .unwrap()
            .iter()
            .any(|g| g.id == admin.id));
    }

    #[test]
    fn group_create_failure_removes_only_the_empty_directory_it_made() {
        let env = env();
        env.group("Ops", None);
        let error = env
            .service
            .group_create(GroupCreateInput {
                name: "ops".into(),
                parent_id: None,
                request_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap_err();
        assert_eq!(error.code, "GROUP_NAME_CONFLICT");
        assert!(env.on_disk("Ops"));
        assert!(
            !env.on_disk("ops")
                || env.root.join("Ops").canonicalize().unwrap()
                    == env.root.join("ops").canonicalize().unwrap()
        );
    }

    #[test]
    fn qa_perf_unchanged_scan_of_2000_files() {
        let env = env();
        for dir in 0..20 {
            let path = env.root.join(format!("g{dir}"));
            fs::create_dir_all(&path).unwrap();
            for file in 0..100 {
                fs::write(
                    path.join(format!("文件{file}.md")),
                    format!("content {dir} {file}"),
                )
                .unwrap();
            }
        }
        let first = env.service.template_scan().unwrap();
        assert_eq!(first.added, 2000);
        let started = Instant::now();
        let quiet = env.service.template_scan().unwrap();
        let elapsed = started.elapsed();
        assert_eq!(
            (quiet.added, quiet.updated, quiet.missing, quiet.restored),
            (0, 0, 0, 0)
        );
        eprintln!("NFR-PERF-07: unchanged scan of 2000 files took {elapsed:?}");
        assert!(elapsed.as_secs_f64() <= 2.0, "took {elapsed:?}");
    }

    #[test]
    fn concurrent_scans_are_coalesced_and_safe() {
        let env = env();
        fs::write(env.root.join("a.md"), "a").unwrap();
        let service = std::sync::Arc::new(env.service);
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let service = std::sync::Arc::clone(&service);
                std::thread::spawn(move || service.template_scan().unwrap())
            })
            .collect();
        let results: Vec<ScanResult> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(results.iter().map(|r| r.added).sum::<i64>() >= 1);
        assert_eq!(
            service.repository.list().unwrap().len(),
            1,
            "imported exactly once"
        );
    }

    use rusqlite::params;

    // ------------------------------------------------ 符号链接逃逸回归（S1）

    fn tree(path: &Path) -> Vec<String> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let p = entry.path();
                out.push(format!(
                    "{}:{}",
                    p.strip_prefix(base).unwrap().display(),
                    fs::read(&p).map(|b| b.len()).unwrap_or(0)
                ));
                if p.is_dir() {
                    walk(base, &p, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(path, path, &mut out);
        out.sort();
        out
    }

    /// 把 `templates/<rel>` 换成指向 `target` 的符号链接（原目录必须为空）。
    fn swap_for_symlink(env: &Env, rel: &str, target: &Path) {
        let link = env.root.join(rel);
        fs::remove_dir(&link).unwrap();
        std::os::unix::fs::symlink(target, &link).unwrap();
    }

    fn assert_fs_rejected<T: std::fmt::Debug>(result: Result<T, AppError>) {
        let error = result.expect_err("symlinked directory must be rejected");
        assert_eq!(error.code, "TEMPLATE_FS_FAILED", "{error:?}");
    }

    #[test]
    fn group_delete_with_symlink_entry_on_exdev_is_rejected_and_fully_rolled_back() {
        let env = env();
        let parent = env.group("父", None);
        let child = env.group("子", Some(&parent.id));
        let file = env
            .import(&[env.source("a.md", b"a")], Some(&child.id))
            .created
            .remove(0);
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "EXTERNAL_SENTINEL").unwrap();
        // 在 .md 之前排序的链接条目，确保普通文件之前/之后都被覆盖。
        std::os::unix::fs::symlink(&secret, env.root.join("父/子/z-link.md")).unwrap();
        std::os::unix::fs::symlink(&secret, env.root.join("父/子/0-link.md")).unwrap();
        let row_before = env.row(&file.id).rel_path;

        super::fsops::FORCE_EXDEV.with(|f| f.set(true));
        let result = env.service.group_delete(GroupDeleteInput {
            id: child.id.clone(),
            revision: child.revision,
        });
        super::fsops::FORCE_EXDEV.with(|f| f.set(false));

        assert_fs_rejected(result);
        // 完全回滚：目录、链接（仍是链接）、文件、数据库行均原样。
        for name in ["z-link.md", "0-link.md"] {
            let meta = fs::symlink_metadata(env.root.join("父/子").join(name)).unwrap();
            assert!(meta.file_type().is_symlink());
            assert!(fs::symlink_metadata(env.root.join("父").join(name)).is_err());
        }
        assert!(env.on_disk("父/子/a.md"));
        assert!(!env.on_disk("父/a.md"));
        assert_eq!(env.row(&file.id).rel_path, row_before);
        assert!(env
            .service
            .repository
            .group_list()
            .unwrap()
            .iter()
            .any(|g| g.id == child.id));
        // 根外文件内容未出现在模板目录中，且目标未被改动。
        let mut stack = vec![env.root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                let meta = fs::symlink_metadata(entry.path()).unwrap();
                if meta.is_dir() {
                    stack.push(entry.path());
                } else if meta.is_file() {
                    assert_ne!(fs::read(entry.path()).unwrap(), b"EXTERNAL_SENTINEL");
                }
            }
        }
        assert_eq!(fs::read_to_string(&secret).unwrap(), "EXTERNAL_SENTINEL");
    }

    #[test]
    fn symlinked_group_directory_rejects_every_write_path() {
        let env = env();
        let admin = env.group("行政", None);
        let other = env.group("资料", None);
        let outside = tempfile::tempdir().unwrap();
        // 已有条目，用于移动/还原/重命名场景。
        let mover = env
            .import(&[env.source("m.md", b"m")], Some(&other.id))
            .created
            .remove(0);
        let trashed = env
            .import(&[env.source("t.md", b"t")], Some(&admin.id))
            .created
            .remove(0);
        env.service.trash_move(&trashed.id).unwrap();
        swap_for_symlink(&env, "行政", outside.path());
        let before = tree(outside.path());

        // 导入
        let src = env.source("x.md", b"x");
        let outcome = env.service.template_import(TemplateImportInput {
            source_paths: vec![src.clone()],
            group_id: Some(admin.id.clone()),
            request_id: uuid::Uuid::new_v4().to_string(),
        });
        assert_fs_rejected(outcome);
        // 新建空白 md / xlsx
        for kind in ["md", "xlsx"] {
            assert_fs_rejected(env.service.template_create_blank(TemplateCreateBlankInput {
                kind: kind.into(),
                title: "新".into(),
                key: format!("k-{kind}"),
                group_id: Some(admin.id.clone()),
                request_id: uuid::Uuid::new_v4().to_string(),
            }));
        }
        // 移动/改分组
        assert_fs_rejected(env.update(&mover, "m", Some(&admin.id)));
        // 还原到被替换的分组目录
        assert_fs_rejected(env.service.trash_restore(&trashed.id));
        // 分组重命名 / 在其下新建子分组
        assert_fs_rejected(env.service.group_update(GroupUpdateInput {
            id: admin.id.clone(),
            revision: admin.revision,
            name: "行政2".into(),
            parent_id: None,
        }));
        assert_fs_rejected(env.service.group_create(GroupCreateInput {
            name: "子".into(),
            parent_id: Some(admin.id.clone()),
            request_id: uuid::Uuid::new_v4().to_string(),
        }));
        assert_fs_rejected(env.service.group_delete(GroupDeleteInput {
            id: admin.id.clone(),
            revision: admin.revision,
        }));

        assert_eq!(tree(outside.path()), before);
        assert!(env.on_disk("资料/m.md"));
    }

    #[test]
    fn symlinked_intermediate_directory_rejects_writes_and_leaves_outside_untouched() {
        let env = env();
        let ops = env.group("运维", None);
        let db = env.group("数据库", Some(&ops.id));
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join("数据库")).unwrap();
        let moved = env
            .import(&[env.source("m.md", b"m")], None)
            .created
            .remove(0);
        fs::remove_dir(env.root.join("运维/数据库")).unwrap();
        swap_for_symlink(&env, "运维", outside.path());
        let before = tree(outside.path());

        assert_fs_rejected(env.service.template_import(TemplateImportInput {
            source_paths: vec![env.source("x.md", b"x")],
            group_id: Some(db.id.clone()),
            request_id: uuid::Uuid::new_v4().to_string(),
        }));
        assert_fs_rejected(env.service.template_create_blank(TemplateCreateBlankInput {
            kind: "md".into(),
            title: "新".into(),
            key: "k-md".into(),
            group_id: Some(db.id.clone()),
            request_id: uuid::Uuid::new_v4().to_string(),
        }));
        assert_fs_rejected(env.update(&moved, "m", Some(&db.id)));
        assert_fs_rejected(env.service.group_update(GroupUpdateInput {
            id: db.id.clone(),
            revision: db.revision,
            name: "DB".into(),
            parent_id: Some(ops.id.clone()),
        }));

        assert_eq!(tree(outside.path()), before);
        assert!(env.on_disk("m.md"));
    }
}
