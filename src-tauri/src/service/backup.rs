//! SPEC-24：zip 备份导出、校验、导入与重置示例数据。
//!
//! 一致性约定：
//! - 导出、导入、重置全程持有模板目录锁（同时挡住扫描与其他模板写操作）。
//! - 导入先把文件解压到应用数据目录下的 `import-staging-<token>/` 并校验大小与 SHA-256；
//!   数据库写入、文件移入都发生在同一个尚未提交的事务内，任何一步失败即回滚事务并撤销已做的文件操作。
//! - 日志只经由调用方记录错误码；本文件不记录备份路径、文件名或分组名。

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use serde_json::Value;

use super::{templates::TemplateFs, SnippetService};
use crate::{
    backup::{
        self, BackupSnippet, ExportFileInput, FileDigest, ParsedBackup, ZipIndex, MAX_JSON_BYTES,
        MAX_ZIP_BYTES,
    },
    error::AppError,
    model::{
        BackupProgress, ExportOutcome, ExportPreview, Group, ImportOutcome, ImportValidation,
    },
    storage::{ExistingSnippet, ImportPlan, PlanFile, PlanGroup, PlanSnippet},
    templates::{
        export::rename_no_replace,
        fsops::{self, fs_error},
        names::{dedupe_stem, file_name, sanitize_stem, split_name},
        paths::{base_name, group_dirs, join_rel, safe_join},
        TRASH_DIR,
    },
};

const STAGING_PREFIX: &str = "import-staging-";
const RESET_PREFIX: &str = "templates-reset-";

pub type Progress<'a> = &'a dyn Fn(BackupProgress);

#[cfg(test)]
fn no_progress(_: BackupProgress) {}

/// 通过校验、等待用户确认的一次导入。
struct Staged {
    backup: ParsedBackup,
    staging: Option<PathBuf>,
    /// 备份内相对路径 → 暂存文件。
    files: HashMap<String, PathBuf>,
}

#[derive(Default)]
pub struct BackupState {
    staged: Mutex<HashMap<String, Staged>>,
    committed: Mutex<HashMap<String, ImportOutcome>>,
    importing: AtomicBool,
    /// 测试注入：在指定阶段让导入失败（stash / place / after_place）。
    fail_stage: Mutex<Option<&'static str>>,
}

struct ImportingGuard<'a>(&'a AtomicBool);

impl<'a> ImportingGuard<'a> {
    fn enter(flag: &'a AtomicBool) -> Result<Self, AppError> {
        if flag.swap(true, Ordering::SeqCst) {
            return Err(AppError::new("IMPORT_FAILED", "已有备份导入或重置正在进行。"));
        }
        Ok(Self(flag))
    }
}

impl Drop for ImportingGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

fn export_failed(message: &str) -> AppError {
    AppError::new("EXPORT_FAILED", message)
}

fn schema_invalid(message: &str) -> AppError {
    AppError::new("IMPORT_SCHEMA_INVALID", message)
}

/// 导出的全部数据（调用方持有模板锁时保证与磁盘一致）。
struct ExportData {
    groups: Vec<Group>,
    snippets: Vec<crate::model::Snippet>,
    rows: HashMap<String, crate::storage::FileRow>,
    settings: Value,
}

