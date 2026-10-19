from pathlib import Path
root = Path.cwd()
src = (root / 'src-tauri/src/templates/fsops.rs').read_text()
def section(start, end):
    return src[src.index(start):src.index(end)]
code = '''use std::{fs::{self, File}, io::{Read,Write}, path::Path, os::unix::fs::{DirBuilderExt,OpenOptionsExt}};
use sha2::{Digest,Sha256};
type AppError = String;
fn fs_error(s: &str) -> AppError {s.into()}
'''
code += section('pub fn ensure_dir(', '/// 创建分组目录') if '/// 创建分组目录' in src else src[src.index('pub fn ensure_dir('):src.index('pub fn ensure_dir(')+src[src.index('pub fn ensure_dir('):].index('\n}')+2]
code += '\n' + section('pub fn sha256_file(', '/// 复制 `src`')
code += section('pub fn copy_with_hash(', '/// 写入新文件')
code += section('pub fn move_file(', '/// 收集目录快照')
code += r'''
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
'''
(root / '.specflow/changes/phase10-file-templates/reviews/evidence-reviewer-mvam3u2u-ho1u1/s2-import-repro.rs').write_text(code)
