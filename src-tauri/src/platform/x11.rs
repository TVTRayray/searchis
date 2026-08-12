/// X11 窗口操作：通过 xdotool CLI 执行。
/// 所有操作封装在 trait 中以便测试替身注入。
use crate::error::AppError;

/// 不透明的 X11 窗口句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle(pub u64);

pub trait WindowManager: Send + Sync {
    /// 获取当前 X11 输入焦点窗口。
    fn get_active_window(&self) -> Result<Option<WindowHandle>, AppError>;
    /// 激活（raise + focus）指定窗口。
    fn activate_window(&self, handle: WindowHandle) -> Result<(), AppError>;
    /// 向指定窗口发送一次固定的 Ctrl+V 键盘事件序列。
    fn inject_ctrl_v(&self, handle: WindowHandle) -> Result<(), AppError>;
    /// 检测键盘注入能力是否可用（xdotool 是否存在且 XTest 可用）。
    fn injection_available(&self) -> bool;
}

/// xdotool 实现：通过 std::process::Command 调用 xdotool。
pub struct XdotoolManager {
    xdotool_path: Option<String>,
}

impl XdotoolManager {
    pub fn new() -> Self {
        let xdotool_path = which_xdotool();
        Self { xdotool_path }
    }
}

impl WindowManager for XdotoolManager {
    fn get_active_window(&self) -> Result<Option<WindowHandle>, AppError> {
        let path = self
            .xdotool_path
            .as_ref()
            .ok_or_else(|| x11_error("xdotool 未安装"))?;
        let output = std::process::Command::new(path)
            .arg("getactivewindow")
            .output()
            .map_err(|e| x11_error(&format!("无法执行 xdotool getactivewindow: {e}")))?;
        if !output.status.success() {
            return Ok(None);
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let wid: u64 = stdout
            .parse()
            .map_err(|_| x11_error("xdotool 返回了无效的窗口 ID"))?;
        if wid == 0 {
            Ok(None)
        } else {
            Ok(Some(WindowHandle(wid)))
        }
    }

    fn activate_window(&self, handle: WindowHandle) -> Result<(), AppError> {
        let path = self
            .xdotool_path
            .as_ref()
            .ok_or_else(|| x11_error("xdotool 未安装"))?;
        let output = std::process::Command::new(path)
            .args(["windowactivate", "--sync", &handle.0.to_string()])
            .output()
            .map_err(|e| x11_error(&format!("无法执行 xdotool windowactivate: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(x11_error(&format!("xdotool windowactivate 失败: {stderr}")));
        }
        Ok(())
    }

    fn inject_ctrl_v(&self, handle: WindowHandle) -> Result<(), AppError> {
        if !self.injection_available() {
            return Err(x11_error("X11 键盘事件注入不可用（xdotool 未安装）"));
        }
        let path = self
            .xdotool_path
            .as_ref()
            .ok_or_else(|| x11_error("xdotool 未安装"))?;
        let output = std::process::Command::new(path)
            .args(["key", "--window", &handle.0.to_string(), "ctrl+v"])
            .output()
            .map_err(|e| x11_error(&format!("无法执行 xdotool key: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(x11_error(&format!("xdotool key ctrl+v 失败: {stderr}")));
        }
        Ok(())
    }

    fn injection_available(&self) -> bool {
        self.xdotool_path.is_some()
    }
}

fn which_xdotool() -> Option<String> {
    std::process::Command::new("which")
        .arg("xdotool")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if path.is_empty() {
                    None
                } else {
                    Some(path)
                }
            } else {
                None
            }
        })
}

fn x11_error(message: &str) -> AppError {
    AppError::new("X11_AUTOMATION_UNAVAILABLE", message)
}
