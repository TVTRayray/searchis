//! SPEC-22：文件模板的数据库操作。每个方法一个事务；文件系统由调用方（service）先行处理。

use std::{collections::HashMap, sync::atomic::Ordering};

use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    db_write_error, ensure_group_exists, find_conflict_key, get_in_transaction, now_string,
    Repository,
};
use crate::{
    error::AppError,
    model::Snippet,
    templates::{
        names::{key_from_stem, title_from_stem, unique_key},
        scan::{DbFile, ScanOp},
    },
};

/// 文件模板的内部视图（含不对外暴露的路径、inode、哈希）。
#[derive(Debug, Clone)]
pub struct FileRow {
    pub id: String,
    #[allow(dead_code)]
    pub key: String,
    pub title: String,
    pub group_id: Option<String>,
    pub rel_path: String,
    pub ext: String,
    pub size: i64,
    pub inode: Option<i64>,
    pub sha256: String,
    pub mtime: String,
    pub original_group_id: Option<String>,
    pub missing: bool,
    pub deleted_at: Option<String>,
    pub revision: i64,
}

#[derive(Debug, Clone)]
pub struct NewFile {
    pub key: String,
    pub title: String,
    pub group_id: Option<String>,
    pub rel_path: String,
    pub ext: String,
    pub size: i64,
    pub inode: Option<i64>,
    pub sha256: String,
    pub mtime: String,
    pub auto_imported: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateFile {
    pub key: String,
    pub normalized_key: String,
    pub title: String,
    pub group_id: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    /// 文件被改名/移动后的新相对路径及其 stat。
    pub new_path: Option<(String, i64, String)>,
}

#[derive(Debug, Clone)]
pub struct FileMove {
    pub id: String,
    pub new_rel_path: String,
    pub new_title: String,
}

/// 删除分组时随事务一并改写的文件模板路径。
#[derive(Debug, Clone, Default)]
pub struct GroupDeleteFs {
    pub prefix_rewrites: Vec<(String, String)>,
    pub file_moves: Vec<FileMove>,
}

const FILE_ROW_COLUMNS: &str = "id, key, title, group_id, file_rel_path, file_ext, file_size,
    file_inode, file_sha256, file_mtime, file_original_group_id, file_missing, deleted_at, revision";

fn row_to_file(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRow> {
    Ok(FileRow {
        id: row.get(0)?,
        key: row.get(1)?,
        title: row.get(2)?,
        group_id: row.get(3)?,
        rel_path: row.get(4)?,
        ext: row.get(5)?,
        size: row.get(6)?,
        inode: row.get(7)?,
        sha256: row.get(8)?,
        mtime: row.get(9)?,
        original_group_id: row.get(10)?,
        missing: row.get(11)?,
        deleted_at: row.get(12)?,
        revision: row.get(13)?,
    })
}

/// 把所有以 `old_prefix/` 开头的相对路径改写为 `new_prefix/`。
pub(super) fn rewrite_prefix(
    transaction: &Transaction<'_>,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<(), AppError> {
    let old = format!("{old_prefix}/");
    let new = format!("{new_prefix}/");
    transaction
        .execute(
            "UPDATE snippets SET file_rel_path = ?2 || substr(file_rel_path, length(?1) + 1)
             WHERE file_rel_path IS NOT NULL AND substr(file_rel_path, 1, length(?1)) = ?1",
            params![old, new],
        )
        .map_err(|_| db_write_error())?;
    Ok(())
}

pub(super) fn apply_delete_fs(
    transaction: &Transaction<'_>,
    fs: &GroupDeleteFs,
    now: &str,
) -> Result<(), AppError> {
    for (old, new) in &fs.prefix_rewrites {
        rewrite_prefix(transaction, old, new)?;
    }
    for item in &fs.file_moves {
        transaction
            .execute(
                "UPDATE snippets SET file_rel_path = ?1, title = ?2, updated_at = ?3,
                        revision = revision + 1
                 WHERE id = ?4 AND kind = 'file'",
                params![item.new_rel_path, item.new_title, now, item.id],
            )
            .map_err(|_| db_write_error())?;
    }
    Ok(())
}

impl Repository {
    fn check_injected_failure(&self) -> Result<(), AppError> {
        if self.fail_file_writes.load(Ordering::SeqCst) {
            return Err(db_write_error());
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn inject_file_write_failure(&self, enabled: bool) {
        self.fail_file_writes.store(enabled, Ordering::SeqCst);
    }

    pub fn file_row(&self, id: &str) -> Result<Option<FileRow>, AppError> {
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        connection
            .query_row(
                &format!("SELECT {FILE_ROW_COLUMNS} FROM snippets WHERE id = ?1 AND kind = 'file'"),
                params![id],
                row_to_file,
            )
            .optional()
            .map_err(|_| db_write_error())
    }

    /// 参与目录扫描的行：未删除的文件模板，以及被标记“文件缺失”的行。
    pub fn file_rows_for_scan(&self) -> Result<Vec<DbFile>, AppError> {
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {FILE_ROW_COLUMNS} FROM snippets
                 WHERE kind = 'file' AND (deleted_at IS NULL OR file_missing = 1)"
            ))
            .map_err(|_| db_write_error())?;
        let rows = statement
            .query_map([], row_to_file)
            .map_err(|_| db_write_error())?;
        rows.map(|row| {
            row.map(|file| DbFile {
                id: file.id,
                rel_path: file.rel_path,
                inode: file.inode,
                size: file.size,
                mtime: file.mtime,
                sha256: file.sha256,
                group_id: file.group_id,
                title: file.title,
                missing: file.missing,
            })
        })
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| db_write_error())
    }

    /// 回收站中（用户放入、文件在 `.trash/`）的文件模板；可按删除时间阈值过滤。
    pub fn trashed_file_rows(&self, threshold: Option<&str>) -> Result<Vec<FileRow>, AppError> {
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {FILE_ROW_COLUMNS} FROM snippets
                 WHERE kind = 'file' AND deleted_at IS NOT NULL AND file_missing = 0
                   AND (?1 IS NULL OR deleted_at <= ?1)"
            ))
            .map_err(|_| db_write_error())?;
        let rows = statement
            .query_map(params![threshold], row_to_file)
            .map_err(|_| db_write_error())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| db_write_error())
    }

    /// 未删除条目总数（文本 + 文件），用于 10,000 上限。
    pub fn active_entry_count(&self) -> Result<i64, AppError> {
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM snippets WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|_| db_write_error())
    }

    pub fn key_taken(&self, normalized_key: &str) -> Result<bool, AppError> {
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        connection
            .query_row(
                "SELECT 1 FROM snippets WHERE normalized_key = ?1",
                params![normalized_key],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(|_| db_write_error())
    }

    /// 幂等：同 requestId 已创建过的条目。
    pub fn snippet_by_request(&self, request_id: &str) -> Result<Option<Snippet>, AppError> {
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        let id: Option<String> = connection
            .query_row(
                "SELECT snippet_id FROM create_requests WHERE request_id = ?1",
                params![request_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| db_write_error())?;
        match id {
            Some(id) => super::query_snippet(&connection, &id),
            None => Ok(None),
        }
    }

    pub fn file_insert(&self, new: NewFile, request_id: Option<&str>) -> Result<Snippet, AppError> {
        let normalized_key = new.key.to_lowercase();
        let snippet = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;
            if let Some(key) = find_conflict_key(&transaction, &normalized_key, None)? {
                return Err(AppError::key_conflict(key));
            }
            ensure_group_exists(&transaction, new.group_id.as_deref())?;
            let id = uuid::Uuid::new_v4().to_string();
            let now = now_string();
            insert_file_row(&transaction, &id, &new, &normalized_key, &now)?;
            if let Some(request_id) = request_id {
                transaction
                    .execute(
                        "INSERT INTO create_requests (request_id, snippet_id, request_fingerprint, created_at)
                         VALUES (?1, ?2, 'template', ?3)",
                        params![request_id, id, now],
                    )
                    .map_err(|_| db_write_error())?;
            }
            self.check_injected_failure()?;
            let snippet = get_in_transaction(&transaction, &id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            snippet
        };
        self.index_upsert(&snippet)?;
        Ok(snippet)
    }

    pub fn file_update(
        &self,
        id: &str,
        revision: i64,
        update: UpdateFile,
    ) -> Result<Snippet, AppError> {
        let snippet = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;
            if let Some(key) = find_conflict_key(&transaction, &update.normalized_key, Some(id))? {
                return Err(AppError::key_conflict(key));
            }
            ensure_group_exists(&transaction, update.group_id.as_deref())?;
            let now = now_string();
            let aliases = serde_json::to_string(&update.aliases).map_err(|_| db_write_error())?;
            let tags = serde_json::to_string(&update.tags).map_err(|_| db_write_error())?;
            let (path, inode, mtime) = match &update.new_path {
                Some((path, inode, mtime)) => {
                    (Some(path.clone()), Some(*inode), Some(mtime.clone()))
                }
                None => (None, None, None),
            };
            let changed = transaction
                .execute(
                    "UPDATE snippets SET key = ?1, normalized_key = ?2, title = ?3, aliases = ?4,
                        tags = ?5, pinned = ?6, group_id = ?7, updated_at = ?8, revision = revision + 1,
                        aliases_norm = ?9, tags_norm = ?10, file_auto_imported = 0,
                        file_rel_path = COALESCE(?11, file_rel_path),
                        file_inode = COALESCE(?12, file_inode),
                        file_mtime = COALESCE(?13, file_mtime)
                     WHERE id = ?14 AND revision = ?15 AND kind = 'file' AND deleted_at IS NULL",
                    params![
                        update.key,
                        update.normalized_key,
                        update.title,
                        aliases,
                        tags,
                        update.pinned,
                        update.group_id,
                        now,
                        update.aliases.join(" ").to_lowercase(),
                        update.tags.join(" ").to_lowercase(),
                        path,
                        inode,
                        mtime,
                        id,
                        revision
                    ],
                )
                .map_err(|_| db_write_error())?;
            if changed == 0 {
                let exists = transaction
                    .query_row("SELECT 1 FROM snippets WHERE id = ?1", params![id], |_| {
                        Ok(())
                    })
                    .optional()
                    .map_err(|_| db_write_error())?
                    .is_some();
                return Err(if exists {
                    AppError::new(
                        "REVISION_CONFLICT",
                        "模板已在其他窗口修改。请重新载入后再保存。",
                    )
                } else {
                    AppError::new("NOT_FOUND", "模板不存在。请重新载入列表。")
                });
            }
            self.check_injected_failure()?;
            let snippet = get_in_transaction(&transaction, id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            snippet
        };
        self.index_upsert(&snippet)?;
        Ok(snippet)
    }

    /// 放入回收站：文件已移到 `.trash/`，记录原分组。
    pub fn file_trash(&self, id: &str, trash_rel_path: &str) -> Result<Snippet, AppError> {
        let snippet = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;
            let current = get_in_transaction(&transaction, id)?
                .ok_or_else(|| AppError::new("NOT_FOUND", "模板不存在。"))?;
            if current.deleted_at.is_some() {
                return Err(AppError::new("ALREADY_TRASHED", "模板已在回收站中。"));
            }
            let now = now_string();
            transaction
                .execute(
                    "UPDATE snippets SET deleted_at = ?1, updated_at = ?1, revision = revision + 1,
                            file_original_group_id = group_id, file_rel_path = ?2
                     WHERE id = ?3",
                    params![now, trash_rel_path, id],
                )
                .map_err(|_| db_write_error())?;
            self.check_injected_failure()?;
            let snippet = get_in_transaction(&transaction, id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            snippet
        };
        self.index_upsert(&snippet)?;
        Ok(snippet)
    }

    /// 从回收站还原：文件已移回 `rel_path`。
    pub fn file_restore(
        &self,
        id: &str,
        rel_path: &str,
        group_id: Option<&str>,
        title: &str,
    ) -> Result<Snippet, AppError> {
        let snippet = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;
            let current = get_in_transaction(&transaction, id)?
                .ok_or_else(|| AppError::new("NOT_FOUND", "模板不存在。"))?;
            if let Some(key) = find_conflict_key(&transaction, &current.normalized_key, Some(id))? {
                return Err(AppError::key_conflict(key));
            }
            ensure_group_exists(&transaction, group_id)?;
            let now = now_string();
            transaction
                .execute(
                    "UPDATE snippets SET deleted_at = NULL, updated_at = ?1, revision = revision + 1,
                            file_rel_path = ?2, group_id = ?3, title = ?4, file_original_group_id = NULL
                     WHERE id = ?5",
                    params![now, rel_path, group_id, title, id],
                )
                .map_err(|_| db_write_error())?;
            self.check_injected_failure()?;
            let snippet = get_in_transaction(&transaction, id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            snippet
        };
        self.index_upsert(&snippet)?;
        Ok(snippet)
    }

    /// 应用一批扫描变更（单事务）。`group_ids` 为 目录 → 分组 id。失败时整批回滚。
    pub fn apply_scan_batch(
        &self,
        ops: &[ScanOp],
        group_ids: &HashMap<String, String>,
    ) -> Result<(), AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        let now = now_string();
        let group_of = |dir: &str| -> Option<String> {
            if dir.is_empty() {
                None
            } else {
                group_ids.get(dir).cloned()
            }
        };
        let mut batch_keys: Vec<String> = Vec::new();
        for op in ops {
            match op {
                ScanOp::Refresh {
                    id,
                    size,
                    mtime,
                    inode,
                    sha256,
                    silent,
                } => {
                    if *silent {
                        transaction
                            .execute(
                                "UPDATE snippets SET file_size = ?1, file_mtime = ?2, file_inode = ?3,
                                        file_sha256 = ?4 WHERE id = ?5",
                                params![size, mtime, inode, sha256, id],
                            )
                            .map_err(|_| db_write_error())?;
                    } else {
                        transaction
                            .execute(
                                "UPDATE snippets SET file_size = ?1, file_mtime = ?2, file_inode = ?3,
                                        file_sha256 = ?4, updated_at = ?6, revision = revision + 1
                                 WHERE id = ?5",
                                params![size, mtime, inode, sha256, id, now],
                            )
                            .map_err(|_| db_write_error())?;
                    }
                }
                ScanOp::Move {
                    id,
                    rel_path,
                    dir,
                    title,
                    size,
                    mtime,
                    inode,
                    sha256,
                } => {
                    transaction
                        .execute(
                            "UPDATE snippets SET file_rel_path = ?1, group_id = ?2, title = ?3,
                                    file_size = ?4, file_mtime = ?5, file_inode = ?6, file_sha256 = ?7,
                                    updated_at = ?8, revision = revision + 1
                             WHERE id = ?9",
                            params![rel_path, group_of(dir), title, size, mtime, inode, sha256, now, id],
                        )
                        .map_err(|_| db_write_error())?;
                }
                ScanOp::Restore {
                    id,
                    rel_path,
                    dir,
                    title,
                    size,
                    mtime,
                    inode,
                    sha256,
                } => {
                    transaction
                        .execute(
                            "UPDATE snippets SET file_rel_path = ?1, group_id = ?2, title = ?3,
                                    file_size = ?4, file_mtime = ?5, file_inode = ?6, file_sha256 = ?7,
                                    deleted_at = NULL, file_missing = 0,
                                    updated_at = ?8, revision = revision + 1
                             WHERE id = ?9",
                            params![rel_path, group_of(dir), title, size, mtime, inode, sha256, now, id],
                        )
                        .map_err(|_| db_write_error())?;
                }
                ScanOp::MarkMissing { id } => {
                    transaction
                        .execute(
                            "UPDATE snippets SET deleted_at = ?1, file_missing = 1,
                                    updated_at = ?1, revision = revision + 1
                             WHERE id = ?2 AND deleted_at IS NULL",
                            params![now, id],
                        )
                        .map_err(|_| db_write_error())?;
                }
                ScanOp::Import {
                    rel_path,
                    dir,
                    stem,
                    ext,
                    size,
                    mtime,
                    inode,
                    sha256,
                } => {
                    let key = unique_key(&key_from_stem(stem), |candidate| {
                        batch_keys.iter().any(|k| k == candidate)
                            || find_conflict_key(&transaction, candidate, None)
                                .map(|found| found.is_some())
                                .unwrap_or(true)
                    });
                    batch_keys.push(key.clone());
                    let new = NewFile {
                        key: key.clone(),
                        title: title_from_stem(stem),
                        group_id: group_of(dir),
                        rel_path: rel_path.clone(),
                        ext: ext.clone(),
                        size: *size,
                        inode: *inode,
                        sha256: sha256.clone(),
                        mtime: mtime.clone(),
                        auto_imported: true,
                    };
                    insert_file_row(
                        &transaction,
                        &uuid::Uuid::new_v4().to_string(),
                        &new,
                        &key.to_lowercase(),
                        &now,
                    )?;
                }
            }
        }
        self.check_injected_failure()?;
        transaction.commit().map_err(|_| db_write_error())?;
        drop(connection);
        self.invalidate_search_index()
    }
}

