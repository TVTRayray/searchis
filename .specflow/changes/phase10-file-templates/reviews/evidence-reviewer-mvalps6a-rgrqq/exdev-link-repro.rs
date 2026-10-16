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

/// 复制 `src` 到 `dst`（调用方给出的临时文件），返回 (sha256, 字节数)，并 fsync。
pub fn copy_with_hash(src: &Path, dst: &Path) -> std::io::Result<(String, u64)> {
    let mut input = File::open(src)?;
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
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(libc::EXDEV) => copy_verify_remove(src, dst),
        Err(_) => Err(fs_error("无法移动模板文件。")),
    }
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
 let a=std::env::temp_dir().join(format!("searchis-review-{}",std::process::id()));
 let b=std::path::PathBuf::from(format!("/dev/shm/searchis-review-{}",std::process::id()));
 fs::create_dir(&a).unwrap(); fs::create_dir(&b).unwrap();
 let external=a.join("outside.txt"); fs::write(&external,b"EXTERNAL_SENTINEL").unwrap();
 let src=a.join("link.txt"); let dst=b.join("link.txt");
 std::os::unix::fs::symlink(&external,&src).unwrap();
 assert_eq!(fs::rename(&src,&dst).unwrap_err().raw_os_error(),Some(libc::EXDEV));
 move_file(&src,&dst).unwrap();
 println!("EXDEV confirmed; destination_is_symlink={}; destination_content={}; source_link_exists={}; external_target_unchanged={}", dst.symlink_metadata().unwrap().file_type().is_symlink(),fs::read_to_string(&dst).unwrap(),src.symlink_metadata().is_ok(),fs::read(&external).unwrap()==b"EXTERNAL_SENTINEL");
 fs::remove_dir_all(&a).unwrap(); fs::remove_dir_all(&b).unwrap();
}
