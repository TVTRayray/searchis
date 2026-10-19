//! SPEC-24：备份格式（v1 JSON / v2 zip）的读写与校验。
//!
//! 本模块与数据库、模板目录解耦：只处理 JSON 结构、zip 结构与字节流。
//! 日志约束：错误消息只含数量、大小与错误码语义，不含备份路径或文件名。

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    error::AppError,
    model::{Group, Snippet, LANGUAGES},
    templates::{paths::validate_rel_path, MAX_FILE_BYTES},
    validation::{normalize_key, normalize_list, validate_group_name},
};

pub const BACKUP_JSON: &str = "backup.json";
pub const TEMPLATES_PREFIX: &str = "templates/";
/// `backup.json` 上限。
pub const MAX_JSON_BYTES: u64 = 100 * 1024 * 1024;
/// zip 文件与解压总量上限。
pub const MAX_ZIP_BYTES: u64 = 1024 * 1024 * 1024;
/// zip 条目数上限。
pub const MAX_ARCHIVE_ENTRIES: usize = 20_000;
/// 备份内条目总数的合理上限（防止畸形输入占满内存）。
const MAX_SNIPPETS: usize = 100_000;
/// 一次校验最多返回的错误行数。
const MAX_ERRORS: usize = 20;

pub fn archive_invalid(message: &str) -> AppError {
    AppError::new("BACKUP_ARCHIVE_INVALID", message)
}

pub fn too_large(message: &str) -> AppError {
    AppError::new("IMPORT_TOO_LARGE", message)
}

fn export_failed(message: &str) -> AppError {
    AppError::new("EXPORT_FAILED", message)
}

// ------------------------------------------------------------------ 数据结构

#[derive(Debug, Clone)]
pub struct BackupGroup {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct BackupFile {
    pub rel_path: String,
    pub ext: String,
    pub size: u64,
    pub sha256: String,
    pub modified_at: String,
    pub original_group_id: Option<String>,
    pub auto_imported: bool,
    pub missing: bool,
}

#[derive(Debug, Clone)]
pub struct BackupSnippet {
    pub id: String,
    /// 已规范化的 Key（与 `normalized_key` 相同，沿用保存时的约定）。
    pub key: String,
    pub title: String,
    pub content: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub sensitive: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub group_id: Option<String>,
    pub language: String,
    pub kind: String,
    pub file: Option<BackupFile>,
}

#[derive(Debug, Clone)]
pub struct ParsedBackup {
    pub version: i64,
    pub groups: Vec<BackupGroup>,
    pub snippets: Vec<BackupSnippet>,
}

impl ParsedBackup {
    /// 需要 zip 内文件的模板（未标记缺失）。
    pub fn referenced_files(&self) -> impl Iterator<Item = &BackupFile> {
        self.snippets
            .iter()
            .filter_map(|s| s.file.as_ref())
            .filter(|f| !f.missing)
    }

    pub fn template_count(&self) -> i64 {
        self.snippets.iter().filter(|s| s.kind == "file").count() as i64
    }
}

// ------------------------------------------------------------------ JSON 解析与校验

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn norm_ts(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?;
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|d| d.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn opt_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_owned)
}

fn string_list(value: Option<&Value>, field: &str) -> Result<Vec<String>, String> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => {
            let mut result = Vec::with_capacity(items.len());
            for item in items {
                result.push(
                    item.as_str()
                        .ok_or_else(|| format!("{field} 必须是字符串数组"))?
                        .to_owned(),
                );
            }
            Ok(result)
        }
        Some(_) => Err(format!("{field} 必须是字符串数组")),
    }
}

