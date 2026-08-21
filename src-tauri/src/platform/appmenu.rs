use crate::error::AppError;
use std::collections::HashMap;
use std::sync::Arc;
use zbus::blocking::connection as blocking_connection;
use zbus::interface;
use zbus::zvariant::{Array, Dict, OwnedValue, Signature, Structure, Value};

fn sig(s: &'static str) -> Signature<'static> {
    Signature::from_static_str_unchecked(s)
}

/// 菜单命令分发句柄：menu command_id -> 既有业务命令。
type MenuDispatch = Arc<dyn Fn(&str) -> Result<String, AppError> + Send + Sync>;

fn dbus_err(message: impl Into<String>) -> AppError {
    AppError::new("DBUS_APPMENU_REGISTRATION_FAILED", message)
}

/// 把任意 D-Bus 值包装为 `v` 变体（`a{sv}` 值 / `av` 元素都要求 variant）。
fn as_variant(value: Value<'_>) -> Result<OwnedValue, AppError> {
    let owned = value
        .try_to_owned()
        .map_err(|e| dbus_err(format!("无法转义 D-Bus 值: {e}")))?;
    OwnedValue::try_from(Value::Value(Box::new(Value::from(owned))))
        .map_err(|e| dbus_err(format!("无法构造 D-Bus 变体: {e}")))
}

/// D-Bus 空值（signature `()`），用于占位返回值。
fn null_owned() -> OwnedValue {
    let arr = Array::new(sig("v"));
    OwnedValue::try_from(Value::Array(arr)).unwrap_or_else(|_| unreachable!())
}

// ============================================================
// D-Bus 交互面（可注入，用于替身测试）
// ============================================================

pub trait AppMenuDbus: Send + Sync {
    fn available(&self) -> bool;
    /// 向 Registrar 注册真实 X11 窗口 XID 与 serviceAndPath。
    fn register_window(&self, window_id: i64, service_and_path: &str) -> Result<(), String>;
    fn unregister_window(&self, window_id: i64);
}

/// 真实实现：通过 `busctl` 与 KDE AppMenu Registrar 交互。
pub struct BusctlAppMenuDbus {
    service: &'static str,
}

impl BusctlAppMenuDbus {
    pub fn new() -> Self {
        Self {
            service: "org.kde.kappmenu",
        }
    }
}

impl Default for BusctlAppMenuDbus {
    fn default() -> Self {
        Self::new()
    }
}

impl AppMenuDbus for BusctlAppMenuDbus {
    fn available(&self) -> bool {
        std::process::Command::new("busctl")
            .args(["--user", "list"])
            .output()
            .map(|o| {
                let output = String::from_utf8_lossy(&o.stdout);
                output.contains("kappmenu") || output.contains("GlobalMenu")
            })
            .unwrap_or(false)
    }

    fn register_window(&self, window_id: i64, service_and_path: &str) -> Result<(), String> {
        let output = std::process::Command::new("busctl")
            .args([
                "--user",
                "call",
                self.service,
                "/MenuBar",
                "org.kde.kappmenu",
                "RegisterWindow",
                "xs",
                &window_id.to_string(),
                service_and_path,
            ])
            .output()
            .map_err(|e| format!("busctl 执行失败: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("RegisterWindow 失败: {stderr}"))
        }
    }

    fn unregister_window(&self, window_id: i64) {
        let _ = std::process::Command::new("busctl")
            .args([
                "--user",
                "call",
                self.service,
                "/MenuBar",
                "org.kde.kappmenu",
                "UnregisterWindow",
                "x",
                &window_id.to_string(),
            ])
            .output();
    }
}

/// 用既有 `xdotool` 定位主窗口（label=main，标题 `Searchis`）的 X11 XID。
fn find_main_window_xid() -> Option<i64> {
    let output = std::process::Command::new("xdotool")
        .args(["search", "--name", "^Searchis$"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().last().and_then(|line| line.trim().parse::<i64>().ok())
}

// ============================================================
// com.canonical.dbusmenu 数据模型
// ============================================================

struct DbusNode {
    label: String,
    enabled: bool,
    checked: bool,
    shortcut: Option<String>,
    command_id: Option<String>,
    children: Vec<i32>,
}

/// `com.canonical.dbusmenu` 服务：持有五组菜单模型与命令分发句柄，
/// 为 KDE Global Menu 提供 GetLayout/Event 等协议方法。
pub struct DbusMenuService {
    dispatch: MenuDispatch,
    nodes: HashMap<i32, DbusNode>,
    top_ids: Vec<i32>,
}

impl DbusMenuService {
    fn new(menu: crate::model::MenuModel, dispatch: MenuDispatch) -> Self {
        let mut nodes = HashMap::new();
        let mut top_ids = Vec::new();
        let mut next = 1i32;
        for group in &menu.groups {
            let group_id = next;
            next += 1;
            let mut child_ids = Vec::new();
            for item in &group.items {
                let item_id = next;
                next += 1;
                nodes.insert(
                    item_id,
                    DbusNode {
                        label: item.label.clone(),
                        enabled: item.enabled,
                        checked: item.checked,
                        shortcut: item.shortcut.clone(),
                        command_id: Some(item.command_id.clone()),
                        children: vec![],
                    },
                );
                child_ids.push(item_id);
            }
            nodes.insert(
                group_id,
                DbusNode {
                    label: group.label.clone(),
                    enabled: true,
                    checked: false,
                    shortcut: None,
                    command_id: None,
                    children: child_ids,
                },
            );
            top_ids.push(group_id);
        }
        Self {
            dispatch,
            nodes,
            top_ids,
        }
    }

    fn children_of(&self, id: i32) -> Vec<i32> {
        if id == 0 {
            self.top_ids.clone()
        } else {
            self.nodes
                .get(&id)
                .map(|node| node.children.clone())
                .unwrap_or_default()
        }
    }

    fn shortcut_value(node: &DbusNode) -> Option<OwnedValue> {
        let shortcut = node.shortcut.as_ref()?;
        let mut outer = Array::new(sig("as"));
        let mut inner = Array::new(sig("s"));
        for key in shortcut.split('+').map(|s| s.trim().to_string()) {
            let _ = inner.append(Value::from(key));
        }
        if outer.append(Value::Array(inner)).is_err() {
            return None;
        }
        as_variant(Value::Array(outer)).ok()
    }

    fn prop_value(&self, node: &DbusNode, name: &str) -> Option<OwnedValue> {
        match name {
            "label" => as_variant(Value::from(node.label.clone())).ok(),
            "enabled" => as_variant(Value::from(node.enabled)).ok(),
            "shortcut" => Self::shortcut_value(node),
            "toggle-type" if node.checked => as_variant(Value::from("checkmark")).ok(),
            "toggle-state" if node.checked => as_variant(Value::from(1i32)).ok(),
            _ => None,
        }
    }

    fn props_of(&self, id: i32) -> HashMap<String, OwnedValue> {
        let mut props = HashMap::new();
        if let Some(node) = self.nodes.get(&id) {
            for name in ["label", "enabled", "shortcut", "toggle-type", "toggle-state"] {
                if let Some(value) = self.prop_value(node, name) {
                    props.insert(name.to_string(), value);
                }
            }
        }
        props
    }

    fn item_value(&self, id: i32, depth: i32) -> Result<OwnedValue, AppError> {
        let node = self.nodes.get(&id).ok_or_else(|| dbus_err("未知菜单项"))?;
        let mut props_dict = Dict::new(sig("s"), sig("v"));
        for (key, val) in self.props_of(id) {
            props_dict
                .append(Value::from(key), Value::from(val))
                .map_err(|e| dbus_err(format!("布局属性构造失败: {e}")))?;
        }
        let mut children_array = Array::new(sig("v"));
        if depth != 0 {
            for child_id in &node.children {
                let child = self.item_value(*child_id, depth - 1)?;
                children_array
                    .append(Value::Value(Box::new(Value::from(child))))
                    .map_err(|e| dbus_err(format!("子菜单构造失败: {e}")))?;
            }
        }
        let structure: Structure<'static> =
            (Value::I32(id), Value::Dict(props_dict), Value::Array(children_array)).into();
        OwnedValue::try_from(Value::Structure(structure))
            .map_err(|e| dbus_err(format!("菜单项序列化失败: {e}")))
    }
}

// ============================================================
// com.canonical.dbusmenu D-Bus 接口（KDE Global Menu 协议）
// ============================================================

#[interface(name = "com.canonical.dbusmenu")]
impl DbusMenuService {
    // 属性名默认取方法名的 PascalCase：version→Version、text_direction→TextDirection、status→Status。
    #[zbus(property)]
    fn version(&self) -> i32 {
        3
    }

    #[zbus(property)]
    fn text_direction(&self) -> String {
        "ltr".to_string()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "normal".to_string()
    }

    /// 返回 (revision, layout)，layout 为 `a(ia{sv}av)` 布局。
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (i32, OwnedValue) {
        let mut layout = Array::new(sig("(ia{sv}av)"));
        for child_id in self.children_of(parent_id) {
            if let Ok(item) = self.item_value(child_id, recursion_depth) {
                let _ = layout.append(Value::from(item));
            }
        }
        let layout_val = Value::Array(layout);
        match OwnedValue::try_from(layout_val) {
            Ok(v) => (1, v),
            Err(_) => (1, null_owned()),
        }
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        ids.into_iter()
            .filter(|id| *id == 0 || self.nodes.contains_key(id))
            .map(|id| (id, self.props_of(id)))
            .collect()
    }

    fn get_property(&self, id: i32, name: String) -> OwnedValue {
        self.nodes
            .get(&id)
            .and_then(|node| self.prop_value(node, &name))
            .unwrap_or_else(null_owned)
    }

    fn event(&self, id: i32, event_id: String, _data: OwnedValue, _timestamp: u32) {
        if event_id == "clicked" {
            if let Some(command_id) = self.nodes.get(&id).and_then(|node| node.command_id.clone()) {
                let _ = (self.dispatch)(&command_id);
            }
        }
    }

    fn about_to_show(&self, _id: i32) -> HashMap<String, OwnedValue> {
        HashMap::new()
    }
}

// ============================================================
// DBusMenu 服务器与 AppMenu 注册器
// ============================================================

pub struct DbusMenuServer;

impl DbusMenuServer {
    /// 在会话总线导出 `com.canonical.dbusmenu`（唯一名 `org.kde.searchis`，路径 `/MenuBar`）。
    /// 返回连接需保持存活（由监视线线程持有）。
    pub fn start(
        menu: crate::model::MenuModel,
    ) -> Result<zbus::blocking::Connection, AppError> {
        let dispatch: MenuDispatch = Arc::new(dispatch_app_command);
        let service = DbusMenuService::new(menu, dispatch);
        blocking_connection::Builder::session()
            .map_err(|e| dbus_err(format!("D-Bus session 连接失败: {e}")))?
            .name("org.kde.searchis")
            .map_err(|e| dbus_err(format!("D-Bus 唯一名注册失败: {e}")))?
            .serve_at("/MenuBar", service)
            .map_err(|e| dbus_err(format!("DBusMenu 对象导出失败: {e}")))?
            .build()
            .map_err(|e| dbus_err(format!("DBusMenu 连接建立失败: {e}")))
    }
}

/// D-Bus AppMenu 注册器：负责导出 DBusMenu + 向 kappmenu Registrar 注册真实 XID，
/// 并后台监视 Registrar 出现/消失以 5 秒内重注册。
pub struct AppMenuRegistrar {
    dbus: Box<dyn AppMenuDbus>,
    server: Option<zbus::blocking::Connection>,
    registered: bool,
    #[cfg(test)]
    test_xid: Option<Option<i64>>,
    #[cfg(test)]
    skip_server_build: bool,
}

impl AppMenuRegistrar {
    pub fn new() -> Self {
        Self::with_dbus(Box::new(BusctlAppMenuDbus::new()))
    }

    pub fn with_dbus(dbus: Box<dyn AppMenuDbus>) -> Self {
        Self {
            dbus,
            server: None,
            registered: false,
            #[cfg(test)]
            test_xid: None,
            #[cfg(test)]
            skip_server_build: false,
        }
    }

    fn find_xid(&self) -> Option<i64> {
        #[cfg(test)]
        if let Some(xid) = self.test_xid {
            return xid;
        }
        find_main_window_xid()
    }

    /// 导出 DBusMenu 并向 Registrar 注册真实 XID（serviceAndPath=`org.kde.searchis/MenuBar`）。
    pub fn register(&mut self, menu: &crate::model::MenuModel) -> Result<(), AppError> {
        #[cfg(test)]
        let should_build_server = !self.skip_server_build;
        #[cfg(not(test))]
        let should_build_server = true;
        if should_build_server && self.server.is_none() {
            match DbusMenuServer::start(menu.clone()) {
                Ok(connection) => self.server = Some(connection),
                Err(e) => return Err(e),
            }
        }
        // 找不到主窗口 XID 或 Registrar 不可用时优雅降级（不阻塞，不假报成功）。
        let Some(xid) = self.find_xid() else {
            self.registered = false;
            return Err(dbus_err("无法定位主窗口 XID，稍后重试注册。"));
        };
        if !self.dbus.available() {
            self.registered = false;
            return Err(dbus_err("D-Bus AppMenu Registrar 不可用，Global Menu 不显示。"));
        }
        match self
            .dbus
            .register_window(xid, "org.kde.searchis/MenuBar")
        {
            Ok(()) => {
                self.registered = true;
                Ok(())
            }
            Err(msg) => {
                self.registered = false;
                Err(dbus_err(format!("D-Bus 注册失败: {msg}")))
            }
        }
    }

    /// 同步状态：Registrar 可用且 XID 存在则确保已注册；不可用则清除注册态以便恢复时重注册。
    pub fn sync(&mut self, menu: &crate::model::MenuModel) -> Result<(), AppError> {
        if !self.dbus.available() {
            self.registered = false;
            return Err(dbus_err("D-Bus AppMenu Registrar 不可用，Global Menu 不显示。"));
        }
        if !self.registered {
            self.register(menu)?;
        }
        Ok(())
    }

    /// 退出时注销。D-Bus session 连接随进程退出自动清理，运行时可不调用；保留契约表面。
    #[allow(dead_code)]
    pub fn unregister(&mut self) {
        if self.registered {
            if let Some(xid) = self.find_xid() {
                self.dbus.unregister_window(xid);
            }
            self.registered = false;
        }
    }

    #[allow(dead_code)] // 由测试与外部契约使用；保持可观测性。
    pub fn is_registered(&self) -> bool {
        self.registered
    }

    /// 检查 D-Bus AppMenu 服务是否可用（IPC `appmenu_check_availability` 复用）。
    pub fn check_availability() -> bool {
        BusctlAppMenuDbus::new().available()
    }
}

impl Default for AppMenuRegistrar {
    fn default() -> Self {
        Self::new()
    }
}

/// 后台监视线程：持有 DBusMenu 连接（保证常驻导出），每 1 秒调用 `sync`；
/// Registrar 消失则清除注册态，重新出现后 1 秒内重注册（满足 AGENT.md 契约的 5 秒内重注册）。
pub fn spawn_appmenu_monitor(
    registrar: std::sync::Arc<std::sync::Mutex<AppMenuRegistrar>>,
    menu: crate::model::MenuModel,
) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Ok(mut reg) = registrar.lock() {
                let _ = reg.sync(&menu);
            }
        }
    });
}

