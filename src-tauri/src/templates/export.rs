//! SPEC-23 导出：把模板文件复制到导出目录。
//!
//! 文件名 `<标题>_<YYYY-MM-DD>.<ext>`，重名加 ` (2)` 后缀，绝不覆盖；
//! 先写同目录临时文件，再用“不覆盖”语义改名；任何失败都不留临时文件。
//! 日志不记录文件名与路径；路径只在成功后随结果返回给 UI。

use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use super::names::{file_name, sanitize_stem};
use crate::error::AppError;

fn export_failed(message: &str) -> AppError {
    AppError::new("EXPORT_FILE_FAILED", message)
}

// ---------------------------------------------------------------- 目录解析

/// `user-dirs.dirs` 中的 `XDG_DOWNLOAD_DIR`；`$HOME` 前缀会被展开。
fn xdg_download_dir(config_home: &Path, home: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(config_home.join("user-dirs.dirs")).ok()?;
    content.lines().find_map(|line| {
        let value = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=")?;
        let value = value.trim().trim_matches('"');
        let expanded = match value.strip_prefix("$HOME") {
            Some(rest) => format!("{}{rest}", home.display()),
            None => value.to_owned(),
        };
        let path = PathBuf::from(expanded);
        path.is_absolute().then_some(path)
    })
}

/// 默认导出目录：XDG 下载目录 → `~/Downloads` → `$HOME`（取第一个存在的目录）。
pub fn default_export_dir(home: Option<&Path>, config_home: Option<&Path>) -> Option<PathBuf> {
    let home = home?;
    let config_home = config_home
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".config"));
    [
        xdg_download_dir(&config_home, home),
        Some(home.join("Downloads")),
        Some(home.to_path_buf()),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_dir())
}

