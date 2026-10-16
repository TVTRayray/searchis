//! 文件系统 IO：目录初始化、快照、带哈希复制、跨文件系统移动。

use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::{
    names::is_ignored_name,
    paths::{join_rel, safe_join, verify_under},
    scan::{FsFile, FsSnapshot},
    MAX_DEPTH, TRASH_DIR,
};
use crate::{error::AppError, model::ScanSkip};

pub fn fs_error(message: &str) -> AppError {
    AppError::new("TEMPLATE_FS_FAILED", message)
}

/// 递归创建目录，新目录权限 0700。
pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| fs_error("无法创建模板目录。请检查应用数据目录权限。"))
}

/// 在 `root` 内创建目录：创建前后各校验一次，拒绝符号链接与越界（缩小 TOCTOU 窗口）。
pub fn ensure_dir_in(root: &Path, path: &Path) -> Result<(), AppError> {
    verify_under(root, path)?;
    ensure_dir(path)?;
    verify_under(root, path)
}

/// 创建 `templates/`、`.trash/` 以及全部分组目录。
pub fn init_dirs(root: &Path, group_dirs: &[String]) -> Result<(), AppError> {
    ensure_dir(root)?;
    let _ = fs::set_permissions(root, fs::Permissions::from_mode(0o700));
    ensure_dir(&root.join(TRASH_DIR))?;
    for dir in group_dirs {
        ensure_dir(&safe_join(root, dir)?)?;
    }
    Ok(())
}

