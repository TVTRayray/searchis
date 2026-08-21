use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    sync::Mutex,
};

use chrono::Utc;
use rand::TryRngCore;
use rusqlite::{params, types::Type, Connection, OptionalExtension, Row, Transaction};
use sha2::{Digest, Sha256};

use crate::{
    error::AppError,
    model::{SearchResponse, Snippet, ValidatedFields},
    search_index::SearchIndex,
};

const SCHEMA_VERSION: i64 = 3;
const KEY_BYTES: usize = 32;

#[derive(Debug)]
pub struct Repository {
    connection: Mutex<Connection>,
    search_index: Mutex<SearchIndex>,
}

impl Repository {
    #[cfg(test)]
    pub fn open(data_dir: &Path) -> Result<Self, AppError> {
        Self::open_with_paths(data_dir, data_dir)
    }

    pub fn open_with_paths(data_dir: &Path, config_dir: &Path) -> Result<Self, AppError> {
        fs::create_dir_all(data_dir)
            .map_err(|_| db_open_error("无法创建应用数据目录。请检查目录权限后重试。"))?;
        fs::create_dir_all(config_dir)
            .map_err(|_| key_error("无法创建应用配置目录。请检查目录权限后重试。"))?;
        let key_path = config_dir.join("database.key");
        let database_path = data_dir.join("searchis.db");
        let key = load_or_create_key(&key_path, database_path.exists())?;
        let is_new_database = !database_path.exists();

        let mut connection = Connection::open(&database_path)
            .map_err(|_| db_open_error("无法打开加密数据库。请检查数据文件权限或恢复正确密钥。"))?;
        if is_new_database {
            let _ = fs::set_permissions(&database_path, fs::Permissions::from_mode(0o600));
        }
        apply_key(&connection, &key)?;
        verify_cipher(&connection)?;
        migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            search_index: Mutex::new(SearchIndex::default()),
        })
    }

    pub fn create(&self, fields: ValidatedFields, request_id: &str) -> Result<Snippet, AppError> {
        let request_fingerprint = fingerprint_fields(&fields)?;
        let snippet = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;

            if let Some((snippet_id, stored_fingerprint)) = transaction
                .query_row(
                    "SELECT snippet_id, request_fingerprint FROM create_requests WHERE request_id = ?1",
                    params![request_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(|_| db_write_error())?
            {
                if stored_fingerprint != request_fingerprint {
                    return Err(AppError::validation(
                        "requestId",
                        "该保存请求已用于不同内容。当前输入已保留；请重新开始新建后再保存。",
                    ));
                }
                let existing =
                    get_in_transaction(&transaction, &snippet_id)?.ok_or_else(db_write_error)?;
                transaction.commit().map_err(|_| db_write_error())?;
                return Ok(existing);
            }

            if let Some(key) = find_conflict_key(&transaction, &fields.normalized_key, None)? {
                return Err(AppError::key_conflict(key));
            }

            let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let id = uuid::Uuid::new_v4().to_string();
            let aliases = serde_json::to_string(&fields.aliases).map_err(|_| db_write_error())?;
            let tags = serde_json::to_string(&fields.tags).map_err(|_| db_write_error())?;
            transaction
                .execute(
                    "INSERT INTO snippets (
                id, key, normalized_key, title, content, aliases, tags, pinned, sensitive,
                usage_count, last_used_at, created_at, updated_at, deleted_at, revision,
                content_norm, aliases_norm, tags_norm
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, NULL, ?10, ?10, NULL, 1, ?11, ?12, ?13)",
                    params![
                        id,
                        fields.key,
                        fields.normalized_key,
                        fields.title,
                        fields.content,
                        aliases,
                        tags,
                        fields.pinned,
                        fields.sensitive,
                        now,
                        fields.content.to_lowercase(),
                        fields.aliases.join(" ").to_lowercase(),
                        fields.tags.join(" ").to_lowercase()
                    ],
                )
                .map_err(|error| {
                    map_write_error(error, &transaction, &fields.normalized_key, None)
                })?;
            transaction.execute(
                "INSERT INTO create_requests (request_id, snippet_id, request_fingerprint, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![request_id, id, request_fingerprint, now],
            ).map_err(|_| db_write_error())?;
            let snippet = get_in_transaction(&transaction, &id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            snippet
        };
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .upsert(&snippet);
        Ok(snippet)
    }

    pub fn update(
        &self,
        id: &str,
        revision: i64,
        fields: ValidatedFields,
    ) -> Result<Snippet, AppError> {
        let snippet = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;
            if let Some(key) = find_conflict_key(&transaction, &fields.normalized_key, Some(id))? {
                return Err(AppError::key_conflict(key));
            }
            let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let aliases = serde_json::to_string(&fields.aliases).map_err(|_| db_write_error())?;
            let tags = serde_json::to_string(&fields.tags).map_err(|_| db_write_error())?;
            let changed = transaction
                .execute(
                    "UPDATE snippets SET key = ?1, normalized_key = ?2, title = ?3, content = ?4,
                aliases = ?5, tags = ?6, pinned = ?7, sensitive = ?8,
                updated_at = ?9, revision = revision + 1,
                content_norm = ?10, aliases_norm = ?11, tags_norm = ?12
             WHERE id = ?13 AND revision = ?14",
                    params![
                        fields.key,
                        fields.normalized_key,
                        fields.title,
                        fields.content,
                        aliases,
                        tags,
                        fields.pinned,
                        fields.sensitive,
                        now,
                        fields.content.to_lowercase(),
                        fields.aliases.join(" ").to_lowercase(),
                        fields.tags.join(" ").to_lowercase(),
                        id,
                        revision
                    ],
                )
                .map_err(|error| {
                    map_write_error(error, &transaction, &fields.normalized_key, Some(id))
                })?;
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
                        "片段已在其他窗口修改。请重新载入后再保存。",
                    )
                } else {
                    AppError::new("NOT_FOUND", "片段不存在。请重新载入列表。")
                });
            }
            let snippet = get_in_transaction(&transaction, id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            snippet
        };
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .upsert(&snippet);
        Ok(snippet)
    }

    pub fn search(&self, query: &str, limit: i64) -> Result<SearchResponse, AppError> {
        let mut index = self
            .search_index
            .lock()
            .map_err(|_| db_open_error("数据库当前不可用。请重启应用。"))?;
        if index.is_empty() {
            let connection = self
                .connection
                .lock()
                .map_err(|_| db_open_error("数据库当前不可用。请重启应用。"))?;
            index.rebuild(&connection)?;
        }
        Ok(index.search(query, limit))
    }

    pub fn is_operation_processed(&self, operation_id: &str) -> Result<bool, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| db_open_error("数据库当前不可用。请重启应用。"))?;
        connection
            .query_row(
                "SELECT 1 FROM operation_requests WHERE operation_id = ?1",
                params![operation_id],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(|_| db_open_error("无法检查操作幂等状态。请重启应用。"))
    }

    pub fn record_usage(
        &self,
        snippet_id: &str,
        operation_id: &str,
    ) -> Result<(Snippet, bool), AppError> {
        let (snippet, counted) = {
            let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
            let transaction = connection.transaction().map_err(|_| db_write_error())?;
            let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let inserted = transaction
                .execute(
                    "INSERT OR IGNORE INTO operation_requests (operation_id, snippet_id, created_at) VALUES (?1, ?2, ?3)",
                    params![operation_id, snippet_id, now],
                )
                .map_err(|_| db_write_error())?;
            let counted = inserted > 0;
            if counted {
                transaction
                    .execute(
                        "UPDATE snippets SET usage_count = usage_count + 1,
                            last_used_at = ?1, updated_at = ?1, revision = revision + 1
                         WHERE id = ?2",
                        params![now, snippet_id],
                    )
                    .map_err(|_| db_write_error())?;
            }
            let snippet =
                get_in_transaction(&transaction, snippet_id)?.ok_or_else(db_write_error)?;
            transaction.commit().map_err(|_| db_write_error())?;
            (snippet, counted)
        };
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .upsert(&snippet);
        Ok((snippet, counted))
    }

    #[cfg(test)]
    pub(crate) fn raw_connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(test)]
    pub(crate) fn refresh_search_index(&self) -> Result<(), AppError> {
        let mut index = self.search_index.lock().map_err(|_| db_write_error())?;
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        index.rebuild(&connection)
    }

    #[cfg(test)]
    pub(crate) fn mark_deleted(&self, id: &str) -> Result<(), AppError> {
        self.connection
            .lock()
            .map_err(|_| db_write_error())?
            .execute(
                "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), id],
            )
            .map(|_| ())
            .map_err(|_| db_write_error())
    }

    pub fn get(&self, id: &str) -> Result<Option<Snippet>, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| db_open_error("数据库当前不可用。请重启应用。"))?;
        query_snippet(&connection, id)
    }

    pub fn list(&self) -> Result<Vec<Snippet>, AppError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| db_open_error("数据库当前不可用。请重启应用。"))?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {SNIPPET_COLUMNS} FROM snippets ORDER BY updated_at DESC, key ASC"
            ))
            .map_err(|_| db_open_error("无法读取片段列表。请检查数据库文件。"))?;
        let rows = statement
            .query_map([], row_to_snippet)
            .map_err(|_| db_open_error("无法读取片段列表。请检查数据库文件。"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| db_open_error("片段数据损坏或无法解码。请从备份恢复。"))
    }

    // === SPEC-06: Settings ===

    pub fn get_settings(&self) -> Result<(crate::model::Settings, i64), AppError> {
        // 先尝试读取，避免重复持锁
        let result = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| db_open_error("数据库当前不可用。请重启应用。"))?;
            connection
                .query_row(
                    "SELECT data, revision FROM settings WHERE id = 1",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(|_| db_open_error("无法读取设置。请重启应用。"))?
        }; // 锁在此处释放
        match result {
            Some((data, rev)) => {
                let settings: crate::model::Settings = serde_json::from_str(&data)
                    .map_err(|_| db_open_error("设置数据损坏。请重新配置。"))?;
                Ok((settings, rev))
            }
            None => {
                let settings = crate::model::Settings::default();
                let data = serde_json::to_string(&settings).map_err(|_| db_write_error())?;
                {
                    let conn = self.connection.lock().map_err(|_| db_write_error())?;
                    conn.execute(
                        "INSERT INTO settings (id, data, revision) VALUES (1, ?1, 1)",
                        params![data],
                    )
                    .map_err(|_| db_write_error())?;
                }
                Ok((settings, 1))
            }
        }
    }

    pub fn update_settings(
        &self,
        settings: &crate::model::Settings,
        expected_revision: i64,
    ) -> Result<i64, AppError> {
        let data = serde_json::to_string(settings).map_err(|_| db_write_error())?;
        let connection = self.connection.lock().map_err(|_| db_write_error())?;
        let changed = connection
            .execute(
                "UPDATE settings SET data = ?1, revision = revision + 1 WHERE id = 1 AND revision = ?2",
                params![data, expected_revision],
            )
            .map_err(|_| db_write_error())?;
        if changed == 0 {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "设置已被其他窗口修改。请重新加载。",
            ));
        }
        let new_rev: i64 = connection
            .query_row("SELECT revision FROM settings WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(|_| db_write_error())?;
        Ok(new_rev)
    }

    // === SPEC-05: 回收站操作 ===

    /// 软删除：设置 deletedAt + bump revision。单事务。
    pub fn trash_move(&self, id: &str) -> Result<Snippet, AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        let snippet = get_in_transaction(&transaction, id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "片段不存在。"))?;
        if snippet.deleted_at.is_some() {
            transaction.commit().map_err(|_| db_write_error())?;
            return Err(AppError::new("ALREADY_TRASHED", "片段已在回收站中。"));
        }
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        transaction
            .execute(
                "UPDATE snippets SET deleted_at = ?1, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                params![now, id],
            )
            .map_err(|_| db_write_error())?;
        let updated = get_in_transaction(&transaction, id)?.ok_or_else(db_write_error)?;
        transaction.commit().map_err(|_| db_write_error())?;
        // 事务提交成功后定向同步索引（updated 含正确 deleted_at）。
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .upsert(&updated);
        Ok(updated)
    }

    /// 还原：清除 deletedAt，检查 Key 冲突，bump revision。单事务。
    pub fn trash_restore(&self, id: &str) -> Result<Snippet, AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        let snippet = get_in_transaction(&transaction, id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "片段不存在。"))?;
        if snippet.deleted_at.is_none() {
            transaction.commit().map_err(|_| db_write_error())?;
            return Err(AppError::new("NOT_TRASHED", "片段不在回收站中。"));
        }
        // 检查 normalizedKey 与当前活跃记录冲突
        if let Some(key) = find_conflict_key(&transaction, &snippet.normalized_key, Some(id))? {
            transaction.commit().map_err(|_| db_write_error())?;
            return Err(AppError::key_conflict(key));
        }
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        transaction
            .execute(
                "UPDATE snippets SET deleted_at = NULL, updated_at = ?1, revision = revision + 1 WHERE id = ?2",
                params![now, id],
            )
            .map_err(|_| db_write_error())?;
        let updated = get_in_transaction(&transaction, id)?.ok_or_else(db_write_error)?;
        transaction.commit().map_err(|_| db_write_error())?;
        // 事务提交成功后定向同步索引（updated 含 deleted_at: None）。
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .upsert(&updated);
        Ok(updated)
    }

    /// 永久删除单条。物理 DELETE。单事务。
    pub fn trash_purge_one(&self, id: &str) -> Result<(), AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        let snippet = get_in_transaction(&transaction, id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "片段不存在。"))?;
        if snippet.deleted_at.is_none() {
            transaction.commit().map_err(|_| db_write_error())?;
            return Err(AppError::new(
                "NOT_TRASHED",
                "片段不在回收站中，无法永久删除。",
            ));
        }
        // 先删除引用，再删除片段
        transaction
            .execute(
                "DELETE FROM create_requests WHERE snippet_id = ?1",
                params![id],
            )
            .map_err(|_| db_write_error())?;
        transaction
            .execute("DELETE FROM snippets WHERE id = ?1", params![id])
            .map_err(|_| db_write_error())?;
        transaction.commit().map_err(|_| db_write_error())?;
        // 事务提交成功后从索引定向移除该 id。
        self.search_index
            .lock()
            .map_err(|_| db_write_error())?
            .remove(id);
        Ok(())
    }

    /// 清空回收站：永久删除全部 deleted_at IS NOT NULL 的记录。单事务。
    pub fn trash_empty(&self) -> Result<i64, AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        // 提交前先收集待删除 id，供删行后定向同步索引。
        let ids = collect_snippet_ids(&transaction, "deleted_at IS NOT NULL", params![])?;
        // 先删除 create_requests 中引用待删除片段的行，再删除 snippets
        transaction
            .execute(
                "DELETE FROM create_requests WHERE snippet_id IN (SELECT id FROM snippets WHERE deleted_at IS NOT NULL)",
                [],
            )
            .map_err(|e| {
                eprintln!("trash_empty delete create_requests error: {e}");
                db_write_error()
            })?;
        let count = transaction
            .execute("DELETE FROM snippets WHERE deleted_at IS NOT NULL", [])
            .map_err(|e| {
                eprintln!("trash_empty DELETE snippets error: {e}");
                db_write_error()
            })? as i64;
        transaction.commit().map_err(|e| {
            eprintln!("trash_empty COMMIT error: {e}");
            db_write_error()
        })?;
        // 事务提交成功后从索引定向移除全部实际被删 id。
        let mut index = self.search_index.lock().map_err(|_| db_write_error())?;
        for id in ids {
            index.remove(&id);
        }
        Ok(count)
    }

    /// 自动清理：永久删除 deleted_at <= threshold 的记录。单事务。
    pub fn auto_purge(&self, threshold: &str) -> Result<i64, AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        // 提交前先收集待删除 id，供删行后定向同步索引。
        let ids = collect_snippet_ids(
            &transaction,
            "deleted_at IS NOT NULL AND deleted_at <= ?1",
            params![threshold],
        )?;
        transaction
            .execute(
                "DELETE FROM create_requests WHERE snippet_id IN (SELECT id FROM snippets WHERE deleted_at IS NOT NULL AND deleted_at <= ?1)",
                params![threshold],
            )
            .map_err(|_| db_write_error())?;
        let count = transaction
            .execute(
                "DELETE FROM snippets WHERE deleted_at IS NOT NULL AND deleted_at <= ?1",
                params![threshold],
            )
            .map_err(|_| db_write_error())? as i64;
        transaction.commit().map_err(|_| db_write_error())?;
        // 事务提交成功后从索引定向移除全部实际被删 id。
        let mut index = self.search_index.lock().map_err(|_| db_write_error())?;
        for id in ids {
            index.remove(&id);
        }
        Ok(count)
    }

    // === SPEC-07: 导入 ===

    pub fn import_snippets(
        &self,
        snippets_raw: &[serde_json::Value],
        _import_token: &str,
    ) -> Result<crate::model::ImportOutcome, AppError> {
        let mut connection = self.connection.lock().map_err(|_| db_write_error())?;
        let transaction = connection.transaction().map_err(|_| db_write_error())?;
        let mut inserted = 0i64;
        let mut updated = 0i64;
        for item in snippets_raw {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::new("IMPORT_SCHEMA_INVALID", "片段缺少 id 字段。"))?;
            let key = item
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::new("IMPORT_SCHEMA_INVALID", "片段缺少 key 字段。"))?;
            let content = item
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::new("IMPORT_SCHEMA_INVALID", "片段缺少 content 字段。"))?;
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or(key);
            let aliases: Vec<String> = serde_json::from_value(
                item.get("aliases")
                    .cloned()
                    .unwrap_or(serde_json::json!([])),
            )
            .unwrap_or_default();
            let tags: Vec<String> =
                serde_json::from_value(item.get("tags").cloned().unwrap_or(serde_json::json!([])))
                    .unwrap_or_default();
            let pinned = item
                .get("pinned")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let sensitive = item
                .get("sensitive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let deleted_at = item
                .get("deletedAt")
                .and_then(|v| v.as_str())
                .map(String::from);
            let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let aliases_json = serde_json::to_string(&aliases).unwrap_or_default();
            let tags_json = serde_json::to_string(&tags).unwrap_or_default();
            let content_norm = content.to_lowercase();
            let aliases_norm = aliases.join(" ").to_lowercase();
            let tags_norm = tags.join(" ").to_lowercase();
            let exists: Option<String> = transaction
                .query_row(
                    "SELECT id FROM snippets WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| db_write_error())?;
            if let Some(_existing_id) = exists {
                // 按 ID 覆盖（更新）
                transaction
                    .execute(
                        "UPDATE snippets SET key = ?1, title = ?2, content = ?3, aliases = ?4, tags = ?5,
                         pinned = ?6, sensitive = ?7, deleted_at = ?8, updated_at = ?9, revision = revision + 1,
                         content_norm = ?10, aliases_norm = ?11, tags_norm = ?12 WHERE id = ?13",
                        params![
                            key, title, content, aliases_json, tags_json,
                            pinned, sensitive, deleted_at, now,
                            content_norm, aliases_norm, tags_norm, id,
                        ],
                    )
                    .map_err(|_| db_write_error())?;
                updated += 1;
            } else {
                // 新增
                let normalized_key = key.to_lowercase();
                transaction
                    .execute(
                        "INSERT INTO snippets (
                            id, key, normalized_key, title, content, aliases, tags, pinned, sensitive,
                            usage_count, last_used_at, created_at, updated_at, deleted_at, revision,
                            content_norm, aliases_norm, tags_norm
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, NULL, ?10, ?10, ?11, 1, ?12, ?13, ?14)",
                        params![
                            id, key, normalized_key, title, content, aliases_json, tags_json,
                            pinned, sensitive, now, deleted_at,
                            content_norm, aliases_norm, tags_norm,
                        ],
                    )
                    .map_err(|_| db_write_error())?;
                inserted += 1;
            }
        }
        transaction.commit().map_err(|_| db_write_error())?;
        Ok(crate::model::ImportOutcome { inserted, updated })
    }

    // === SPEC-07: 重置 ===

    pub fn reset_to_examples(&self) -> Result<i64, AppError> {
        // 示例数据：至少 3 条片段
        let examples = vec![
            serde_json::json!({
                "id": "ex-1",
                "key": "hello-searchis",
                "title": "Searchis 问候",
                "content": "你好！很高兴认识你。这是我通过 Searchis 快速粘贴的第一条文本片段。",
                "aliases": [],
                "tags": ["示例"],
                "pinned": true,
                "sensitive": false,
            }),
            serde_json::json!({
                "id": "ex-2",
                "key": "email-work",
                "title": "工作邮箱",
                "content": "example@work.com",
                "aliases": ["工作邮箱"],
                "tags": ["工作"],
                "pinned": false,
                "sensitive": true,
            }),
            serde_json::json!({
                "id": "ex-3",
                "key": "addr-home",
                "title": "家庭地址",
                "content": "XX市XX区XX街道",
                "aliases": [],
                "tags": ["地址", "生活"],
                "pinned": false,
                "sensitive": true,
            }),
        ];
        self.import_snippets(&examples, "reset")
            .map(|o| o.inserted + o.updated)
    }
}

