use crate::error::AppError;
use tauri::menu::{Menu, MenuItem, Submenu};

/// 构建 FR-KDE-01 定义的五组菜单（用于 Tauri 原生应用菜单 / KDE Global Menu）。
pub fn create_app_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    // 1. Searchis 菜单
    let show_picker =
        MenuItem::with_id(app, "open_search_window", "显示检索窗口", true, Some("Alt+O"))?;
    let show_manager =
        MenuItem::with_id(app, "open_manager_window", "显示管理窗口", true, None::<&str>)?;
    let settings =
        MenuItem::with_id(app, "open_settings", "设置", true, None::<&str>)?;
    let quit =
        MenuItem::with_id(app, "app_quit", "退出", true, None::<&str>)?;
    let searchis_menu = Submenu::with_items(
        app,
        "Searchis",
        true,
        &[&show_picker, &show_manager, &settings, &quit],
    )?;

    // 2. 文件 菜单
    let new_snippet =
        MenuItem::with_id(app, "snippet_create", "新建片段", true, None::<&str>)?;
    let import =
        MenuItem::with_id(app, "import_validate", "导入", true, None::<&str>)?;
    let export =
        MenuItem::with_id(app, "export_preview", "导出", true, None::<&str>)?;
    let file_menu = Submenu::with_items(
        app,
        "文件",
        true,
        &[&new_snippet, &import, &export],
    )?;

    // 3. 编辑 菜单
    let edit_current =
        MenuItem::with_id(app, "snippet_edit", "编辑当前片段", true, None::<&str>)?;
    let copy_current =
        MenuItem::with_id(app, "copy_current", "复制当前片段", true, None::<&str>)?;
    let edit_menu = Submenu::with_items(
        app,
        "编辑",
        true,
        &[&edit_current, &copy_current],
    )?;

    // 4. 视图 菜单
    let view_picker =
        MenuItem::with_id(app, "view_picker", "检索窗口", true, None::<&str>)?;
    let view_manager =
        MenuItem::with_id(app, "view_manager", "管理窗口", true, None::<&str>)?;
    let view_trash =
        MenuItem::with_id(app, "show_trash", "回收站", true, None::<&str>)?;
    let view_menu = Submenu::with_items(
        app,
        "视图",
        true,
        &[&view_picker, &view_manager, &view_trash],
    )?;

    // 5. 帮助 菜单
    let shortcuts =
        MenuItem::with_id(app, "show_shortcuts", "快捷键", true, None::<&str>)?;
    let about =
        MenuItem::with_id(app, "show_about", "关于", true, None::<&str>)?;
    let help_menu = Submenu::with_items(
        app,
        "帮助",
        true,
        &[&shortcuts, &about],
    )?;

    Menu::with_items(
        app,
        &[
            &searchis_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &help_menu,
        ],
    )
}

/// D-Bus / KDE AppMenu 状态检测组件
pub struct AppMenuRegistrar;

impl AppMenuRegistrar {
    /// 检查 KDE AppMenu / Global Menu 服务是否可用（IPC `appmenu_check_availability` 复用）。
    pub fn check_availability() -> bool {
        std::process::Command::new("busctl")
            .args(["--user", "list"])
            .output()
            .map(|o| {
                let output = String::from_utf8_lossy(&o.stdout);
                output.contains("kappmenu") || output.contains("GlobalMenu")
            })
            .unwrap_or(false)
    }
}

// ============================================================
// 五组菜单模型（FR-KDE-01）
// ============================================================

