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

// === SPEC-06: Settings (PRD 8.2) ===

/// PRD 8.2 Settings Schema。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub global_shortcut: String,
    pub auto_paste: bool,
    /// 固定 false，PRD 硬约束。不可由用户操作。
    pub restore_clipboard: bool,
    pub launch_at_login: bool,
    pub theme: String, // "dark" | "light" | "system"
    pub max_results_count: i64,
    /// null 表示关闭，默认 null。
    pub trash_auto_purge_days: Option<i64>,
    pub onboarding_completed_at: Option<String>,
    pub schema_version: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            global_shortcut: "Alt+O".into(),
            auto_paste: true,
            restore_clipboard: false,
            launch_at_login: true,
            theme: "system".into(),
            max_results_count: 20,
            trash_auto_purge_days: None,
            onboarding_completed_at: None,
            schema_version: 1,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateInput {
    pub key: String,
    pub value: serde_json::Value,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponse {
    pub settings: Settings,
    pub revision: i64,
    pub autostart_actual: bool,
}

// === SPEC-03: 全局快捷键与自动粘贴 ===

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteInput {
    pub snippet_id: String,
    pub operation_id: String,
    pub auto_paste: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteOutcome {
    pub snippet: SearchResultItem,
    /// "copied" | "pasted" | "degraded"
    pub action: String,
    pub target_valid: bool,
    pub counted: bool,
}

/// 快捷键注册输入（预留，暂未使用）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RegisterShortcutInput {
    pub accelerator: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutInfo {
    pub current: String,
    pub registered: bool,
}

/// 窗口切换结果（预留，暂未使用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ToggleResult {
    pub visible: bool,
    pub target_token: Option<String>,
}

// === SPEC-05: 回收站 ===

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashPurgeInput {
    pub id: String,
    pub confirmation_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEmptyInput {
    pub confirmation_token: String,
    #[allow(dead_code)]
    pub expected_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEmptyResult {
    pub deleted_count: i64,
}

// === SPEC-06: XDG Autostart ===

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartInput {
    pub enabled: bool,
}

// === SPEC-07: 备份、恢复与重置 ===

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub estimated_size_bytes: u64,
    pub sensitive_count: i64,
    pub snippet_count: i64,
    pub has_settings: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutcome {
    pub path: String,
    pub size_bytes: u64,
    pub snippet_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ImportPreview {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportValidation {
    pub valid: bool,
    pub snippet_count: i64,
    pub settings_present: bool,
    pub schema_version: i64,
    pub errors: Vec<String>,
    pub import_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCommitInput {
    pub path: String,
    pub import_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub inserted: i64,
    pub updated: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetExamplesInput {
    pub confirm: bool,
}

// === SPEC-08: Onboarding ===

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentCapability {
    pub name: String,
    pub detected: bool,
    pub supported: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDetection {
    pub distro: EnvironmentCapability,
    pub kde_version: EnvironmentCapability,
    pub x11: EnvironmentCapability,
    pub kglobalaccel: EnvironmentCapability,
    pub clipboard: EnvironmentCapability,
    pub x11_inject: EnvironmentCapability,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingProgressInput {
    pub step: i32,
    pub revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingCompleteInput {
    pub action: String, // "finish" or "skip"
}

// === SPEC-09: KDE Plasma Global Menu ===

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuGroup {
    pub id: String,
    pub label: String,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub command_id: String,
    pub enabled: bool,
    pub checked: bool,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuModel {
    pub groups: Vec<MenuGroup>,
    pub version: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMenuCommandInput {
    pub command_id: String,
}
