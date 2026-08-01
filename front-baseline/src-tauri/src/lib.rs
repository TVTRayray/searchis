mod commands;
mod error;
mod logging;
mod model;
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
                },
                Err(error) => {
                    logger.failure("database_startup", &error);
                    AppState {
                        service: None,
                        startup_error: Some(error),
                        logger: Arc::clone(&logger),
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snippet_create,
            commands::snippet_get,
            commands::snippet_list,
            commands::snippet_update,
        ])
        .run(tauri::generate_context!())
        .expect("Searchis Tauri runtime failed");
}