/// 构建 FR-KDE-01 定义的五组菜单模型。
pub fn build_menu_model(
    current_snippet: Option<&str>,
    current_view: &str,
) -> crate::model::MenuModel {
    let has_snippet = current_snippet.is_some();

    crate::model::MenuModel {
        version: 1,
        groups: vec![
            // 1. Searchis 主菜单
            crate::model::MenuGroup {
                id: "searchis".into(),
                label: "Searchis".into(),
                items: vec![
                    crate::model::MenuItem {
                        id: "show-picker".into(),
                        label: "显示检索窗口".into(),
                        command_id: "open_search_window".into(),
                        enabled: true,
                        checked: false,
                        shortcut: Some("Alt+O".into()),
                    },
                    crate::model::MenuItem {
                        id: "show-manager".into(),
                        label: "显示管理窗口".into(),
                        command_id: "open_manager_window".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "settings".into(),
                        label: "设置".into(),
                        command_id: "open_settings".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "quit".into(),
                        label: "退出".into(),
                        command_id: "app_quit".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                ],
            },
            // 2. 文件
            crate::model::MenuGroup {
                id: "file".into(),
                label: "文件".into(),
                items: vec![
                    crate::model::MenuItem {
                        id: "new-snippet".into(),
                        label: "新建片段".into(),
                        command_id: "snippet_create".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "import".into(),
                        label: "导入".into(),
                        command_id: "import_validate".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "export".into(),
                        label: "导出".into(),
                        command_id: "export_preview".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                ],
            },
            // 3. 编辑
            crate::model::MenuGroup {
                id: "edit".into(),
                label: "编辑".into(),
                items: vec![
                    crate::model::MenuItem {
                        id: "edit-current".into(),
                        label: "编辑当前片段".into(),
                        command_id: "snippet_edit".into(),
                        enabled: has_snippet,
                        checked: false,
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "copy-current".into(),
                        label: "复制当前片段".into(),
                        command_id: "copy_current".into(),
                        enabled: has_snippet,
                        checked: false,
                        shortcut: None,
                    },
                ],
            },
            // 4. 视图
            crate::model::MenuGroup {
                id: "view".into(),
                label: "视图".into(),
                items: vec![
                    crate::model::MenuItem {
                        id: "view-picker".into(),
                        label: "检索窗口".into(),
                        command_id: "open_search_window".into(),
                        enabled: true,
                        checked: current_view == "picker",
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "view-manager".into(),
                        label: "管理窗口".into(),
                        command_id: "open_manager_window".into(),
                        enabled: true,
                        checked: current_view == "manager",
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "view-trash".into(),
                        label: "回收站".into(),
                        command_id: "show_trash".into(),
                        enabled: true,
                        checked: current_view == "trash",
                        shortcut: None,
                    },
                ],
            },
            // 5. 帮助
            crate::model::MenuGroup {
                id: "help".into(),
                label: "帮助".into(),
                items: vec![
                    crate::model::MenuItem {
                        id: "shortcuts".into(),
                        label: "快捷键".into(),
                        command_id: "show_shortcuts".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                    crate::model::MenuItem {
                        id: "about".into(),
                        label: "关于".into(),
                        command_id: "show_about".into(),
                        enabled: true,
                        checked: false,
                        shortcut: None,
                    },
                ],
            },
        ],
    }
}

/// 分发菜单命令到对应的应用命令
pub fn dispatch_app_command(command_id: &str) -> Result<String, AppError> {
    match command_id {
        "open_search_window" => Ok("OPEN_SEARCH_WINDOW".into()),
        "open_manager_window" => Ok("OPEN_MANAGER_WINDOW".into()),
        "open_settings" => Ok("OPEN_SETTINGS".into()),
        "app_quit" => Ok("APP_QUIT".into()),
        "snippet_create" => Ok("SNIPPET_CREATE".into()),
        "snippet_edit" => Ok("SNIPPET_EDIT".into()),
        "copy_current" => Ok("COPY_CURRENT".into()),
        "import_validate" => Ok("IMPORT_VALIDATE".into()),
        "export_preview" => Ok("EXPORT_PREVIEW".into()),
        "show_trash" => Ok("SHOW_TRASH".into()),
        "show_shortcuts" => Ok("SHOW_SHORTCUTS".into()),
        "show_about" => Ok("SHOW_ABOUT".into()),
        _ => Err(AppError::new(
            "INVALID_COMMAND",
            format!("未知命令: {command_id}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_model_has_five_groups() {
        let menu = build_menu_model(None, "");
        assert_eq!(menu.groups.len(), 5);
        assert_eq!(menu.groups[0].id, "searchis");
        assert_eq!(menu.groups[1].id, "file");
        assert_eq!(menu.groups[2].id, "edit");
        assert_eq!(menu.groups[3].id, "view");
        assert_eq!(menu.groups[4].id, "help");
    }

    #[test]
    fn menu_model_reflects_snippet_and_view_state() {
        let menu_no_snippet = build_menu_model(None, "manager");
        let edit_group = &menu_no_snippet.groups[2];
        assert!(!edit_group.items[0].enabled);
        assert!(!edit_group.items[1].enabled);

        let view_group = &menu_no_snippet.groups[3];
        assert!(view_group.items[1].checked);
        assert!(!view_group.items[0].checked);

        let menu_with_snippet = build_menu_model(Some("snp_123"), "trash");
        let edit_group_active = &menu_with_snippet.groups[2];
        assert!(edit_group_active.items[0].enabled);
        assert!(edit_group_active.items[1].enabled);

        let view_group_trash = &menu_with_snippet.groups[3];
        assert!(view_group_trash.items[2].checked);
    }

    #[test]
    fn dispatch_known_commands_success() {
        assert_eq!(
            dispatch_app_command("open_search_window").unwrap(),
            "OPEN_SEARCH_WINDOW"
        );
        assert_eq!(
            dispatch_app_command("open_manager_window").unwrap(),
            "OPEN_MANAGER_WINDOW"
        );
        assert_eq!(
            dispatch_app_command("open_settings").unwrap(),
            "OPEN_SETTINGS"
        );
        assert_eq!(dispatch_app_command("app_quit").unwrap(), "APP_QUIT");
        assert_eq!(
            dispatch_app_command("snippet_create").unwrap(),
            "SNIPPET_CREATE"
        );
        assert_eq!(
            dispatch_app_command("snippet_edit").unwrap(),
            "SNIPPET_EDIT"
        );
        assert_eq!(dispatch_app_command("copy_current").unwrap(), "COPY_CURRENT");
        assert_eq!(
            dispatch_app_command("import_validate").unwrap(),
            "IMPORT_VALIDATE"
        );
        assert_eq!(
            dispatch_app_command("export_preview").unwrap(),
            "EXPORT_PREVIEW"
        );
        assert_eq!(dispatch_app_command("show_trash").unwrap(), "SHOW_TRASH");
        assert_eq!(
            dispatch_app_command("show_shortcuts").unwrap(),
            "SHOW_SHORTCUTS"
        );
        assert_eq!(dispatch_app_command("show_about").unwrap(), "SHOW_ABOUT");
    }

    #[test]
    fn dispatch_unknown_command_maps_to_invalid_command() {
        let err = dispatch_app_command("no-such-command").unwrap_err();
        assert_eq!(err.code, "INVALID_COMMAND");
    }

    #[test]
    fn check_availability_does_not_panic() {
        let _ = AppMenuRegistrar::check_availability();
    }
}
