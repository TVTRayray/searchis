use crate::{
    clipboard::ClipboardWriter,
    error::AppError,
    model::{
        CopyInput, CopyOutcome, CreateSnippetInput, PrepareNewOutcome, SearchInput, SearchResponse,
        SearchResultItem, Snippet, UpdateSnippetInput,
    },
    storage::Repository,
    validation::{normalize_key, validate_create, validate_update},
};

pub struct SnippetService {
    repository: Repository,
}

impl SnippetService {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }

    pub fn create(&self, input: CreateSnippetInput) -> Result<Snippet, AppError> {
        let fields = validate_create(&input)?;
        self.repository.create(fields, &input.request_id)
    }

    pub fn get(&self, id: &str) -> Result<Snippet, AppError> {
        self.repository
            .get(id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "片段不存在。请重新载入列表。"))
    }

    pub fn list(&self) -> Result<Vec<Snippet>, AppError> {
        self.repository.list()
    }

    pub fn update(&self, input: UpdateSnippetInput) -> Result<Snippet, AppError> {
        let fields = validate_update(&input)?;
        self.repository.update(&input.id, input.revision, fields)
    }

    pub fn search(&self, input: SearchInput) -> Result<SearchResponse, AppError> {
        if !(5..=100).contains(&input.limit) {
            return Err(AppError::validation("limit", "结果数量必须在 5–100 之间。"));
        }
        self.repository.search(&input.query, input.limit)
    }

    pub fn copy(
        &self,
        input: CopyInput,
        clipboard: &dyn ClipboardWriter,
    ) -> Result<CopyOutcome, AppError> {
        if uuid::Uuid::parse_str(&input.operation_id).is_err() {
            return Err(AppError::validation(
                "operationId",
                "operationId 必须是 UUID。",
            ));
        }
        let snippet = self.get(&input.id)?;
        if snippet.deleted_at.is_some() {
            return Err(AppError::new(
                "NOT_FOUND",
                "片段不存在或已删除。请重新搜索后重试。",
            ));
        }
        // AC-14：同一 operationId 只处理一次。重复投递在写剪贴板之前即短路，
        // 避免覆盖用户两次投递之间复制的新内容。
        if self
            .repository
            .is_operation_processed(&input.operation_id)?
        {
            return Ok(CopyOutcome {
                snippet: to_search_item(&snippet),
                counted: false,
            });
        }
        clipboard.write_text(snippet.content.clone())?;
        let (updated, counted) = self
            .repository
            .record_usage(&input.id, &input.operation_id)
            .map_err(|_| {
                AppError::new(
                    "DB_WRITE_FAILED",
                    "已复制到剪贴板，但使用统计未更新。请稍后重试。",
                )
            })?;
        Ok(CopyOutcome {
            snippet: to_search_item(&updated),
            counted,
        })
    }

    pub fn prepare_new(&self, raw_query: &str) -> Result<PrepareNewOutcome, AppError> {
        let normalized_key = normalize_key(raw_query)?;
        Ok(PrepareNewOutcome { normalized_key })
    }
}

