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
    model::{Snippet, ValidatedFields},
};

const SCHEMA_VERSION: i64 = 2;
const KEY_BYTES: usize = 32;

#[derive(Debug)]
pub struct Repository {
    connection: Mutex<Connection>,
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
        })
    }

    pub fn create(&self, fields: ValidatedFields, request_id: &str) -> Result<Snippet, AppError> {
        let request_fingerprint = fingerprint_fields(&fields)?;
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
                usage_count, last_used_at, created_at, updated_at, deleted_at, revision
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, NULL, ?10, ?10, NULL, 1)",
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
                    now
                ],
            )
            .map_err(|error| map_write_error(error, &transaction, &fields.normalized_key, None))?;
        transaction.execute(
            "INSERT INTO create_requests (request_id, snippet_id, request_fingerprint, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![request_id, id, request_fingerprint, now],
        ).map_err(|_| db_write_error())?;
        let snippet = get_in_transaction(&transaction, &id)?.ok_or_else(db_write_error)?;
        transaction.commit().map_err(|_| db_write_error())?;
        Ok(snippet)
    }

    pub fn update(
        &self,
        id: &str,
        revision: i64,
        fields: ValidatedFields,
    ) -> Result<Snippet, AppError> {
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
                updated_at = ?9, revision = revision + 1
             WHERE id = ?10 AND revision = ?11",
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
        Ok(snippet)
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
}

const SNIPPET_COLUMNS: &str = "id, key, normalized_key, title, content, aliases, tags, pinned, sensitive, usage_count, last_used_at, created_at, updated_at, deleted_at, revision";

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
            revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1)
         );
         CREATE UNIQUE INDEX IF NOT EXISTS snippets_normalized_key_unique ON snippets(normalized_key);
         CREATE TABLE IF NOT EXISTS create_requests (
            request_id TEXT PRIMARY KEY NOT NULL,
            snippet_id TEXT NOT NULL REFERENCES snippets(id),
            request_fingerprint TEXT NOT NULL,
            created_at TEXT NOT NULL
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
        Some(1) => migrate_v1_request_fingerprints(&transaction)?,
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
}