fn valid_ext(ext: &str) -> bool {
    ext.chars().count() <= 16 && ext.chars().all(|ch| ch.is_alphanumeric() && !ch.is_uppercase())
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

/// 完整校验备份 JSON（v1 或 v2）。返回错误行列表（最多 20 行），不写任何数据。
pub fn parse_backup(value: &Value) -> Result<ParsedBackup, Vec<String>> {
    let Some(obj) = value.as_object() else {
        return Err(vec!["备份内容必须是 JSON 对象".into()]);
    };
    let version = obj.get("schemaVersion").and_then(Value::as_i64).unwrap_or(0);
    if version != 1 && version != 2 {
        return Err(vec![format!("schemaVersion 必须为 1 或 2，实际为 {version}")]);
    }
    let mut errors: Vec<String> = Vec::new();
    let mut fail = |message: String| {
        if errors.len() < MAX_ERRORS {
            errors.push(message);
        }
    };
    let now = now_string();

    // ---- 分组（仅 v2）
    let mut groups: Vec<BackupGroup> = Vec::new();
    if version == 2 {
        match obj.get("groups").and_then(Value::as_array) {
            None => fail("缺少 groups 数组".into()),
            Some(items) => {
                let mut ids: HashSet<String> = HashSet::new();
                for (i, item) in items.iter().enumerate() {
                    let Some(id) = item.get("id").and_then(Value::as_str).filter(|s| !s.is_empty() && s.len() <= 64) else {
                        fail(format!("分组 {i}: id 无效"));
                        continue;
                    };
                    if !ids.insert(id.to_owned()) {
                        fail(format!("分组 {i}: id 重复"));
                        continue;
                    }
                    let name = match validate_group_name(item.get("name").and_then(Value::as_str).unwrap_or("")) {
                        Ok(name) => name,
                        Err(_) => {
                            fail(format!("分组 {i}: 名称无效"));
                            continue;
                        }
                    };
                    let created = norm_ts(item.get("createdAt")).unwrap_or_else(|| now.clone());
                    let updated = norm_ts(item.get("updatedAt")).unwrap_or_else(|| created.clone());
                    groups.push(BackupGroup {
                        id: id.to_owned(),
                        name,
                        parent_id: opt_string(item.get("parentId")),
                        sort_order: item.get("sortOrder").and_then(Value::as_i64).unwrap_or(0),
                        updated_at: if updated < created { created.clone() } else { updated },
                        created_at: created,
                    });
                }
                check_group_tree(&groups, &mut fail);
            }
        }
    }
    let group_ids: HashSet<&str> = groups.iter().map(|g| g.id.as_str()).collect();

    // ---- 条目
    let mut snippets: Vec<BackupSnippet> = Vec::new();
    match obj.get("snippets").and_then(Value::as_array) {
        None => fail("缺少 snippets 数组".into()),
        Some(items) if items.len() > MAX_SNIPPETS => fail("条目数量超过上限".into()),
        Some(items) => {
            let mut ids: HashSet<String> = HashSet::new();
            let mut keys: HashSet<String> = HashSet::new();
            let mut paths: HashSet<String> = HashSet::new();
            for (i, item) in items.iter().enumerate() {
                match parse_snippet(item, version, &group_ids, &now) {
                    Err(message) => fail(format!("片段 {i}: {message}")),
                    Ok(snippet) => {
                        if !ids.insert(snippet.id.clone()) {
                            fail(format!("片段 {i}: id 重复"));
                            continue;
                        }
                        if !keys.insert(snippet.key.clone()) {
                            fail(format!("片段 {i}: Key 重复"));
                            continue;
                        }
                        if let Some(file) = &snippet.file {
                            if !paths.insert(file.rel_path.clone()) {
                                fail(format!("片段 {i}: 文件路径重复"));
                                continue;
                            }
                        }
                        snippets.push(snippet);
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(ParsedBackup {
            version,
            groups,
            snippets,
        })
    } else {
        Err(errors)
    }
}

/// 备份内分组树校验：父级存在、无环、深度 ≤ 3、同级名称不区分大小写唯一。
fn check_group_tree(groups: &[BackupGroup], fail: &mut impl FnMut(String)) {
    let by_id: HashMap<&str, &BackupGroup> = groups.iter().map(|g| (g.id.as_str(), g)).collect();
    for group in groups {
        let mut depth = 1;
        let mut cursor = group.parent_id.as_deref();
        while let Some(parent) = cursor {
            let Some(node) = by_id.get(parent) else {
                fail("分组的父级不存在".into());
                break;
            };
            depth += 1;
            if depth > 3 || node.id == group.id {
                fail("分组层级超过 3 层或存在环".into());
                break;
            }
            cursor = node.parent_id.as_deref();
        }
    }
    let mut siblings: HashSet<(String, String)> = HashSet::new();
    for group in groups {
        let key = (
            group.parent_id.clone().unwrap_or_default(),
            group.name.to_lowercase(),
        );
        if !siblings.insert(key) {
            fail("同级存在同名分组".into());
        }
    }
}

fn parse_snippet(
    item: &Value,
    version: i64,
    group_ids: &HashSet<&str>,
    now: &str,
) -> Result<BackupSnippet, String> {
    let id = item
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .ok_or("缺少或无效的 id 字段")?
        .to_owned();
    let raw_key = item.get("key").and_then(Value::as_str).ok_or("缺少 key 字段")?;
    let key = normalize_key(raw_key).map_err(|_| "key 不合法".to_owned())?;
    let kind = if version == 1 {
        "text".to_owned()
    } else {
        match item.get("kind").and_then(Value::as_str).unwrap_or("text") {
            kind @ ("text" | "file") => kind.to_owned(),
            _ => return Err("kind 必须是 text 或 file".into()),
        }
    };
    let title = item
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&key)
        .to_owned();
    if title.chars().count() > 100 {
        return Err("title 超过 100 个字符".into());
    }
    let content = match item.get("content") {
        Some(Value::String(text)) => text.clone(),
        None if kind == "file" => String::new(),
        _ => return Err("缺少 content 字段".into()),
    };
    if kind == "text" && (content.is_empty() || content.len() > 100 * 1024) {
        return Err("content 大小必须为 1 byte–100 KB".into());
    }
    if kind == "file" && !content.is_empty() {
        return Err("文件模板的 content 必须为空".into());
    }
    let aliases = normalize_list(&string_list(item.get("aliases"), "aliases")?, "aliases", 64)
        .map_err(|_| "aliases 不合法".to_owned())?;
    let tags = normalize_list(&string_list(item.get("tags"), "tags")?, "tags", 32)
        .map_err(|_| "tags 不合法".to_owned())?;
    let language = if version == 1 {
        "plain".to_owned()
    } else {
        let language = item.get("language").and_then(Value::as_str).unwrap_or("plain");
        if !LANGUAGES.contains(&language) {
            return Err("language 不受支持".into());
        }
        language.to_owned()
    };
    let group_id = if version == 1 {
        None
    } else {
        let group_id = opt_string(item.get("groupId"));
        if let Some(group) = &group_id {
            if !group_ids.contains(group.as_str()) {
                return Err("groupId 引用了不存在的分组".into());
            }
        }
        group_id
    };
    let created_at = norm_ts(item.get("createdAt")).unwrap_or_else(|| now.to_owned());
    let mut updated_at = norm_ts(item.get("updatedAt")).unwrap_or_else(|| created_at.clone());
    if updated_at < created_at {
        updated_at = created_at.clone();
    }
    let sensitive = item.get("sensitive").and_then(Value::as_bool).unwrap_or(false);
    let file = if kind == "file" {
        if sensitive {
            return Err("文件模板不能标记为敏感".into());
        }
        Some(parse_file(item.get("file"), group_ids)?)
    } else {
        None
    };
    Ok(BackupSnippet {
        id,
        key,
        title,
        content,
        aliases,
        tags,
        pinned: item.get("pinned").and_then(Value::as_bool).unwrap_or(false),
        sensitive,
        usage_count: item
            .get("usageCount")
            .and_then(Value::as_i64)
            .filter(|n| *n >= 0)
            .unwrap_or(0),
        last_used_at: norm_ts(item.get("lastUsedAt")),
        deleted_at: norm_ts(item.get("deletedAt")),
        created_at,
        updated_at,
        group_id,
        language,
        kind,
        file,
    })
}

fn parse_file(value: Option<&Value>, group_ids: &HashSet<&str>) -> Result<BackupFile, String> {
    let file = value.and_then(Value::as_object).ok_or("文件模板缺少 file 对象")?;
    let rel_path = file
        .get("relativePath")
        .and_then(Value::as_str)
        .ok_or("file.relativePath 缺失")?;
    validate_rel_path(rel_path).map_err(|_| "file.relativePath 不合法".to_owned())?;
    let ext = file.get("extension").and_then(Value::as_str).unwrap_or("");
    if !valid_ext(ext) {
        return Err("file.extension 不合法".into());
    }
    let size = file
        .get("sizeBytes")
        .and_then(Value::as_u64)
        .ok_or("file.sizeBytes 缺失")?;
    if size > MAX_FILE_BYTES {
        return Err("file.sizeBytes 超过 20 MB".into());
    }
    let sha256 = file
        .get("contentSha256")
        .and_then(Value::as_str)
        .filter(|s| valid_sha(s))
        .ok_or("file.contentSha256 不合法")?
        .to_lowercase();
    let original_group_id = opt_string(file.get("originalGroupId"))
        .filter(|id| group_ids.contains(id.as_str()));
    Ok(BackupFile {
        rel_path: rel_path.to_owned(),
        ext: ext.to_owned(),
        size,
        sha256,
        modified_at: norm_ts(file.get("modifiedAt")).unwrap_or_else(now_string),
        original_group_id,
        auto_imported: file.get("autoImported").and_then(Value::as_bool).unwrap_or(false),
        missing: file.get("missing").and_then(Value::as_bool).unwrap_or(false),
    })
}

// ------------------------------------------------------------------ JSON 构造（导出）

/// 导出时每个模板文件的实际摘要；`None` 表示导出时文件已不在磁盘上。
#[derive(Debug, Clone)]
pub struct FileDigest {
    pub size: u64,
    pub sha256: String,
}

/// 构造 v2 `backup.json`。文件元数据以导出时读取到的实际字节为准。
pub fn build_backup_json(
    groups: &[Group],
    snippets: &[Snippet],
    file_rows: &HashMap<String, crate::storage::FileRow>,
    digests: &HashMap<String, Option<FileDigest>>,
    settings: &Value,
) -> Value {
    let groups: Vec<Value> = groups
        .iter()
        .map(|g| {
            json!({
                "id": g.id,
                "name": g.name,
                "parentId": g.parent_id,
                "depth": g.depth,
                "sortOrder": g.sort_order,
                "createdAt": g.created_at,
                "updatedAt": g.updated_at,
                "revision": g.revision,
            })
        })
        .collect();
    let snippets: Vec<Value> = snippets
        .iter()
        .map(|s| {
            let mut value = serde_json::to_value(s).unwrap_or(Value::Null);
            if let Some(map) = value.as_object_mut() {
                for key in ["fileExt", "fileSize", "fileMtime", "fileMissing", "fileAutoImported"] {
                    map.remove(key);
                }
                map.insert("file".into(), file_json(s, file_rows, digests));
            }
            value
        })
        .collect();
    json!({
        "schemaVersion": 2,
        "exportedAt": now_string(),
        "appVersion": env!("CARGO_PKG_VERSION"),
        "groups": groups,
        "snippets": snippets,
        "settings": settings,
    })
}

fn file_json(
    snippet: &Snippet,
    rows: &HashMap<String, crate::storage::FileRow>,
    digests: &HashMap<String, Option<FileDigest>>,
) -> Value {
    if snippet.kind != "file" {
        return Value::Null;
    }
    let Some(row) = rows.get(&snippet.id) else {
        return Value::Null;
    };
    let digest = digests.get(&snippet.id).cloned().flatten();
    let missing = row.missing || digest.is_none();
    let (size, sha) = match (&digest, missing) {
        (Some(d), _) => (d.size as i64, d.sha256.clone()),
        (None, _) => (row.size, row.sha256.clone()),
    };
    json!({
        "relativePath": row.rel_path,
        "extension": row.ext,
        "sizeBytes": size,
        "contentSha256": sha,
        "modifiedAt": row.mtime,
        "originalGroupId": row.original_group_id,
        "autoImported": snippet.file_auto_imported,
        "missing": missing,
    })
}

/// 备份中的设置：去掉本机相关项（PRD 8.3）。
pub fn backup_settings(settings: &crate::model::Settings) -> Value {
    let mut value = serde_json::to_value(settings).unwrap_or(Value::Null);
    if let Some(map) = value.as_object_mut() {
        for key in ["onboardingCompletedAt", "exportDir", "openWithByExtension"] {
            map.remove(key);
        }
    }
    value
}

// ------------------------------------------------------------------ zip 读取与校验

#[derive(Debug, Clone)]
pub struct ZipEntryInfo {
    pub index: usize,
    pub size: u64,
}

/// 通过结构校验的 zip：`backup.json` 内容与 `templates/` 下文件的索引。
#[derive(Debug)]
pub struct ZipIndex {
    pub backup: Value,
    pub files: HashMap<String, ZipEntryInfo>,
}

/// 判断文件头是否为 zip（PK\x03\x04，空 zip 为 PK\x05\x06）。
pub fn looks_like_zip(path: &Path) -> bool {
    let mut head = [0u8; 4];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .map(|_| head == *b"PK\x03\x04" || head == *b"PK\x05\x06")
        .unwrap_or(false)
}

fn check_entry_name(name: &str, is_dir: bool) -> Result<Option<String>, AppError> {
    let bad = |message: &str| archive_invalid(message);
    if name.is_empty() || name.contains('\\') || name.contains('\0') || name.starts_with('/') {
        return Err(bad("备份包包含非法路径。"));
    }
    let trimmed = name.strip_suffix('/').unwrap_or(name);
    if trimmed
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(bad("备份包包含非法路径。"));
    }
    if name == BACKUP_JSON && !is_dir {
        return Ok(None);
    }
    if is_dir {
        // 目录条目只允许位于 templates/ 之下（或 templates/ 本身）。
        if trimmed == "templates" || trimmed.starts_with(TEMPLATES_PREFIX) {
            return Ok(None);
        }
        return Err(bad("备份包包含未被引用的内容。"));
    }
    let Some(rel) = name.strip_prefix(TEMPLATES_PREFIX) else {
        return Err(bad("备份包包含 templates/ 之外的内容。"));
    };
    validate_rel_path(rel).map_err(|_| bad("备份包包含非法路径。"))?;
    Ok(Some(rel.to_owned()))
}

/// 校验 zip 结构并读出 `backup.json`。不解压模板文件。
pub fn inspect_zip(path: &Path) -> Result<ZipIndex, AppError> {
    let meta = fs::metadata(path).map_err(|_| AppError::new("IMPORT_FAILED", "无法读取备份文件。"))?;
    if meta.len() > MAX_ZIP_BYTES {
        return Err(too_large("备份包超过 1 GB。"));
    }
    let file = File::open(path).map_err(|_| AppError::new("IMPORT_FAILED", "无法读取备份文件。"))?;
    let mut archive =
        ZipArchive::new(file).map_err(|_| archive_invalid("不是有效的 zip 备份包。"))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(too_large("备份包条目数超过 20,000。"));
    }
    let mut names: HashSet<String> = HashSet::new();
    let mut files: HashMap<String, ZipEntryInfo> = HashMap::new();
    let mut backup_index: Option<(usize, u64)> = None;
    let mut total: u64 = 0;
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|_| archive_invalid("备份包条目无法读取。"))?;
        let name = entry.name().to_owned();
        if entry.encrypted() {
            return Err(archive_invalid("备份包不应包含加密条目。"));
        }
        if let Some(mode) = entry.unix_mode() {
            // 只接受普通文件与目录；符号链接、设备等一律拒绝。
            let kind = mode & 0o170000;
            if kind != 0 && kind != 0o100000 && kind != 0o040000 {
                return Err(archive_invalid("备份包包含符号链接或特殊文件。"));
            }
        }
        if entry.is_symlink() {
            return Err(archive_invalid("备份包包含符号链接或特殊文件。"));
        }
        let rel = check_entry_name(&name, entry.is_dir())?;
        if !names.insert(name.clone()) {
            return Err(archive_invalid("备份包包含重复条目。"));
        }
        if entry.is_dir() {
            continue;
        }
        let size = entry.size();
        total = total.saturating_add(size);
        if total > MAX_ZIP_BYTES {
            return Err(too_large("备份包解压后超过 1 GB。"));
        }
        match rel {
            None => {
                if size > MAX_JSON_BYTES {
                    return Err(too_large("backup.json 超过 100 MB。"));
                }
                backup_index = Some((index, size));
            }
            Some(rel) => {
                if size > MAX_FILE_BYTES {
                    return Err(too_large("备份包内有文件超过 20 MB。"));
                }
                files.insert(rel, ZipEntryInfo { index, size });
            }
        }
    }
    let (index, _) = backup_index.ok_or_else(|| archive_invalid("备份包缺少 backup.json。"))?;
    let entry = archive
        .by_index(index)
        .map_err(|_| archive_invalid("backup.json 无法读取。"))?;
    let mut bytes = Vec::new();
    entry
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| archive_invalid("backup.json 无法读取。"))?;
    if bytes.len() as u64 > MAX_JSON_BYTES {
        return Err(too_large("backup.json 超过 100 MB。"));
    }
    let backup: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::new("IMPORT_SCHEMA_INVALID", "backup.json 格式错误。"))?;
    Ok(ZipIndex { backup, files })
}

