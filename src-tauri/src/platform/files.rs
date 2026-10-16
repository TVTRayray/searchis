//! SPEC-22 平台适配：xdg-open、MIME 判定、`.desktop` 枚举与启动。
//!
//! 所有外部程序都以参数数组启动，不经 shell。

use std::{
    collections::HashSet,
    fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub exec: String,
    pub mime_types: Vec<String>,
}

fn open_failed(message: &str) -> AppError {
    AppError::new("TEMPLATE_OPEN_FAILED", message)
}

fn missing_xdg_utils() -> AppError {
    open_failed("无法启动 xdg-open，请安装 xdg-utils。")
}

// ---------------------------------------------------------------- MIME

pub fn mime_by_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "md" | "markdown" => "text/markdown",
        "txt" | "log" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "doc" => "application/msword",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "ppt" => "application/vnd.ms-powerpoint",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odt" => "application/vnd.oasis.opendocument.text",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// `xdg-mime query filetype`（最多等 2 秒），失败或超时时按扩展名回退。
pub fn mime_of(path: &Path) -> String {
    if let Some(mime) = query_mime(path) {
        return mime;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    mime_by_extension(ext).to_owned()
}

fn query_mime(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut child = Command::new("xdg-mime")
        .arg("query")
        .arg("filetype")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(15)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut output = String::new();
    child.stdout.take()?.read_to_string(&mut output).ok()?;
    let mime = output.trim().to_owned();
    mime.contains('/').then_some(mime)
}

// ---------------------------------------------------------------- .desktop

/// 解析 `.desktop` 内容；隐藏、非应用、终端应用返回 None。
pub fn parse_desktop(content: &str, id: &str) -> Option<DesktopEntry> {
    let mut in_entry = false;
    let (mut name, mut exec, mut icon) = (None, None, None);
    let mut mime_types = Vec::new();
    let (mut no_display, mut hidden, mut terminal) = (false, false, false);
    let mut kind = String::from("Application");
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Name" => name = Some(value.to_owned()),
            "Exec" => exec = Some(value.to_owned()),
            "Icon" => icon = Some(value.to_owned()),
            "Type" => kind = value.to_owned(),
            "MimeType" => {
                mime_types = value
                    .split(';')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }
    if no_display || hidden || terminal || kind != "Application" {
        return None;
    }
    Some(DesktopEntry {
        id: id.to_owned(),
        name: name?,
        icon,
        exec: exec?,
        mime_types,
    })
}

fn application_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    let mut dirs: Vec<PathBuf> = home.into_iter().collect();
    dirs.extend(
        system
            .split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
    );
    dirs.into_iter().map(|d| d.join("applications")).collect()
}

fn collect_desktop_files(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if path.is_dir() {
            if depth < 2 {
                collect_desktop_files(&path, &format!("{prefix}{name}-"), depth + 1, out);
            }
        } else if name.ends_with(".desktop") {
            out.push((format!("{prefix}{name}"), path));
        }
    }
}

/// 支持该 MIME 的应用（按 XDG 目录优先级去重，按名称排序）。
pub fn desktop_entries_for(mime: &str) -> Vec<DesktopEntry> {
    desktop_entries_in(&application_dirs(), mime)
}

pub fn desktop_entries_in(dirs: &[PathBuf], mime: &str) -> Vec<DesktopEntry> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for dir in dirs {
        let mut files = Vec::new();
        collect_desktop_files(dir, "", 0, &mut files);
        for (id, path) in files {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Some(entry) = parse_desktop(&content, &id) else {
                continue;
            };
            let accepts = entry
                .mime_types
                .iter()
                .any(|m| m == mime || (mime.starts_with("text/") && m == "text/plain"));
            if accepts {
                result.push(entry);
            }
        }
    }
    result.sort_by_key(|entry| entry.name.to_lowercase());
    result
}

pub fn find_desktop_entry(desktop_id: &str, mime: &str) -> Option<DesktopEntry> {
    desktop_entries_for(mime)
        .into_iter()
        .find(|entry| entry.id == desktop_id)
}