// ============================================================
// 五组菜单模型
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
    use crate::model::MenuModel;
    use std::sync::Mutex;

    struct MockState {
        available: bool,
        register_result: Result<(), String>,
        registered_xid: Vec<(i64, String)>,
    }

    impl Default for MockState {
        fn default() -> Self {
            Self {
                available: true,
                register_result: Ok(()),
                registered_xid: vec![],
            }
        }
    }

    struct MockAppMenuDbus {
        state: Arc<Mutex<MockState>>,
    }

    impl AppMenuDbus for MockAppMenuDbus {
        fn available(&self) -> bool {
            self.state.lock().unwrap().available
        }
        fn register_window(&self, window_id: i64, service_and_path: &str) -> Result<(), String> {
            self.state
                .lock()
                .unwrap()
                .registered_xid
                .push((window_id, service_and_path.to_string()));
            self.state.lock().unwrap().register_result.clone()
        }
        fn unregister_window(&self, _window_id: i64) {}
    }

    fn empty_menu() -> MenuModel {
        MenuModel {
            version: 1,
            groups: vec![],
        }
    }

    fn five_group_menu() -> MenuModel {
        build_menu_model(None, "")
    }

    fn state() -> Arc<Mutex<MockState>> {
        Arc::new(Mutex::new(MockState::default()))
    }

    /// 构造一个跳过真实 zbus 服务器、且 XID 可控的注册器，用于确定性的替身测试。
    fn stub_registrar(state: &Arc<Mutex<MockState>>, xid: Option<i64>) -> AppMenuRegistrar {
        let mut reg = AppMenuRegistrar::with_dbus(Box::new(MockAppMenuDbus {
            state: Arc::clone(state),
        }));
        reg.skip_server_build = true;
        reg.test_xid = Some(xid);
        reg
    }

    #[test]
    fn register_uses_real_xid_and_service_and_path() {
        let state = state();
        let mut reg = stub_registrar(&state, Some(4242));
        reg.register(&empty_menu()).unwrap();
        assert!(reg.is_registered());
        let entries = state.lock().unwrap().registered_xid.clone();
        assert_eq!(entries, vec![(4242, "org.kde.searchis/MenuBar".to_string())]);
    }

    #[test]
    fn register_maps_unavailable_to_dbus_error_without_marking() {
        let state = state();
        let mut reg = stub_registrar(&state, Some(1));
        state.lock().unwrap().available = false;
        let err = reg.register(&empty_menu()).unwrap_err();
        assert_eq!(err.code, "DBUS_APPMENU_REGISTRATION_FAILED");
        assert!(!reg.is_registered());
    }

    #[test]
    fn register_maps_backend_failure_to_dbus_error() {
        let state = state();
        let mut reg = stub_registrar(&state, Some(1));
        state.lock().unwrap().register_result = Err("bus off".into());
        let err = reg.register(&empty_menu()).unwrap_err();
        assert_eq!(err.code, "DBUS_APPMENU_REGISTRATION_FAILED");
        assert!(err.message.contains("bus off"));
        assert!(!reg.is_registered());
    }

    #[test]
    fn register_degrades_when_xid_missing() {
        let state = state();
        let mut reg = stub_registrar(&state, None);
        let err = reg.register(&empty_menu()).unwrap_err();
        assert_eq!(err.code, "DBUS_APPMENU_REGISTRATION_FAILED");
        assert!(!reg.is_registered());
        assert_eq!(state.lock().unwrap().registered_xid.len(), 0);
    }

    #[test]
    fn sync_clears_and_reregisters_after_registrar_reappears() {
        let state = state();
        let mut reg = stub_registrar(&state, Some(100));
        // 初始注册成功。
        reg.register(&empty_menu()).unwrap();
        assert!(reg.is_registered());
        let after_up = state.lock().unwrap().registered_xid.len();

        // Registrar 消失：sync 清除注册态，且不再注册。
        state.lock().unwrap().available = false;
        let _ = reg.sync(&empty_menu());
        assert!(!reg.is_registered());
        assert_eq!(state.lock().unwrap().registered_xid.len(), after_up);

        // 重新出现：sync 重新注册（第二次 RegisterWindow）。
        state.lock().unwrap().available = true;
        reg.sync(&empty_menu()).unwrap();
        assert!(reg.is_registered());
        assert_eq!(state.lock().unwrap().registered_xid.len(), after_up + 1);
    }

    #[test]
    fn dbusmenu_layout_has_five_groups() {
        let service = DbusMenuService::new(five_group_menu(), Arc::new(dispatch_app_command));
        // 顶层 children = 5 组。
        assert_eq!(service.children_of(0).len(), 5);
        // 每组有子项（Searchis 主菜单 4 项）。
        let first = service.children_of(0)[0];
        assert!(!service.children_of(first).is_empty());
        // 组属性含 label。
        let props = service.props_of(first);
        assert!(props.contains_key("label"));
        // 布局可序列化（不 panic），revision 恒为 1。
        let (revision, _layout) = service.get_layout(0, 1, vec![]);
        assert_eq!(revision, 1);
    }

    #[test]
    fn dbusmenu_event_clicked_dispatches_command() {
        let recorded = Arc::new(Mutex::new(Vec::<String>::new()));
        let dispatch: MenuDispatch = {
            let recorded = Arc::clone(&recorded);
            Arc::new(move |command_id| {
                recorded.lock().unwrap().push(command_id.to_string());
                Ok("OK".to_string())
            })
        };
        let service = DbusMenuService::new(five_group_menu(), dispatch);
        // 找到第一个可点击项（编辑组的 command 映射）。
        let top = service.children_of(0);
        let group = service.children_of(top[0]);
        assert!(!group.is_empty());
        service.event(group[0], "clicked".to_string(), null_owned(), 0);
        let log = recorded.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], "open_search_window");
    }

    #[test]
    fn dispatch_unknown_command_maps_to_invalid_command() {
        let err = dispatch_app_command("no-such-command").unwrap_err();
        assert_eq!(err.code, "INVALID_COMMAND");
        assert_eq!(dispatch_app_command("open_search_window").unwrap(), "OPEN_SEARCH_WINDOW");
        assert_eq!(dispatch_app_command("app_quit").unwrap(), "APP_QUIT");
    }
}
