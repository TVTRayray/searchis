use std::sync::Arc;

use crate::{
    clipboard::TauriClipboardWriter,
    error::AppError,
    logging::DiagnosticLogger,
    model::{
        CopyInput, CopyOutcome, CreateSnippetInput, PrepareNewOutcome, SearchInput, SearchResponse,
        Snippet, UpdateSnippetInput,
    },
    service::SnippetService,
};

pub struct AppState {
    pub service: Option<Arc<SnippetService>>,
    pub startup_error: Option<AppError>,
    pub logger: Arc<DiagnosticLogger>,
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
