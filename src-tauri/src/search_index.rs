use std::cmp::Ordering;

use rusqlite::Connection;

use crate::{
    error::AppError,
    model::{SearchResponse, SearchResultItem, Snippet},
};

/// 检索内存索引。数据完全派生自 snippets 表（含 content_norm/aliases_norm/tags_norm），
/// 可由数据库全量重建，不构成第二份不可恢复的领域真相。
#[derive(Debug, Clone)]
struct IndexedSnippet {
    id: String,
    key: String,
    title: String,
    aliases: Vec<String>,
    tags: Vec<String>,
    aliases_norm: String,
    tags_norm: String,
    content_norm: String,
    pinned: bool,
    sensitive: bool,
    usage_count: i64,
    last_used_at: Option<String>,
    updated_at: String,
    deleted_at: Option<String>,
}

impl From<&Snippet> for IndexedSnippet {
    fn from(snippet: &Snippet) -> Self {
        Self {
            id: snippet.id.clone(),
            key: snippet.key.clone(),
            title: snippet.title.clone(),
            aliases: snippet.aliases.clone(),
            tags: snippet.tags.clone(),
            aliases_norm: snippet.aliases.join(" ").to_lowercase(),
            tags_norm: snippet.tags.join(" ").to_lowercase(),
            content_norm: snippet.content.to_lowercase(),
            pinned: snippet.pinned,
            sensitive: snippet.sensitive,
            usage_count: snippet.usage_count,
            last_used_at: snippet.last_used_at.clone(),
            updated_at: snippet.updated_at.clone(),
            deleted_at: snippet.deleted_at.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct SearchIndex {
    items: Vec<IndexedSnippet>,
}

impl SearchIndex {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn rebuild(&mut self, connection: &Connection) -> Result<(), AppError> {
        let mut statement = connection
            .prepare(
                "SELECT id, key, title, aliases, tags, aliases_norm, tags_norm, content_norm, pinned, sensitive,
                        usage_count, last_used_at, updated_at, deleted_at
                 FROM snippets",
            )
            .map_err(|_| AppError::new("DB_OPEN_FAILED", "无法加载检索索引。请检查数据库文件。"))?;
        let rows = statement
            .query_map([], |row| {
                let aliases: String = row.get(3)?;
                let tags: String = row.get(4)?;
                let aliases = serde_json::from_str(&aliases).unwrap_or_default();
                let tags = serde_json::from_str(&tags).unwrap_or_default();
                Ok(IndexedSnippet {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    title: row.get(2)?,
                    aliases,
                    tags,
                    aliases_norm: row.get(5)?,
                    tags_norm: row.get(6)?,
                    content_norm: row.get(7)?,
                    pinned: row.get(8)?,
                    sensitive: row.get(9)?,
                    usage_count: row.get(10)?,
                    last_used_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    deleted_at: row.get(13)?,
                })
            })
            .map_err(|_| AppError::new("DB_OPEN_FAILED", "无法加载检索索引。请检查数据库文件。"))?;
        self.items = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| AppError::new("DB_OPEN_FAILED", "检索索引数据损坏。请从备份恢复。"))?;
        Ok(())
    }

    pub fn upsert(&mut self, entry: &Snippet) {
        if let Some(existing) = self.items.iter_mut().find(|item| item.id == entry.id) {
            *existing = IndexedSnippet::from(entry);
        } else {
            self.items.push(IndexedSnippet::from(entry));
        }
    }

    pub fn search(&self, query: &str, limit: i64) -> SearchResponse {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.search_empty(limit);
        }
        let needle = trimmed.to_lowercase();
        let mut matches: Vec<(u8, &IndexedSnippet)> = self
            .items
            .iter()
            .filter(|item| item.deleted_at.is_none())
            .filter_map(|item| {
                let rank = rank_of(item, &needle);
                (rank > 0).then_some((rank, item))
            })
            .collect();
        matches.sort_by(|(rank_a, a), (rank_b, b)| {
            rank_b
                .cmp(rank_a)
                .then_with(|| b.pinned.cmp(&a.pinned))
                .then_with(|| b.usage_count.cmp(&a.usage_count))
                .then_with(|| compare_last_used_desc(a, b))
                .then_with(|| a.key.cmp(&b.key))
        });
        let total = matches.len() as i64;
        let items = matches
            .into_iter()
            .take(limit as usize)
            .map(|(_, item)| to_search_item(item))
            .collect();
        SearchResponse { items, total }
    }

    fn search_empty(&self, limit: i64) -> SearchResponse {
        let mut matches: Vec<&IndexedSnippet> = self
            .items
            .iter()
            .filter(|item| item.deleted_at.is_none())
            .collect();
        matches.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| b.usage_count.cmp(&a.usage_count))
                .then_with(|| compare_last_used_desc(a, b))
                .then_with(|| b.updated_at.cmp(&a.updated_at))
                .then_with(|| a.key.cmp(&b.key))
        });
        let total = matches.len() as i64;
        let items = matches
            .into_iter()
            .take(limit as usize)
            .map(to_search_item)
            .collect();
        SearchResponse { items, total }
    }
}

fn rank_of(item: &IndexedSnippet, needle: &str) -> u8 {
    // key 在保存时已规范化为小写。
    if item.key == needle {
        return 7;
    }
    if item.key.starts_with(needle) {
        return 6;
    }
    if item.key.contains(needle) {
        return 5;
    }
    if item.aliases_norm.contains(needle) {
        return 4;
    }
    if item.title.to_lowercase().contains(needle) {
        return 3;
    }
    if item.tags_norm.contains(needle) {
        return 2;
    }
    if item.content_norm.contains(needle) {
        return 1;
    }
    0
}

fn compare_last_used_desc(a: &IndexedSnippet, b: &IndexedSnippet) -> Ordering {
    match (a.last_used_at.as_deref(), b.last_used_at.as_deref()) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn to_search_item(item: &IndexedSnippet) -> SearchResultItem {
    SearchResultItem {
        id: item.id.clone(),
        key: item.key.clone(),
        title: item.title.clone(),
        aliases: item.aliases.clone(),
        tags: item.tags.clone(),
        pinned: item.pinned,
        sensitive: item.sensitive,
        usage_count: item.usage_count,
        last_used_at: item.last_used_at.clone(),
    }
}