const SNIPPET_COLUMNS: &str = "id, key, normalized_key, title, content, aliases, tags, pinned, sensitive, usage_count, last_used_at, created_at, updated_at, deleted_at, revision";

/// 在事务内收集按给定 WHERE 子句匹配的待删除 snippet id，供提交后定向同步索引。
/// `condition` 从 "WHERE " 之后开始，`params` 必须与占位符一一对应。
fn collect_snippet_ids<P: rusqlite::Params>(
    transaction: &Transaction<'_>,
    condition: &str,
    params: P,
) -> Result<Vec<String>, AppError> {
    let sql = format!("SELECT id FROM snippets WHERE {condition}");
    let mut statement = transaction.prepare(&sql).map_err(|_| db_write_error())?;
    let rows = statement
        .query_map(params, |row| row.get::<_, String>(0))
        .map_err(|_| db_write_error())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| db_write_error())
}

fn query_snippet(connection: &Connection, id: &str) -> Result<Option<Snippet>, AppError> {
    connection
        .query_row(
            &format!("SELECT {SNIPPET_COLUMNS} FROM snippets WHERE id = ?1"),
            params![id],
            row_to_snippet,
        )
        .optional()
        .map_err(|_| db_open_error("无法读取指定片段。请重新载入。"))
}

fn get_in_transaction(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<Option<Snippet>, AppError> {
    transaction
        .query_row(
            &format!("SELECT {SNIPPET_COLUMNS} FROM snippets WHERE id = ?1"),
            params![id],
            row_to_snippet,
        )
        .optional()
        .map_err(|_| db_write_error())
}

fn row_to_snippet(row: &Row<'_>) -> rusqlite::Result<Snippet> {
    let aliases: String = row.get(5)?;
    let tags: String = row.get(6)?;
    let aliases = serde_json::from_str(&aliases).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, Type::Text, Box::new(error))
    })?;
    let tags = serde_json::from_str(&tags).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(6, Type::Text, Box::new(error))
    })?;
    Ok(Snippet {
        id: row.get(0)?,
        key: row.get(1)?,
        normalized_key: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        aliases,
        tags,
        pinned: row.get(7)?,
        sensitive: row.get(8)?,
        usage_count: row.get(9)?,
        last_used_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        deleted_at: row.get(13)?,
        revision: row.get(14)?,
    })
}

