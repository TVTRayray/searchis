//! SPEC-24：备份导入与重置的数据库部分。每个方法恰好一个事务。
//!
//! 导入的“文件移入模板目录”发生在事务提交之前（见 [`Repository::apply_import`]）：
//! 文件步骤失败时事务直接回滚，不需要事后补偿。

use rusqlite::{params, Transaction};

use super::{
    db_write_error,
    files::{row_to_file, FILE_ROW_COLUMNS},
    FileRow, Repository,
};
use crate::{backup::BackupSnippet, error::AppError};

/// 规划阶段需要的现有条目视图。
#[derive(Debug, Clone)]
pub struct ExistingSnippet {
    pub id: String,
    pub key: String,
    pub normalized_key: String,
    pub kind: String,
    pub rel_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlanGroup {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub depth: i64,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PlanFile {
    pub rel_path: String,
    pub ext: String,
    pub size: i64,
    pub inode: Option<i64>,
    pub sha256: String,
    pub mtime: String,
    pub original_group_id: Option<String>,
    pub auto_imported: bool,
    pub missing: bool,
}

#[derive(Debug, Clone)]
pub struct PlanSnippet {
    pub item: BackupSnippet,
    /// 最终分组（已按合并结果解析）。
    pub group_id: Option<String>,
    pub file: Option<PlanFile>,
    /// 本地已有同 id 条目（覆盖更新）。
    pub update: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ImportPlan {
    /// 新增分组，父级在前。
    pub groups: Vec<PlanGroup>,
    /// 因 Key 冲突被导入记录取代的现有条目。
    pub delete_ids: Vec<String>,
    pub snippets: Vec<PlanSnippet>,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    pub inserted: i64,
    pub updated: i64,
    pub replaced: i64,
    pub groups_added: i64,
}

fn write_failed<T>(_: T) -> AppError {
    db_write_error()
}

fn insert_snippet(
    transaction: &Transaction<'_>,
    item: &BackupSnippet,
    group_id: Option<&str>,
    file: Option<&PlanFile>,
) -> Result<(), AppError> {
    let aliases = serde_json::to_string(&item.aliases).map_err(write_failed)?;
    let tags = serde_json::to_string(&item.tags).map_err(write_failed)?;
    let is_file = file.is_some();
    let content_norm = if is_file { String::new() } else { item.content.to_lowercase() };
    transaction
        .execute(
            "INSERT INTO snippets (
                id, key, normalized_key, title, content, aliases, tags, pinned, sensitive,
                usage_count, last_used_at, created_at, updated_at, deleted_at, revision,
                content_norm, aliases_norm, tags_norm, group_id, language, kind,
                file_rel_path, file_ext, file_size, file_inode, file_sha256, file_mtime,
                file_original_group_id, file_auto_imported, file_missing
             ) VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1,
                       ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)",
            params![
                item.id,
                item.key,
                item.title,
                item.content,
                aliases,
                tags,
                item.pinned,
                item.sensitive && !is_file,
                item.usage_count,
                item.last_used_at,
                item.created_at,
                item.updated_at,
                item.deleted_at,
                content_norm,
                item.aliases.join(" ").to_lowercase(),
                item.tags.join(" ").to_lowercase(),
                group_id,
                item.language,
                item.kind,
                file.map(|f| f.rel_path.as_str()),
                file.map(|f| f.ext.as_str()),
                file.map(|f| f.size),
                file.and_then(|f| f.inode),
                file.map(|f| f.sha256.as_str()),
                file.map(|f| f.mtime.as_str()),
                file.and_then(|f| f.original_group_id.as_deref()),
                file.is_some_and(|f| f.auto_imported),
                file.is_some_and(|f| f.missing),
            ],
        )
        .map_err(write_failed)?;
    Ok(())
}

fn update_snippet(
    transaction: &Transaction<'_>,
    item: &BackupSnippet,
    group_id: Option<&str>,
    file: Option<&PlanFile>,
) -> Result<(), AppError> {
    let aliases = serde_json::to_string(&item.aliases).map_err(write_failed)?;
    let tags = serde_json::to_string(&item.tags).map_err(write_failed)?;
    let is_file = file.is_some();
    let content_norm = if is_file { String::new() } else { item.content.to_lowercase() };
    transaction
        .execute(
            "UPDATE snippets SET
                key = ?2, normalized_key = ?2, title = ?3, content = ?4, aliases = ?5, tags = ?6,
                pinned = ?7, sensitive = ?8, usage_count = ?9, last_used_at = ?10,
                created_at = ?11, updated_at = ?12, deleted_at = ?13, revision = revision + 1,
                content_norm = ?14, aliases_norm = ?15, tags_norm = ?16, group_id = ?17, language = ?18,
                file_rel_path = ?19, file_ext = ?20, file_size = ?21, file_inode = ?22,
                file_sha256 = ?23, file_mtime = ?24, file_original_group_id = ?25,
                file_auto_imported = ?26, file_missing = ?27
             WHERE id = ?1",
            params![
                item.id,
                item.key,
                item.title,
                item.content,
                aliases,
                tags,
                item.pinned,
                item.sensitive && !is_file,
                item.usage_count,
                item.last_used_at,
                item.created_at,
                item.updated_at,
                item.deleted_at,
                content_norm,
                item.aliases.join(" ").to_lowercase(),
                item.tags.join(" ").to_lowercase(),
                group_id,
                item.language,
                file.map(|f| f.rel_path.as_str()),
                file.map(|f| f.ext.as_str()),
                file.map(|f| f.size),
                file.and_then(|f| f.inode),
                file.map(|f| f.sha256.as_str()),
                file.map(|f| f.mtime.as_str()),
                file.and_then(|f| f.original_group_id.as_deref()),
                file.is_some_and(|f| f.auto_imported),
                file.is_some_and(|f| f.missing),
            ],
        )
        .map_err(write_failed)?;
    Ok(())
}

fn delete_snippet(transaction: &Transaction<'_>, id: &str) -> Result<(), AppError> {
    transaction
        .execute("DELETE FROM create_requests WHERE snippet_id = ?1", params![id])
        .map_err(write_failed)?;
    transaction
        .execute("DELETE FROM snippets WHERE id = ?1", params![id])
        .map_err(write_failed)?;
    Ok(())
}

impl Repository {
    /// 备份导出用：全部分组与全部文件模板行（含回收站与缺失）。
    pub fn backup_file_rows(&self) -> Result<Vec<FileRow>, AppError> {
        let connection = self.connection.lock().map_err(write_failed)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {FILE_ROW_COLUMNS} FROM snippets WHERE kind = 'file'"
            ))
            .map_err(write_failed)?;
        let rows = statement.query_map([], row_to_file).map_err(write_failed)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(write_failed)
    }

    pub fn file_row_count(&self) -> Result<i64, AppError> {
        let connection = self.connection.lock().map_err(write_failed)?;
        connection
            .query_row("SELECT COUNT(*) FROM snippets WHERE kind = 'file'", [], |row| row.get(0))
            .map_err(write_failed)
    }

    /// 导入规划所需的现有条目快照（含回收站）。
    pub fn import_snapshot(&self) -> Result<Vec<ExistingSnippet>, AppError> {
        let connection = self.connection.lock().map_err(write_failed)?;
        let mut statement = connection
            .prepare("SELECT id, key, normalized_key, kind, file_rel_path FROM snippets")
            .map_err(write_failed)?;
        let rows = statement
            .query_map([], |row| {
                Ok(ExistingSnippet {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    normalized_key: row.get(2)?,
                    kind: row.get(3)?,
                    rel_path: row.get(4)?,
                })
            })
            .map_err(write_failed)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(write_failed)
    }

    /// 在单一事务内应用导入计划。`between` 在全部数据库写入之后、提交之前执行
    /// （调用方在此把文件移入模板目录）；它返回错误时事务整体回滚。
    pub fn apply_import(
        &self,
        plan: &ImportPlan,
        between: impl FnOnce() -> Result<(), AppError>,
    ) -> Result<ApplyOutcome, AppError> {
        let mut outcome = ApplyOutcome::default();
        {
            let mut connection = self.connection.lock().map_err(write_failed)?;
            let transaction = connection.transaction().map_err(write_failed)?;
            for group in &plan.groups {
                transaction
                    .execute(
                        "INSERT INTO groups (id, name, name_norm, parent_id, depth, sort_order,
                                             created_at, updated_at, revision)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
                        params![
                            group.id,
                            group.name,
                            group.name.to_lowercase(),
                            group.parent_id,
                            group.depth,
                            group.sort_order,
                            group.created_at,
                            group.updated_at
                        ],
                    )
                    .map_err(write_failed)?;
                outcome.groups_added += 1;
            }
            for id in &plan.delete_ids {
                delete_snippet(&transaction, id)?;
                outcome.replaced += 1;
            }
            // 被覆盖的文件条目先让出相对路径，避免互换路径时撞唯一索引。
            for snippet in plan.snippets.iter().filter(|s| s.update && s.file.is_some()) {
                transaction
                    .execute(
                        "UPDATE snippets SET file_rel_path = '.import-pending/' || id WHERE id = ?1",
                        params![snippet.item.id],
                    )
                    .map_err(write_failed)?;
            }
            for snippet in &plan.snippets {
                if snippet.update {
                    update_snippet(
                        &transaction,
                        &snippet.item,
                        snippet.group_id.as_deref(),
                        snippet.file.as_ref(),
                    )?;
                    outcome.updated += 1;
                } else {
                    insert_snippet(
                        &transaction,
                        &snippet.item,
                        snippet.group_id.as_deref(),
                        snippet.file.as_ref(),
                    )?;
                    outcome.inserted += 1;
                }
            }
            self.check_injected_failure()?;
            between()?;
            if self.fail_import_commit.load(std::sync::atomic::Ordering::SeqCst) {
                // 模拟 COMMIT 本身失败：事务随 drop 回滚。
                drop(transaction);
                return Err(write_failed(()));
            }
            transaction.commit().map_err(write_failed)?;
        }
        self.invalidate_index()?;
        Ok(outcome)
    }

    #[cfg(test)]
    pub(crate) fn inject_import_commit_failure(&self, enabled: bool) {
        self.fail_import_commit
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    /// 重置：清空全部条目与分组（及其幂等记录），写入示例数据。单事务。
    pub fn reset_all(&self, examples: &[BackupSnippet]) -> Result<i64, AppError> {
        {
            let mut connection = self.connection.lock().map_err(write_failed)?;
            let transaction = connection.transaction().map_err(write_failed)?;
            for table in [
                "create_requests",
                "operation_requests",
                "group_requests",
                "snippets",
                "groups",
            ] {
                transaction
                    .execute(&format!("DELETE FROM {table}"), [])
                    .map_err(write_failed)?;
            }
            for item in examples {
                insert_snippet(&transaction, item, None, None)?;
            }
            self.check_injected_failure()?;
            transaction.commit().map_err(write_failed)?;
        }
        self.invalidate_index()?;
        Ok(examples.len() as i64)
    }
}
