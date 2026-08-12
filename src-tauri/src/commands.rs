use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::{
    clipboard::TauriClipboardWriter,
    error::AppError,
    logging::DiagnosticLogger,
    model::{
        AutostartInput, CopyInput, CopyOutcome, CreateSnippetInput, ManagerRequest, PasteInput,
        PasteOutcome, SearchInput, SearchResponse, SettingsResponse, SettingsUpdateInput,
        ShortcutInfo, Snippet, TrashEmptyInput, TrashEmptyResult, TrashPurgeInput,
        UpdateSnippetInput,
    },
    platform::{WindowHandle, WindowManager},
    service::SnippetService,
};

// ============================================================
// 应用状态
// ============================================================

pub struct AppState {
    pub service: Option<Arc<SnippetService>>,
    pub startup_error: Option<AppError>,
    pub logger: Arc<DiagnosticLogger>,
    pub manager_request: Mutex<Option<ManagerRequest>>,
    // SPEC-03
    pub window_manager: Arc<dyn WindowManager>,
    pub search_visible: Mutex<bool>,
    pub target_window: Mutex<Option<WindowHandle>>,
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

// ============================================================
// SPEC-01/02: CRUD / 检索 / 复制（既有命令）
// ============================================================

#[tauri::command]
pub fn snippet_create(
    state: tauri::State<'_, AppState>,
    input: CreateSnippetInput,
) -> Result<Snippet, AppError> {
    audited(&state, "snippet_create", |s| s.create(input))
}

#[tauri::command]
pub fn snippet_get(state: tauri::State<'_, AppState>, id: String) -> Result<Snippet, AppError> {
    audited(&state, "snippet_get", |s| s.get(&id))
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
    audited(&state, "snippet_update", |s| s.update(input))
}

#[tauri::command]
pub fn search_snippets(
    state: tauri::State<'_, AppState>,
    input: SearchInput,
) -> Result<SearchResponse, AppError> {
    audited(&state, "search_snippets", |s| s.search(input))
}

#[tauri::command]
pub fn copy_snippet(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input: CopyInput,
) -> Result<CopyOutcome, AppError> {
    let clipboard = TauriClipboardWriter::new(app);
    audited(&state, "copy_snippet", |s| s.copy(input, &clipboard))
}

#[tauri::command]
pub fn prepare_new_snippet(
    state: tauri::State<'_, AppState>,
    raw_query: String,
) -> Result<crate::model::PrepareNewOutcome, AppError> {
    audited(&state, "prepare_new_snippet", |s| s.prepare_new(&raw_query))
}

// ============================================================
// SPEC-11: 窗口管理（双窗口形态）
// ============================================================

#[tauri::command]
pub fn open_search_window(app: tauri::AppHandle) -> Result<(), AppError> {
    open_search_window_inner(&app)
}

#[tauri::command]
pub fn close_search_window(app: tauri::AppHandle) -> Result<(), AppError> {
    close_search_window_inner(&app)
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
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::new("WINDOW_UNAVAILABLE", "应用窗口不可用。请重启应用。"))?;
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
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

// ============================================================
// SPEC-03: 平台能力检测
// ============================================================

#[tauri::command]
pub fn detect_capabilities(
    state: tauri::State<'_, AppState>,
) -> Result<crate::platform::PlatformCapabilities, AppError> {
    let x11 = state.window_manager.injection_available();
    let kga = crate::platform::detect_kglobalaccel_available();
    Ok(crate::platform::PlatformCapabilities {
        clipboard_write: true,
        x11_inject: x11,
        kglobalaccel: kga,
    })
}

// ============================================================
// SPEC-03: 全局快捷键注册（通过 D-Bus）
// ============================================================

#[tauri::command]
pub fn register_shortcut(accelerator: String) -> Result<ShortcutInfo, AppError> {
    let manager = crate::platform::KGlobalAccelManager::new("searchis", "toggle_picker");
    if !manager.available() {
        return Err(AppError::new(
            "KGLOBALACCEL_UNAVAILABLE",
            "KGlobalAccel D-Bus 服务不可用。请手动添加应用内快捷键。",
        ));
    }
    let previous = manager.register(&accelerator)?;
    Ok(ShortcutInfo {
        current: if previous.is_empty() {
            accelerator.clone()
        } else {
            previous
        },
        registered: true,
    })
}

// ============================================================
// SPEC-03: 切换检索窗口（全局快捷键触发 / 应用内 Alt+O）
// ============================================================

#[tauri::command]
pub fn toggle_picker(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    toggle_picker_inner(&app, &state)
}

/// toggle_picker 的内部实现，可从 D-Bus 信号回调直接调用。
pub fn toggle_picker_inner(app: &tauri::AppHandle, state: &AppState) -> Result<(), AppError> {
    let is_visible = *state
        .search_visible
        .lock()
        .map_err(|_| AppError::new("INTERNAL_ERROR", "窗口状态锁失败。"))?;

    if is_visible {
        close_search_window_inner(app)?;
        *state
            .search_visible
            .lock()
            .map_err(|_| AppError::new("INTERNAL_ERROR", ""))? = false;
        *state
            .target_window
            .lock()
            .map_err(|_| AppError::new("INTERNAL_ERROR", ""))? = None;
    } else {
        // 捕获呼出前的前台窗口
        if let Ok(Some(target)) = state.window_manager.get_active_window() {
            *state
                .target_window
                .lock()
                .map_err(|_| AppError::new("INTERNAL_ERROR", ""))? = Some(target);
        }
        open_search_window_inner(app)?;
        *state
            .search_visible
            .lock()
            .map_err(|_| AppError::new("INTERNAL_ERROR", ""))? = true;
    }
    Ok(())
}

// ============================================================
// SPEC-03: 自动粘贴事务
// ============================================================

#[tauri::command]
pub fn execute_paste(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    input: PasteInput,
) -> Result<PasteOutcome, AppError> {
    let clipboard = TauriClipboardWriter::new(app);
    let wm = state.window_manager.clone();
    audited(&state, "execute_paste", |service| {
        service.paste(input, &clipboard, wm.as_ref())
    })
}

// ============================================================
// SPEC-05: 回收站操作
// ============================================================

#[tauri::command]
pub fn trash_move(state: tauri::State<'_, AppState>, id: String) -> Result<Snippet, AppError> {
    audited(&state, "trash_move", |service| service.trash_move(&id))
}

#[tauri::command]
pub fn trash_restore(state: tauri::State<'_, AppState>, id: String) -> Result<Snippet, AppError> {
    audited(&state, "trash_restore", |service| {
        service.trash_restore(&id)
    })
}

#[tauri::command]
pub fn trash_purge_one(
    state: tauri::State<'_, AppState>,
    input: TrashPurgeInput,
) -> Result<(), AppError> {
    // 简单 token 校验：格式为 purge_{id}_{timestamp}
    if !input.confirmation_token.starts_with("purge_") {
        return Err(AppError::new("INVALID_TOKEN", "确认令牌无效，请重新确认。"));
    }
    audited(&state, "trash_purge_one", |service| {
        service.trash_purge_one(&input.id)
    })
}

#[tauri::command]
pub fn trash_empty(
    state: tauri::State<'_, AppState>,
    input: TrashEmptyInput,
) -> Result<TrashEmptyResult, AppError> {
    if !input.confirmation_token.starts_with("empty_") {
        return Err(AppError::new("INVALID_TOKEN", "确认令牌无效，请重新确认。"));
    }
    let count = audited(&state, "trash_empty", |service| service.trash_empty())?;
    Ok(TrashEmptyResult {
        deleted_count: count,
    })
}

#[tauri::command]
pub fn trash_auto_purge(
    state: tauri::State<'_, AppState>,
    days: Option<i64>,
) -> Result<i64, AppError> {
    audited(&state, "trash_auto_purge", |service| {
        service.auto_purge(days)
    })
}

// ============================================================
// SPEC-06: 设置
// ============================================================

#[tauri::command]
pub fn settings_get(state: tauri::State<'_, AppState>) -> Result<SettingsResponse, AppError> {
    let actual = autostart_get().unwrap_or(false);
    audited(&state, "settings_get", |service| {
        service.get_settings(actual)
    })
}

#[tauri::command]
pub fn settings_update(
    state: tauri::State<'_, AppState>,
    input: SettingsUpdateInput,
) -> Result<crate::model::Settings, AppError> {
    audited(&state, "settings_update", |service| {
        service.update_settings(&input.key, &input.value, input.revision)
    })
}

// === XDG Autostart ===

fn autostart_desktop_path() -> Result<std::path::PathBuf, AppError> {
    let config_home = std::env::var("XDG_CONFIG_HOME").or_else(|_| {
        let home = std::env::var("HOME")
            .map_err(|_| AppError::new("AUTOSTART_FAILED", "无法获取 HOME 环境变量。"))?;
        Ok(format!("{}/.config", home))
    })?;
    Ok(std::path::PathBuf::from(config_home)
        .join("autostart")
        .join("io.searchis.desktop"))
}

#[tauri::command]
pub fn autostart_get() -> Result<bool, AppError> {
    let path = autostart_desktop_path()?;
    if !path.exists() {
        return Ok(false);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|_| AppError::new("AUTOSTART_UPDATE_FAILED", "无法读取自启动配置文件。"))?;
    Ok(content.contains("X-GNOME-Autostart-enabled=true")
        || (content.contains("X-GNOME-Autostart-enabled")
            && !content.contains("X-GNOME-Autostart-enabled=false")))
}

fn atomic_write(path: &std::path::Path, content: &str) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("AUTOSTART_UPDATE_FAILED", "无效的自启动目录。"))?;
    std::fs::create_dir_all(parent)
        .map_err(|_| AppError::new("AUTOSTART_UPDATE_FAILED", "无法创建自启动目录。"))?;
    let temp = parent.join(format!(".io.searchis.desktop.{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temp, content)
        .map_err(|_| AppError::new("AUTOSTART_UPDATE_FAILED", "无法写入临时自启动文件。"))?;
    let result = std::fs::rename(&temp, path)
        .map_err(|_| AppError::new("AUTOSTART_UPDATE_FAILED", "无法原子替换自启动文件。"));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[tauri::command]
pub fn autostart_set(_app: tauri::AppHandle, input: AutostartInput) -> Result<bool, AppError> {
    let path = autostart_desktop_path()?;
    let old_content = std::fs::read(&path).ok();
    let exe_path = std::env::current_exe()
        .map_err(|_| AppError::new("AUTOSTART_UPDATE_FAILED", "无法获取可执行文件路径。"))?;
    let content = if input.enabled {
        format!(
            "[Desktop Entry]\nType=Application\nName=Searchis\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            exe_path.display()
        )
    } else if path.exists() {
        let mut existing = std::fs::read_to_string(&path)
            .map_err(|_| AppError::new("AUTOSTART_UPDATE_FAILED", "无法读取自启动配置文件。"))?;
        existing = existing.replace(
            "X-GNOME-Autostart-enabled=true",
            "X-GNOME-Autostart-enabled=false",
        );
        if !existing.contains("X-GNOME-Autostart-enabled") {
            existing.push_str("\nX-GNOME-Autostart-enabled=false\n");
        }
        existing
    } else {
        return Ok(false);
    };
    atomic_write(&path, &content)?;
    match autostart_get() {
        Ok(actual) if actual == input.enabled => Ok(actual),
        Ok(_) | Err(_) => {
            match old_content {
                Some(bytes) => {
                    let _ = std::fs::write(&path, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(&path);
                }
            }
            Err(AppError::new(
                "AUTOSTART_UPDATE_FAILED",
                "自启动实际状态与请求不一致，已恢复旧状态。",
            ))
        }
    }
}

// ============================================================
// SPEC-07: 备份、恢复与重置
// ============================================================

#[tauri::command]
pub fn export_preview(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<crate::model::ExportPreview, AppError> {
    audited(&state, "export_preview", |service| {
        service.export_preview(&path)
    })
}

#[tauri::command]
pub fn export_confirm(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<crate::model::ExportOutcome, AppError> {
    audited(&state, "export_confirm", |service| {
        service.export_confirm(&path)
    })
}

#[tauri::command]
pub fn import_validate(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<crate::model::ImportValidation, AppError> {
    audited(&state, "import_validate", |service| {
        service.import_validate(&path)
    })
}

#[tauri::command]
pub fn import_commit(
    state: tauri::State<'_, AppState>,
    input: crate::model::ImportCommitInput,
) -> Result<crate::model::ImportOutcome, AppError> {
    audited(&state, "import_commit", |service| {
        service.import_commit(&input.path, &input.import_token)
    })
}

#[tauri::command]
pub fn reset_examples(
    state: tauri::State<'_, AppState>,
    input: crate::model::ResetExamplesInput,
) -> Result<i64, AppError> {
    if !input.confirm {
        return Err(AppError::new("CONFIRM_REQUIRED", "请先确认重置操作。"));
    }
    audited(&state, "reset_examples", |service| service.reset_examples())
}

// ============================================================
// SPEC-08: Onboarding
// ============================================================

#[tauri::command]
pub fn environment_detect(
    state: tauri::State<'_, AppState>,
) -> Result<crate::model::EnvironmentDetection, AppError> {
    audited(&state, "environment_detect", |service| {
        service.environment_detect()
    })
}

#[tauri::command]
pub fn onboarding_progress(
    state: tauri::State<'_, AppState>,
    input: crate::model::OnboardingProgressInput,
) -> Result<i64, AppError> {
    audited(&state, "onboarding_progress", |service| {
        service.onboarding_progress(input.step, input.revision)
    })
}

#[tauri::command]
pub fn onboarding_complete(
    state: tauri::State<'_, AppState>,
    input: crate::model::OnboardingCompleteInput,
) -> Result<Option<String>, AppError> {
    audited(&state, "onboarding_complete", |service| {
        service.onboarding_complete(&input.action)
    })
}

// ============================================================
// SPEC-09: KDE Plasma Global Menu
// ============================================================

#[tauri::command]
pub fn appmenu_get_model(
    current_snippet: Option<String>,
    current_view: String,
) -> Result<crate::model::MenuModel, AppError> {
    Ok(crate::platform::build_menu_model(
        current_snippet.as_deref(),
        &current_view,
    ))
}

#[tauri::command]
pub fn appmenu_dispatch(input: crate::model::AppMenuCommandInput) -> Result<String, AppError> {
    crate::platform::dispatch_app_command(&input.command_id)
}

#[tauri::command]
pub fn appmenu_check_availability() -> Result<bool, AppError> {
    Ok(crate::platform::AppMenuRegistrar::check_availability())
}

// ============================================================
// 辅助函数
// ============================================================

pub fn open_search_window_inner(app: &tauri::AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("search")
        .ok_or_else(|| AppError::new("WINDOW_UNAVAILABLE", "检索窗口不可用。请重启应用。"))?;
    let _ = window.set_size(tauri::LogicalSize::new(780.0, 500.0));
    let _ = window.center();
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if let Some(window) = app.get_webview_window("search") {
            let _ = window.set_focus();
        }
    });
    Ok(())
}

pub fn close_search_window_inner(app: &tauri::AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("search")
        .ok_or_else(|| AppError::new("WINDOW_UNAVAILABLE", "应用窗口不可用。请重启应用。"))?;
    let _ = window.hide();
    Ok(())
}
