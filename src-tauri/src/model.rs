use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub key: String,
    pub normalized_key: String,
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
    pub revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnippetInput {
    pub key: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub pinned: bool,
    pub request_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnippetInput {
    pub id: String,
    pub revision: i64,
    pub key: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedFields {
    pub key: String,
    pub normalized_key: String,
    pub title: String,
    pub content: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub sensitive: bool,
    pub pinned: bool,
}

/// 检索结果条目。刻意不携带正文，避免敏感正文经 UI 泄露。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    pub id: String,
    pub key: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub sensitive: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub items: Vec<SearchResultItem>,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInput {
    pub query: String,
    pub limit: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyInput {
    pub id: String,
    pub operation_id: String,
    /// 由前端决定复制后是否关闭检索窗口；后端仅用于契约与未来窗口语义。
    #[allow(dead_code)]
    pub keep_open: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyOutcome {
    pub snippet: SearchResultItem,
    /// operationId 首次处理时 true；重复投递时为 false。
    pub counted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareNewOutcome {
    pub normalized_key: String,
}

/// 检索窗口跳转管理窗口时携带的请求（编辑目标或新建预填 Key）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagerRequest {
    #[serde(default)]
    pub edit_id: Option<String>,
    #[serde(default)]
    pub prefill_key: Option<String>,
}
