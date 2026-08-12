/// 平台能力检测。
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub clipboard_write: bool,
    pub x11_inject: bool,
    pub kglobalaccel: bool,
}

pub fn detect_kglobalaccel_available() -> bool {
    super::kglobalaccel::KGlobalAccelManager::new("", "").available()
}