impl SnippetService {
    pub(super) fn recover_backup_leftovers(&self) {
        let Some(fs) = self.templates.as_ref() else { return };
        let Some(base) = fs.root.parent() else { return };
        let Ok(entries) = fs::read_dir(base) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let path = entry.path();
            if name.starts_with(STAGING_PREFIX) {
                let _ = fs::remove_dir_all(&path);
            } else if name.starts_with(RESET_PREFIX) {
                self.recover_reset_dir(&fs.root, &path);
            }
        }
    }

    /// 重置过程中崩溃的遗留：数据库里还有文件模板，说明事务没有提交，把旧目录改回去。
    fn recover_reset_dir(&self, root: &Path, leftover: &Path) {
        let uncommitted = self.repository.file_row_count().map(|n| n > 0).unwrap_or(false);
        if uncommitted && !contains_regular_file(root) {
            if fs::remove_dir_all(root).is_ok() {
                let _ = fs::rename(leftover, root);
            }
        } else if !uncommitted {
            let _ = fs::remove_dir_all(leftover);
        }
    }

    fn collect_export(&self) -> Result<ExportData, AppError> {
        let groups = self.repository.group_list()?;
        let snippets = self.repository.list()?;
        let rows = self
            .repository
            .backup_file_rows()?
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect();
        let settings = backup::backup_settings(&self.repository.get_settings()?.0);
        Ok(ExportData {
            groups,
            snippets,
            rows,
            settings,
        })
    }

    // ------------------------------------------------------------ 导出

    pub fn export_preview(&self, _path: &str) -> Result<ExportPreview, AppError> {
        let data = self.collect_export()?;
        let digests: HashMap<String, Option<FileDigest>> = data
            .rows
            .values()
            .map(|row| {
                (
                    row.id.clone(),
                    (!row.missing).then(|| FileDigest {
                        size: row.size.max(0) as u64,
                        sha256: row.sha256.clone(),
                    }),
                )
            })
            .collect();
        let json = backup::build_backup_json(
            &data.groups,
            &data.snippets,
            &data.rows,
            &digests,
            &data.settings,
        );
        let json_len = serde_json::to_vec(&json)
            .map_err(|_| export_failed("序列化估算失败。"))?
            .len() as u64;
        let template_bytes: u64 = data
            .rows
            .values()
            .filter(|row| !row.missing)
            .map(|row| row.size.max(0) as u64)
            .sum();
        let estimated = json_len + template_bytes;
        Ok(ExportPreview {
            estimated_size_bytes: estimated,
            sensitive_count: data.snippets.iter().filter(|s| s.sensitive).count() as i64,
            snippet_count: data.snippets.len() as i64,
            has_settings: true,
            template_count: data.snippets.iter().filter(|s| s.kind == "file").count() as i64,
            template_bytes,
            exceeds_limit: estimated > MAX_ZIP_BYTES || json_len > MAX_JSON_BYTES,
        })
    }

    #[cfg(test)]
    pub fn export_confirm(&self, path: &str) -> Result<ExportOutcome, AppError> {
        self.export_confirm_with(path, &no_progress)
    }

    pub fn export_confirm_with(
        &self,
        path: &str,
        progress: Progress<'_>,
    ) -> Result<ExportOutcome, AppError> {
        let target = export_target(path)?;
        let fs = self.tfs_opt()?;
        let _guard = fs.map(|fs| fs.guard());
        let data = self.collect_export()?;
        let preview = self.export_preview(path)?;
        if preview.exceeds_limit {
            return Err(export_failed(&format!(
                "备份内容超过上限（估算 {} 字节；backup.json ≤ 100 MB，zip ≤ 1 GB）。",
                preview.estimated_size_bytes
            )));
        }
        let mut inputs: Vec<ExportFileInput> = Vec::new();
        if let Some(fs) = fs {
            for snippet in data.snippets.iter().filter(|s| s.kind == "file") {
                let Some(row) = data.rows.get(&snippet.id) else { continue };
                if row.missing {
                    continue;
                }
                let abs = safe_join(&fs.root, &row.rel_path)?;
                inputs.push(ExportFileInput {
                    snippet_id: row.id.clone(),
                    rel_path: row.rel_path.clone(),
                    abs_path: abs,
                });
            }
        }
        let name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("backup.zip")
            .to_owned();
        let temp = target.with_file_name(format!(".{name}.{}.part", uuid::Uuid::new_v4()));
        let cleanup = TempGuard(temp.clone());
        let size = backup::write_archive(
            &temp,
            &inputs,
            |digests| {
                let json = backup::build_backup_json(
                    &data.groups,
                    &data.snippets,
                    &data.rows,
                    digests,
                    &data.settings,
                );
                serde_json::to_vec_pretty(&json).map_err(|_| export_failed("序列化备份数据失败。"))
            },
            &mut |done, total| {
                progress(BackupProgress {
                    phase: "export",
                    done,
                    total,
                })
            },
            preview.template_bytes,
        )?;
        match rename_no_replace(&temp, &target) {
            Ok(()) => cleanup.disarm(),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(export_failed("目标文件已存在，未覆盖。请换一个文件名。"));
            }
            Err(_) => return Err(export_failed("无法保存备份文件。")),
        }
        Ok(ExportOutcome {
            path: target.to_string_lossy().into_owned(),
            size_bytes: size,
            snippet_count: data.snippets.len() as i64,
            template_count: preview.template_count,
        })
    }

    // ------------------------------------------------------------ 导入：校验

    #[cfg(test)]
    pub fn import_validate(&self, path: &str) -> Result<ImportValidation, AppError> {
        self.import_validate_with(path, &no_progress)
    }

    pub fn import_validate_with(
        &self,
        path: &str,
        progress: Progress<'_>,
    ) -> Result<ImportValidation, AppError> {
        self.discard_all_staged();
        let source = Path::new(path);
        let meta = fs::metadata(source)
            .map_err(|_| AppError::new("IMPORT_FAILED", "无法读取备份文件。"))?;
        if !meta.is_file() {
            return Err(AppError::new("IMPORT_FAILED", "所选路径不是文件。"));
        }
        let is_zip = backup::looks_like_zip(source);
        let (value, index): (Value, Option<ZipIndex>) = if is_zip {
            let index = backup::inspect_zip(source)?;
            (index.backup.clone(), Some(index))
        } else {
            if meta.len() > MAX_JSON_BYTES {
                return Err(backup::too_large("导入文件超过 100 MB。"));
            }
            let bytes = fs::read(source)
                .map_err(|_| AppError::new("IMPORT_FAILED", "无法读取备份文件。"))?;
            let value = serde_json::from_slice(&bytes)
                .map_err(|_| AppError::new("IMPORT_FAILED", "JSON 格式错误。"))?;
            (value, None)
        };
        let token = uuid::Uuid::new_v4().to_string();
        let mut validation = ImportValidation {
            valid: false,
            snippet_count: value
                .get("snippets")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len() as i64),
            settings_present: value.get("settings").is_some(),
            schema_version: value.get("schemaVersion").and_then(Value::as_i64).unwrap_or(0),
            errors: Vec::new(),
            import_token: token.clone(),
            backup_version: 0,
            group_count: 0,
            template_count: 0,
            new_count: 0,
            update_count: 0,
            conflict_count: 0,
            conflict_keys: Vec::new(),
        };
        let parsed = match backup::parse_backup(&value) {
            Ok(parsed) => parsed,
            Err(errors) => {
                validation.errors = errors;
                return Ok(validation);
            }
        };
        if parsed.version == 2 && !is_zip {
            validation.errors = vec!["v2 备份必须是 zip 备份包。".into()];
            return Ok(validation);
        }
        if let Some(index) = &index {
            backup::check_references(&parsed, index)?;
        }
        let existing = self.repository.import_snapshot()?;
        let analysis = analyze(&parsed, &existing);
        validation.backup_version = parsed.version;
        validation.group_count = parsed.groups.len() as i64;
        validation.template_count = parsed.template_count();
        validation.new_count = analysis.new_count;
        validation.update_count = analysis.update_count;
        validation.conflict_count = analysis.conflicts.len() as i64;
        validation.conflict_keys = analysis.conflicts.iter().take(20).cloned().collect();

        let mut staging = None;
        let mut files = HashMap::new();
        if let (Some(index), true) = (&index, parsed.referenced_files().next().is_some()) {
            let fs = self.tfs()?;
            let base = fs
                .root
                .parent()
                .ok_or_else(|| fs_error("模板目录位置无效。"))?;
            let dir = base.join(format!("{STAGING_PREFIX}{token}"));
            fsops::ensure_dir(&dir)?;
            staging = Some(dir.clone());
            let extracted = backup::extract_files(source, index, &parsed, &dir, &mut |done, total| {
                progress(BackupProgress {
                    phase: "extract",
                    done,
                    total,
                })
            });
            match extracted {
                Ok(map) => files = map,
                Err(error) => {
                    let _ = fs::remove_dir_all(&dir);
                    return Err(error);
                }
            }
        }
        lock(&self.backup.staged).insert(
            token,
            Staged {
                backup: parsed,
                staging,
                files,
            },
        );
        validation.valid = true;
        Ok(validation)
    }

    /// 放弃一次尚未提交的导入（用户取消预览）。
    pub fn import_discard(&self, token: &str) {
        if let Some(staged) = lock(&self.backup.staged).remove(token) {
            remove_staging(&staged.staging);
        }
    }

    fn discard_all_staged(&self) {
        let drained: Vec<Staged> = lock(&self.backup.staged).drain().map(|(_, v)| v).collect();
        for staged in drained {
            remove_staging(&staged.staging);
        }
    }

    // ------------------------------------------------------------ 导入：提交

    #[cfg(test)]
    pub fn import_commit(
        &self,
        input: &crate::model::ImportCommitInput,
    ) -> Result<ImportOutcome, AppError> {
        self.import_commit_with(input, &no_progress)
    }

    pub fn import_commit_with(
        &self,
        input: &crate::model::ImportCommitInput,
        progress: Progress<'_>,
    ) -> Result<ImportOutcome, AppError> {
        let token = input.import_token.as_str();
        let staged = lock(&self.backup.staged).remove(token);
        let Some(staged) = staged else {
            if let Some(done) = lock(&self.backup.committed).get(token) {
                return Ok(done.clone());
            }
            return Err(AppError::new(
                "IMPORT_FAILED",
                "导入会话已失效。请重新选择备份文件。",
            ));
        };
        let result = (|| {
            let _importing = ImportingGuard::enter(&self.backup.importing)?;
            progress(BackupProgress {
                phase: "apply",
                done: 0,
                total: 1,
            });
            let fs = self.tfs_opt()?;
            let _guard = fs.map(|fs| fs.guard());
            self.run_import(&staged, token, input.overwrite_conflicts, fs)
        })();
        remove_staging(&staged.staging);
        let outcome = result?;
        lock(&self.backup.committed).insert(token.to_owned(), outcome.clone());
        // 导入期间扫描被锁挡住；完成后补一次，让 inode 等元数据与磁盘收敛。
        let _ = self.template_scan();
        Ok(outcome)
    }

    fn run_import(
        &self,
        staged: &Staged,
        token: &str,
        overwrite: bool,
        fs: Option<&TemplateFs>,
    ) -> Result<ImportOutcome, AppError> {
        let groups = self.repository.group_list()?;
        let existing = self.repository.import_snapshot()?;
        let root = fs.map(|fs| fs.root.as_path());
        let built = build_plan(&staged.backup, &staged.files, &groups, &existing, overwrite, root)?;
        let mut ops = FileOps::default();
        let stage = self.take_fail_stage();
        let applied = self.repository.apply_import(&built.plan, || {
            let Some(fs) = fs else {
                return if built.placements.is_empty() && built.stash.is_empty() {
                    Ok(())
                } else {
                    Err(fs_error("模板目录未启用。"))
                };
            };
            // 暂存目录：覆盖/取代的旧文件先挪到这里，成功后随暂存目录一并删除。
            let base = fs
                .root
                .parent()
                .ok_or_else(|| fs_error("模板目录位置无效。"))?;
            let stash_dir = base.join(format!("{STAGING_PREFIX}{token}")).join("stash");
            ops.apply(&fs.root, &stash_dir, &built, stage)
        });
        match applied {
            Ok(outcome) => Ok(ImportOutcome {
                inserted: outcome.inserted,
                updated: outcome.updated,
                replaced: outcome.replaced,
                skipped: built.skipped,
                groups_added: outcome.groups_added,
            }),
            Err(error) => {
                ops.undo();
                Err(error)
            }
        }
    }

    fn take_fail_stage(&self) -> Option<&'static str> {
        lock(&self.backup.fail_stage).take()
    }

    #[cfg(test)]
    pub(crate) fn inject_import_failure(&self, stage: &'static str) {
        *lock(&self.backup.fail_stage) = Some(stage);
    }

    // ------------------------------------------------------------ 设置视图：路径

    pub fn settings_paths(&self) -> Result<crate::model::SettingsPaths, AppError> {
        let fs = self.tfs()?;
        Ok(crate::model::SettingsPaths {
            templates_dir: fs.root.to_string_lossy().into_owned(),
            default_export_dir: crate::templates::export::resolve_export_dir(None)
                .ok()
                .map(|p| p.to_string_lossy().into_owned()),
        })
    }

    pub fn templates_dir_open(&self) -> Result<(), AppError> {
        let fs = self.tfs()?;
        crate::platform::files::xdg_open(&fs.root)
    }

    // ------------------------------------------------------------ 重置

    pub fn reset_examples(&self) -> Result<i64, AppError> {
        let _importing = ImportingGuard::enter(&self.backup.importing)?;
        let examples = example_snippets();
        let Some(fs) = self.tfs_opt()? else {
            return self.repository.reset_all(&examples);
        };
        let _guard = fs.guard();
        let temp = fs
            .root
            .with_file_name(format!("{RESET_PREFIX}{}", uuid::Uuid::new_v4()));
        let renamed = match fs::rename(&fs.root, &temp) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(fs_error("无法重置模板目录。")),
        };
        let restore = |fs: &TemplateFs| {
            if renamed {
                let _ = fs::remove_dir_all(&fs.root);
                fs::rename(&temp, &fs.root).is_ok()
            } else {
                true
            }
        };
        if let Err(error) = fsops::init_dirs(&fs.root, &[]) {
            if !restore(fs) {
                return Err(fs_error("重置失败，且模板目录未能还原。重启应用将自动恢复。"));
            }
            return Err(error);
        }
        match self.repository.reset_all(&examples) {
            Ok(count) => {
                fs.ready.store(true, Ordering::SeqCst);
                if renamed {
                    // 删除失败不影响结果：遗留目录会在下次启动时清理。
                    let _ = fs::remove_dir_all(&temp);
                }
                Ok(count)
            }
            Err(error) => {
                if !restore(fs) {
                    return Err(fs_error("重置失败，且模板目录未能还原。重启应用将自动恢复。"));
                }
                fs.ready.store(false, Ordering::SeqCst);
                Err(error)
            }
        }
    }
}

