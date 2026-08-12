/// KGlobalAccel D-Bus 集成：通过 gdbus CLI 与 KDE 全局快捷键管理器交互。
use crate::error::AppError;

const KGA_SERVICE: &str = "org.kde.globalaccel";
const KGA_OBJECT: &str = "/kglobalaccel";
const KGA_INTERFACE: &str = "org.kde.globalaccel";

/// 全局快捷键管理器（gdbus CLI 封装）。
pub struct KGlobalAccelManager {
    available: bool,
    component: String,
    action: String,
}

impl KGlobalAccelManager {
    pub fn new(component: &str, action: &str) -> Self {
        let available = Self::probe();
        Self {
            available,
            component: component.to_string(),
            action: action.to_string(),
        }
    }

    pub fn available(&self) -> bool {
        self.available
    }

    /// 获取当前已注册的快捷键。
    pub fn current_shortcut(&self) -> Result<String, AppError> {
        if !self.available {
            return Ok(String::new());
        }
        let output = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                KGA_SERVICE,
                "--object-path",
                KGA_OBJECT,
                "--method",
                &format!("{KGA_INTERFACE}.shortcut"),
                &self.component,
                &self.action,
            ])
            .output()
            .map_err(|e| dbus_error(&format!("gdbus 调用失败: {e}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(parse_shortcut_from_gdbus(&stdout))
    }

    /// 注册指定快捷键，返回之前的快捷键。
    pub fn register(&self, accelerator: &str) -> Result<String, AppError> {
        let previous = self.current_shortcut()?;
        let shortcut_tuple = format!("[('{accelerator}', '')]");
        let output = std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                KGA_SERVICE,
                "--object-path",
                KGA_OBJECT,
                "--method",
                &format!("{KGA_INTERFACE}.setShortcut"),
                &self.component,
                &self.action,
                &shortcut_tuple,
                "false",
            ])
            .output()
            .map_err(|e| dbus_error(&format!("gdbus 调用失败: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AppError::new(
                "SHORTCUT_CONFLICT",
                format!("快捷键 \"{accelerator}\" 注册失败: {stderr}"),
            ));
        }
        Ok(previous)
    }

    fn probe() -> bool {
        std::process::Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                KGA_SERVICE,
                "--object-path",
                KGA_OBJECT,
                "--method",
                "org.freedesktop.DBus.Peer.Ping",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// 信号监听（MVP：不使用 D-Bus 信号订阅，由 Tauri 内部快捷键事件驱动）。
pub fn start_signal_listener<F>(_component: String, _action: String, _callback: F)
where
    F: Fn() + Send + Sync + 'static,
{
    // MVP: 信号接收通过 Tauri global-shortcut 事件循环处理，无需 D-Bus 信号线程。
    // 后续可用 zbus async 或 dbus-rs 替代 gdbus CLI 以获得原生信号支持。
}

fn parse_shortcut_from_gdbus(output: &str) -> String {
    // gdbus 返回格式如 "(('Alt+O', ''),)"
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "()" {
        return String::new();
    }
    // 简单解析：提取第一个括号内的字符串
    if let Some(start) = trimmed.find('\'') {
        if let Some(end) = trimmed[start + 1..].find('\'') {
            return trimmed[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}

fn dbus_error(message: &str) -> AppError {
    AppError::new("KGLOBALACCEL_UNAVAILABLE", message)
}