fn find_conflict_key(
    transaction: &Transaction<'_>,
    normalized_key: &str,
    excluding_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    transaction
        .query_row(
            "SELECT key FROM snippets WHERE normalized_key = ?1 AND (?2 IS NULL OR id <> ?2)",
            params![normalized_key, excluding_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| db_write_error())
}

fn map_write_error(
    error: rusqlite::Error,
    transaction: &Transaction<'_>,
    normalized_key: &str,
    excluding_id: Option<&str>,
) -> AppError {
    if matches!(error, rusqlite::Error::SqliteFailure(ref value, _) if value.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE)
        || matches!(error, rusqlite::Error::SqliteFailure(ref value, _) if value.code == rusqlite::ErrorCode::ConstraintViolation)
    {
        if let Ok(Some(key)) = find_conflict_key(transaction, normalized_key, excluding_id) {
            return AppError::key_conflict(key);
        }
    }
    db_write_error()
}

fn fingerprint_fields(fields: &ValidatedFields) -> Result<String, AppError> {
    let serialized = serde_json::to_vec(fields).map_err(|_| db_write_error())?;
    Ok(hex::encode(Sha256::digest(serialized)))
}

fn apply_key(connection: &Connection, key: &[u8]) -> Result<(), AppError> {
    let key_literal = format!("x'{}'", hex::encode(key));
    connection
        .pragma_update(None, "key", key_literal)
        .map_err(|_| db_open_error("无法应用数据库密钥。请恢复正确密钥后重试。"))
}

fn verify_cipher(connection: &Connection) -> Result<(), AppError> {
    let cipher_version: Option<String> = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .optional()
        .map_err(|_| db_open_error("SQLCipher 初始化失败。禁止使用明文数据库。"))?;
    if cipher_version.as_deref().unwrap_or_default().is_empty() {
        return Err(db_open_error(
            "当前构建未启用 SQLCipher，已拒绝创建明文数据库。",
        ));
    }
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| db_open_error("数据库密钥错误或数据库文件已损坏。请恢复正确密钥或备份。"))
}