// ---------------------------------------------------------------- 辅助

struct TempGuard(PathBuf);

impl TempGuard {
    fn disarm(self) {
        std::mem::forget(self);
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn remove_staging(staging: &Option<PathBuf>) {
    if let Some(dir) = staging {
        let _ = fs::remove_dir_all(dir);
    }
}

fn contains_regular_file(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else { return false };
    entries.flatten().any(|entry| {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_file() => true,
            Ok(kind) if kind.is_dir() => contains_regular_file(&path),
            _ => false,
        }
    })
}

/// 导出目标：绝对路径、父目录存在；缺少 `.zip` 扩展名时补上。
fn export_target(path: &str) -> Result<PathBuf, AppError> {
    let mut target = PathBuf::from(path);
    if !target.is_absolute() || target.file_name().is_none() {
        return Err(export_failed("备份文件路径无效。"));
    }
    if !target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        let mut name = target.file_name().unwrap_or_default().to_os_string();
        name.push(".zip");
        target.set_file_name(name);
    }
    if !target.parent().is_some_and(Path::is_dir) {
        return Err(export_failed("保存位置不存在。"));
    }
    if target.symlink_metadata().is_ok() {
        return Err(export_failed("目标文件已存在，未覆盖。请换一个文件名。"));
    }
    Ok(target)
}

struct Analysis {
    new_count: i64,
    update_count: i64,
    /// 与导入记录规范化 Key 相同但 id 不同的现有条目的 Key。
    conflicts: Vec<String>,
}

fn analyze(parsed: &ParsedBackup, existing: &[ExistingSnippet]) -> Analysis {
    let by_id: HashSet<&str> = existing.iter().map(|e| e.id.as_str()).collect();
    let by_key: HashMap<&str, &ExistingSnippet> =
        existing.iter().map(|e| (e.normalized_key.as_str(), e)).collect();
    let mut analysis = Analysis {
        new_count: 0,
        update_count: 0,
        conflicts: Vec::new(),
    };
    for snippet in &parsed.snippets {
        if by_id.contains(snippet.id.as_str()) {
            analysis.update_count += 1;
        } else {
            analysis.new_count += 1;
        }
        if let Some(found) = by_key.get(snippet.key.as_str()) {
            if found.id != snippet.id && !analysis.conflicts.contains(&found.key) {
                analysis.conflicts.push(found.key.clone());
            }
        }
    }
    analysis
}

// ---------------------------------------------------------------- 导入计划

struct Built {
    plan: ImportPlan,
    /// （暂存文件，目标相对路径）
    placements: Vec<(PathBuf, String)>,
    /// 需要挪走的旧文件（相对路径）。
    stash: Vec<String>,
    /// 需要创建的新分组目录（父级在前）。
    new_dirs: Vec<String>,
    skipped: i64,
}

fn suffixed(name: &str, suffix: u32) -> String {
    let tail = format!(" ({suffix})");
    let keep = 64usize.saturating_sub(tail.chars().count());
    let head: String = name.chars().take(keep).collect();
    format!("{head}{tail}")
}

