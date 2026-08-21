mod clipboard;
mod commands;
mod error;
mod logging;
mod model;
mod platform;
mod search_index;
mod service;
mod storage;
mod validation;

use std::sync::Arc;

use commands::AppState;
use error::AppError;
use logging::DiagnosticLogger;
use service::SnippetService;
use storage::Repository;
use tauri::Manager;

/// D-Bus 信号回调：通过 AppHandle 获取 AppState 执行 toggle 逻辑。
fn toggle_from_signal(app: &tauri::AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    commands::toggle_picker_inner(app, &state)
}

pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_clipboard_manager::init());
    #[cfg(feature = "e2e-webdriver")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .setup(|app| {
            let paths = app.path();
            let data_dir = paths.app_data_dir().map_err(|error| error.to_string())?;
            let config_dir = paths.app_config_dir().map_err(|error| error.to_string())?;
            let log_dir = paths.app_log_dir().map_err(|error| error.to_string())?;
            let logger = Arc::new(DiagnosticLogger::open(&log_dir));

            // 平台组件
            let window_manager =
                Arc::new(platform::XdotoolManager::new()) as Arc<dyn platform::WindowManager>;
            let shortcut_manager = Arc::new(platform::KGlobalAccelManager::new(
                "searchis",
                "toggle_picker",
            ));

            // E2E must not alter the user's KDE-global Alt+O registration.
            if std::env::var_os("SEARCHIS_E2E").is_none() {
                // 注册默认快捷键（Alt+O）
                if shortcut_manager.available() {
                    match shortcut_manager.register("Alt+O") {
                        Ok(prev) => {
                            if prev.is_empty() {
                                logger.record("shortcut_register", "OK_Alt+O");
                            } else {
                                logger.record("shortcut_register", &format!("replaced={prev}"));
                            }
                        }
                        Err(e) => {
                            logger.failure("shortcut_register", &e);
                        }
                    }
                } else {
                    logger.record("shortcut_register", "KGLOBALACCEL_UNAVAILABLE");
                }

                // 启动 D-Bus 信号监听
                let handle_for_signal = app.handle().clone();
                platform::start_signal_listener(
                    "searchis".into(),
                    "toggle_picker".into(),
                    move || {
                        let _ = toggle_from_signal(&handle_for_signal);
                    },
                );
            }

            let state = match Repository::open_with_paths(&data_dir, &config_dir) {
                Ok(repository) => AppState {
                    service: Some(Arc::new(SnippetService::new(repository))),
                    startup_error: None,
                    logger: Arc::clone(&logger),
                    manager_request: std::sync::Mutex::new(None),
                    window_manager,
                    search_visible: std::sync::Mutex::new(false),
                    target_window: std::sync::Mutex::new(None),
                },
                Err(error) => {
                    logger.failure("database_startup", &error);
                    AppState {
                        service: None,
                        startup_error: Some(error),
                        logger: Arc::clone(&logger),
                        manager_request: std::sync::Mutex::new(None),
                        window_manager,
                        search_visible: std::sync::Mutex::new(false),
                        target_window: std::sync::Mutex::new(None),
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

            // SPEC-16: 启动时注册 KDE AppMenu 五组菜单，并后台监视 Registrar 出现/消失以 5 秒内重连。
            // 无 Global Menu / D-Bus 不可用时不阻塞启动，注册失败记录可观测状态。
            {
                let menu = platform::build_menu_model(None, "");
                let registrar = Arc::new(std::sync::Mutex::new(platform::AppMenuRegistrar::new()));
                match registrar.lock() {
                    Ok(mut reg) => match reg.register(&menu) {
                        Ok(()) => logger.record("appmenu_register", "OK_registered"),
                        Err(e) => logger.failure("appmenu_register", &e),
                    },
                    Err(_) => logger.record("appmenu_register", "MUTEX_POISONED"),
                }
                platform::spawn_appmenu_monitor(registrar, menu);
            }

            // 检索窗口 WM 关闭拦截
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
            // CRUD / 检索 / 复制
            commands::snippet_create,
            commands::snippet_get,
            commands::snippet_list,
            commands::snippet_update,
            commands::search_snippets,
            commands::copy_snippet,
            commands::prepare_new_snippet,
            // 窗口管理
            commands::open_search_window,
            commands::close_search_window,
            commands::open_manager_window,
            commands::take_manager_request,
            // SPEC-03
            commands::detect_capabilities,
            commands::register_shortcut,
            commands::toggle_picker,
            commands::execute_paste,
            // SPEC-05
            commands::trash_move,
            commands::trash_restore,
            commands::trash_purge_one,
            commands::trash_empty,
            commands::trash_auto_purge,
            // SPEC-06
            commands::settings_get,
            commands::settings_update,
            commands::autostart_get,
            commands::autostart_set,
            // SPEC-07
            commands::export_preview,
            commands::export_confirm,
            commands::import_validate,
            commands::import_commit,
            commands::reset_examples,
            // SPEC-08
            commands::environment_detect,
            commands::onboarding_progress,
            commands::onboarding_complete,
            // SPEC-09
            commands::appmenu_get_model,
            commands::appmenu_dispatch,
            commands::appmenu_check_availability,
        ])
        .run(tauri::generate_context!())
        .expect("Searchis Tauri runtime failed");
}
