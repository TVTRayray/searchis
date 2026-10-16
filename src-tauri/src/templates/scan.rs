//! 目录扫描：纯函数核心。输入数据库快照 + 目录快照，输出变更计划，不做任何 IO。

use std::collections::{HashMap, HashSet};

use super::{
    names::{split_name, title_from_stem},
    paths::{base_name, parent_dir},
    MAX_DEPTH, MAX_FILE_BYTES,
};
use crate::{model::ScanSkip, validation::validate_group_name};

/// 数据库中参与扫描的文件模板行（含“文件缺失”行）。
#[derive(Debug, Clone)]
pub struct DbFile {
    pub id: String,
    pub rel_path: String,
    pub inode: Option<i64>,
    pub size: i64,
    pub mtime: String,
    pub sha256: String,
    pub group_id: Option<String>,
    pub title: String,
    pub missing: bool,
}

#[derive(Debug, Clone)]
pub struct FsFile {
    pub rel_path: String,
    pub inode: Option<i64>,
    pub size: u64,
    pub mtime: String,
}

#[derive(Debug, Clone, Default)]
pub struct FsSnapshot {
    pub files: Vec<FsFile>,
    /// 目录快照中的全部相对目录（不含根），含空目录。
    pub dirs: Vec<String>,
    /// IO 层在收集快照时已经跳过的条目（符号链接、超深目录、非法名称）。
    pub skipped: Vec<ScanSkip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanOp {
    /// 内容元数据变化；`silent` 表示只刷新 inode/mtime，不计入“更新”也不递增 revision。
    Refresh {
        id: String,
        size: i64,
        mtime: String,
        inode: Option<i64>,
        sha256: String,
        silent: bool,
    },
    /// 改名或移动（含分组变化）。
    Move {
        id: String,
        rel_path: String,
        dir: String,
        title: String,
        size: i64,
        mtime: String,
        inode: Option<i64>,
        sha256: String,
    },
    /// “文件缺失”条目因文件回来而恢复。
    Restore {
        id: String,
        rel_path: String,
        dir: String,
        title: String,
        size: i64,
        mtime: String,
        inode: Option<i64>,
        sha256: String,
    },
    MarkMissing {
        id: String,
    },
    Import {
        rel_path: String,
        dir: String,
        stem: String,
        ext: String,
        size: i64,
        mtime: String,
        inode: Option<i64>,
        sha256: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ScanPlan {
    /// 需要新建的分组目录，按深度升序。
    pub new_dirs: Vec<String>,
    pub ops: Vec<ScanOp>,
    pub skipped: Vec<ScanSkip>,
}

/// 统计一批变更：(新增, 更新, 缺失, 恢复)。
pub fn count_ops(ops: &[ScanOp]) -> (i64, i64, i64, i64) {
    let (mut added, mut updated, mut missing, mut restored) = (0, 0, 0, 0);
    for op in ops {
        match op {
            ScanOp::Import { .. } => added += 1,
            ScanOp::Refresh { silent, .. } => {
                if !silent {
                    updated += 1;
                }
            }
            ScanOp::Move { .. } => updated += 1,
            ScanOp::Restore { .. } => restored += 1,
            ScanOp::MarkMissing { .. } => missing += 1,
        }
    }
    (added, updated, missing, restored)
}

#[cfg(test)]
impl ScanPlan {
    pub fn counts(&self) -> (i64, i64, i64, i64) {
        count_ops(&self.ops)
    }
}

fn skip(reason: &str) -> ScanSkip {
    ScanSkip {
        reason: reason.to_owned(),
    }
}

fn dir_depth(dir: &str) -> usize {
    if dir.is_empty() {
        0
    } else {
        dir.split('/').count()
    }
}

fn stem_of(rel_path: &str) -> String {
    split_name(base_name(rel_path)).0
}

/// `hasher(rel_path)` 返回文件 SHA-256；`capacity` 为还能导入的条目数。
/// `group_dirs` 为现有分组的 目录 → 分组 id。
pub fn plan(
    db: &[DbFile],
    group_dirs: &HashMap<String, String>,
    fs: &FsSnapshot,
    capacity: i64,
    hasher: &dyn Fn(&str) -> Option<String>,
) -> ScanPlan {
    let mut plan = ScanPlan {
        skipped: fs.skipped.clone(),
        ..ScanPlan::default()
    };

    // === 1. 需要新建的分组目录，以及被阻断（非法名 / 大小写冲突 / 过深）的目录 ===
    let existing_lower: HashSet<String> = group_dirs.keys().map(|d| d.to_lowercase()).collect();
    let mut blocked: HashSet<String> = HashSet::new();
    let mut new_dirs: Vec<String> = Vec::new();
    let mut wanted: Vec<&str> = fs.dirs.iter().map(String::as_str).collect();
    for file in &fs.files {
        wanted.push(parent_dir(&file.rel_path));
    }
    wanted.sort_by_key(|d| (dir_depth(d), d.to_string()));
    wanted.dedup();
    for dir in wanted {
        if dir.is_empty() || group_dirs.contains_key(dir) || new_dirs.iter().any(|d| d == dir) {
            continue;
        }
        let parent = parent_dir(dir);
        let name = base_name(dir);
        let reason = if dir_depth(dir) > MAX_DEPTH {
            Some("depth_exceeded")
        } else if validate_group_name(name).is_err() {
            Some("invalid_name")
        } else if existing_lower.contains(&dir.to_lowercase())
            || new_dirs
                .iter()
                .any(|d| d.to_lowercase() == dir.to_lowercase())
        {
            Some("group_conflict")
        } else if blocked.contains(parent) {
            Some("blocked_parent")
        } else {
            None
        };
        match reason {
            None => new_dirs.push(dir.to_owned()),
            Some("blocked_parent") => {
                blocked.insert(dir.to_owned());
            }
            Some(reason) => {
                blocked.insert(dir.to_owned());
                plan.skipped.push(skip(reason));
            }
        }
    }
    plan.new_dirs = new_dirs;
    let is_blocked = |dir: &str| -> bool {
        let mut cursor = dir;
        while !cursor.is_empty() {
            if blocked.contains(cursor) {
                return true;
            }
            cursor = parent_dir(cursor);
        }
        false
    };
    let expected_group = |dir: &str| -> Option<&String> { group_dirs.get(dir) };

    // === 2. 路径匹配 ===
    let fs_by_path: HashMap<&str, &FsFile> =
        fs.files.iter().map(|f| (f.rel_path.as_str(), f)).collect();
    let mut matched_fs: HashSet<&str> = HashSet::new();
    let mut matched_db: HashSet<&str> = HashSet::new();
    for row in db {
        let Some(file) = fs_by_path.get(row.rel_path.as_str()) else {
            continue;
        };
        matched_fs.insert(file.rel_path.as_str());
        matched_db.insert(row.id.as_str());
        let dir = parent_dir(&file.rel_path);
        let group_mismatch = row.group_id.as_ref() != expected_group(dir);
        let unchanged = row.size as u64 == file.size && row.mtime == file.mtime;
        if file.size > MAX_FILE_BYTES {
            plan.skipped.push(skip("too_large"));
            continue;
        }
        if unchanged && !row.missing && !group_mismatch {
            if file.inode.is_some() && file.inode != row.inode {
                plan.ops.push(ScanOp::Refresh {
                    id: row.id.clone(),
                    size: row.size,
                    mtime: row.mtime.clone(),
                    inode: file.inode,
                    sha256: row.sha256.clone(),
                    silent: true,
                });
            }
            continue;
        }
        let sha = if unchanged {
            row.sha256.clone()
        } else {
            match hasher(&file.rel_path) {
                Some(sha) => sha,
                None => continue,
            }
        };
        let size = file.size as i64;
        if row.missing {
            plan.ops.push(ScanOp::Restore {
                id: row.id.clone(),
                rel_path: file.rel_path.clone(),
                dir: dir.to_owned(),
                title: row.title.clone(),
                size,
                mtime: file.mtime.clone(),
                inode: file.inode,
                sha256: sha,
            });
        } else if group_mismatch {
            plan.ops.push(ScanOp::Move {
                id: row.id.clone(),
                rel_path: file.rel_path.clone(),
                dir: dir.to_owned(),
                title: row.title.clone(),
                size,
                mtime: file.mtime.clone(),
                inode: file.inode,
                sha256: sha,
            });
        } else {
            let content_changed = sha != row.sha256 || size != row.size;
            plan.ops.push(ScanOp::Refresh {
                id: row.id.clone(),
                size,
                mtime: file.mtime.clone(),
                inode: file.inode,
                sha256: sha,
                silent: !content_changed,
            });
        }
    }

    let mut loose_db: Vec<&DbFile> = db
        .iter()
        .filter(|r| !matched_db.contains(r.id.as_str()))
        .collect();
    let mut loose_fs: Vec<&FsFile> = fs
        .files
        .iter()
        .filter(|f| !matched_fs.contains(f.rel_path.as_str()))
        .collect();
    loose_fs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    // 过大 / 位于被阻断目录的文件不参与后续匹配与导入。
    loose_fs.retain(|file| {
        if file.size > MAX_FILE_BYTES {
            plan.skipped.push(skip("too_large"));
            return false;
        }
        !is_blocked(parent_dir(&file.rel_path))
    });

    let claim =
        |plan: &mut ScanPlan, row: &DbFile, file: &FsFile, known_sha: Option<String>| -> bool {
            let sha = match known_sha {
                Some(sha) => sha,
                None => {
                    if row.size as u64 == file.size && row.mtime == file.mtime {
                        row.sha256.clone()
                    } else {
                        match hasher(&file.rel_path) {
                            Some(sha) => sha,
                            None => return false,
                        }
                    }
                }
            };
            let dir = parent_dir(&file.rel_path).to_owned();
            let title = if stem_of(&row.rel_path) == stem_of(&file.rel_path) {
                row.title.clone()
            } else {
                title_from_stem(&stem_of(&file.rel_path))
            };
            let size = file.size as i64;
            let op = if row.missing {
                ScanOp::Restore {
                    id: row.id.clone(),
                    rel_path: file.rel_path.clone(),
                    dir,
                    title,
                    size,
                    mtime: file.mtime.clone(),
                    inode: file.inode,
                    sha256: sha,
                }
            } else {
                ScanOp::Move {
                    id: row.id.clone(),
                    rel_path: file.rel_path.clone(),
                    dir,
                    title,
                    size,
                    mtime: file.mtime.clone(),
                    inode: file.inode,
                    sha256: sha,
                }
            };
            plan.ops.push(op);
            true
        };

    // === 3. inode 匹配（缺失行不参与：inode 可能已被复用）===
    let mut claimed_fs: HashSet<&str> = HashSet::new();
    let mut claimed_db: HashSet<&str> = HashSet::new();
    let mut inode_count: HashMap<i64, usize> = HashMap::new();
    for file in &loose_fs {
        if let Some(inode) = file.inode {
            *inode_count.entry(inode).or_default() += 1;
        }
    }
    for row in &loose_db {
        if row.missing {
            continue;
        }
        let Some(inode) = row.inode else { continue };
        if inode_count.get(&inode) != Some(&1) {
            continue;
        }
        let Some(file) = loose_fs
            .iter()
            .find(|f| f.inode == Some(inode) && !claimed_fs.contains(f.rel_path.as_str()))
        else {
            continue;
        };
        if claim(&mut plan, row, file, None) {
            claimed_fs.insert(file.rel_path.as_str());
            claimed_db.insert(row.id.as_str());
        }
    }
    loose_db.retain(|r| !claimed_db.contains(r.id.as_str()));
    loose_fs.retain(|f| !claimed_fs.contains(f.rel_path.as_str()));

    // === 4. 哈希匹配：仅当哈希在剩余两侧都恰好出现一次 ===
    let mut fs_hashes: Vec<(&FsFile, Option<String>)> = loose_fs
        .iter()
        .map(|file| (*file, hasher(&file.rel_path)))
        .collect();
    let mut fs_by_hash: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, (_, sha)) in fs_hashes.iter().enumerate() {
        if let Some(sha) = sha {
            fs_by_hash.entry(sha.as_str()).or_default().push(index);
        }
    }
    let mut db_by_hash: HashMap<&str, Vec<&DbFile>> = HashMap::new();
    for row in &loose_db {
        db_by_hash.entry(row.sha256.as_str()).or_default().push(row);
    }
    let mut hash_claimed_fs: HashSet<usize> = HashSet::new();
    let mut hash_claimed_db: HashSet<&str> = HashSet::new();
    for (sha, rows) in &db_by_hash {
        let (Some(indices), 1) = (fs_by_hash.get(sha), rows.len()) else {
            continue;
        };
        if indices.len() != 1 {
            continue;
        }
        let index = indices[0];
        let (file, known) = &fs_hashes[index];
        if claim(&mut plan, rows[0], file, known.clone()) {
            hash_claimed_fs.insert(index);
            hash_claimed_db.insert(rows[0].id.as_str());
        }
    }
    let mut remaining_fs: Vec<(&FsFile, Option<String>)> = Vec::new();
    for (index, entry) in fs_hashes.drain(..).enumerate() {
        if !hash_claimed_fs.contains(&index) {
            remaining_fs.push(entry);
        }
    }

    // === 5. 缺失与导入 ===
    for row in loose_db {
        if !hash_claimed_db.contains(row.id.as_str()) && !row.missing {
            plan.ops.push(ScanOp::MarkMissing { id: row.id.clone() });
        }
    }
    let mut remaining_capacity = capacity.max(0);
    for (file, sha) in remaining_fs {
        let Some(sha) = sha else { continue };
        if remaining_capacity == 0 {
            plan.skipped.push(skip("limit"));
            continue;
        }
        remaining_capacity -= 1;
        let (stem, ext) = split_name(base_name(&file.rel_path));
        plan.ops.push(ScanOp::Import {
            rel_path: file.rel_path.clone(),
            dir: parent_dir(&file.rel_path).to_owned(),
            stem,
            ext,
            size: file.size as i64,
            mtime: file.mtime.clone(),
            inode: file.inode,
            sha256: sha,
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(id: &str, path: &str, inode: i64, sha: &str) -> DbFile {
        DbFile {
            id: id.into(),
            rel_path: path.into(),
            inode: Some(inode),
            size: 10,
            mtime: "t1".into(),
            sha256: sha.into(),
            group_id: group_for(path),
            title: stem_of(path),
            missing: false,
        }
    }

    fn group_for(path: &str) -> Option<String> {
        match parent_dir(path) {
            "" => None,
            dir => Some(format!("g:{dir}")),
        }
    }

    fn fsf(path: &str, inode: i64) -> FsFile {
        FsFile {
            rel_path: path.into(),
            inode: Some(inode),
            size: 10,
            mtime: "t1".into(),
        }
    }

    fn groups(dirs: &[&str]) -> HashMap<String, String> {
        dirs.iter()
            .map(|d| (d.to_string(), format!("g:{d}")))
            .collect()
    }

    fn run(
        db_rows: Vec<DbFile>,
        files: Vec<FsFile>,
        dirs: &[&str],
        hashes: &[(&str, &str)],
    ) -> ScanPlan {
        let map: HashMap<String, String> = hashes
            .iter()
            .map(|(p, h)| (p.to_string(), h.to_string()))
            .collect();
        let snapshot = FsSnapshot {
            files,
            dirs: dirs.iter().map(|d| d.to_string()).collect(),
            skipped: vec![],
        };
        plan(&db_rows, &groups(dirs), &snapshot, 10_000, &|path| {
            map.get(path).cloned()
        })
    }

    #[test]
    fn unchanged_files_produce_no_ops() {
        let result = run(
            vec![db("1", "行政/a.md", 5, "h")],
            vec![fsf("行政/a.md", 5)],
            &["行政"],
            &[],
        );
        assert!(result.ops.is_empty() && result.new_dirs.is_empty());
    }

    #[test]
    fn atomic_save_changes_inode_and_hash_but_keeps_identity() {
        let mut file = fsf("a.md", 99);
        file.size = 12;
        file.mtime = "t2".into();
        let result = run(
            vec![db("1", "a.md", 5, "old")],
            vec![file],
            &[],
            &[("a.md", "new")],
        );
        assert_eq!(result.ops.len(), 1);
        assert!(
            matches!(&result.ops[0], ScanOp::Refresh { id, sha256, silent: false, .. } if id == "1" && sha256 == "new")
        );
        assert_eq!(result.counts(), (0, 1, 0, 0));
    }

    #[test]
    fn rename_and_move_keep_identity_through_inode() {
        let result = run(
            vec![db("1", "运维/值班表.xlsx", 7, "h")],
            vec![fsf("行政/排班表.xlsx", 7)],
            &["运维", "行政"],
            &[],
        );
        assert_eq!(result.ops.len(), 1);
        match &result.ops[0] {
            ScanOp::Move {
                id,
                rel_path,
                dir,
                title,
                ..
            } => {
                assert_eq!(id, "1");
                assert_eq!(rel_path, "行政/排班表.xlsx");
                assert_eq!(dir, "行政");
                assert_eq!(title, "排班表");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn cross_directory_move_keeps_title_when_stem_unchanged() {
        let result = run(
            vec![db("1", "运维/a.md", 7, "h")],
            vec![fsf("行政/a.md", 7)],
            &["运维", "行政"],
            &[],
        );
        assert!(matches!(&result.ops[0], ScanOp::Move { title, .. } if title == "a"));
    }

    #[test]
    fn hash_matching_requires_a_unique_candidate_on_both_sides() {
        let unique = run(
            vec![db("1", "a.md", 1, "h")],
            vec![fsf("b.md", 2)],
            &[],
            &[("b.md", "h")],
        );
        assert!(
            matches!(&unique.ops[0], ScanOp::Move { id, rel_path, .. } if id == "1" && rel_path == "b.md")
        );

        let ambiguous = run(
            vec![db("1", "a.md", 1, "h")],
            vec![fsf("b.md", 2), fsf("c.md", 3)],
            &[],
            &[("b.md", "h"), ("c.md", "h")],
        );
        assert_eq!(ambiguous.counts(), (2, 0, 1, 0));
        let two_rows = run(
            vec![db("1", "a.md", 1, "h"), db("2", "z.md", 9, "h")],
            vec![fsf("b.md", 2)],
            &[],
            &[("b.md", "h")],
        );
        assert_eq!(two_rows.counts(), (1, 0, 2, 0));
    }

    #[test]
    fn external_delete_marks_missing_and_return_restores() {
        let gone = run(vec![db("1", "a.md", 1, "h")], vec![], &[], &[]);
        assert_eq!(gone.ops, vec![ScanOp::MarkMissing { id: "1".into() }]);

        let mut row = db("1", "a.md", 1, "h");
        row.missing = true;
        let again = run(vec![row.clone()], vec![], &[], &[]);
        assert!(again.ops.is_empty(), "already missing rows stay quiet");

        let back = run(vec![row], vec![fsf("a.md", 44)], &[], &[("a.md", "h")]);
        assert_eq!(back.counts(), (0, 0, 0, 1));
        assert!(matches!(&back.ops[0], ScanOp::Restore { id, .. } if id == "1"));
    }

    #[test]
    fn missing_rows_do_not_match_by_inode() {
        let mut row = db("1", "a.md", 7, "h");
        row.missing = true;
        let result = run(
            vec![row],
            vec![fsf("other.md", 7)],
            &[],
            &[("other.md", "zzz")],
        );
        assert_eq!(result.counts(), (1, 0, 0, 0));
    }

    #[test]
    fn new_files_import_and_new_directories_become_groups() {
        let result = run(
            vec![],
            vec![fsf("新目录/子/a.md", 1), fsf("b.xlsx", 2)],
            &["新目录", "新目录/子", "空目录"],
            &[("新目录/子/a.md", "h1"), ("b.xlsx", "h2")],
        );
        // groups() 已把 dirs 当作现有分组；改为全部不存在
        let snapshot = FsSnapshot {
            files: vec![fsf("新目录/子/a.md", 1), fsf("b.xlsx", 2)],
            dirs: vec!["新目录".into(), "新目录/子".into(), "空目录".into()],
            skipped: vec![],
        };
        let fresh = plan(&[], &HashMap::new(), &snapshot, 10_000, &|_| {
            Some("h".into())
        });
        assert_eq!(fresh.new_dirs, vec!["新目录", "空目录", "新目录/子"]);
        assert_eq!(fresh.counts(), (2, 0, 0, 0));
        assert_eq!(result.counts(), (2, 0, 0, 0));
    }

    #[test]
    fn too_deep_invalid_and_conflicting_directories_are_skipped() {
        let snapshot = FsSnapshot {
            files: vec![
                fsf("a/b/c/d/x.md", 1),
                fsf("ok/y.md", 2),
                fsf("Ops/z.md", 3),
            ],
            dirs: vec![
                "a".into(),
                "a/b".into(),
                "a/b/c".into(),
                "a/b/c/d".into(),
                "ok".into(),
                "Ops".into(),
            ],
            skipped: vec![],
        };
        let existing = groups(&["ops"]);
        let result = plan(&[], &existing, &snapshot, 10_000, &|_| Some("h".into()));
        let reasons: Vec<&str> = result.skipped.iter().map(|s| s.reason.as_str()).collect();
        assert!(reasons.contains(&"depth_exceeded"));
        assert!(reasons.contains(&"group_conflict"));
        assert_eq!(result.counts().0, 1, "only ok/y.md imports");
    }

    #[test]
    fn oversize_files_and_capacity_limit_are_skipped() {
        let mut big = fsf("big.bin", 1);
        big.size = MAX_FILE_BYTES + 1;
        let snapshot = FsSnapshot {
            files: vec![big, fsf("a.md", 2), fsf("b.md", 3)],
            ..FsSnapshot::default()
        };
        let result = plan(&[], &HashMap::new(), &snapshot, 1, &|_| Some("h".into()));
        let reasons: Vec<&str> = result.skipped.iter().map(|s| s.reason.as_str()).collect();
        assert_eq!(reasons, vec!["too_large", "limit"]);
        assert_eq!(result.counts().0, 1);
    }
}