fn build_plan(
    backup: &ParsedBackup,
    staged_files: &HashMap<String, PathBuf>,
    groups: &[Group],
    existing: &[ExistingSnippet],
    overwrite: bool,
    root: Option<&Path>,
) -> Result<Built, AppError> {
    let mut plan = ImportPlan::default();

    // ---- 分组：按 ID 合并；本地已有的保持原样，新分组挂到（已解析的）父级下，同级重名加 (2)
    struct Node {
        depth: i64,
    }
    let mut known: HashMap<String, Node> = groups
        .iter()
        .map(|g| {
            (
                g.id.clone(),
                Node { depth: g.depth },
            )
        })
        .collect();
    let mut siblings: HashSet<(String, String)> = groups
        .iter()
        .map(|g| (g.parent_id.clone().unwrap_or_default(), g.name.to_lowercase()))
        .collect();
    let mut all_groups: Vec<Group> = groups.to_vec();
    let mut pending: Vec<&backup::BackupGroup> = backup
        .groups
        .iter()
        .filter(|g| !known.contains_key(&g.id))
        .collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut next = Vec::new();
        for group in pending {
            let parent_depth = match &group.parent_id {
                None => Some(0),
                Some(parent) => known.get(parent).map(|n| n.depth),
            };
            let Some(parent_depth) = parent_depth else {
                next.push(group);
                continue;
            };
            let depth = parent_depth + 1;
            if depth > 3 {
                return Err(schema_invalid("分组层级超过 3 层。"));
            }
            let parent_key = group.parent_id.clone().unwrap_or_default();
            let mut name = group.name.clone();
            let mut n = 2;
            while siblings.contains(&(parent_key.clone(), name.to_lowercase())) {
                name = suffixed(&group.name, n);
                n += 1;
            }
            siblings.insert((parent_key, name.to_lowercase()));
            known.insert(
                group.id.clone(),
                Node { depth },
            );
            all_groups.push(Group {
                id: group.id.clone(),
                name: name.clone(),
                parent_id: group.parent_id.clone(),
                depth,
                sort_order: group.sort_order,
                created_at: group.created_at.clone(),
                updated_at: group.updated_at.clone(),
                revision: 1,
                snippet_count: 0,
            });
            plan.groups.push(PlanGroup {
                id: group.id.clone(),
                name,
                parent_id: group.parent_id.clone(),
                depth,
                sort_order: group.sort_order,
                created_at: group.created_at.clone(),
                updated_at: group.updated_at.clone(),
            });
        }
        if next.len() == before {
            return Err(schema_invalid("分组父级无法解析。"));
        }
        pending = next;
    }
    let dirs = group_dirs(&all_groups);
    let new_dirs: Vec<String> = plan
        .groups
        .iter()
        .filter_map(|g| dirs.get(&g.id).cloned())
        .collect();

    // ---- Key 冲突
    let existing_by_id: HashMap<&str, &ExistingSnippet> =
        existing.iter().map(|e| (e.id.as_str(), e)).collect();
    let existing_by_key: HashMap<&str, &ExistingSnippet> =
        existing.iter().map(|e| (e.normalized_key.as_str(), e)).collect();
    let mut deleting: Vec<&ExistingSnippet> = Vec::new();
    for snippet in &backup.snippets {
        if let Some(found) = existing_by_key.get(snippet.key.as_str()) {
            if found.id != snippet.id && !deleting.iter().any(|d| d.id == found.id) {
                deleting.push(found);
            }
        }
    }
    if !deleting.is_empty() && !overwrite {
        return Err(AppError::new(
            "IMPORT_CONFLICT",
            format!("有 {} 条 Key 与现有条目冲突。请选择覆盖或取消导入。", deleting.len()),
        ));
    }
    plan.delete_ids = deleting.iter().map(|d| d.id.clone()).collect();
    let deleting_ids: HashSet<&str> = deleting.iter().map(|d| d.id.as_str()).collect();

    // 将被替换或删除的现有文件模板：它们的旧路径可被新文件复用，旧文件先挪走。
    let mut replaced_rels: HashSet<String> = HashSet::new();
    for e in existing {
        let replaced = deleting_ids.contains(e.id.as_str())
            || backup.snippets.iter().any(|s| s.id == e.id && s.file.as_ref().is_some_and(|f| !f.missing));
        if replaced {
            if let Some(rel) = &e.rel_path {
                replaced_rels.insert(rel.clone());
            }
        }
    }
    let mut db_paths: HashSet<String> = existing
        .iter()
        .filter(|e| !deleting_ids.contains(e.id.as_str()))
        .filter_map(|e| e.rel_path.clone())
        .filter(|rel| !replaced_rels.contains(rel))
        .collect();

    let mut placements = Vec::new();
    let mut skipped = 0;
    for item in &backup.snippets {
        let same = existing_by_id
            .get(item.id.as_str())
            .filter(|e| !deleting_ids.contains(e.id.as_str()));
        if let Some(found) = same {
            if found.kind != item.kind {
                return Err(schema_invalid("条目类型与本地同 ID 条目不一致。"));
            }
        }
        let update = same.is_some();
        let mut plan_file = None;
        if let Some(file) = &item.file {
            if file.missing && update {
                skipped += 1;
                continue;
            }
            let dir = if item.deleted_at.is_some() {
                TRASH_DIR.to_owned()
            } else {
                match &item.group_id {
                    None => String::new(),
                    Some(group) => dirs
                        .get(group)
                        .cloned()
                        .ok_or_else(|| schema_invalid("条目的分组无法解析。"))?,
                }
            };
            let base = if item.deleted_at.is_some() {
                file_name(&item.id, &file.ext)
            } else if file.rel_path.starts_with(&format!("{TRASH_DIR}/")) {
                file_name(&sanitize_stem(&item.title), &file.ext)
            } else {
                base_name(&file.rel_path).to_owned()
            };
            let (stem, _) = split_name(&base);
            let occupied = |name: &str| {
                let rel = join_rel(&dir, name);
                db_paths.contains(&rel)
                    || placements.iter().any(|(_, placed): &(PathBuf, String)| *placed == rel)
                    || plan_paths_contains(&plan, &rel)
                    || (!replaced_rels.contains(&rel)
                        && root.is_some_and(|root| root.join(&rel).symlink_metadata().is_ok()))
            };
            let stem = if base == file_name(&stem, &file.ext) {
                stem
            } else {
                // 文件名与扩展名字段不一致（如无扩展名）：整体当作主干。
                base.clone()
            };
            let chosen = dedupe_stem(&stem, &file.ext, occupied);
            let rel = join_rel(&dir, &file_name(&chosen, &file.ext));
            db_paths.insert(rel.clone());
            let mut inode = None;
            let mut mtime = file.modified_at.clone();
            if !file.missing {
                let staged = staged_files
                    .get(&file.rel_path)
                    .ok_or_else(|| backup::archive_invalid("缺少已校验的模板文件。"))?;
                set_mtime(staged, &file.modified_at);
                if let Ok(stat) = fsops::stat_regular(staged) {
                    inode = Some(stat.inode);
                    mtime = stat.mtime;
                }
                placements.push((staged.clone(), rel.clone()));
            }
            plan_file = Some(PlanFile {
                rel_path: rel,
                ext: file.ext.clone(),
                size: file.size as i64,
                inode,
                sha256: file.sha256.clone(),
                mtime,
                original_group_id: file.original_group_id.clone(),
                auto_imported: file.auto_imported,
                missing: file.missing,
            });
        }
        plan.snippets.push(PlanSnippet {
            item: item.clone(),
            group_id: item.group_id.clone(),
            file: plan_file,
            update,
        });
    }
    let stash: Vec<String> = {
        let mut list: Vec<String> = replaced_rels.into_iter().collect();
        list.sort();
        list
    };
    Ok(Built {
        plan,
        placements,
        stash,
        new_dirs,
        skipped,
    })
}

fn plan_paths_contains(plan: &ImportPlan, rel: &str) -> bool {
    plan.snippets
        .iter()
        .any(|s| s.file.as_ref().is_some_and(|f| f.rel_path == rel))
}

fn set_mtime(path: &Path, rfc3339: &str) {
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(rfc3339) else { return };
    let time: std::time::SystemTime = parsed.with_timezone(&chrono::Utc).into();
    if let Ok(file) = fs::OpenOptions::new().write(true).open(path) {
        let _ = file.set_modified(time);
    }
}

// ---------------------------------------------------------------- 文件操作与撤销

#[derive(Default)]
struct FileOps {
    created_dirs: Vec<PathBuf>,
    stashed: Vec<(PathBuf, PathBuf)>,
    placed: Vec<PathBuf>,
}

impl FileOps {
    fn apply(
        &mut self,
        root: &Path,
        stash_dir: &Path,
        built: &Built,
        fail_stage: Option<&'static str>,
    ) -> Result<(), AppError> {
        let fail_at = |stage: &str| -> Result<(), AppError> {
            if fail_stage == Some(stage) {
                Err(fs_error("injected import failure"))
            } else {
                Ok(())
            }
        };
        for dir in &built.new_dirs {
            let path = safe_join(root, dir)?;
            if path.symlink_metadata().is_err() {
                fsops::ensure_dir_in(root, &path)?;
                self.created_dirs.push(path);
            }
        }
        if !built.stash.is_empty() {
            fsops::ensure_dir(stash_dir)?;
        }
        for (n, rel) in built.stash.iter().enumerate() {
            let src = safe_join(root, rel)?;
            if src.symlink_metadata().is_err() {
                continue;
            }
            let dst = stash_dir.join(format!("s{n}"));
            fsops::move_file(&src, &dst)?;
            self.stashed.push((src, dst));
            fail_at("stash")?;
        }
        for (n, (staged, rel)) in built.placements.iter().enumerate() {
            let dst = safe_join(root, rel)?;
            if let Some(parent) = dst.parent() {
                if parent.symlink_metadata().is_err() {
                    fsops::ensure_dir_in(root, parent)?;
                    self.created_dirs.push(parent.to_path_buf());
                }
            }
            fsops::move_file(staged, &dst)?;
            self.placed.push(dst);
            if n == 0 {
                fail_at("place")?;
            }
        }
        fail_at("after_place")
    }

