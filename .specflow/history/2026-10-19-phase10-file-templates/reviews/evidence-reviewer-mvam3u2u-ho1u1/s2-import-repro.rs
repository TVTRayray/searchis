use std::{fs::{self, File}, io::{Read,Write}, path::Path, os::unix::fs::{DirBuilderExt,OpenOptionsExt}};
use sha2::{Digest,Sha256};
type AppError = String;
fn fs_error(s: &str) -> AppError {s.into()}
pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| fs_error("无法创建模板目录。请检查应用数据目录权限。"))
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


fn main() {
 let a=std::env::temp_dir().join(format!("searchis-review-s2-{}",std::process::id()));
 let b=std::path::PathBuf::from(format!("/dev/shm/searchis-review-s2-{}",std::process::id()));
 fs::create_dir(&a).unwrap(); fs::create_dir(&b).unwrap();
 let external=a.join("outside.md"); fs::write(&external,b"EXTERNAL_SENTINEL").unwrap();
 let src=a.join("link.md"); let dst=b.join("link.md");
 std::os::unix::fs::symlink(&external,&src).unwrap();
 assert_eq!(fs::rename(&src,&dst).unwrap_err().raw_os_error(),Some(libc::EXDEV));
 assert!(move_file(&src,&dst).is_err());
 assert!(src.symlink_metadata().unwrap().file_type().is_symlink());
 assert!(!dst.exists());
 assert_eq!(fs::read_dir(&b).unwrap().count(),0);
 assert_eq!(fs::read(&external).unwrap(),b"EXTERNAL_SENTINEL");
 println!("S2 real EXDEV: rejected; link unchanged; destination absent; target unchanged");
 let copy=a.join("import.tmp");
 let err=copy_with_hash(&src,&copy).unwrap_err();
 assert_eq!(err.raw_os_error(),Some(libc::ELOOP));
 assert!(!copy.exists());
 assert_eq!(fs::read_link(&src).unwrap(),external);
 println!("Import helper: user-selected external source link rejected with ELOOP; link unchanged");
 copy_with_hash(&external,&copy).unwrap();
 assert_eq!(fs::read(&copy).unwrap(),b"EXTERNAL_SENTINEL");
 println!("Control: regular source succeeds; copied contents match");
 fs::remove_dir_all(&a).unwrap(); fs::remove_dir_all(&b).unwrap();
}
