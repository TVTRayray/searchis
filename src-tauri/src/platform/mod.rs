mod appmenu;
pub mod capability;
pub mod kglobalaccel;
pub mod socket_ipc;
mod x11;

pub use appmenu::{
    build_menu_model, create_app_menu, dispatch_app_command, AppMenuRegistrar,
};
pub use capability::{detect_kglobalaccel_available, PlatformCapabilities};
pub use kglobalaccel::{start_signal_listener, KGlobalAccelManager};
pub use socket_ipc::{send_command, start_socket_server};
pub use x11::{WindowHandle, WindowManager, XdotoolManager};