fn migrate(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|_| db_open_error("无法启动数据库迁移事务。原数据未修改。"))?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS metadata (
            schema_version INTEGER NOT NULL CHECK (schema_version >= 1)
         );
         CREATE TABLE IF NOT EXISTS snippets (
            id TEXT PRIMARY KEY NOT NULL,
            key TEXT NOT NULL,
            normalized_key TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            aliases TEXT NOT NULL,
            tags TEXT NOT NULL,
            pinned INTEGER NOT NULL CHECK (pinned IN (0, 1)),
            sensitive INTEGER NOT NULL CHECK (sensitive IN (0, 1)),
            usage_count INTEGER NOT NULL DEFAULT 0 CHECK (usage_count >= 0),
            last_used_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL CHECK (updated_at >= created_at),
            deleted_at TEXT,
            revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
            content_norm TEXT NOT NULL DEFAULT '',
            aliases_norm TEXT NOT NULL DEFAULT '',
            tags_norm TEXT NOT NULL DEFAULT ''
         );
         CREATE UNIQUE INDEX IF NOT EXISTS snippets_normalized_key_unique ON snippets(normalized_key);
         CREATE TABLE IF NOT EXISTS create_requests (
            request_id TEXT PRIMARY KEY NOT NULL,
            snippet_id TEXT NOT NULL REFERENCES snippets(id),
            request_fingerprint TEXT NOT NULL,
            created_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS operation_requests (
            operation_id TEXT PRIMARY KEY NOT NULL,
            snippet_id TEXT NOT NULL,
            created_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            data TEXT NOT NULL DEFAULT '{}',
            revision INTEGER NOT NULL DEFAULT 1
         );"
    ).map_err(|_| db_open_error("数据库 Schema 迁移失败。原数据未修改。"))?;
    let version: Option<i64> = transaction
        .query_row("SELECT schema_version FROM metadata LIMIT 1", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|_| db_open_error("无法读取数据库 Schema 版本。"))?;
    match version {
        None => {
            transaction
                .execute(
                    "INSERT INTO metadata(schema_version) VALUES (?1)",
                    params![SCHEMA_VERSION],
                )
                .map_err(|_| db_open_error("无法写入数据库 Schema 版本。"))?;
        }
        Some(1) => {
            migrate_v1_request_fingerprints(&transaction)?;
            migrate_v2_to_v3(&transaction)?;
        }
        Some(2) => migrate_v2_to_v3(&transaction)?,
        Some(SCHEMA_VERSION) => {}
        Some(_) => {
            return Err(db_open_error(
                "数据库 Schema 版本不受支持。请升级应用或恢复兼容备份。",
            ))
        }
    }
    transaction
        .commit()
        .map_err(|_| db_open_error("数据库 Schema 迁移提交失败。原数据未修改。"))
}

