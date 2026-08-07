use crate::error::AppError;

/// 系统剪贴板写入口。生产实现使用 Tauri 剪贴板插件；测试注入替身。
pub trait ClipboardWriter: Send + Sync {
    fn write_text(&self, text: String) -> Result<(), AppError>;
}

pub struct TauriClipboardWriter {
    app: tauri::AppHandle,
}

impl TauriClipboardWriter {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl ClipboardWriter for TauriClipboardWriter {
    fn write_text(&self, text: String) -> Result<(), AppError> {
        use tauri_plugin_clipboard_manager::ClipboardExt;
        self.app.clipboard().write_text(text).map_err(|_| {
            AppError::new(
                "CLIPBOARD_WRITE_FAILED",
                "无法写入系统剪贴板。请检查剪贴板管理器后重试。",
            )
        })
    }
}
