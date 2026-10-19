//! 其他文件类型：`text/uri-list`、`x-special/gnome-copied-files`、`text/plain`（绝对路径）。

use std::path::Path;

use super::{copy_failed, FormatSet};
use crate::error::AppError;

/// 对路径字节做 RFC 3986 百分号编码（保留 `/` 与非保留字符）。
pub fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn convert(path: &Path) -> Result<FormatSet, AppError> {
    if !path.is_absolute() {
        return Err(copy_failed("无法生成文件路径。请重新扫描后重试。"));
    }
    let uri = file_uri(path);
    let mut set = FormatSet::default();
    set.push("text/uri-list", format!("{uri}\r\n").into_bytes());
    set.push(
        "x-special/gnome-copied-files",
        format!("copy\n{uri}").into_bytes(),
    );
    let plain = path.to_string_lossy().into_owned().into_bytes();
    set.push("text/plain", plain.clone());
    set.push("UTF8_STRING", plain);
    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_encodes_non_ascii_and_spaces_and_keeps_slashes() {
        let uri = file_uri(Path::new("/home/u/模板/a b.pdf"));
        assert_eq!(
            uri,
            "file:///home/u/%E6%A8%A1%E6%9D%BF/a%20b.pdf"
        );
    }

    #[test]
    fn convert_provides_uri_list_gnome_copied_and_plain_path() {
        let set = convert(Path::new("/t/x y.pdf")).unwrap();
        let get = |name: &str| {
            String::from_utf8(set.formats.iter().find(|(n, _)| n == name).unwrap().1.clone())
                .unwrap()
        };
        assert_eq!(get("text/uri-list"), "file:///t/x%20y.pdf\r\n");
        assert_eq!(get("x-special/gnome-copied-files"), "copy\nfile:///t/x%20y.pdf");
        assert_eq!(get("text/plain"), "/t/x y.pdf");
        assert_eq!(get("UTF8_STRING"), "/t/x y.pdf");
    }

    #[test]
    fn relative_path_is_rejected() {
        assert_eq!(
            convert(Path::new("x.pdf")).unwrap_err().code,
            "FORMAT_COPY_FAILED"
        );
    }
}
