mod appmenu;
pub mod capability;
pub mod kglobalaccel;
mod x11;

pub use appmenu::{build_menu_model, dispatch_app_command, AppMenuRegistrar};
pub use capability::{detect_kglobalaccel_available, PlatformCapabilities};
pub use kglobalaccel::{start_signal_listener, KGlobalAccelManager};
pub use x11::{WindowHandle, WindowManager, XdotoolManager};