fn insert_file_row(
    transaction: &Transaction<'_>,
    id: &str,
    new: &NewFile,
    normalized_key: &str,
    now: &str,
) -> Result<(), AppError> {
    transaction
        .execute(
            "INSERT INTO snippets (
                id, key, normalized_key, title, content, aliases, tags, pinned, sensitive,
                usage_count, last_used_at, created_at, updated_at, deleted_at, revision,
                content_norm, aliases_norm, tags_norm, group_id, language, kind,
                file_rel_path, file_ext, file_size, file_inode, file_sha256, file_mtime,
                file_auto_imported, file_missing
             ) VALUES (?1, ?2, ?3, ?4, '', '[]', '[]', 0, 0, 0, NULL, ?5, ?5, NULL, 1,
                       '', '', '', ?6, 'plain', 'file', ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0)",
            params![
                id,
                new.key,
                normalized_key,
                new.title,
                now,
                new.group_id,
                new.rel_path,
                new.ext,
                new.size,
                new.inode,
                new.sha256,
                new.mtime,
                new.auto_imported
            ],
        )
        .map_err(|error| {
            if matches!(&error, rusqlite::Error::SqliteFailure(value, _)
                if value.code == rusqlite::ErrorCode::ConstraintViolation)
            {
                if let Ok(Some(key)) = find_conflict_key(transaction, normalized_key, None) {
                    return AppError::key_conflict(key);
                }
            }
            db_write_error()
        })?;
    Ok(())
}

impl Repository {
    fn index_upsert(&self, snippet: &Snippet) -> Result<(), AppError> {
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .upsert(snippet);
        Ok(())
    }

    fn invalidate_search_index(&self) -> Result<(), AppError> {
        self.invalidate_index()
    }
}
