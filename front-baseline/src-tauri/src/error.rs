use serde::Serialize;

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[error("{code}: {message}")]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_key: Option<String>,
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
            conflict_key: None,
        }
    }

    pub fn validation(field: &str, message: impl Into<String>) -> Self {
        Self {
            code: "VALIDATION_FAILED",
            message: message.into(),
            field: Some(field.into()),
            conflict_key: None,
        }
    }

    pub fn key_conflict(key: String) -> Self {
        Self {
            code: "KEY_CONFLICT",
            message: "Key 已存在，请更换后重试。".into(),
            field: Some("key".into()),
            conflict_key: Some(key),
        }
    }
}