fn file_url(path: &str) -> String {
    let mut encoded = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// 按 Desktop Entry 规范切分 `Exec`（双引号内反斜杠转义）。
fn split_exec(exec: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut in_quote = false;
    let mut chars = exec.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                has_token = true;
            }
            '\\' if in_quote => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
                has_token = true;
            }
            ch if ch.is_whitespace() && !in_quote => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            ch => {
                current.push(ch);
                has_token = true;
            }
        }
    }
    if has_token {
        args.push(current);
    }
    args
}

/// 展开 `Exec`：`%f %F` → 路径，`%u %U` → file URL，`%%` → `%`，其余字段码删除。
/// 没有任何文件字段码时把路径追加为最后一个参数。
pub fn expand_exec(exec: &str, path: &str) -> Vec<String> {
    let url = file_url(path);
    let mut used_file_code = false;
    let mut result = Vec::new();
    for arg in split_exec(exec) {
        let mut out = String::new();
        let mut chars = arg.chars();
        let mut had_code = false;
        while let Some(ch) = chars.next() {
            if ch != '%' {
                out.push(ch);
                continue;
            }
            match chars.next() {
                Some('%') => out.push('%'),
                Some('f') | Some('F') => {
                    out.push_str(path);
                    used_file_code = true;
                    had_code = true;
                }
                Some('u') | Some('U') => {
                    out.push_str(&url);
                    used_file_code = true;
                    had_code = true;
                }
                Some(_) | None => had_code = true,
            }
        }
        if out.is_empty() && had_code {
            continue;
        }
        result.push(out);
    }
    if !used_file_code {
        result.push(path.to_owned());
    }
    result
}

fn detach(mut command: Command) -> std::io::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command.spawn()?;
    // 回收子进程，避免僵尸。
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

// ---------------------------------------------------------------- 启动（E2E 替身）

/// E2E 构建里不启动外部程序，只把调用参数追加到 `SEARCHIS_E2E_OPEN_LOG`。
#[cfg(feature = "e2e-webdriver")]
fn record_open_call(kind: &str, path: &Path, desktop_id: Option<&str>) -> bool {
    use std::io::Write;
    let Some(log) = std::env::var_os("SEARCHIS_E2E_OPEN_LOG") else {
        return false;
    };
    let line = serde_json::json!({
        "kind": kind,
        "path": path.to_string_lossy(),
        "desktopId": desktop_id,
    });
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
    true
}

#[cfg(not(feature = "e2e-webdriver"))]
fn record_open_call(_kind: &str, _path: &Path, _desktop_id: Option<&str>) -> bool {
    false
}

/// `xdg-open <path>`。启动失败或在 1.5 秒内以非零状态退出时报告 `TEMPLATE_OPEN_FAILED`。
pub fn xdg_open(path: &Path) -> Result<(), AppError> {
    if record_open_call("xdg-open", path, None) {
        return Ok(());
    }
    let mut child = Command::new("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|_| missing_xdg_utils())?;
    let deadline = Instant::now() + Duration::from_millis(1500);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(open_failed("xdg-open 无法打开该文件。请检查默认应用设置。"))
                };
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => break,
        }
    }
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// 用指定的 `.desktop` 应用打开文件。
pub fn launch_desktop(entry: &DesktopEntry, path: &Path) -> Result<(), AppError> {
    if record_open_call("desktop", path, Some(&entry.id)) {
        return Ok(());
    }
    let args = expand_exec(&entry.exec, &path.to_string_lossy());
    let Some((program, rest)) = args.split_first() else {
        return Err(open_failed("所选应用的启动命令无效。"));
    };
    let mut command = Command::new(program);
    command.args(rest);
    detach(command).map_err(|_| open_failed("无法启动所选应用。"))
}