pub fn mtime_string(meta: &fs::Metadata) -> String {
    let time = meta.modified().ok().map(DateTime::<Utc>::from);
    time.unwrap_or_else(Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub struct FileStat {
    pub inode: i64,
    pub size: i64,
    pub mtime: String,
}

/// 只接受普通文件（不跟随符号链接）。
pub fn stat_regular(path: &Path) -> Result<FileStat, AppError> {
    let meta = fs::symlink_metadata(path).map_err(|_| fs_error("模板文件不可访问。"))?;
    if !meta.file_type().is_file() {
        return Err(fs_error("模板路径不是普通文件。"));
    }
    Ok(FileStat {
        inode: meta.ino() as i64,
        size: meta.len() as i64,
        mtime: mtime_string(&meta),
    })
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 复制 `src` 到 `dst`（调用方给出的临时文件），返回 (sha256, 字节数)，并 fsync。
pub fn copy_with_hash(src: &Path, dst: &Path) -> std::io::Result<(String, u64)> {
    // 源端 O_NOFOLLOW：源是符号链接时报错，不跟随读取链接目标。
    let mut input = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(src)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode_600()
        .custom_flags(libc::O_NOFOLLOW)
        .open(dst)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
        total += read as u64;
    }
    output.sync_all()?;
    Ok((hex::encode(hasher.finalize()), total))
}

trait ModeExt {
    fn mode_600(&mut self) -> &mut Self;
}
impl ModeExt for fs::OpenOptions {
    fn mode_600(&mut self) -> &mut Self {
        std::os::unix::fs::OpenOptionsExt::mode(self, 0o600)
    }
}

/// 写入新文件：临时文件 → fsync → rename。目标已存在时拒绝覆盖。
pub fn write_new_file(dst: &Path, bytes: &[u8]) -> Result<(), AppError> {
    if dst.symlink_metadata().is_ok() {
        return Err(fs_error("目标文件已存在。"));
    }
    let tmp = dst.with_file_name(format!(".new-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode_600()
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, dst)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|_| fs_error("无法写入模板文件。"))
}

/// 移动文件：优先 rename；跨文件系统时复制、校验哈希、再删除源。拒绝覆盖已存在的目标。
pub fn move_file(src: &Path, dst: &Path) -> Result<(), AppError> {
    if dst.symlink_metadata().is_ok() {
        return Err(fs_error("目标位置已存在同名文件。"));
    }
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    // 移动前先识别条目类型：符号链接只允许同设备 rename（移动链接本身）。
    let src_is_symlink = fs::symlink_metadata(src)
        .map_err(|_| fs_error("无法移动模板文件。"))?
        .file_type()
        .is_symlink();
    match rename_for_move(src, dst) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(libc::EXDEV) => {
            if src_is_symlink {
                return Err(fs_error("无法跨设备移动符号链接。"));
            }
            copy_verify_remove(src, dst)
        }
        Err(_) => Err(fs_error("无法移动模板文件。")),
    }
}

#[cfg(test)]
thread_local! {
    /// 测试钩子：让 `move_file` 的 rename 返回 EXDEV。
    pub static FORCE_EXDEV: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn rename_for_move(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if FORCE_EXDEV.with(|flag| flag.get()) {
        return Err(std::io::Error::from_raw_os_error(libc::EXDEV));
    }
    fs::rename(src, dst)
}

fn copy_verify_remove(src: &Path, dst: &Path) -> Result<(), AppError> {
    let tmp = dst.with_file_name(format!(".move-{}.tmp", uuid::Uuid::new_v4()));
    let outcome = (|| -> std::io::Result<()> {
        let (sha, _) = copy_with_hash(src, &tmp)?;
        if sha256_file(&tmp)? != sha {
            return Err(std::io::Error::other("hash mismatch"));
        }
        fs::rename(&tmp, dst)?;
        Ok(())
    })();
    match outcome {
        Ok(()) => fs::remove_file(src).map_err(|_| fs_error("无法移动模板文件。")),
        Err(_) => {
            let _ = fs::remove_file(&tmp);
            Err(fs_error("无法移动模板文件。"))
        }
    }
}

/// 收集目录快照：不跟随符号链接；隐藏/临时文件与 `.trash` 被忽略；超过 3 层的目录不进入。
pub fn snapshot(root: &Path) -> Result<FsSnapshot, AppError> {
    let mut snapshot = FsSnapshot::default();
    walk(root, "", 0, &mut snapshot)?;
    Ok(snapshot)
}

fn walk(root: &Path, rel: &str, depth: usize, out: &mut FsSnapshot) -> Result<(), AppError> {
    let dir = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    let entries = fs::read_dir(&dir).map_err(|_| fs_error("无法读取模板目录。"))?;
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            out.skipped.push(ScanSkip {
                reason: "invalid_name".into(),
            });
            continue;
        };
        if is_ignored_name(name) {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        let child_rel = join_rel(rel, name);
        let kind = meta.file_type();
        if kind.is_symlink() {
            out.skipped.push(ScanSkip {
                reason: "symlink".into(),
            });
        } else if kind.is_dir() {
            if depth + 1 > MAX_DEPTH {
                out.skipped.push(ScanSkip {
                    reason: "depth_exceeded".into(),
                });
                continue;
            }
            out.dirs.push(child_rel.clone());
            walk(root, &child_rel, depth + 1, out)?;
        } else if kind.is_file() {
            out.files.push(FsFile {
                rel_path: child_rel,
                inode: Some(meta.ino() as i64),
                size: meta.len(),
                mtime: mtime_string(&meta),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_applies_ignore_rules_symlink_and_depth_limits() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path();
        fs::create_dir_all(base.join("a/b/c/d")).unwrap();
        fs::create_dir_all(base.join(".trash")).unwrap();
        fs::write(base.join(".trash/x.md"), "x").unwrap();
        fs::write(base.join("a/keep.md"), "k").unwrap();
        fs::write(base.join("a/b/c/deep.md"), "d").unwrap();
        fs::write(base.join("a/b/c/d/too-deep.md"), "t").unwrap();
        fs::write(base.join("~$lock.docx"), "l").unwrap();
        fs::write(base.join("scratch.tmp"), "l").unwrap();
        fs::write(base.join(".hidden"), "h").unwrap();
        std::os::unix::fs::symlink("/etc/passwd", base.join("passwd.md")).unwrap();
        std::os::unix::fs::symlink("/etc", base.join("etc-dir")).unwrap();

        let snap = snapshot(base).unwrap();
        let mut files: Vec<&str> = snap.files.iter().map(|f| f.rel_path.as_str()).collect();
        files.sort();
        assert_eq!(files, vec!["a/b/c/deep.md", "a/keep.md"]);
        let mut dirs = snap.dirs.clone();
        dirs.sort();
        assert_eq!(dirs, vec!["a", "a/b", "a/b/c"]);
        let reasons: Vec<&str> = snap.skipped.iter().map(|s| s.reason.as_str()).collect();
        assert_eq!(reasons.iter().filter(|r| **r == "symlink").count(), 2);
        assert_eq!(
            reasons.iter().filter(|r| **r == "depth_exceeded").count(),
            1
        );
    }

    #[test]
    fn write_new_file_refuses_to_overwrite_and_leaves_no_temp() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("a.md");
        write_new_file(&target, b"one").unwrap();
        assert!(write_new_file(&target, b"two").is_err());
        assert_eq!(fs::read(&target).unwrap(), b"one");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn init_dirs_uses_private_permissions() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("templates");
        init_dirs(&root, &["运维/数据库".into()]).unwrap();
        for dir in [root.clone(), root.join(".trash"), root.join("运维")] {
            assert_eq!(
                fs::metadata(dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn copy_with_hash_matches_sha256_file() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("s.bin");
        fs::write(&src, vec![7u8; 100_000]).unwrap();
        let dst = root.path().join("d.bin");
        let (sha, size) = copy_with_hash(&src, &dst).unwrap();
        assert_eq!(size, 100_000);
        assert_eq!(sha, sha256_file(&dst).unwrap());
    }

    #[test]
    fn copy_with_hash_rejects_symlink_source() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "EXTERNAL_SENTINEL").unwrap();
        let link = root.path().join("link.md");
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        let dst = root.path().join("out.tmp");
        assert!(copy_with_hash(&link, &dst).is_err());
        if dst.symlink_metadata().is_ok() {
            assert!(fs::read(&dst).unwrap().is_empty());
        }
        assert_eq!(fs::read_to_string(&secret).unwrap(), "EXTERNAL_SENTINEL");
    }

    #[test]
    fn copy_verify_remove_rejects_symlink_source_and_keeps_link() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "EXTERNAL_SENTINEL").unwrap();
        let link = root.path().join("link.md");
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        let dst = root.path().join("moved.md");
        assert!(copy_verify_remove(&link, &dst).is_err());
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert!(dst.symlink_metadata().is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
