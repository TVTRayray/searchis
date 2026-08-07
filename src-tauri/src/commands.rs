use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::{
    clipboard::TauriClipboardWriter,
    error::AppError,
    logging::DiagnosticLogger,
    model::{
        CopyInput, CopyOutcome, CreateSnippetInput, ManagerRequest, PrepareNewOutcome, SearchInput,
        SearchResponse, Snippet, UpdateSnippetInput,
    },
    service::SnippetService,
};

pub struct AppState {
    pub service: Option<Arc<SnippetService>>,
    pub startup_error: Option<AppError>,
    pub logger: Arc<DiagnosticLogger>,
    /// 检索窗口跳转管理窗口的待消费请求（Ctrl+E / Ctrl+N）。
    pub manager_request: Mutex<Option<ManagerRequest>>,
}

impl AppState {
    fn service(&self) -> Result<&SnippetService, AppError> {
        self.service.as_deref().ok_or_else(|| {
            self.startup_error.clone().unwrap_or_else(|| {
                AppError::new(
                    "DB_OPEN_FAILED",
                    "数据库未初始化。请重启应用或检查数据目录。",
                )
            })
        })
    }
}

fn audited<T>(
    state: &AppState,
    operation: &str,
    action: impl FnOnce(&SnippetService) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let result = state.service().and_then(action);
    if let Err(error) = &result {
        state.logger.failure(operation, error);
    }
    result
}

#[tauri::command]
pub fn snippet_create(
    state: tauri::State<'_, AppState>,
    input: CreateSnippetInput,
) -> Result<Snippet, AppError> {
    audited(&state, "snippet_create", |service| service.create(input))
}

#[tauri::command]
pub fn snippet_get(state: tauri::State<'_, AppState>, id: String) -> Result<Snippet, AppError> {
    audited(&state, "snippet_get", |service| service.get(&id))
}

#[tauri::command]
pub fn snippet_list(state: tauri::State<'_, AppState>) -> Result<Vec<Snippet>, AppError> {
    audited(&state, "snippet_list", SnippetService::list)
}

#[tauri::command]
pub fn snippet_update(
    state: tauri::State<'_, AppState>,
    input: UpdateSnippetInput,
) -> Result<Snippet, AppError> {
    audited(&state, "snippet_update", |service| service.update(input))
}

#[tauri::command]
pub fn search_snippets(
    state: tauri::State<'_, AppState>,
    input: SearchInput,
) -> Result<SearchResponse, AppError> {
    audited(&state, "search_snippets", |service| service.search(input))
}

#[tauri::command]
pub fn copy_snippet(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input: CopyInput,
) -> Result<CopyOutcome, AppError> {
    let clipboard = TauriClipboardWriter::new(app);
    audited(&state, "copy_snippet", |service| {
        service.copy(input, &clipboard)
    })
}

#[tauri::command]
pub fn prepare_new_snippet(
    state: tauri::State<'_, AppState>,
    raw_query: String,
) -> Result<PrepareNewOutcome, AppError> {
    audited(&state, "prepare_new_snippet", |service| {
        service.prepare_new(&raw_query)
    })
}

#[tauri::command]
pub fn open_search_window(app: tauri::AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("search")
        .ok_or_else(|| AppError::new("WINDOW_UNAVAILABLE", "检索窗口不可用。请重启应用。"))?;
    // KDE/X11：以 visible:false 创建的窗口 show 后尺寸可能未恢复（曾观测到 10×10），
    // 先强制重设几何并居中，再显示，确保窗口按配置尺寸映射。
    let _ = window.set_size(tauri::LogicalSize::new(780.0, 500.0));
    let _ = window.center();
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    // GTK 窗口映射为异步过程，延迟后再聚焦 webview，避免对未映射窗口
    // 调用 XSetInputFocus 产生 BadMatch（RF1 根因）。
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if let Some(window) = app.get_webview_window("search") {
            let _ = window.set_focus();
        }
    });
    Ok(())
}

#[tauri::command]
pub fn close_search_window(app: tauri::AppHandle) -> Result<(), AppError> {
    window_op(&app, "search", |window| {
        let _ = window.hide();
    })
}

#[tauri::command]
pub fn open_manager_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: Option<ManagerRequest>,
) -> Result<(), AppError> {
    if let Some(request) = request {
        *state
            .manager_request
            .lock()
            .map_err(|_| AppError::new("INTERNAL_ERROR", "无法写入窗口跳转请求。请重启应用。"))? =
            Some(request);
    }
    window_op(&app, "main", |window| {
        let _ = window.show();
        let _ = window.set_focus();
    })
}

#[tauri::command]
pub fn take_manager_request(
    state: tauri::State<'_, AppState>,
) -> Result<Option<ManagerRequest>, AppError> {
    state
        .manager_request
        .lock()
        .map(|mut slot| slot.take())
        .map_err(|_| AppError::new("INTERNAL_ERROR", "无法读取窗口跳转请求。请重启应用。"))
}

fn window_op(
    app: &tauri::AppHandle,
    label: &str,
    op: impl FnOnce(&tauri::WebviewWindow),
) -> Result<(), AppError> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| AppError::new("WINDOW_UNAVAILABLE", "应用窗口不可用。请重启应用。"))?;
    op(&window);
    Ok(())
}