/// 解析设置里的导出目录；`None` 用默认目录。显式配置的目录不存在时不自动创建。
pub fn resolve_export_dir(configured: Option<&str>) -> Result<PathBuf, AppError> {
    if let Some(configured) = configured.filter(|value| !value.is_empty()) {
        let path = PathBuf::from(configured);
        if !path.is_absolute() || !path.is_dir() {
            return Err(export_failed("导出目录不存在。请在设置中重新选择导出目录。"));
        }
        return Ok(path);
    }
    #[cfg(feature = "e2e-webdriver")]
    if let Some(dir) = std::env::var_os("SEARCHIS_E2E_EXPORT_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute());
    default_export_dir(home.as_deref(), config_home.as_deref())
        .ok_or_else(|| export_failed("找不到可用的导出目录。请在设置中选择导出目录。"))
}

// ---------------------------------------------------------------- 复制

/// 删除临时文件的守卫：成功改名后 `disarm`。
struct TempFile {
    path: PathBuf,
    armed: bool,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// 不覆盖改名：已存在目标时返回 `AlreadyExists`。
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    let from_c = CString::new(from.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let to_c = CString::new(to.as_os_str().as_bytes())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    const RENAME_NOREPLACE: libc::c_uint = 1;
    // SAFETY: 两个指针都指向以 NUL 结尾、在调用期间存活的 CString。
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            from_c.as_ptr(),
            libc::AT_FDCWD,
            to_c.as_ptr(),
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    match error.raw_os_error() {
        // 文件系统不支持 RENAME_NOREPLACE：用硬链接（同样不覆盖），再不行就独占创建后复制。
        Some(libc::ENOSYS | libc::EINVAL | libc::EOPNOTSUPP) => link_no_replace(from, to),
        _ => Err(error),
    }
}

fn link_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    match fs::hard_link(from, to) {
        Ok(()) => fs::remove_file(from),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(error),
        Err(_) => {
            let mut source = File::open(from)?;
            let mut target = OpenOptions::new().write(true).create_new(true).open(to)?;
            if let Err(error) = io::copy(&mut source, &mut target).and_then(|_| target.sync_all()) {
                drop(target);
                let _ = fs::remove_file(to);
                return Err(error);
            }
            fs::remove_file(from)
        }
    }
}

/// 把 `source` 导出到 `dir`，返回最终路径。
pub fn export_copy(
    source: &Path,
    dir: &Path,
    title: &str,
    ext: &str,
    date: &str,
) -> Result<PathBuf, AppError> {
    if !dir.is_dir() {
        return Err(export_failed("导出目录不存在。请在设置中重新选择导出目录。"));
    }
    let stem = format!("{}_{date}", sanitize_stem(title));
    let temp_path = dir.join(format!(".searchis-export-{}.tmp", uuid::Uuid::new_v4().simple()));
    let mut temp = TempFile {
        path: temp_path.clone(),
        armed: true,
    };
    {
        let mut input = File::open(source).map_err(|_| export_failed("无法读取模板文件。"))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|_| export_failed("无法写入导出目录。请检查目录权限。"))?;
        io::copy(&mut input, &mut output)
            .and_then(|_| output.sync_all())
            .map_err(|_| export_failed("写入导出文件失败。请检查磁盘空间与目录权限。"))?;
    }
    for suffix in 1..=10_000u32 {
        let candidate = if suffix == 1 {
            stem.clone()
        } else {
            format!("{stem} ({suffix})")
        };
        let destination = dir.join(file_name(&candidate, ext));
        if destination.symlink_metadata().is_ok() {
            continue;
        }
        match rename_no_replace(&temp_path, &destination) {
            Ok(()) => {
                temp.armed = false;
                return Ok(destination);
            }
            // 检查之后同名文件被外部创建：换下一个后缀。
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(export_failed("无法保存导出文件。请检查目录权限。")),
        }
    }
    Err(export_failed("同名导出文件过多。请清理导出目录后重试。"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn source(dir: &Path, bytes: &[u8]) -> PathBuf {
        let path = dir.join("src.xlsx");
        fs::write(&path, bytes).unwrap();
        path
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn exports_with_title_date_and_extension() {
        let work = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let src = source(work.path(), b"data");
        let path = export_copy(&src, out.path(), "报销单", "xlsx", "2026-10-16").unwrap();
        assert_eq!(path, out.path().join("报销单_2026-10-16.xlsx"));
        assert_eq!(fs::read(&path).unwrap(), b"data");
        assert_eq!(names(out.path()), ["报销单_2026-10-16.xlsx"]);
    }

    #[test]
    fn illegal_title_characters_are_replaced() {
        let work = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let src = source(work.path(), b"x");
        let path = export_copy(&src, out.path(), "a/b:c*d", "md", "2026-10-16").unwrap();
        assert_eq!(path.file_name().unwrap(), "a-b-c-d_2026-10-16.md");
    }

    #[test]
    fn existing_names_get_numeric_suffix_and_are_never_overwritten() {
        let work = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        fs::write(out.path().join("t_2026-10-16.md"), "mine").unwrap();
        fs::write(out.path().join("t_2026-10-16 (2).md"), "mine2").unwrap();
        let src = source(work.path(), b"new");
        let path = export_copy(&src, out.path(), "t", "md", "2026-10-16").unwrap();
        assert_eq!(path.file_name().unwrap(), "t_2026-10-16 (3).md");
        assert_eq!(fs::read(out.path().join("t_2026-10-16.md")).unwrap(), b"mine");
        assert_eq!(fs::read(out.path().join("t_2026-10-16 (2).md")).unwrap(), b"mine2");
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(names(out.path()).len(), 3, "no temp residue");
    }

    #[test]
    fn dangling_symlink_with_same_name_counts_as_taken() {
        let work = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/nonexistent/x", out.path().join("t_2026-10-16.md")).unwrap();
        let src = source(work.path(), b"new");
        let path = export_copy(&src, out.path(), "t", "md", "2026-10-16").unwrap();
        assert_eq!(path.file_name().unwrap(), "t_2026-10-16 (2).md");
    }

    #[test]
    fn rename_no_replace_refuses_existing_target_and_keeps_both() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        let error = rename_no_replace(&a, &b).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&a).unwrap(), b"a");
        assert_eq!(fs::read(&b).unwrap(), b"b");
        let c = dir.path().join("c");
        rename_no_replace(&a, &c).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read(&c).unwrap(), b"a");
    }

    #[test]
    fn link_fallback_never_overwrites_and_moves_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        assert_eq!(link_no_replace(&a, &b).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        let c = dir.path().join("c");
        link_no_replace(&a, &c).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read(&c).unwrap(), b"a");
    }

    #[test]
    fn missing_export_dir_fails_without_creating_it() {
        let work = tempfile::tempdir().unwrap();
        let src = source(work.path(), b"x");
        let missing = work.path().join("nope");
        let error = export_copy(&src, &missing, "t", "md", "2026-10-16").unwrap_err();
        assert_eq!(error.code, "EXPORT_FILE_FAILED");
        assert!(!missing.exists());
    }

    #[test]
    fn missing_source_leaves_no_residue() {
        let out = tempfile::tempdir().unwrap();
        let error =
            export_copy(&out.path().join("gone.md"), out.path(), "t", "md", "2026-10-16").unwrap_err();
        assert_eq!(error.code, "EXPORT_FILE_FAILED");
        assert!(names(out.path()).is_empty());
    }

    #[test]
    fn unwritable_dir_fails_without_residue() {
        let work = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let src = source(work.path(), b"x");
        fs::set_permissions(out.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let result = export_copy(&src, out.path(), "t", "md", "2026-10-16");
        fs::set_permissions(out.path(), fs::Permissions::from_mode(0o700)).unwrap();
        if unsafe { libc::geteuid() } != 0 {
            assert_eq!(result.unwrap_err().code, "EXPORT_FILE_FAILED");
        }
        assert!(names(out.path()).is_empty());
    }

    #[test]
    fn default_dir_prefers_xdg_then_downloads_then_home() {
        let home = tempfile::tempdir().unwrap();
        let cfg = home.path().join(".config");
        fs::create_dir_all(&cfg).unwrap();
        assert_eq!(default_export_dir(Some(home.path()), None), Some(home.path().to_path_buf()));
        fs::create_dir(home.path().join("Downloads")).unwrap();
        assert_eq!(
            default_export_dir(Some(home.path()), None),
            Some(home.path().join("Downloads"))
        );
        fs::create_dir(home.path().join("下载")).unwrap();
        fs::write(
            cfg.join("user-dirs.dirs"),
            "# c\nXDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_DOWNLOAD_DIR=\"$HOME/下载\"\n",
        )
        .unwrap();
        assert_eq!(default_export_dir(Some(home.path()), None), Some(home.path().join("下载")));
        // 配置指向不存在的目录时回退
        fs::write(cfg.join("user-dirs.dirs"), "XDG_DOWNLOAD_DIR=\"$HOME/missing\"\n").unwrap();
        assert_eq!(
            default_export_dir(Some(home.path()), Some(&cfg)),
            Some(home.path().join("Downloads"))
        );
        assert_eq!(default_export_dir(None, None), None);
    }

    #[test]
    fn configured_dir_must_exist_and_is_not_created() {
        let base = tempfile::tempdir().unwrap();
        let missing = base.path().join("x");
        assert_eq!(
            resolve_export_dir(Some(missing.to_str().unwrap())).unwrap_err().code,
            "EXPORT_FILE_FAILED"
        );
        assert!(!missing.exists());
        assert_eq!(
            resolve_export_dir(Some(base.path().to_str().unwrap())).unwrap(),
            base.path()
        );
        assert_eq!(
            resolve_export_dir(Some("relative/dir")).unwrap_err().code,
            "EXPORT_FILE_FAILED"
        );
    }
}