    /// 撤销已做的文件操作：删掉新放入的文件，把旧文件挪回原位，移除本次创建的空目录。
    fn undo(&mut self) {
        for path in self.placed.drain(..).rev() {
            let _ = fs::remove_file(path);
        }
        for (src, stashed) in self.stashed.drain(..).rev() {
            let _ = fsops::move_file(&stashed, &src);
        }
        for dir in self.created_dirs.drain(..).rev() {
            let _ = fs::remove_dir(dir);
        }
    }
}

// ---------------------------------------------------------------- 示例数据

fn example_snippets() -> Vec<BackupSnippet> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let make = |id: &str, key: &str, title: &str, content: &str, aliases: &[&str], tags: &[&str], pinned: bool, sensitive: bool| BackupSnippet {
        id: id.into(),
        key: key.into(),
        title: title.into(),
        content: content.into(),
        aliases: aliases.iter().map(|s| (*s).to_owned()).collect(),
        tags: tags.iter().map(|s| (*s).to_owned()).collect(),
        pinned,
        sensitive,
        usage_count: 0,
        last_used_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        deleted_at: None,
        group_id: None,
        language: "plain".into(),
        kind: "text".into(),
        file: None,
    };
    vec![
        make(
            "ex-1",
            "hello-searchis",
            "Searchis 问候",
            "你好！很高兴认识你。这是我通过 Searchis 快速粘贴的第一条文本片段。",
            &[],
            &["示例"],
            true,
            false,
        ),
        make("ex-2", "email-work", "工作邮箱", "example@work.com", &["工作邮箱"], &["工作"], false, true),
        make("ex-3", "addr-home", "家庭地址", "XX市XX区XX街道", &[], &["地址", "生活"], false, true),
    ]
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::{
        model::{
            CreateSnippetInput, GroupCreateInput, ImportCommitInput, TemplateCreateBlankInput,
        },
        storage::Repository,
    };
    use zip::{write::SimpleFileOptions, ZipWriter};

    struct Env {
        dir: tempfile::TempDir,
        service: SnippetService,
        root: PathBuf,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let root = dir.path().join("templates");
        let service = SnippetService::new(repo).with_templates(root.clone());
        Env { dir, service, root }
    }

    impl Env {
        fn group(&self, name: &str, parent: Option<&str>) -> Group {
            self.service
                .group_create(GroupCreateInput {
                    name: name.into(),
                    parent_id: parent.map(String::from),
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap()
        }

        fn text(&self, key: &str, content: &str, group: Option<&str>, language: &str) -> crate::model::Snippet {
            self.service
                .create(CreateSnippetInput {
                    key: key.into(),
                    title: format!("标题 {key}"),
                    content: content.into(),
                    aliases: vec!["别名".into()],
                    tags: vec!["标签".into()],
                    sensitive: false,
                    pinned: true,
                    group_id: group.map(String::from),
                    language: Some(language.into()),
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap()
        }

        fn file(&self, kind: &str, title: &str, key: &str, group: Option<&str>, body: &[u8]) -> crate::model::Snippet {
            let snippet = self
                .service
                .template_create_blank(TemplateCreateBlankInput {
                    kind: kind.into(),
                    title: title.into(),
                    key: key.into(),
                    group_id: group.map(String::from),
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap();
            let row = self.service.repository.file_row(&snippet.id).unwrap().unwrap();
            fs::write(self.root.join(&row.rel_path), body).unwrap();
            snippet
        }

        fn export(&self, name: &str) -> PathBuf {
            let target = self.dir.path().join(name);
            self.service.export_confirm(target.to_str().unwrap()).unwrap();
            target
        }

        fn validate(&self, path: &Path) -> Result<ImportValidation, AppError> {
            self.service.import_validate(path.to_str().unwrap())
        }

        fn commit(&self, token: &str, overwrite: bool) -> Result<ImportOutcome, AppError> {
            self.service.import_commit(&ImportCommitInput {
                path: String::new(),
                import_token: token.into(),
                overwrite_conflicts: overwrite,
            })
        }

        fn import(&self, path: &Path) -> ImportOutcome {
            let validation = self.validate(path).unwrap();
            assert!(validation.valid, "{:?}", validation.errors);
            self.commit(&validation.import_token, false).unwrap()
        }

        /// 库的可比较快照：分组、条目元数据与文件哈希。
        fn dump(&self) -> Vec<String> {
            let mut lines: Vec<String> = self
                .service
                .group_list()
                .unwrap()
                .iter()
                .map(|g| format!("G|{}|{}|{:?}", g.id, g.name, g.parent_id))
                .collect();
            for s in self.service.list().unwrap() {
                let file = self.service.repository.file_row(&s.id).unwrap();
                lines.push(format!(
                    "S|{}|{}|{}|{}|{:?}|{}|{:?}|{}|{:?}|{:?}|{}",
                    s.id,
                    s.key,
                    s.title,
                    s.kind,
                    s.group_id,
                    s.language,
                    s.deleted_at.is_some(),
                    s.content,
                    s.tags,
                    s.aliases,
                    s.pinned,
                ));
                if let Some(f) = file {
                    lines.push(format!("F|{}|{}|{}|{}", f.id, f.rel_path, f.sha256, f.missing));
                }
            }
            lines.sort();
            lines
        }

        fn leftovers(&self) -> Vec<String> {
            fs::read_dir(self.dir.path())
                .unwrap()
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.starts_with(STAGING_PREFIX) || n.starts_with(RESET_PREFIX) || n.ends_with(".part") || n.starts_with('.'))
                .collect()
        }
    }

    fn sha(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(bytes))
    }

    fn make_zip(path: &Path, entries: &[(&str, Vec<u8>)]) {
        let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
        for (name, bytes) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    fn file_backup(body: &[u8], sha_override: Option<String>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 2,
            "groups": [{"id": "g1", "name": "运维"}],
            "snippets": [{
                "id": "f1", "key": "doc-1", "title": "文档", "kind": "file", "groupId": "g1",
                "file": {"relativePath": "运维/文档.md", "extension": "md", "sizeBytes": body.len(),
                         "contentSha256": sha_override.unwrap_or_else(|| sha(body)),
                         "modifiedAt": "2026-03-04T05:06:07.000Z"}
            }]
        }))
        .unwrap()
    }

    // ------------------------------------------------------------ AC-36 往返

    #[test]
    fn zip_roundtrip_after_reset_restores_everything_with_identical_hashes() {
        let env = env();
        let ops = env.group("运维", None);
        let db = env.group("数据库", Some(&ops.id));
        env.text("slow-query", "select 1;", Some(&db.id), "sql");
        env.text("loose", "no group", None, "plain");
        let md = env.file("md", "慢查询", "slow-md", Some(&db.id), "# 慢查询\n内容".as_bytes());
        let xlsx = env.file("xlsx", "报表", "report-xlsx", None, b"PK-not-really-xlsx-bytes");
        let trashed = env.file("md", "旧笔记", "old-note", Some(&ops.id), b"old note body");
        // 数据库哈希先与磁盘一致（回收站中的行不参与扫描），便于逐项比较。
        env.service.template_scan().unwrap();
        env.service.trash_move(&trashed.id).unwrap();
        let before = env.dump();
        let archive = env.export("backup.zip");
        let bytes = fs::read(env.root.join(env.service.repository.file_row(&md.id).unwrap().unwrap().rel_path)).unwrap();
        assert_eq!(bytes, "# 慢查询\n内容".as_bytes());
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());

        // 重置：条目、分组、模板文件全部清掉
        env.service.reset_examples().unwrap();
        assert!(env.service.group_list().unwrap().is_empty());
        assert!(!contains_regular_file(&env.root));
        assert_eq!(env.service.list().unwrap().len(), 3);

        let outcome = env.import(&archive);
        assert_eq!(outcome.groups_added, 2);
        assert_eq!(outcome.inserted, 5);
        env.service.template_scan().unwrap();
        let after: Vec<String> = env
            .dump()
            .into_iter()
            .filter(|l| !l.contains("hello-searchis") && !l.contains("email-work") && !l.contains("addr-home"))
            .collect();
        assert_eq!(after, before);
        for id in [&md.id, &xlsx.id, &trashed.id] {
            let row = env.service.repository.file_row(id).unwrap().unwrap();
            let on_disk = fs::read(env.root.join(&row.rel_path)).unwrap();
            assert_eq!(sha(&on_disk), row.sha256);
        }
        let trashed_row = env.service.repository.file_row(&trashed.id).unwrap().unwrap();
        assert!(trashed_row.rel_path.starts_with(".trash/"));
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
    }

    #[test]
    fn export_uses_actual_file_bytes_and_marks_vanished_files_missing() {
        let env = env();
        let kept = env.file("md", "保留", "kept", None, b"v1");
        let gone = env.file("md", "消失", "gone", None, b"x");
        // 外部改了内容（数据库哈希已过期），另一个文件被删但尚未扫描
        let kept_row = env.service.repository.file_row(&kept.id).unwrap().unwrap();
        fs::write(env.root.join(&kept_row.rel_path), b"edited externally").unwrap();
        let gone_row = env.service.repository.file_row(&gone.id).unwrap().unwrap();
        fs::remove_file(env.root.join(&gone_row.rel_path)).unwrap();
        let archive = env.export("b.zip");
        let index = backup::inspect_zip(&archive).unwrap();
        let parsed = backup::parse_backup(&index.backup).unwrap();
        let kept_file = parsed.snippets.iter().find(|s| s.id == kept.id).unwrap().file.clone().unwrap();
        assert_eq!(kept_file.sha256, sha(b"edited externally"));
        assert!(!kept_file.missing);
        let gone_file = parsed.snippets.iter().find(|s| s.id == gone.id).unwrap().file.clone().unwrap();
        assert!(gone_file.missing);
        backup::check_references(&parsed, &index).unwrap();
    }

    #[test]
    fn export_never_overwrites_appends_zip_and_leaves_no_temp_file() {
        let env = env();
        env.text("a", "x", None, "plain");
        let existing = env.dir.path().join("exists.zip");
        fs::write(&existing, b"keep me").unwrap();
        let error = env.service.export_confirm(existing.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "EXPORT_FAILED");
        assert_eq!(fs::read(&existing).unwrap(), b"keep me");
        let outcome = env
            .service
            .export_confirm(env.dir.path().join("plain-name").to_str().unwrap())
            .unwrap();
        assert!(outcome.path.ends_with("plain-name.zip"));
        assert!(Path::new(&outcome.path).exists());
        assert!(env
            .service
            .export_confirm("relative/x.zip")
            .is_err());
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
    }

    #[test]
    fn export_preview_reports_counts_and_sensitive_snippets() {
        let env = env();
        env.text("a", "x", None, "plain");
        env.file("md", "n", "n1", None, b"12345");
        let preview = env.service.export_preview("").unwrap();
        assert_eq!(preview.snippet_count, 2);
        assert_eq!(preview.template_count, 1);
        assert_eq!(preview.template_bytes, env.service.repository.backup_file_rows().unwrap()[0].size as u64);
        assert!(!preview.exceeds_limit);
    }

    // ------------------------------------------------------------ 拒绝

    #[test]
    fn malicious_archives_are_rejected_before_anything_is_written() {
        let env = env();
        for name in ["../evil.sh", "/tmp/evil.sh", "templates/../../evil.sh", "templates/a\\..\\evil"] {
            let archive = env.dir.path().join("evil.zip");
            make_zip(&archive, &[(BACKUP_JSON_NAME, br#"{"schemaVersion":2,"groups":[],"snippets":[]}"#.to_vec()), (name, b"boom".to_vec())]);
            let before = env.dump();
            let error = env.validate(&archive).unwrap_err();
            assert_eq!(error.code, "BACKUP_ARCHIVE_INVALID", "{name}");
            assert!(!error.message.contains(archive.to_str().unwrap()));
            assert_eq!(env.dump(), before);
            assert!(!env.dir.path().parent().unwrap().join("evil.sh").exists());
            assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
        }
    }

    const BACKUP_JSON_NAME: &str = "backup.json";

    #[test]
    fn unreferenced_and_hash_mismatched_files_are_rejected_and_staging_is_cleaned() {
        let env = env();
        let body = b"hello template";
        let archive = env.dir.path().join("x.zip");
        make_zip(&archive, &[(BACKUP_JSON_NAME, file_backup(body, None)), ("templates/运维/文档.md", body.to_vec()), ("templates/extra.md", b"?".to_vec())]);
        assert_eq!(env.validate(&archive).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
        make_zip(&archive, &[(BACKUP_JSON_NAME, file_backup(body, Some(sha(b"different")))), ("templates/运维/文档.md", body.to_vec())]);
        let error = env.validate(&archive).unwrap_err();
        assert_eq!(error.code, "BACKUP_ARCHIVE_INVALID");
        assert!(!error.message.contains("文档"));
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
        assert!(env.service.group_list().unwrap().is_empty());
    }

    #[test]
    fn schema_errors_are_reported_without_staging() {
        let env = env();
        let archive = env.dir.path().join("bad.zip");
        make_zip(&archive, &[(BACKUP_JSON_NAME, br#"{"schemaVersion":2,"groups":[{"id":"a","name":"A","parentId":"a"}],"snippets":[]}"#.to_vec())]);
        let validation = env.validate(&archive).unwrap();
        assert!(!validation.valid);
        assert!(!validation.errors.is_empty());
        assert!(env.leftovers().is_empty());
        assert_eq!(env.commit(&validation.import_token, false).unwrap_err().code, "IMPORT_FAILED");
    }

    // ------------------------------------------------------------ v1 与合并

    #[test]
    fn v1_json_imports_as_ungrouped_plain_text_and_commit_is_idempotent() {
        let env = env();
        let path = env.dir.path().join("old.json");
        fs::write(
            &path,
            serde_json::json!({"schemaVersion":1,"exportedAt":"x","appVersion":"0","snippets":[
                {"id":"a1","key":"Old Key","title":"旧","content":"正文","aliases":[],"tags":["t"],"pinned":true,"sensitive":false}
            ],"settings":{}})
            .to_string(),
        )
        .unwrap();
        let validation = env.validate(&path).unwrap();
        assert!(validation.valid && validation.backup_version == 1);
        assert_eq!((validation.new_count, validation.update_count), (1, 0));
        let first = env.commit(&validation.import_token, false).unwrap();
        assert_eq!(first.inserted, 1);
        let snippet = env.service.get("a1").unwrap();
        assert_eq!((snippet.kind.as_str(), snippet.language.as_str(), snippet.key.as_str()), ("text", "plain", "old-key"));
        assert_eq!(snippet.group_id, None);
        let before = env.dump();
        let again = env.commit(&validation.import_token, false).unwrap();
        assert_eq!((again.inserted, again.updated), (1, 0));
        assert_eq!(env.dump(), before);
    }

    #[test]
    fn key_conflicts_need_explicit_overwrite_and_replace_the_old_entry_and_its_file() {
        let env = env();
        let local = env.file("md", "本地", "shared-key", None, b"local body");
        let local_row = env.service.repository.file_row(&local.id).unwrap().unwrap();
        let path = env.dir.path().join("c.json");
        fs::write(&path, serde_json::json!({"schemaVersion":1,"snippets":[
            {"id":"other-id","key":"shared-key","title":"导入","content":"imported"}]}).to_string()).unwrap();
        let validation = env.validate(&path).unwrap();
        assert_eq!(validation.conflict_count, 1);
        assert_eq!(validation.conflict_keys, vec!["shared-key".to_string()]);
        let before = env.dump();
        let error = env.commit(&validation.import_token, false).unwrap_err();
        assert_eq!(error.code, "IMPORT_CONFLICT");
        assert_eq!(env.dump(), before);
        assert!(env.root.join(&local_row.rel_path).exists());
        // 取消后重新校验再覆盖
        let validation = env.validate(&path).unwrap();
        let outcome = env.commit(&validation.import_token, true).unwrap();
        assert_eq!((outcome.inserted, outcome.replaced), (1, 1));
        assert!(env.service.get(&local.id).is_err());
        assert_eq!(env.service.get("other-id").unwrap().content, "imported");
        assert!(!env.root.join(&local_row.rel_path).exists());
    }

    #[test]
    fn groups_merge_by_id_and_rename_same_level_duplicates() {
        let env = env();
        let local = env.group("运维", None);
        let archive = env.dir.path().join("g.zip");
        let backup = serde_json::json!({"schemaVersion":2,"groups":[
            {"id": local.id, "name": "运维"},
            {"id": "new-1", "name": "运维 "},
            {"id": "new-2", "name": "子级", "parentId": "new-1"}],
            "snippets":[]});
        make_zip(&archive, &[(BACKUP_JSON_NAME, serde_json::to_vec(&backup).unwrap())]);
        // 备份内同级同名（"运维" 与 "运维 " 去空白后相同）本身非法
        assert!(!env.validate(&archive).unwrap().valid);
        let backup = serde_json::json!({"schemaVersion":2,"groups":[
            {"id": local.id, "name": "运维"},
            {"id": "new-1", "name": "新根"},
            {"id": "new-2", "name": "运维", "parentId": "new-1"}],
            "snippets":[]});
        make_zip(&archive, &[(BACKUP_JSON_NAME, serde_json::to_vec(&backup).unwrap())]);
        let outcome = env.import(&archive);
        assert_eq!(outcome.groups_added, 2);
        assert_eq!(env.service.group_list().unwrap().len(), 3);
        assert!(env.root.join("新根/运维").is_dir());
    }

    #[test]
    fn imported_file_gets_numeric_suffix_when_name_is_taken_and_keeps_local_files() {
        let env = env();
        let body = b"imported body";
        fs::create_dir_all(env.root.join("运维")).unwrap();
        fs::write(env.root.join("运维/文档.md"), b"unrelated stray file").unwrap();
        let archive = env.dir.path().join("f.zip");
        make_zip(&archive, &[(BACKUP_JSON_NAME, file_backup(body, None)), ("templates/运维/文档.md", body.to_vec())]);
        env.import(&archive);
        assert_eq!(fs::read(env.root.join("运维/文档.md")).unwrap(), b"unrelated stray file");
        let row = env.service.repository.file_row("f1").unwrap().unwrap();
        assert_eq!(row.rel_path, "运维/文档 (2).md");
        assert_eq!(fs::read(env.root.join(&row.rel_path)).unwrap(), body);
        assert_eq!(row.sha256, sha(body));
        assert_eq!(row.mtime, "2026-03-04T05:06:07.000Z");
    }

    // ------------------------------------------------------------ 失败回滚

    #[test]
    fn injected_failures_at_every_stage_leave_database_and_template_dir_unchanged() {
        for stage in ["stash", "place", "after_place"] {
            let env = env();
            // 本地已有同 id 文件模板（会被备份里的版本覆盖，因此会经历“挪走旧文件”阶段）
            let ops = env.group("运维", None);
            let existing = env.file("md", "文档", "doc-1", Some(&ops.id), b"local version");
            let archive = env.dir.path().join("src.zip");
            let body = b"backup version";
            let index_json = serde_json::to_vec(&serde_json::json!({
                "schemaVersion": 2,
                "groups": [{"id": ops.id, "name": "运维"}, {"id": "new-g", "name": "新分组"}],
                "snippets": [
                    {"id": existing.id, "key": "doc-1", "title": "文档", "kind": "file", "groupId": "new-g",
                     "file": {"relativePath": "新分组/文档.md", "extension": "md", "sizeBytes": body.len(),
                              "contentSha256": sha(body), "modifiedAt": "2026-01-01T00:00:00.000Z"}},
                    {"id": "extra", "key": "extra-key", "title": "另一个", "kind": "file",
                     "file": {"relativePath": "另一个.md", "extension": "md", "sizeBytes": body.len(),
                              "contentSha256": sha(body), "modifiedAt": "2026-01-01T00:00:00.000Z"}}]
            }))
            .unwrap();
            make_zip(&archive, &[(BACKUP_JSON_NAME, index_json), ("templates/新分组/文档.md", body.to_vec()), ("templates/另一个.md", body.to_vec())]);
            let db_before = env.dump();
            let tree_before = tree(&env.root);
            let validation = env.validate(&archive).unwrap();
            assert!(validation.valid);
            env.service.inject_import_failure(stage);
            let error = env.commit(&validation.import_token, false).unwrap_err();
            assert_eq!(error.code, "TEMPLATE_FS_FAILED", "{stage}");
            assert_eq!(env.dump(), db_before, "{stage}");
            assert_eq!(tree(&env.root), tree_before, "{stage}");
            assert_eq!(fs::read(env.root.join("运维/文档.md")).unwrap(), b"local version", "{stage}");
            assert!(env.leftovers().is_empty(), "{stage}: {:?}", env.leftovers());
            // 失败后可重新校验并成功导入
            let validation = env.validate(&archive).unwrap();
            env.commit(&validation.import_token, false).unwrap();
            let row = env.service.repository.file_row(&existing.id).unwrap().unwrap();
            assert_eq!(row.rel_path, "新分组/文档.md");
            assert_eq!(fs::read(env.root.join("新分组/文档.md")).unwrap(), body);
            assert!(!env.root.join("运维/文档.md").exists());
            assert!(env.leftovers().is_empty(), "{stage}: {:?}", env.leftovers());
        }
    }

    #[test]
    fn commit_failure_after_files_are_moved_restores_database_and_template_dir() {
        let env = env();
        let ops = env.group("运维", None);
        let existing = env.file("md", "文档", "doc-1", Some(&ops.id), b"local version");
        let archive = env.dir.path().join("src.zip");
        let body = b"backup version";
        let index_json = serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 2,
            "groups": [{"id": ops.id, "name": "运维"}, {"id": "new-g", "name": "新分组"}],
            "snippets": [
                {"id": existing.id, "key": "doc-1", "title": "文档", "kind": "file", "groupId": "new-g",
                 "file": {"relativePath": "新分组/文档.md", "extension": "md", "sizeBytes": body.len(),
                          "contentSha256": sha(body), "modifiedAt": "2026-01-01T00:00:00.000Z"}},
                {"id": "extra", "key": "extra-key", "title": "另一个", "kind": "file",
                 "file": {"relativePath": "另一个.md", "extension": "md", "sizeBytes": body.len(),
                          "contentSha256": sha(body), "modifiedAt": "2026-01-01T00:00:00.000Z"}}]
        }))
        .unwrap();
        make_zip(&archive, &[(BACKUP_JSON_NAME, index_json), ("templates/新分组/文档.md", body.to_vec()), ("templates/另一个.md", body.to_vec())]);
        let db_before = env.dump();
        let tree_before = tree(&env.root);
        let validation = env.validate(&archive).unwrap();
        assert!(validation.valid);
        env.service.repository.inject_import_commit_failure(true);
        let error = env.commit(&validation.import_token, false).unwrap_err();
        assert_eq!(error.code, "DB_WRITE_FAILED");
        env.service.repository.inject_import_commit_failure(false);
        assert_eq!(env.dump(), db_before);
        assert_eq!(tree(&env.root), tree_before);
        assert_eq!(fs::read(env.root.join("运维/文档.md")).unwrap(), b"local version");
        assert!(!env.root.join("新分组").exists());
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
        // 失败后可重新校验并成功导入
        let validation = env.validate(&archive).unwrap();
        env.commit(&validation.import_token, false).unwrap();
        let row = env.service.repository.file_row(&existing.id).unwrap().unwrap();
        assert_eq!(row.rel_path, "新分组/文档.md");
        assert_eq!(fs::read(env.root.join("新分组/文档.md")).unwrap(), body);
        assert!(!env.root.join("运维/文档.md").exists());
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
    }

    #[test]
    fn database_failure_inside_the_import_transaction_rolls_back_without_touching_files() {
        let env = env();
        let body = b"payload";
        let archive = env.dir.path().join("src.zip");
        make_zip(&archive, &[(BACKUP_JSON_NAME, file_backup(body, None)), ("templates/运维/文档.md", body.to_vec())]);
        let before = env.dump();
        let tree_before = tree(&env.root);
        let validation = env.validate(&archive).unwrap();
        env.service.repository.inject_file_write_failure(true);
        assert!(env.commit(&validation.import_token, false).is_err());
        env.service.repository.inject_file_write_failure(false);
        assert_eq!(env.dump(), before);
        assert_eq!(tree(&env.root), tree_before);
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
    }

    fn tree(root: &Path) -> Vec<String> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                let rel = path.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                if path.is_dir() {
                    out.push(format!("{rel}/"));
                    walk(base, &path, out);
                } else {
                    out.push(format!("{rel}:{}", sha(&fs::read(&path).unwrap())));
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    // ------------------------------------------------------------ 重置

    #[test]
    fn reset_wipes_templates_and_restores_directory_when_the_database_transaction_fails() {
        let env = env();
        let group = env.group("运维", None);
        env.file("md", "文档", "doc", Some(&group.id), b"precious");
        env.text("t", "text", None, "plain");
        let before = env.dump();
        let tree_before = tree(&env.root);

        env.service.repository.inject_file_write_failure(true);
        assert!(env.service.reset_examples().is_err());
        env.service.repository.inject_file_write_failure(false);
        assert_eq!(env.dump(), before);
        assert_eq!(tree(&env.root), tree_before);
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());

        assert_eq!(env.service.reset_examples().unwrap(), 3);
        assert!(env.service.group_list().unwrap().is_empty());
        assert!(!contains_regular_file(&env.root));
        assert!(env.root.join(".trash").is_dir());
        assert!(env.leftovers().is_empty(), "{:?}", env.leftovers());
        assert_eq!(env.service.list().unwrap().len(), 3);
        // 重置后目录仍可正常使用
        env.file("md", "新", "new", None, b"x");
    }

    #[test]
    fn startup_cleans_staging_dirs_and_restores_an_interrupted_reset() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("templates");
        {
            let repo = Repository::open(dir.path()).unwrap();
            let service = SnippetService::new(repo).with_templates(root.clone());
            service
                .template_create_blank(TemplateCreateBlankInput {
                    kind: "md".into(),
                    title: "保命".into(),
                    key: "keep".into(),
                    group_id: None,
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap();
        }
        // 模拟：重置改名后崩溃（数据库未提交，新目录为空），外加遗留的导入暂存目录
        let leftover = dir.path().join(format!("{RESET_PREFIX}dead"));
        fs::rename(&root, &leftover).unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(dir.path().join(format!("{STAGING_PREFIX}dead/stash"))).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let service = SnippetService::new(repo).with_templates(root.clone());
        assert!(!leftover.exists());
        assert!(!dir.path().join(format!("{STAGING_PREFIX}dead")).exists());
        let row = service.repository.backup_file_rows().unwrap().remove(0);
        assert!(root.join(&row.rel_path).is_file());
    }

    #[test]
    fn startup_discards_reset_leftovers_once_the_reset_was_committed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("templates");
        let leftover = dir.path().join(format!("{RESET_PREFIX}done"));
        fs::create_dir_all(leftover.join("a")).unwrap();
        fs::write(leftover.join("a/old.md"), b"x").unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let _service = SnippetService::new(repo).with_templates(root);
        assert!(!leftover.exists());
    }

    // ------------------------------------------------------------ 设置

    #[test]
    fn export_dir_setting_is_validated_and_cleared_with_null() {
        let env = env();
        let update = |value: serde_json::Value| {
            let (_, revision) = env.service.repository.get_settings().unwrap();
            env.service.update_settings("exportDir", &value, revision)
        };
        let good = env.dir.path().join("exports");
        fs::create_dir_all(&good).unwrap();
        assert_eq!(update(serde_json::json!(good)).unwrap().export_dir, Some(good.to_string_lossy().into_owned()));
        let file = env.dir.path().join("a-file");
        fs::write(&file, b"x").unwrap();
        for bad in [
            serde_json::json!(env.dir.path().join("missing")),
            serde_json::json!(file),
            serde_json::json!("relative/dir"),
            serde_json::json!(5),
        ] {
            let error = update(bad).unwrap_err();
            assert_eq!(error.code, "VALIDATION_FAILED");
        }
        assert_eq!(env.service.get_settings(false).unwrap().settings.export_dir, Some(good.to_string_lossy().into_owned()));
        if unsafe { libc::geteuid() } != 0 {
            let locked = env.dir.path().join("locked");
            fs::create_dir_all(&locked).unwrap();
            fs::set_permissions(&locked, std::os::unix::fs::PermissionsExt::from_mode(0o500)).unwrap();
            assert_eq!(update(serde_json::json!(locked)).unwrap_err().code, "VALIDATION_FAILED");
            fs::set_permissions(&locked, std::os::unix::fs::PermissionsExt::from_mode(0o700)).unwrap();
        }
        assert_eq!(update(serde_json::Value::Null).unwrap().export_dir, None);
    }

    #[test]
    fn backup_contains_no_machine_specific_settings() {
        let env = env();
        env.text("a", "x", None, "plain");
        let archive = env.export("s.zip");
        let index = backup::inspect_zip(&archive).unwrap();
        let settings = index.backup.get("settings").unwrap().as_object().unwrap();
        for key in ["exportDir", "openWithByExtension", "onboardingCompletedAt"] {
            assert!(!settings.contains_key(key), "{key}");
        }
        assert_eq!(index.backup["schemaVersion"], 2);
    }

    /// NFR：500 MB 模板总量的导出/导入耗时（不设硬指标，只记录）。
    /// 运行：`cargo test --lib qa_perf_500mb -- --ignored --nocapture`
    #[test]
    #[ignore = "writes 500 MB of data; run manually to record timings"]
    fn qa_perf_500mb_export_and_import() {
        use std::time::Instant;
        let env = env();
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut chunk = vec![0u8; 1024 * 1024];
        for n in 0..25 {
            let snippet = env.file("md", &format!("大文件 {n}"), &format!("big-{n}"), None, b"");
            let row = env.service.repository.file_row(&snippet.id).unwrap().unwrap();
            let mut out = fs::File::create(env.root.join(&row.rel_path)).unwrap();
            for _ in 0..20 {
                for byte in chunk.iter_mut() {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    *byte = (seed >> 24) as u8;
                }
                out.write_all(&chunk).unwrap();
            }
        }
        let started = Instant::now();
        let archive = env.dir.path().join("big.zip");
        let outcome = env.service.export_confirm(archive.to_str().unwrap()).unwrap();
        let export = started.elapsed();
        env.service.reset_examples().unwrap();
        let started = Instant::now();
        let validation = env.validate(&archive).unwrap();
        let validate = started.elapsed();
        let started = Instant::now();
        env.commit(&validation.import_token, false).unwrap();
        let commit = started.elapsed();
        println!(
            "QA-PERF 500MB: zip={} bytes export={export:?} validate(extract+sha)={validate:?} commit={commit:?}",
            outcome.size_bytes
        );
    }
}
