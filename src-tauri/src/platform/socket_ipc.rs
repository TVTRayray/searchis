use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.is_empty() {
            return PathBuf::from(runtime_dir).join("searchis.sock");
        }
    }
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/searchis-{uid}.sock"))
}

/// 向正在运行的 Searchis 进程发送指令 (如 toggle / show-manager / quit)。
/// 如果发送成功返回 true，否则返回 false。
pub fn send_command(cmd: &str) -> bool {
    let sock_path = get_socket_path();
    if let Ok(mut stream) = UnixStream::connect(&sock_path) {
        let payload = format!("{}\n", cmd);
        if stream.write_all(payload.as_bytes()).is_ok() {
            let _ = stream.flush();
            return true;
        }
    }
    false
}

/// 在后台线程监听 Unix Domain Socket，支持 sxhkd 和 CLI 秒级呼出/控制。
pub fn start_socket_server(app: tauri::AppHandle) {
    let sock_path = get_socket_path();
    let _ = std::fs::remove_file(&sock_path);

    std::thread::spawn(move || {
        let listener = match UnixListener::bind(&sock_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[Searchis Socket IPC] 无法绑定 Unix Socket: {e}");
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_ok() {
                        let cmd = line.trim();
                        match cmd {
                            "toggle" | "search" | "open-search" => {
                                let _ = crate::commands::open_search_window_inner(&app);
                            }
                            "show-manager" | "manager" => {
                                if let Some(w) = app.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                            "settings" => {
                                if let Some(w) = app.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                                let _ = app.emit("navigate", "settings");
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
}
