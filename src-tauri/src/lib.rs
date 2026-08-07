mod clipboard;
mod commands;
mod error;
mod logging;
mod model;
mod search_index;
mod service;
mod storage;
mod validation;

use std::sync::Arc;

use commands::AppState;
use logging::DiagnosticLogger;
use service::SnippetService;
use storage::Repository;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let paths = app.path();
            let data_dir = paths.app_data_dir().map_err(|error| error.to_string())?;
            let config_dir = paths.app_config_dir().map_err(|error| error.to_string())?;
            let log_dir = paths.app_log_dir().map_err(|error| error.to_string())?;
            let logger = Arc::new(DiagnosticLogger::open(&log_dir));
            let state = match Repository::open_with_paths(&data_dir, &config_dir) {
                Ok(repository) => AppState {
                    service: Some(Arc::new(SnippetService::new(repository))),
                    startup_error: None,
                    logger: Arc::clone(&logger),
                    manager_request: std::sync::Mutex::new(None),
                },
                Err(error) => {
                    logger.failure("database_startup", &error);
                    AppState {
                        service: None,
                        startup_error: Some(error),
                        logger: Arc::clone(&logger),
                        manager_request: std::sync::Mutex::new(None),
                    }
                }
            };
            let environment_code = format!(
                "OK_{}_{}_{}_{}",
                std::env::consts::OS,
                std::env::consts::ARCH,
                std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into()),
                std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into()),
            );
            logger.record("startup_environment", &environment_code);
            app.manage(state);
            // 检索窗口：拦截 WM 关闭（Alt+F4）转为隐藏，避免窗口被永久销毁后无法再呼出。
            if let Some(search_window) = app.get_webview_window("search") {
                let window_for_close = search_window.clone();
                search_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snippet_create,
            commands::snippet_get,
            commands::snippet_list,
            commands::snippet_update,
            commands::search_snippets,
            commands::copy_snippet,
            commands::prepare_new_snippet,
            commands::open_search_window,
            commands::close_search_window,
            commands::open_manager_window,
            commands::take_manager_request,
        ])
        .run(tauri::generate_context!())
        .expect("Searchis Tauri runtime failed");
}