/// 原生文件选择对话框（kdialog，缺失时回退 zenity）。取消返回空列表。
/// E2E 构建读取 `SEARCHIS_E2E_PICK_LIST` 指向的文本文件（每行一个路径），不弹出对话框。
pub fn pick_files() -> Result<Vec<String>, AppError> {
    #[cfg(feature = "e2e-webdriver")]
    if let Some(list) = std::env::var_os("SEARCHIS_E2E_PICK_LIST") {
        let content = fs::read_to_string(list).unwrap_or_default();
        return Ok(content
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect());
    }
    let attempts: [(&str, Vec<&str>); 2] = [
        (
            "kdialog",
            vec!["--getopenfilename", "--multiple", "--separate-output"],
        ),
        (
            "zenity",
            vec!["--file-selection", "--multiple", "--separator=\n"],
        ),
    ];
    for (program, args) in attempts {
        let output = Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        match output {
            Ok(output) if output.status.success() => {
                return Ok(String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect());
            }
            Ok(_) => return Ok(Vec::new()),
            Err(_) => continue,
        }
    }
    Err(AppError::new(
        "TEMPLATE_FS_FAILED",
        "找不到文件选择对话框（kdialog/zenity）。请改用拖入文件导入。",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_application_entries_and_hides_hidden_or_terminal_ones() {
        let content = "[Desktop Entry]\nType=Application\nName=WPS 表格\nExec=et %f\nIcon=wps-office-et\nMimeType=application/vnd.ms-excel;application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;\n";
        let entry = parse_desktop(content, "wps-office-et.desktop").unwrap();
        assert_eq!(entry.name, "WPS 表格");
        assert_eq!(entry.mime_types.len(), 2);
        assert!(parse_desktop(&format!("{content}NoDisplay=true\n"), "x").is_none());
        assert!(parse_desktop(&format!("{content}Hidden=true\n"), "x").is_none());
        assert!(parse_desktop(&format!("{content}Terminal=true\n"), "x").is_none());
        assert!(parse_desktop("[Desktop Entry]\nType=Link\nName=a\nExec=b\n", "x").is_none());
        // 其他分组里的 Exec 不能串进来
        assert!(parse_desktop("[Desktop Action a]\nName=a\nExec=b\n", "x").is_none());
    }

    #[test]
    fn exec_field_codes_are_expanded_per_spec() {
        assert_eq!(
            expand_exec("libreoffice --calc %U", "/t/a b.xlsx"),
            vec!["libreoffice", "--calc", "file:///t/a%20b.xlsx"]
        );
        assert_eq!(
            expand_exec("typora %f", "/t/a.md"),
            vec!["typora", "/t/a.md"]
        );
        assert_eq!(
            expand_exec("app --name=%c --icon %i %F", "/t/a.md"),
            vec!["app", "--name=", "--icon", "/t/a.md"]
        );
        assert_eq!(
            expand_exec("app 100%% --file=%f", "/t/a.md"),
            vec!["app", "100%", "--file=/t/a.md"]
        );
        // 没有文件字段码时追加路径
        assert_eq!(expand_exec("editor", "/t/a.md"), vec!["editor", "/t/a.md"]);
        // 引号内空格与转义
        assert_eq!(
            expand_exec(r#""/opt/My App/run" --x "a \"b\"" %f"#, "/t/a.md"),
            vec!["/opt/My App/run", "--x", r#"a "b""#, "/t/a.md"]
        );
    }

    #[test]
    fn desktop_lookup_filters_by_mime_dedupes_by_id_and_prefers_earlier_dirs() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let write = |dir: &Path, name: &str, body: &str| {
            fs::write(dir.join(name), body).unwrap();
        };
        write(
            first.path(),
            "calc.desktop",
            "[Desktop Entry]\nType=Application\nName=User Calc\nExec=a %f\nMimeType=text/csv;\n",
        );
        write(
            second.path(),
            "calc.desktop",
            "[Desktop Entry]\nType=Application\nName=System Calc\nExec=b %f\nMimeType=text/csv;\n",
        );
        write(
            second.path(),
            "edit.desktop",
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=e %f\nMimeType=text/plain;\n",
        );
        let dirs = vec![first.path().to_path_buf(), second.path().to_path_buf()];
        let csv = desktop_entries_in(&dirs, "text/csv");
        let names: Vec<&str> = csv.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"User Calc") && !names.contains(&"System Calc"));
        // text/* 同时接受 text/plain 编辑器
        let md = desktop_entries_in(&dirs, "text/markdown");
        assert_eq!(
            md.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            vec!["Editor"]
        );
        assert!(desktop_entries_in(&dirs, "image/png").is_empty());
    }

    #[test]
    fn extension_fallback_covers_office_formats() {
        assert_eq!(mime_by_extension("MD"), "text/markdown");
        assert!(mime_by_extension("xlsx").contains("spreadsheetml"));
        assert_eq!(mime_by_extension("zzz"), "application/octet-stream");
    }
}
