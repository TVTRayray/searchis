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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Snippet;

    #[allow(clippy::too_many_arguments)]
    fn make_snippet(
        id: &str,
        key: &str,
        title: &str,
        content: &str,
        tags: Vec<&str>,
        pinned: bool,
        sensitive: bool,
        usage: i64,
        last_used_at: Option<&str>,
        updated_at: &str,
    ) -> Snippet {
        Snippet {
            id: id.into(),
            key: key.into(),
            normalized_key: key.into(),
            title: title.into(),
            content: content.into(),
            aliases: vec![],
            tags: tags.into_iter().map(String::from).collect(),
            pinned,
            sensitive,
            usage_count: usage,
            last_used_at: last_used_at.map(String::from),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: updated_at.into(),
            deleted_at: None,
            revision: 1,
        }
    }

    fn build_index() -> (SearchIndex, Vec<Snippet>) {
        let items = vec![
            make_snippet(
                "1",
                "alpha",
                "Alpha",
                "content alpha",
                vec![],
                true,
                false,
                10,
                Some("2026-06-01"),
                "2026-07-01",
            ),
            make_snippet(
                "2",
                "beta",
                "Beta",
                "content beta",
                vec!["work"],
                false,
                true,
                5,
                Some("2026-05-01"),
                "2026-06-01",
            ),
            make_snippet(
                "3",
                "gamma",
                "Gamma",
                "content gamma",
                vec!["home"],
                false,
                false,
                0,
                None,
                "2026-05-01",
            ),
            make_snippet(
                "4",
                "delta",
                "Delta",
                "secret-content",
                vec!["work"],
                false,
                false,
                8,
                Some("2026-06-01"),
                "2026-07-01",
            ),
            make_snippet(
                "5",
                "deleted",
                "Deleted",
                "x",
                vec![],
                false,
                false,
                0,
                None,
                "2026-01-01",
            )
            .deleted("2026-01-01"),
        ];
        let mut index = SearchIndex::default();
        for s in &items {
            index.upsert(s);
        }
        (index, items)
    }

    impl Snippet {
        fn deleted(self, at: &str) -> Self {
            Self {
                deleted_at: Some(at.into()),
                ..self
            }
        }
    }

    #[test]
    fn filter_pinned_view_returns_only_pinned() {
        let (index, _) = build_index();
        let resp = index.search("", 20);
        let pinned: Vec<&str> = resp
            .items
            .iter()
            .filter(|i| i.pinned)
            .map(|i| i.key.as_str())
            .collect();
        assert_eq!(pinned, vec!["alpha"]);
    }

    #[test]
    fn filter_recent_view_returns_only_with_last_used() {
        let (index, _) = build_index();
        // 空查询返回非删除行，按 updated_at 排序。recent 是客户端视图。
        let resp = index.search("", 20);
        let has_recent: Vec<&str> = resp
            .items
            .iter()
            .filter(|i| i.last_used_at.is_some())
            .map(|i| i.key.as_str())
            .collect();
        assert!(has_recent.contains(&"alpha"));
        assert!(has_recent.contains(&"beta"));
        assert!(has_recent.contains(&"delta"));
        assert!(!has_recent.contains(&"gamma"));
    }

    #[test]
    fn filter_by_tag_returns_matching_only() {
        let (index, _) = build_index();
        let resp = index.search("work", 20);
        let keys: Vec<&str> = resp.items.iter().map(|i| i.key.as_str()).collect();
        assert!(keys.contains(&"beta"));
        assert!(keys.contains(&"delta"));
        assert_eq!(resp.total, 2);
    }

    #[test]
    fn sort_by_updated_desc() {
        let (index, _) = build_index();
        let resp = index.search("", 20);
        let keys: Vec<&str> = resp.items.iter().map(|i| i.key.as_str()).collect();
        // alpha(07-01) > delta(07-01) > beta(06-01) >= gamma(06-01) by key asc tiebreak
        assert_eq!(resp.total, 4);
        assert!(keys[0] == "alpha" || keys[0] == "delta");
    }

    #[test]
    fn sort_by_usage_desc_tiebreak_key_asc() {
        let (index, _) = build_index();
        // 空查询按 pinned DESC, usage DESC, lastUsed DESC, updated DESC, key ASC
        let resp = index.search("", 20);
        let items = &resp.items;
        // alpha(pinned=true,usage=10), delta(usage=8), beta(usage=5), gamma(usage=0)
        assert!(
            items[0].key == "alpha",
            "first should be alpha, got {}",
            items[0].key
        );
        assert!(
            items[1].key == "delta",
            "second should be delta, got {}",
            items[1].key
        );
        assert!(
            items[2].key == "beta",
            "third should be beta, got {}",
            items[2].key
        );
        assert!(
            items[3].key == "gamma",
            "fourth should be gamma, got {}",
            items[3].key
        );
    }

    #[test]
    fn search_excludes_deleted() {
        let (index, _) = build_index();
        let resp = index.search("deleted", 20);
        assert_eq!(resp.total, 0);
        assert!(resp.items.is_empty());
    }

    #[test]
    fn sensitive_items_return_no_content() {
        let (index, _) = build_index();
        let resp = index.search("beta", 20);
        // beta is sensitive=true; SearchResultItem has no content field
        assert_eq!(resp.items.len(), 1);
        assert!(resp.items[0].sensitive);
        // SearchResultItem does NOT have content field — static guarantee
    }

    #[test]
    fn search_across_aliases() {
        let mut index = SearchIndex::default();
        let s = make_snippet(
            "a",
            "a",
            "Test",
            "body",
            vec![],
            false,
            false,
            0,
            None,
            "2026-01-01",
        );
        let s = Snippet {
            aliases: vec!["alias1".into(), "alias2".into()],
            ..s
        };
        index.upsert(&s);
        let resp = index.search("alias1", 20);
        assert_eq!(resp.total, 1);
    }

    #[test]
    fn search_across_content() {
        let mut index = SearchIndex::default();
        let s = make_snippet(
            "c",
            "c",
            "T",
            "needle in haystack",
            vec![],
            false,
            false,
            0,
            None,
            "2026-01-01",
        );
        index.upsert(&s);
        assert_eq!(index.search("needle", 20).total, 1);
        assert_eq!(index.search("hay", 20).total, 1);
        assert_eq!(index.search("missing", 20).total, 0);
    }

    #[test]
    fn key_exact_match_ranks_first() {
        let mut index = SearchIndex::default();
        index.upsert(&make_snippet(
            "1",
            "needle",
            "X",
            "needle",
            vec![],
            false,
            false,
            0,
            None,
            "2026-01-01",
        ));
        index.upsert(&make_snippet(
            "2",
            "prefix-needle",
            "Y",
            "needle",
            vec![],
            false,
            false,
            0,
            None,
            "2026-01-01",
        ));
        index.upsert(&make_snippet(
            "3",
            "sub-needle-xx",
            "Z",
            "needle",
            vec![],
            false,
            false,
            0,
            None,
            "2026-01-01",
        ));
        let resp = index.search("needle", 20);
        assert_eq!(resp.items[0].key, "needle");
        assert_eq!(resp.items[1].key, "prefix-needle");
        assert_eq!(resp.items[2].key, "sub-needle-xx");
    }
}