fn migrate_v1_request_fingerprints(transaction: &Transaction<'_>) -> Result<(), AppError> {
    transaction
        .execute_batch(
            "ALTER TABLE create_requests
             ADD COLUMN request_fingerprint TEXT NOT NULL DEFAULT '';",
        )
        .map_err(|_| db_open_error("Schema v1 幂等记录迁移失败。原数据未修改。"))?;
    let requests = {
        let mut statement = transaction
            .prepare("SELECT request_id, snippet_id FROM create_requests")
            .map_err(|_| db_open_error("无法读取 Schema v1 幂等记录。原数据未修改。"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| db_open_error("无法读取 Schema v1 幂等记录。原数据未修改。"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| db_open_error("Schema v1 幂等记录损坏。原数据未修改。"))?
    };
    for (request_id, snippet_id) in requests {
        let snippet = get_in_transaction(transaction, &snippet_id)
            .map_err(|_| db_open_error("Schema v1 幂等记录引用无效。原数据未修改。"))?
            .ok_or_else(|| db_open_error("Schema v1 幂等记录引用缺失。原数据未修改。"))?;
        let fields = ValidatedFields {
            key: snippet.key,
            normalized_key: snippet.normalized_key,
            title: snippet.title,
            content: snippet.content,
            aliases: snippet.aliases,
            tags: snippet.tags,
            sensitive: snippet.sensitive,
            pinned: snippet.pinned,
        };
        let fingerprint = fingerprint_fields(&fields)
            .map_err(|_| db_open_error("无法迁移 Schema v1 幂等指纹。原数据未修改。"))?;
        transaction
            .execute(
                "UPDATE create_requests SET request_fingerprint = ?1 WHERE request_id = ?2",
                params![fingerprint, request_id],
            )
            .map_err(|_| db_open_error("无法写入 Schema v1 幂等指纹。原数据未修改。"))?;
    }
    transaction
        .execute(
            "UPDATE metadata SET schema_version = ?1",
            params![SCHEMA_VERSION],
        )
        .map_err(|_| db_open_error("无法更新数据库 Schema 版本。原数据未修改。"))?;
    Ok(())
}

fn migrate_v2_to_v3(transaction: &Transaction<'_>) -> Result<(), AppError> {
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS operation_requests (
                operation_id TEXT PRIMARY KEY NOT NULL,
                snippet_id TEXT NOT NULL,
                created_at TEXT NOT NULL
             );",
        )
        .map_err(|_| db_open_error("Schema v2 操作幂等表迁移失败。原数据未修改。"))?;
    for (column, table) in [
        ("content_norm", "snippets"),
        ("aliases_norm", "snippets"),
        ("tags_norm", "snippets"),
    ] {
        if !column_exists(transaction, table, column)? {
            transaction
                .execute_batch(&format!(
                    "ALTER TABLE {table} ADD COLUMN {column} TEXT NOT NULL DEFAULT '';"
                ))
                .map_err(|_| db_open_error("Schema v2 检索列迁移失败。原数据未修改。"))?;
        }
    }
    backfill_norm_columns(transaction)?;
    transaction
        .execute(
            "UPDATE metadata SET schema_version = ?1",
            params![SCHEMA_VERSION],
        )
        .map_err(|_| db_open_error("无法更新数据库 Schema 版本。原数据未修改。"))?;
    Ok(())
}

fn column_exists(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
) -> Result<bool, AppError> {
    let mut statement = transaction
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|_| db_open_error("无法检查数据库表结构。原数据未修改。"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| db_open_error("无法检查数据库表结构。原数据未修改。"))?;
    for name in columns {
        if name.map_err(|_| db_open_error("无法检查数据库表结构。原数据未修改。"))? == column
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn backfill_norm_columns(transaction: &Transaction<'_>) -> Result<(), AppError> {
    let rows: Vec<(String, String, String, String)> = {
        let mut statement = transaction
            .prepare("SELECT id, content, aliases, tags FROM snippets")
            .map_err(|_| db_open_error("无法读取片段以回填检索列。原数据未修改。"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| db_open_error("无法读取片段以回填检索列。原数据未修改。"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| db_open_error("检索列回填数据损坏。原数据未修改。"))?
    };
    for (id, content, aliases, tags) in rows {
        let aliases_norm = serde_json::from_str::<Vec<String>>(&aliases)
            .unwrap_or_default()
            .join(" ")
            .to_lowercase();
        let tags_norm = serde_json::from_str::<Vec<String>>(&tags)
            .unwrap_or_default()
            .join(" ")
            .to_lowercase();
        transaction
            .execute(
                "UPDATE snippets SET content_norm = ?1, aliases_norm = ?2, tags_norm = ?3 WHERE id = ?4",
                params![content.to_lowercase(), aliases_norm, tags_norm, id],
            )
            .map_err(|_| db_open_error("检索列回填写入失败。原数据未修改。"))?;
    }
    Ok(())
}

fn load_or_create_key(path: &Path, database_exists: bool) -> Result<Vec<u8>, AppError> {
    if path.exists() {
        return read_key(path);
    }
    if database_exists {
        return Err(key_error(
            "数据库密钥文件缺失。为避免数据丢失，未打开数据库；请恢复原 database.key。",
        ));
    }
    create_key(path)
}

fn read_key(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| key_error("无法安全打开数据库密钥。请检查文件是否为符号链接及其权限。"))?;
    let metadata = file
        .metadata()
        .map_err(|_| key_error("无法检查已打开的数据库密钥。请修复文件权限。"))?;
    if !metadata.is_file() {
        return Err(key_error("数据库密钥必须是普通文件。"));
    }
    if metadata.permissions().mode() & 0o777 != 0o600
        || metadata.uid() != unsafe { libc::geteuid() }
    {
        return Err(key_error(
            "数据库密钥权限或所有者不安全。请将所有者改为当前用户并执行 chmod 600 database.key。",
        ));
    }
    let mut bytes = Vec::with_capacity(KEY_BYTES);
    file.read_to_end(&mut bytes)
        .map_err(|_| key_error("无法读取数据库密钥。请检查文件权限或恢复密钥。"))?;
    if bytes.len() != KEY_BYTES {
        return Err(key_error("数据库密钥长度无效。请恢复原始 database.key。"));
    }
    Ok(bytes)
}

fn create_key(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut key = vec![0u8; KEY_BYTES];
    rand::rngs::OsRng
        .try_fill_bytes(&mut key)
        .map_err(|_| key_error("系统随机数生成失败，未创建数据库。请检查操作系统后重试。"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| key_error("无法原子创建数据库密钥。请检查数据目录中是否已有异常文件。"))?;
    if file.write_all(&key).and_then(|_| file.sync_all()).is_err() {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(key_error(
            "数据库密钥写入失败且已清理不完整文件。请检查磁盘空间后重试。",
        ));
    }
    drop(file);
    let parent = path
        .parent()
        .ok_or_else(|| key_error("数据库密钥目录无效，未创建数据库。"))?;
    if File::open(parent)
        .and_then(|directory| directory.sync_all())
        .is_err()
    {
        let _ = fs::remove_file(path);
        return Err(key_error(
            "无法持久化数据库密钥目录项，已清理密钥且未创建数据库。请检查文件系统。",
        ));
    }
    read_key(path)
}

fn key_error(message: &str) -> AppError {
    AppError::new("DB_KEY_UNAVAILABLE", message)
}
fn db_open_error(message: &str) -> AppError {
    AppError::new("DB_OPEN_FAILED", message)
}
fn db_write_error() -> AppError {
    AppError::new(
        "DB_WRITE_FAILED",
        "保存数据库失败，输入已保留。请检查磁盘空间和数据目录权限后重试。",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CreateSnippetInput;
    use crate::validation::validate_create;

    fn input(request_id: String, key: &str) -> CreateSnippetInput {
        CreateSnippetInput {
            key: key.into(),
            title: "标题".into(),
            content: "line 1\n😀".into(),
            aliases: vec![],
            tags: vec![],
            sensitive: false,
            pinned: false,
            request_id,
        }
    }

    #[test]
    fn encrypted_database_reopens_and_plain_sqlite_cannot_read_it() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        repository
            .create(
                validate_create(&input(request_id.clone(), "Hello World")).unwrap(),
                &request_id,
            )
            .unwrap();
        drop(repository);
        assert_eq!(
            Repository::open(directory.path())
                .unwrap()
                .list()
                .unwrap()
                .len(),
            1
        );

        let plain_sqlite = std::process::Command::new("sqlite3")
            .arg(directory.path().join("searchis.db"))
            .arg("SELECT count(*) FROM snippets;")
            .output()
            .unwrap();
        assert!(!plain_sqlite.status.success());
    }

    #[test]
    fn stores_database_and_key_in_separate_directories() {
        let data = tempfile::tempdir().unwrap();
        let config = tempfile::tempdir().unwrap();
        drop(Repository::open_with_paths(data.path(), config.path()).unwrap());
        assert!(data.path().join("searchis.db").is_file());
        assert!(!data.path().join("database.key").exists());
        assert!(config.path().join("database.key").is_file());
    }

    #[test]
    fn rejects_missing_or_wide_permission_key_without_replacing_database() {
        let directory = tempfile::tempdir().unwrap();
        drop(Repository::open(directory.path()).unwrap());
        let database = directory.path().join("searchis.db");
        let original_length = fs::metadata(&database).unwrap().len();
        fs::remove_file(directory.path().join("database.key")).unwrap();
        assert_eq!(
            Repository::open(directory.path()).unwrap_err().code,
            "DB_KEY_UNAVAILABLE"
        );
        assert_eq!(fs::metadata(&database).unwrap().len(), original_length);

        let second = tempfile::tempdir().unwrap();
        drop(Repository::open(second.path()).unwrap());
        fs::set_permissions(
            second.path().join("database.key"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert_eq!(
            Repository::open(second.path()).unwrap_err().code,
            "DB_KEY_UNAVAILABLE"
        );

        let third = tempfile::tempdir().unwrap();
        let target = third.path().join("target.key");
        fs::write(&target, [0xA5; KEY_BYTES]).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        std::os::unix::fs::symlink(&target, third.path().join("database.key")).unwrap();
        assert_eq!(
            Repository::open(third.path()).unwrap_err().code,
            "DB_KEY_UNAVAILABLE"
        );
    }

    #[test]
    fn wrong_key_is_rejected_without_replacing_database() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        repository
            .create(
                validate_create(&input(request_id.clone(), "protected")).unwrap(),
                &request_id,
            )
            .unwrap();
        drop(repository);
        let database = directory.path().join("searchis.db");
        let original = fs::read(&database).unwrap();
        fs::write(directory.path().join("database.key"), [0xA5; KEY_BYTES]).unwrap();
        assert_eq!(
            Repository::open(directory.path()).unwrap_err().code,
            "DB_OPEN_FAILED"
        );
        assert_eq!(fs::read(database).unwrap(), original);
    }

    #[test]
    fn create_is_idempotent_and_unique_key_includes_soft_deleted_rows() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let first_input = input(request_id.clone(), "Hello World");
        let first = repository
            .create(validate_create(&first_input).unwrap(), &request_id)
            .unwrap();
        let again = repository
            .create(validate_create(&first_input).unwrap(), &request_id)
            .unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(repository.list().unwrap().len(), 1);

        let mut changed_input = first_input.clone();
        changed_input.title = "不同标题".into();
        let mismatch = repository
            .create(validate_create(&changed_input).unwrap(), &request_id)
            .unwrap_err();
        assert_eq!(mismatch.code, "VALIDATION_FAILED");
        assert_eq!(repository.list().unwrap()[0].title, first.title);

        repository
            .connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), first.id],
            )
            .unwrap();
        let conflict_request = uuid::Uuid::new_v4().to_string();
        let error = repository
            .create(
                validate_create(&input(conflict_request.clone(), "HELLO   WORLD")).unwrap(),
                &conflict_request,
            )
            .unwrap_err();
        assert_eq!(error.code, "KEY_CONFLICT");
        assert_eq!(repository.list().unwrap().len(), 1);
    }

    #[test]
    fn failed_create_transaction_rolls_back_snippet_insert() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER reject_create_request BEFORE INSERT ON create_requests
                 BEGIN SELECT RAISE(ABORT, 'simulated write failure'); END;",
            )
            .unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let error = repository
            .create(
                validate_create(&input(request_id.clone(), "rollback")).unwrap(),
                &request_id,
            )
            .unwrap_err();
        assert_eq!(error.code, "DB_WRITE_FAILED");
        assert!(repository.list().unwrap().is_empty());
    }

    #[test]
    fn failed_update_transaction_keeps_original_record() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let created = repository
            .create(
                validate_create(&input(request_id.clone(), "update-rollback")).unwrap(),
                &request_id,
            )
            .unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER reject_snippet_update BEFORE UPDATE ON snippets
                 BEGIN SELECT RAISE(ABORT, 'simulated update failure'); END;",
            )
            .unwrap();
        let mut fields =
            validate_create(&input(uuid::Uuid::new_v4().to_string(), "changed")).unwrap();
        fields.title = "should rollback".into();
        assert_eq!(
            repository
                .update(&created.id, created.revision, fields)
                .unwrap_err()
                .code,
            "DB_WRITE_FAILED"
        );
        assert_eq!(repository.get(&created.id).unwrap().unwrap(), created);
    }

    #[test]
    fn migrates_legacy_v1_request_fingerprints_and_preserves_idempotency() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let original_input = input(request_id.clone(), "legacy");
        let created = repository
            .create(validate_create(&original_input).unwrap(), &request_id)
            .unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "ALTER TABLE create_requests DROP COLUMN request_fingerprint;
                 UPDATE metadata SET schema_version = 1;",
            )
            .unwrap();
        drop(repository);

        let migrated = Repository::open(directory.path()).unwrap();
        let retried = migrated
            .create(validate_create(&original_input).unwrap(), &request_id)
            .unwrap();
        assert_eq!(retried.id, created.id);
        assert_eq!(migrated.list().unwrap().len(), 1);
    }

    #[test]
    fn unsupported_migration_version_preserves_database_file() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        repository
            .connection
            .lock()
            .unwrap()
            .execute("UPDATE metadata SET schema_version = ?1", params![99])
            .unwrap();
        drop(repository);
        let database = directory.path().join("searchis.db");
        let before = fs::read(&database).unwrap();
        assert_eq!(
            Repository::open(directory.path()).unwrap_err().code,
            "DB_OPEN_FAILED"
        );
        assert_eq!(fs::read(database).unwrap(), before);
    }

    // === QA MT1: multiline + emoji persistence through restart + edit ===
    #[test]
    fn qa_mt1_multiline_emoji_persists_and_edits_across_restart() {
        let directory = tempfile::tempdir().unwrap();
        let content = "第一行：你好世界\n第二行：😀🎉🔥\nThird line: ASCII\n\t第四行带 tab 缩进\n第五行：émojì 测试 💾";
        let request_id = uuid::Uuid::new_v4().to_string();
        let create_input = CreateSnippetInput {
            key: "mt1-emoji-test".into(),
            title: "MT1 持久化验证 😀".into(),
            content: content.into(),
            aliases: vec!["emoji".into(), "多行".into()],
            tags: vec!["qa".into(), "验收".into()],
            sensitive: true,
            pinned: true,
            request_id: request_id.clone(),
        };
        let fields = validate_create(&create_input).unwrap();

        // Act 1: create
        let repo1 = Repository::open(directory.path()).unwrap();
        let created = repo1.create(fields, &request_id).unwrap();
        assert_eq!(created.key, "mt1-emoji-test");
        assert_eq!(created.normalized_key, "mt1-emoji-test");
        assert_eq!(created.title, "MT1 持久化验证 😀");
        assert_eq!(created.content, content);
        assert_eq!(created.aliases, vec!["emoji", "多行"]);
        assert_eq!(created.tags, vec!["qa", "验收"]);
        assert!(created.sensitive);
        assert!(created.pinned);
        assert_eq!(created.revision, 1);
        assert_eq!(created.usage_count, 0);
        assert!(created.last_used_at.is_none());
        let original_created_at = created.created_at.clone();
        let original_updated_at = created.updated_at.clone();
        let original_id = created.id.clone();
        let record_count = repo1.list().unwrap().len();
        assert_eq!(record_count, 1);
        drop(repo1); // simulate full exit

        // Act 2: restart and read
        let repo2 = Repository::open(directory.path()).unwrap();
        let reloaded = repo2.get(&original_id).unwrap().unwrap();
        assert_eq!(reloaded.key, "mt1-emoji-test");
        assert_eq!(reloaded.title, "MT1 持久化验证 😀");
        assert_eq!(
            reloaded.content, content,
            "multiline+emoji content must survive restart"
        );
        assert_eq!(reloaded.aliases, vec!["emoji", "多行"]);
        assert_eq!(reloaded.tags, vec!["qa", "验收"]);
        assert!(reloaded.sensitive);
        assert!(reloaded.pinned);
        assert_eq!(reloaded.revision, 1);
        assert_eq!(reloaded.created_at, original_created_at);
        assert_eq!(reloaded.updated_at, original_updated_at);
        assert_eq!(reloaded.usage_count, 0);
        assert!(reloaded.deleted_at.is_none());
        assert_eq!(repo2.list().unwrap().len(), 1);

        // Act 3: edit
        let new_content = "编辑后的内容\n✅ 持久化编辑验证通过\n第二行已修改 💾";
        let edit_fields = ValidatedFields {
            key: "mt1-emoji-test".into(),
            normalized_key: "mt1-emoji-test".into(),
            title: "MT1 已编辑 ✓".into(),
            content: new_content.into(),
            aliases: vec!["emoji".into(), "多行".into(), "编辑".into()],
            tags: vec!["qa".into(), "验收".into(), "编辑过".into()],
            sensitive: false,
            pinned: false,
        };
        let edited = repo2.update(&original_id, 1, edit_fields).unwrap();
        assert_eq!(edited.id, original_id);
        assert_eq!(edited.title, "MT1 已编辑 ✓");
        assert_eq!(edited.content, new_content);
        assert_eq!(edited.aliases, vec!["emoji", "多行", "编辑"]);
        assert_eq!(edited.tags, vec!["qa", "验收", "编辑过"]);
        assert!(!edited.sensitive);
        assert!(!edited.pinned);
        assert_eq!(edited.revision, 2, "revision must bump on edit");
        assert_eq!(
            edited.created_at, original_created_at,
            "created_at must not change"
        );
        assert!(
            edited.updated_at > original_updated_at,
            "updated_at must advance"
        );
        assert_eq!(repo2.list().unwrap().len(), 1);
    }

    // === QA AC-02: Key conflict (existing + soft-deleted) ===
    // Note: existing test create_is_idempotent_and_unique_key_includes_soft_deleted_rows
    // already covers the AC-02 scenario. This test adds explicit AC-02 wording verification.
    #[test]
    fn qa_ac02_key_conflict_with_hello_world_case_insensitive() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();

        // Given: key=hello-world exists
        let r1 = uuid::Uuid::new_v4().to_string();
        let created = repo
            .create(
                validate_create(&CreateSnippetInput {
                    key: "hello-world".into(),
                    title: "Hello".into(),
                    content: "world".into(),
                    aliases: vec![],
                    tags: vec![],
                    sensitive: false,
                    pinned: false,
                    request_id: r1.clone(),
                })
                .unwrap(),
                &r1,
            )
            .unwrap();
        assert_eq!(created.key, "hello-world");
        let count_before = repo.list().unwrap().len();

        // When: create Hello-World and save
        let r2 = uuid::Uuid::new_v4().to_string();
        let err = repo
            .create(
                validate_create(&CreateSnippetInput {
                    key: "Hello-World".into(),
                    title: "Conflict".into(),
                    content: "should fail".into(),
                    aliases: vec![],
                    tags: vec![],
                    sensitive: false,
                    pinned: false,
                    request_id: r2.clone(),
                })
                .unwrap(),
                &r2,
            )
            .unwrap_err();

        // Then: reject, show conflict key, count unchanged
        assert_eq!(err.code, "KEY_CONFLICT");
        assert_eq!(err.conflict_key.as_deref(), Some("hello-world"));
        assert_eq!(err.field.as_deref(), Some("key"));
        assert_eq!(repo.list().unwrap().len(), count_before);

        // Also: key conflict for soft-deleted row
        repo.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), created.id],
            )
            .unwrap();
        let r3 = uuid::Uuid::new_v4().to_string();
        let err2 = repo
            .create(
                validate_create(&CreateSnippetInput {
                    key: "HELLO-WORLD".into(),
                    title: "Trash conflict".into(),
                    content: "still fail".into(),
                    aliases: vec![],
                    tags: vec![],
                    sensitive: false,
                    pinned: false,
                    request_id: r3.clone(),
                })
                .unwrap(),
                &r3,
            )
            .unwrap_err();
        assert_eq!(
            err2.code, "KEY_CONFLICT",
            "soft-deleted key must still block new creation"
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn create_with(
        repo: &Repository,
        key: &str,
        title: &str,
        content: &str,
        aliases: Vec<&str>,
        tags: Vec<&str>,
        sensitive: bool,
        pinned: bool,
    ) -> Snippet {
        let request_id = uuid::Uuid::new_v4().to_string();
        repo.create(
            validate_create(&CreateSnippetInput {
                key: key.into(),
                title: title.into(),
                content: content.into(),
                aliases: aliases.into_iter().map(String::from).collect(),
                tags: tags.into_iter().map(String::from).collect(),
                sensitive,
                pinned,
                request_id: request_id.clone(),
            })
            .unwrap(),
            &request_id,
        )
        .unwrap()
    }

    fn set_usage(repo: &Repository, id: &str, count: i64, last_used: Option<&str>) {
        repo.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE snippets SET usage_count = ?1, last_used_at = ?2 WHERE id = ?3",
                params![count, last_used, id],
            )
            .unwrap();
        repo.refresh_search_index().unwrap();
    }

    #[test]
    fn search_orders_by_prd_match_priority() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        let exact = create_with(&repo, "alpha", "t", "c", vec![], vec![], false, false);
        let prefix = create_with(&repo, "alpha-beta", "t", "c", vec![], vec![], false, false);
        let subkey = create_with(&repo, "xxalphayy", "t", "c", vec![], vec![], false, false);
        let alias = create_with(
            &repo,
            "other-a",
            "t",
            "c",
            vec!["alpha"],
            vec![],
            false,
            false,
        );
        let title = create_with(
            &repo,
            "other-b",
            "Alpha title",
            "c",
            vec![],
            vec![],
            false,
            false,
        );
        let tag = create_with(
            &repo,
            "other-c",
            "t",
            "c",
            vec![],
            vec!["alpha-tag"],
            false,
            false,
        );
        let content = create_with(
            &repo,
            "other-d",
            "t",
            "body contains alpha here",
            vec![],
            vec![],
            false,
            false,
        );

        let result = repo.search("alpha", 20).unwrap();
        let keys: Vec<&str> = result.items.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "alpha",
                "alpha-beta",
                "xxalphayy",
                "other-a",
                "other-b",
                "other-c",
                "other-d"
            ]
        );
        assert_eq!(result.total, 7);
        assert!(
            exact.id == result.items[0].id
                && prefix.id == result.items[1].id
                && subkey.id == result.items[2].id
        );
        assert!(
            alias.id == result.items[3].id
                && title.id == result.items[4].id
                && tag.id == result.items[5].id
                && content.id == result.items[6].id
        );
    }

    #[test]
    fn search_tie_breaks_by_pinned_usage_last_used_key() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        let za = create_with(
            &repo,
            "za",
            "Alpha title",
            "c",
            vec![],
            vec![],
            false,
            false,
        );
        let aa = create_with(
            &repo,
            "aa",
            "Alpha title",
            "c",
            vec![],
            vec![],
            false,
            false,
        );
        let _pinned = create_with(&repo, "bb", "Alpha title", "c", vec![], vec![], false, true);
        let used = create_with(
            &repo,
            "cc",
            "Alpha title",
            "c",
            vec![],
            vec![],
            false,
            false,
        );
        set_usage(&repo, &used.id, 5, Some("2026-01-01T00:00:00Z"));
        set_usage(&repo, &za.id, 1, Some("2026-01-01T00:00:00Z"));
        set_usage(&repo, &aa.id, 1, Some("2025-01-01T00:00:00Z"));

        let result = repo.search("Alpha", 20).unwrap();
        let keys: Vec<&str> = result.items.iter().map(|i| i.key.as_str()).collect();
        // pinned first; then usage 5; then usage 1: lastUsed newer (za) before aa; key asc as final tie
        assert_eq!(keys, vec!["bb", "cc", "za", "aa"]);
    }

    #[test]
    fn search_empty_query_orders_by_pinned_usage_last_used_updated_key() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        let z = create_with(&repo, "z-key", "z", "c", vec![], vec![], false, false);
        let a = create_with(&repo, "a-key", "a", "c", vec![], vec![], false, false);
        let _pinned = create_with(&repo, "m-key", "m", "c", vec![], vec![], false, true);
        let used = create_with(&repo, "b-key", "b", "c", vec![], vec![], false, false);
        set_usage(&repo, &used.id, 9, Some("2026-01-01T00:00:00Z"));
        set_usage(&repo, &z.id, 1, Some("2026-01-01T00:00:00Z"));
        set_usage(&repo, &a.id, 1, None);

        let result = repo.search("", 20).unwrap();
        let keys: Vec<&str> = result.items.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(keys, vec!["m-key", "b-key", "z-key", "a-key"]);
        assert_eq!(result.total, 4);
    }

    #[test]
    fn search_excludes_soft_deleted_and_is_case_insensitive() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        let alive = create_with(&repo, "Hello-World", "t", "c", vec![], vec![], false, false);
        let deleted = create_with(&repo, "hello-other", "t", "c", vec![], vec![], false, false);
        repo.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), deleted.id],
            )
            .unwrap();
        repo.refresh_search_index().unwrap();

        let result = repo.search("HELLO", 20).unwrap();
        let ids: Vec<&str> = result.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec![alive.id.as_str()]);
        assert_eq!(result.items[0].key, "hello-world");
        assert_eq!(result.total, 1);
    }

    #[test]
    fn search_respects_limit_and_reports_total() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        for i in 0..30 {
            create_with(
                &repo,
                &format!("key-{i:02}"),
                "Alpha title",
                "c",
                vec![],
                vec![],
                false,
                false,
            );
        }
        let result = repo.search("alpha", 10).unwrap();
        assert_eq!(result.items.len(), 10);
        assert_eq!(result.total, 30);
    }

    #[test]
    fn record_usage_is_idempotent_per_operation_id() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        let created = create_with(&repo, "counter", "t", "c", vec![], vec![], false, false);
        let operation = uuid::Uuid::new_v4().to_string();

        let (after_first, counted_first) = repo.record_usage(&created.id, &operation).unwrap();
        assert!(counted_first);
        assert_eq!(after_first.usage_count, 1);
        assert_eq!(after_first.revision, 2);
        assert!(after_first.last_used_at.is_some());
        assert_eq!(after_first.id, created.id);
        assert_eq!(after_first.created_at, created.created_at);

        let (after_retry, counted_retry) = repo.record_usage(&created.id, &operation).unwrap();
        assert!(!counted_retry);
        assert_eq!(after_retry.usage_count, 1);
        assert_eq!(after_retry.revision, 2);

        let other = uuid::Uuid::new_v4().to_string();
        let (after_other, counted_other) = repo.record_usage(&created.id, &other).unwrap();
        assert!(counted_other);
        assert_eq!(after_other.usage_count, 2);
        assert_eq!(after_other.revision, 3);
    }

    #[test]
    fn migrates_v2_operation_requests_table() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        repo.connection
            .lock()
            .unwrap()
            .execute_batch(
                "DROP TABLE operation_requests;
                 ALTER TABLE snippets DROP COLUMN content_norm;
                 ALTER TABLE snippets DROP COLUMN aliases_norm;
                 ALTER TABLE snippets DROP COLUMN tags_norm;
                 UPDATE metadata SET schema_version = 2;",
            )
            .unwrap();
        drop(repo);

        let migrated = Repository::open(directory.path()).unwrap();
        let version: i64 = migrated
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT schema_version FROM metadata", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let created = create_with(
            &migrated,
            "legacy-v2",
            "标题",
            "hello world content",
            vec![],
            vec![],
            false,
            false,
        );
        let found = migrated.search("hello", 20).unwrap();
        assert_eq!(found.items.len(), 1);
        assert_eq!(found.items[0].id, created.id);
        let (used, counted) = migrated
            .record_usage(&created.id, &uuid::Uuid::new_v4().to_string())
            .unwrap();
        assert!(counted);
        assert_eq!(used.usage_count, 1);
    }

    // === QA PERF: 10,000 × 2 KB dataset, p95 ≤ 50ms / p99 ≤ 100ms ===
    #[test]
    fn qa_perf_search_10000_snippets_p95_p99() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        {
            let mut connection = repo.connection.lock().unwrap();
            let transaction = connection.transaction().unwrap();
            {
                let mut statement = transaction
                    .prepare(
                        "INSERT INTO snippets (
                            id, key, normalized_key, title, content, aliases, tags, pinned, sensitive,
                            usage_count, last_used_at, created_at, updated_at, deleted_at, revision,
                            content_norm, aliases_norm, tags_norm
                         ) VALUES (?1, ?2, ?3, ?4, ?5, '[]', '[]', 0, 0, 0, NULL, ?6, ?6, NULL, 1, ?7, '', '')",
                    )
                    .unwrap();
                let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                for i in 0..10_000 {
                    let body = format!("snippet {i} 中文正文 扩展内容 {}", "x".repeat(2_000));
                    statement
                        .execute(params![
                            uuid::Uuid::new_v4().to_string(),
                            format!("key-{i:05}"),
                            format!("key-{i:05}"),
                            format!("标题 {i}"),
                            body,
                            now,
                            body.to_lowercase()
                        ])
                        .unwrap();
                }
            }
            transaction.commit().unwrap();
        }
        // 预热
        repo.search("snippet", 20).unwrap();

        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let start = std::time::Instant::now();
            let result = repo.search("snippet", 20).unwrap();
            samples.push(start.elapsed().as_millis() as u64);
            assert_eq!(result.total, 10_000);
        }
        samples.sort_unstable();
        let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
        let p99 = samples[samples.len() - 1];
        eprintln!(
            "PERF search: min={}ms p95={}ms p99={}ms samples={}",
            samples[0],
            p95,
            p99,
            samples.len()
        );
        assert!(p95 <= 50, "p95 {p95}ms exceeds 50ms limit");
        assert!(p99 <= 100, "p99 {p99}ms exceeds 100ms limit");
    }

    #[test]
    fn revision_conflict_keeps_existing_record_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let created = repository
            .create(
                validate_create(&input(request_id.clone(), "key")).unwrap(),
                &request_id,
            )
            .unwrap();
        let mut fields =
            validate_create(&input(uuid::Uuid::new_v4().to_string(), "changed")).unwrap();
        fields.title = "updated".into();
        let updated = repository.update(&created.id, 1, fields.clone()).unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.created_at, created.created_at);
        assert_eq!(updated.usage_count, created.usage_count);
        assert_eq!(updated.last_used_at, created.last_used_at);
        assert_eq!(
            repository.update(&created.id, 1, fields).unwrap_err().code,
            "REVISION_CONFLICT"
        );
        assert_eq!(repository.get(&created.id).unwrap().unwrap().revision, 2);
    }

    // === SPEC-05: 回收站测试 ===

    fn create_for_trash(repo: &Repository, key: &str) -> Snippet {
        let rid = uuid::Uuid::new_v4().to_string();
        repo.create(
            validate_create(&CreateSnippetInput {
                key: key.into(),
                title: "Trash Test".into(),
                content: "content".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: rid.clone(),
            })
            .unwrap(),
            &rid,
        )
        .unwrap()
    }

    #[test]
    fn trash_move_sets_deleted_at_and_bumps_revision() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s = create_for_trash(&repo, "mover");
        assert_eq!(s.revision, 1);
        assert!(s.deleted_at.is_none());

        let moved = repo.trash_move(&s.id).unwrap();
        assert!(moved.deleted_at.is_some());
        assert_eq!(moved.revision, 2);
        // deleted_at 设置后仍在 list 中（list 返回所有记录）
        let list = repo.list().unwrap();
        assert!(list
            .iter()
            .any(|item| item.id == moved.id && item.deleted_at.is_some()));
    }

    #[test]
    fn trash_restore_sets_normal() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s1 = create_for_trash(&repo, "restore-me");
        repo.trash_move(&s1.id).unwrap();
        let restored = repo.trash_restore(&s1.id).unwrap();
        assert!(restored.deleted_at.is_none());
        assert_eq!(restored.revision, 3);
    }

    #[test]
    fn trash_purge_one_removes_row() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s = create_for_trash(&repo, "purge-me");
        repo.trash_move(&s.id).unwrap();
        repo.trash_purge_one(&s.id).unwrap();
        assert!(repo.get(&s.id).unwrap().is_none());
    }

    #[test]
    fn trash_empty_removes_all_trashed() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s1 = create_for_trash(&repo, "e1");
        let s2 = create_for_trash(&repo, "e2");
        let s3 = create_for_trash(&repo, "e3");
        repo.trash_move(&s1.id).unwrap();
        repo.trash_move(&s2.id).unwrap();
        let count = repo.trash_empty().unwrap();
        assert_eq!(count, 2);
        assert!(repo.get(&s1.id).unwrap().is_none());
        assert!(repo.get(&s2.id).unwrap().is_none());
        assert!(repo.get(&s3.id).unwrap().is_some());
    }

    // === SPEC-15: 回收站操作与检索索引一致性 ===

    #[test]
    fn trash_move_excludes_id_from_search_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s = create_for_trash(&repo, "move-vis");
        assert_eq!(repo.search("move-vis", 20).unwrap().total, 1);
        repo.trash_move(&s.id).unwrap();
        // 索引已同步 deleted_at：检索不再返回该软删除片段。
        assert_eq!(repo.search("move-vis", 20).unwrap().total, 0);
    }

    #[test]
    fn trash_restore_reincludes_id_in_search_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s = create_for_trash(&repo, "restore-vis");
        repo.trash_move(&s.id).unwrap();
        assert_eq!(repo.search("restore-vis", 20).unwrap().total, 0);
        repo.trash_restore(&s.id).unwrap();
        // 索引已同步 deleted_at=None：还原后重新可检索。
        assert_eq!(repo.search("restore-vis", 20).unwrap().total, 1);
    }

    #[test]
    fn trash_purge_one_removes_id_from_search_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s = create_for_trash(&repo, "purge-vis");
        repo.trash_move(&s.id).unwrap();
        repo.trash_purge_one(&s.id).unwrap();
        assert_eq!(repo.search("purge-vis", 20).unwrap().total, 0);
    }

    #[test]
    fn trash_empty_removes_ids_from_search_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s1 = create_for_trash(&repo, "empty-vis-1");
        let s2 = create_for_trash(&repo, "empty-vis-2");
        let _s3 = create_for_trash(&repo, "empty-vis-3");
        repo.trash_move(&s1.id).unwrap();
        repo.trash_move(&s2.id).unwrap();
        assert_eq!(repo.search("empty-vis-1", 20).unwrap().total, 0);
        assert_eq!(repo.search("empty-vis-2", 20).unwrap().total, 0);
        assert_eq!(repo.search("empty-vis-3", 20).unwrap().total, 1);
        repo.trash_empty().unwrap();
        assert_eq!(repo.search("empty-vis-1", 20).unwrap().total, 0);
        assert_eq!(repo.search("empty-vis-2", 20).unwrap().total, 0);
        assert_eq!(repo.search("empty-vis-3", 20).unwrap().total, 1);
    }

    #[test]
    fn auto_purge_removes_ids_from_search_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        // 用原始 UPDATE 直接设置 deleted_at，模拟索引仍持有陈旧 deleted_at=None 的状态
        // （对应本 spec 修复前永久删除后快速搜索仍返回旧记录的根因）。
        let _kept = create_for_trash(&repo, "ap-kept");
        let purged = create_for_trash(&repo, "ap-purged");
        let old = (Utc::now() - chrono::Duration::days(31))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        repo.connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
                params![old, purged.id],
            )
            .unwrap();
        // 索引未同步前，旧 deleted_at=None 会让该片段仍被检索（复现旧 bug 场景）。
        assert_eq!(repo.search("ap-purged", 20).unwrap().total, 1);

        let threshold = (Utc::now() - chrono::Duration::days(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        repo.auto_purge(&threshold).unwrap();
        // auto_purge 提交后定向移除 id：即使索引之前陈旧，也正确排除被删片段。
        assert_eq!(repo.search("ap-purged", 20).unwrap().total, 0);
        assert_eq!(repo.search("ap-kept", 20).unwrap().total, 1);
    }

    #[test]
    fn auto_purge_removes_expired_only() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let s29 = create_for_trash(&repo, "old29");
        let s30 = create_for_trash(&repo, "old30");
        let s31 = create_for_trash(&repo, "old31");
        // 直接设置 deleted_at
        let conn = repo.connection.lock().unwrap();
        let t29 = (Utc::now() - chrono::Duration::days(29))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let t30 = (Utc::now() - chrono::Duration::days(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let t31 = (Utc::now() - chrono::Duration::days(31))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
            params![t29, s29.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
            params![t30, s30.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE snippets SET deleted_at = ?1 WHERE id = ?2",
            params![t31, s31.id],
        )
        .unwrap();
        drop(conn);

        let threshold = (Utc::now() - chrono::Duration::days(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let count = repo.auto_purge(&threshold).unwrap();
        assert_eq!(count, 2); // s30 and s31 purged, s29 retained
        assert!(repo.get(&s29.id).unwrap().is_some()); // 29 天的保留
        assert!(repo.get(&s30.id).unwrap().is_none()); // 30 天的删除
        assert!(repo.get(&s31.id).unwrap().is_none()); // 31 天的删除
    }

    // === SPEC-06: Settings tests ===

    #[test]
    fn settings_get_returns_default_on_first_access() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let (settings, rev) = repo.get_settings().unwrap();
        assert_eq!(rev, 1);
        assert_eq!(settings.global_shortcut, "Alt+O");
        assert_eq!(settings.max_results_count, 20);
        assert!(settings.trash_auto_purge_days.is_none());
        assert_eq!(settings.schema_version, 1);
    }

    #[test]
    fn settings_update_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let (_, rev1) = repo.get_settings().unwrap();
        let mut settings = repo.get_settings().unwrap().0;
        settings.max_results_count = 50;
        settings.theme = "dark".into();
        let rev2 = repo.update_settings(&settings, rev1).unwrap();
        assert_eq!(rev2, 2);
        let (read_back, rev3) = repo.get_settings().unwrap();
        assert_eq!(rev3, 2);
        assert_eq!(read_back.max_results_count, 50);
        assert_eq!(read_back.theme, "dark");
    }

    #[test]
    fn settings_revision_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let (_, rev1) = repo.get_settings().unwrap();
        let settings = repo.get_settings().unwrap().0;
        // 模拟并发：先更新到 rev2
        repo.update_settings(&settings, rev1).unwrap();
        // 再用 rev1 尝试更新应该失败
        let result = repo.update_settings(&settings, rev1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "REVISION_CONFLICT");
    }
}
