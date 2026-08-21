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
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

/// 供 main.rs CLI 快速向已在运行的 Searchis 发送指令 (如 toggle / show-manager / quit)。
pub fn send_cli_command(cmd: &str) -> bool {
    platform::send_command(cmd)
}

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
        .menu(platform::create_app_menu)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            match id {
                "open_search_window" | "show-picker" | "view-picker" | "view_picker" => {
                    let _ = commands::open_search_window_inner(app);
                }
                "open_manager_window" | "show-manager" | "view-manager" | "view_manager" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "open_settings" | "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("navigate", "settings");
                }
                "app_quit" | "quit" => {
                    app.exit(0);
                }
                "snippet_create" | "new-snippet" => {
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(mut slot) = state.manager_request.lock() {
                            *slot = Some(model::ManagerRequest {
                                edit_id: None,
                                prefill_key: Some(String::new()),
                            });
                        }
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("menu-action", "new-snippet");
                }
                "import_validate" | "import" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("menu-action", "import");
                }
                "export_preview" | "export" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("menu-action", "export");
                }
                "snippet_edit" | "edit-current" => {
                    let _ = app.emit("menu-action", "edit-current");
                }
                "copy_current" | "copy-current" => {
                    let _ = app.emit("menu-action", "copy-current");
                }
                "show_trash" | "view-trash" | "view_trash" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("navigate", "trash");
                }
                "show_shortcuts" | "shortcuts" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("menu-action", "shortcuts");
                }
                "show_about" | "about" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    let _ = app.emit("menu-action", "about");
                }
                _ => {
                    let _ = platform::dispatch_app_command(id);
                }
            }
        })
        .on_window_event(|window, event| {
            // 点击窗口关闭按钮（X）时隐藏窗口并常驻后台，不退出整个应用程序
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let paths = app.path();
            let data_dir = paths.app_data_dir().map_err(|error| error.to_string())?;
            let config_dir = paths.app_config_dir().map_err(|error| error.to_string())?;
            let log_dir = paths.app_log_dir().map_err(|error| error.to_string())?;
            let logger = Arc::new(DiagnosticLogger::open(&log_dir));

            // 启动 Unix Domain Socket 服务，支持 sxhkd 和 CLI 秒级呼出与单实例通信
            platform::start_socket_server(app.handle().clone());

            // 系统托盘（System Tray）配置
            let tray_search = MenuItem::with_id(app, "tray-search", "显示快速检索 (Alt+O)", true, None::<&str>)?;
            let tray_manager = MenuItem::with_id(app, "tray-manager", "打开管理窗口", true, None::<&str>)?;
            let tray_settings = MenuItem::with_id(app, "tray-settings", "偏好设置", true, None::<&str>)?;
            let tray_quit = MenuItem::with_id(app, "tray-quit", "退出 Searchis", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&tray_search, &tray_manager, &tray_settings, &tray_quit])?;

            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("Searchis - 片段检索与管理")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "tray-search" => {
                        let _ = commands::open_search_window_inner(app);
                    }
                    "tray-manager" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "tray-settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("navigate", "settings");
                    }
                    "tray-quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let _ = commands::open_search_window_inner(app);
                    }
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }

            tray_builder.build(app)?;

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
                                logger.record("shortcut_register", "OK_registered");
                            } else {
                                logger.record("shortcut_register", &format!("OK_updated_from_{prev}"));
                            }
                        }
                        Err(error) => {
                            logger.failure("shortcut_register", &error);
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

            // SPEC-16: 注册 KDE AppMenu 五组菜单（通过 Tauri 原生应用菜单集成 KDE Global Menu）
            logger.record("appmenu_register", "OK_registered");

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
