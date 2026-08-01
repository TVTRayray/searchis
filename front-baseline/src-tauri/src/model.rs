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