/// 引用一致性：每个未缺失的模板必须有对应文件且大小一致；不得有多余文件。
pub fn check_references(backup: &ParsedBackup, index: &ZipIndex) -> Result<(), AppError> {
    let mut referenced: HashSet<&str> = HashSet::new();
    for file in backup.referenced_files() {
        let Some(entry) = index.files.get(&file.rel_path) else {
            return Err(archive_invalid("备份包缺少 backup.json 引用的模板文件。"));
        };
        if entry.size != file.size {
            return Err(archive_invalid("备份包内文件大小与 backup.json 不一致。"));
        }
        referenced.insert(file.rel_path.as_str());
    }
    if index.files.keys().any(|rel| !referenced.contains(rel.as_str())) {
        return Err(archive_invalid("备份包包含未被 backup.json 引用的文件。"));
    }
    Ok(())
}

/// 把被引用的模板文件解压到 `dest_dir`（调用方创建，0700），逐个校验大小与 SHA-256。
/// 返回 相对路径 → 暂存文件路径。进度以字节计。
pub fn extract_files(
    zip_path: &Path,
    index: &ZipIndex,
    backup: &ParsedBackup,
    dest_dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<HashMap<String, PathBuf>, AppError> {
    let file = File::open(zip_path).map_err(|_| AppError::new("IMPORT_FAILED", "无法读取备份文件。"))?;
    let mut archive = ZipArchive::new(file).map_err(|_| archive_invalid("不是有效的 zip 备份包。"))?;
    let total: u64 = backup.referenced_files().map(|f| f.size).sum();
    let mut done: u64 = 0;
    let mut staged: HashMap<String, PathBuf> = HashMap::new();
    let mut buffer = vec![0u8; 64 * 1024];
    for (n, wanted) in backup.referenced_files().enumerate() {
        let info = index
            .files
            .get(&wanted.rel_path)
            .ok_or_else(|| archive_invalid("备份包缺少 backup.json 引用的模板文件。"))?;
        let entry = archive
            .by_index(info.index)
            .map_err(|_| archive_invalid("备份包内文件无法读取。"))?;
        if entry.size() != wanted.size {
            return Err(archive_invalid("备份包内文件大小与 backup.json 不一致。"));
        }
        let target = dest_dir.join(format!("f{n}"));
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&target)
            .map_err(|_| AppError::new("TEMPLATE_FS_FAILED", "无法创建导入临时文件。"))?;
        // 多读 1 字节以发现“声明大小小于实际内容”的炸弹。
        let mut limited = entry.take(wanted.size + 1);
        let mut hasher = Sha256::new();
        let mut written: u64 = 0;
        loop {
            let read = limited
                .read(&mut buffer)
                .map_err(|_| archive_invalid("备份包内文件校验失败。"))?;
            if read == 0 {
                break;
            }
            written += read as u64;
            if written > wanted.size {
                return Err(archive_invalid("备份包内文件大小与声明不一致。"));
            }
            hasher.update(&buffer[..read]);
            output
                .write_all(&buffer[..read])
                .map_err(|_| AppError::new("TEMPLATE_FS_FAILED", "无法写入导入临时文件。"))?;
            done += read as u64;
            progress(done, total);
        }
        output
            .sync_all()
            .map_err(|_| AppError::new("TEMPLATE_FS_FAILED", "无法写入导入临时文件。"))?;
        if written != wanted.size || hex::encode(hasher.finalize()) != wanted.sha256 {
            return Err(archive_invalid("备份包内文件哈希与 backup.json 不一致。"));
        }
        staged.insert(wanted.rel_path.clone(), target);
    }
    Ok(staged)
}