fn to_search_item(snippet: &Snippet) -> SearchResultItem {
    SearchResultItem {
        id: snippet.id.clone(),
        key: snippet.key.clone(),
        title: snippet.title.clone(),
        aliases: snippet.aliases.clone(),
        tags: snippet.tags.clone(),
        pinned: snippet.pinned,
        sensitive: snippet.sensitive,
        usage_count: snippet.usage_count,
        last_used_at: snippet.last_used_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Repository;

    struct FakeClipboard {
        fail: bool,
        written: std::sync::Mutex<Vec<String>>,
    }

    impl ClipboardWriter for FakeClipboard {
        fn write_text(&self, text: String) -> Result<(), AppError> {
            if self.fail {
                return Err(AppError::new("CLIPBOARD_WRITE_FAILED", "模拟剪贴板失败"));
            }
            self.written.lock().unwrap().push(text);
            Ok(())
        }
    }

    fn create_snippet(service: &SnippetService, key: &str, content: &str) -> Snippet {
        let request_id = uuid::Uuid::new_v4().to_string();
        service
            .create(CreateSnippetInput {
                key: key.into(),
                title: "标题".into(),
                content: content.into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: request_id.clone(),
            })
            .unwrap()
    }

    #[test]
    fn copy_writes_content_to_clipboard_and_counts_once_per_operation() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let snippet = create_snippet(&service, "copy-key", "需要复制的内容 😀");
        let clipboard = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };

        let operation = uuid::Uuid::new_v4().to_string();
        let first = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: operation.clone(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap();
        assert!(first.counted);
        assert_eq!(first.snippet.usage_count, 1);
        assert_eq!(
            clipboard.written.lock().unwrap().as_slice(),
            &["需要复制的内容 😀".to_string()]
        );

        let retry = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: operation.clone(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap();
        assert!(!retry.counted);
        assert_eq!(retry.snippet.usage_count, 1);
        // AC-14：重复投递不再写剪贴板，避免覆盖用户新复制的内容。
        assert_eq!(clipboard.written.lock().unwrap().len(), 1);
    }

    #[test]
    fn copy_clipboard_success_but_stats_failure_reports_database_error() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        repository
            .raw_connection()
            .execute_batch(
                "CREATE TRIGGER reject_operation BEFORE INSERT ON operation_requests
                 BEGIN SELECT RAISE(ABORT, 'simulated stats failure'); END;",
            )
            .unwrap();
        let service = SnippetService::new(repository);
        let snippet = create_snippet(&service, "stats-fail", "正文");
        let clipboard = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };

        let error = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(error.code, "DB_WRITE_FAILED");
        assert!(
            error.message.contains("已复制"),
            "提示必须说明已复制、统计未更新"
        );
        assert_eq!(clipboard.written.lock().unwrap().len(), 1);
        let after = service.get(&snippet.id).unwrap();
        assert_eq!(after.usage_count, 0, "统计失败不得计数");
        assert_eq!(after.revision, 1);
    }

    #[test]
    fn copy_clipboard_failure_does_not_count() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let snippet = create_snippet(&service, "fail-key", "正文");
        let clipboard = FakeClipboard {
            fail: true,
            written: std::sync::Mutex::new(vec![]),
        };

        let error = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(error.code, "CLIPBOARD_WRITE_FAILED");
        let after = service.get(&snippet.id).unwrap();
        assert_eq!(after.usage_count, 0);
        assert_eq!(after.revision, 1);
    }

    #[test]
    fn copy_rejects_invalid_operation_id_and_deleted_snippet() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let snippet = create_snippet(&service, "edge-key", "正文");
        let clipboard = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };

        let bad_operation = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: "not-a-uuid".into(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(bad_operation.code, "VALIDATION_FAILED");

        service.repository.mark_deleted(&snippet.id).unwrap();
        let deleted = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(deleted.code, "NOT_FOUND");
        assert_eq!(clipboard.written.lock().unwrap().len(), 0);
    }

    #[test]
    fn prepare_new_normalizes_key_and_rejects_invalid_input() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let outcome = service.prepare_new("  Hello   World ").unwrap();
        assert_eq!(outcome.normalized_key, "hello-world");
        assert!(service.prepare_new("   ").is_err());
        assert!(service.prepare_new("bad/key!").is_err());
    }

    #[test]
    fn search_validates_limit_bounds() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        assert!(service
            .search(SearchInput {
                query: "x".into(),
                limit: 3
            })
            .is_err());
        assert!(service
            .search(SearchInput {
                query: "x".into(),
                limit: 101
            })
            .is_err());
        assert!(service
            .search(SearchInput {
                query: "x".into(),
                limit: 20
            })
            .is_ok());
    }
}
