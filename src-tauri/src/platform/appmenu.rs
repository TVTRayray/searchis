use crate::error::AppError;

/// D-Bus AppMenu 注册器（SPEC-09）
/// 通过 D-Bus 向 KDE Plasma Global Menu 注册应用菜单。
#[allow(dead_code)]
pub struct AppMenuRegistrar {
    bus: Option<zbus::blocking::Connection>,
    registered_id: Option<String>,
}

#[allow(dead_code)]
impl AppMenuRegistrar {
    pub fn new() -> Self {
        let bus = zbus::blocking::Connection::session().ok();
        Self {
            bus,
            registered_id: None,
        }
    }

    pub fn register(&mut self, _menu: &crate::model::MenuModel) -> Result<(), AppError> {
        if let Some(ref _conn) = self.bus {
            // 尝试注册到 AppMenu Registrar
            let result = std::process::Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.kde.kappmenu",
                    "/MenuBar",
                    "org.kde.kappmenu",
                    "RegisterWindow",
                    "s",
                    "searchis",
                ])
                .output();
            match result {
                Ok(output) if output.status.success() => {
                    self.registered_id = Some("searchis".to_string());
                    Ok(())
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(AppError::new(
                        "DBUS_APPMENU_REGISTRATION_FAILED",
                        format!("D-Bus 注册失败: {stderr}"),
                    ))
                }
                Err(e) => Err(AppError::new(
                    "DBUS_APPMENU_REGISTRATION_FAILED",
                    format!("busctl 执行失败: {e}"),
                )),
            }
        } else {
            Err(AppError::new(
                "DBUS_APPMENU_REGISTRATION_FAILED",
                "D-Bus session 连接不可用。",
            ))
        }
    }

    pub fn unregister(&mut self) {
        if self.registered_id.is_some() {
            let _ = std::process::Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.kde.kappmenu",
                    "/MenuBar",
                    "org.kde.kappmenu",
                    "UnregisterWindow",
                    "s",
                    "searchis",
                ])
                .output();
            self.registered_id = None;
        }
    }

    pub fn is_registered(&self) -> bool {
        self.registered_id.is_some()
    }

    /// 检查 D-Bus AppMenu 服务是否可用（通过 busctl 列出 kappmenu 服务）
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