// ------------------------------------------------------------------ zip 写入（导出）

#[derive(Debug, Clone)]
pub struct ExportFileInput {
    pub snippet_id: String,
    pub rel_path: String,
    pub abs_path: PathBuf,
}

/// 写出 zip 到 `target`（调用方给出的新临时文件）：先写全部模板文件并边写边算哈希，
/// 再用实际摘要构造 `backup.json` 写入。返回 zip 字节数。
pub fn write_archive(
    target: &Path,
    files: &[ExportFileInput],
    build_json: impl FnOnce(&HashMap<String, Option<FileDigest>>) -> Result<Vec<u8>, AppError>,
    progress: &mut dyn FnMut(u64, u64),
    total_hint: u64,
) -> Result<u64, AppError> {
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(target)
        .map_err(|_| export_failed("无法创建备份文件。"))?;
    let mut zip = ZipWriter::new(output);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);
    let mut digests: HashMap<String, Option<FileDigest>> = HashMap::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut done: u64 = 0;
    for input in files {
        validate_rel_path(&input.rel_path).map_err(|_| export_failed("模板路径不合法。"))?;
        let opened = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&input.abs_path);
        let mut source = match opened {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                digests.insert(input.snippet_id.clone(), None);
                continue;
            }
            Err(_) => return Err(export_failed("无法读取模板文件。")),
        };
        if !source
            .metadata()
            .map(|m| m.file_type().is_file())
            .unwrap_or(false)
        {
            digests.insert(input.snippet_id.clone(), None);
            continue;
        }
        zip.start_file(format!("{TEMPLATES_PREFIX}{}", input.rel_path), options)
            .map_err(|_| export_failed("无法写入备份文件。"))?;
        let mut hasher = Sha256::new();
        let mut size: u64 = 0;
        loop {
            let read = source
                .read(&mut buffer)
                .map_err(|_| export_failed("无法读取模板文件。"))?;
            if read == 0 {
                break;
            }
            size += read as u64;
            if size > MAX_FILE_BYTES {
                return Err(export_failed("有模板文件超过 20 MB，无法备份。"));
            }
            hasher.update(&buffer[..read]);
            zip.write_all(&buffer[..read])
                .map_err(|_| export_failed("无法写入备份文件。"))?;
            done += read as u64;
            progress(done, total_hint);
        }
        digests.insert(
            input.snippet_id.clone(),
            Some(FileDigest {
                size,
                sha256: hex::encode(hasher.finalize()),
            }),
        );
    }
    let json = build_json(&digests)?;
    if json.len() as u64 > MAX_JSON_BYTES {
        return Err(export_failed(&format!(
            "backup.json 超过 100 MB 上限（估算 {} 字节）。",
            json.len()
        )));
    }
    zip.start_file(BACKUP_JSON, options)
        .map_err(|_| export_failed("无法写入备份文件。"))?;
    zip.write_all(&json)
        .map_err(|_| export_failed("无法写入备份文件。"))?;
    let output = zip.finish().map_err(|_| export_failed("无法完成备份文件。"))?;
    output
        .sync_all()
        .map_err(|_| export_failed("备份文件同步失败。"))?;
    let size = output
        .metadata()
        .map_err(|_| export_failed("无法读取备份文件大小。"))?
        .len();
    if size > MAX_ZIP_BYTES {
        return Err(export_failed(&format!("备份包超过 1 GB 上限（{size} 字节）。")));
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_with(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());
        let options = SimpleFileOptions::default();
        for (name, bytes) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        file
    }

    const EMPTY: &[u8] = br#"{"schemaVersion":2,"groups":[],"snippets":[]}"#;

    #[test]
    fn accepts_minimal_archive() {
        let zip = zip_with(&[("backup.json", EMPTY)]);
        let index = inspect_zip(zip.path()).unwrap();
        let parsed = parse_backup(&index.backup).unwrap();
        assert_eq!(parsed.version, 2);
        check_references(&parsed, &index).unwrap();
    }

    #[test]
    fn rejects_traversal_absolute_backslash_and_foreign_entries() {
        for name in ["templates/../evil.sh", "/etc/evil", "templates/a\\b", "other/x", "../evil", "templates//x"] {
            let zip = zip_with(&[("backup.json", EMPTY), (name, b"x")]);
            let error = inspect_zip(zip.path()).unwrap_err();
            assert_eq!(error.code, "BACKUP_ARCHIVE_INVALID", "{name}");
        }
    }

    #[test]
    fn rejects_symlink_entries() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());
        zip.start_file("backup.json", SimpleFileOptions::default()).unwrap();
        zip.write_all(EMPTY).unwrap();
        zip.add_symlink("templates/link", "/etc/passwd", SimpleFileOptions::default())
            .unwrap();
        zip.finish().unwrap();
        assert_eq!(inspect_zip(file.path()).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
    }

    #[test]
    fn rejects_missing_backup_json_and_garbage() {
        let zip = zip_with(&[("templates/a.md", b"x")]);
        assert_eq!(inspect_zip(zip.path()).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
        let junk = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(junk.path(), b"not a zip").unwrap();
        assert_eq!(inspect_zip(junk.path()).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
    }

    fn v2_with_file(sha: &str, size: u64) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schemaVersion": 2, "groups": [],
            "snippets": [{
                "id": "s1", "key": "t1", "title": "T", "kind": "file",
                "file": {"relativePath": "a.md", "extension": "md", "sizeBytes": size,
                         "contentSha256": sha, "modifiedAt": "2026-01-01T00:00:00.000Z"}
            }]
        }))
        .unwrap()
    }

    fn sha_of(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    #[test]
    fn rejects_unreferenced_missing_and_size_mismatched_files() {
        let body = b"hello";
        let json = v2_with_file(&sha_of(body), body.len() as u64);
        // 多余文件
        let zip = zip_with(&[("backup.json", &json), ("templates/a.md", body), ("templates/extra.md", b"x")]);
        let index = inspect_zip(zip.path()).unwrap();
        let parsed = parse_backup(&index.backup).unwrap();
        assert_eq!(check_references(&parsed, &index).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
        // 缺文件
        let zip = zip_with(&[("backup.json", &json)]);
        let index = inspect_zip(zip.path()).unwrap();
        assert_eq!(check_references(&parsed, &index).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
        // 大小不符
        let zip = zip_with(&[("backup.json", &json), ("templates/a.md", b"hello!!")]);
        let index = inspect_zip(zip.path()).unwrap();
        assert_eq!(check_references(&parsed, &index).unwrap_err().code, "BACKUP_ARCHIVE_INVALID");
    }

    #[test]
    fn extraction_rejects_hash_mismatch_and_accepts_exact_content() {
        let body = b"hello";
        let dir = tempfile::tempdir().unwrap();
        let good = v2_with_file(&sha_of(body), 5);
        let zip = zip_with(&[("backup.json", &good), ("templates/a.md", body)]);
        let index = inspect_zip(zip.path()).unwrap();
        let parsed = parse_backup(&index.backup).unwrap();
        let staged = extract_files(zip.path(), &index, &parsed, dir.path(), &mut |_, _| {}).unwrap();
        assert_eq!(std::fs::read(&staged["a.md"]).unwrap(), body);

        let bad = v2_with_file(&sha_of(b"other"), 5);
        let zip = zip_with(&[("backup.json", &bad), ("templates/a.md", body)]);
        let index = inspect_zip(zip.path()).unwrap();
        let parsed = parse_backup(&index.backup).unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let error = extract_files(zip.path(), &index, &parsed, dir2.path(), &mut |_, _| {}).unwrap_err();
        assert_eq!(error.code, "BACKUP_ARCHIVE_INVALID");
    }

    #[test]
    fn parse_rejects_schema_problems_without_panicking() {
        assert!(parse_backup(&json!([])).is_err());
        assert!(parse_backup(&json!({"schemaVersion": 3})).is_err());
        let cyc = json!({"schemaVersion":2,"groups":[
            {"id":"a","name":"A","parentId":"b"},{"id":"b","name":"B","parentId":"a"}],"snippets":[]});
        assert!(parse_backup(&cyc).is_err());
        let deep = json!({"schemaVersion":2,"groups":[
            {"id":"a","name":"A"},{"id":"b","name":"B","parentId":"a"},
            {"id":"c","name":"C","parentId":"b"},{"id":"d","name":"D","parentId":"c"}],"snippets":[]});
        assert!(parse_backup(&deep).is_err());
        let dup = json!({"schemaVersion":2,"groups":[],"snippets":[
            {"id":"1","key":"k","content":"x"},{"id":"2","key":"K","content":"y"}]});
        assert!(parse_backup(&dup).is_err());
        let bad_group = json!({"schemaVersion":2,"groups":[],"snippets":[
            {"id":"1","key":"k","content":"x","groupId":"nope"}]});
        assert!(parse_backup(&bad_group).is_err());
        let bad_lang = json!({"schemaVersion":2,"groups":[],"snippets":[
            {"id":"1","key":"k","content":"x","language":"cobol"}]});
        assert!(parse_backup(&bad_lang).is_err());
        let traversal = json!({"schemaVersion":2,"groups":[],"snippets":[
            {"id":"1","key":"k","kind":"file","file":{"relativePath":"../x","extension":"md","sizeBytes":1,
              "contentSha256":"0000000000000000000000000000000000000000000000000000000000000000"}}]});
        assert!(parse_backup(&traversal).is_err());
    }

    #[test]
    fn v1_items_become_ungrouped_plain_text() {
        let v1 = json!({"schemaVersion":1,"snippets":[
            {"id":"1","key":"K 1","content":"x","groupId":"g","language":"sql","kind":"file"}]});
        let parsed = parse_backup(&v1).unwrap();
        let item = &parsed.snippets[0];
        assert_eq!((item.kind.as_str(), item.language.as_str()), ("text", "plain"));
        assert_eq!(item.group_id, None);
        assert_eq!(item.key, "k-1");
    }
}
