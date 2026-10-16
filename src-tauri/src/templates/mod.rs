//! SPEC-22 文件模板：命名规则、路径安全、目录扫描（纯函数核心 + IO 层）。
//!
//! 日志约束：本模块与调用方都不得记录文件名、相对路径或分组名。
pub mod fsops;
pub mod names;
pub mod paths;
pub mod scan;

/// 单文件上限 20 MB。
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
/// 未删除条目合计上限。
pub const MAX_ENTRIES: i64 = 10_000;
/// 扫描分批事务大小。
pub const SCAN_BATCH: usize = 500;
/// 分组最大深度（模板目录最多 3 层子目录）。
pub const MAX_DEPTH: usize = 3;
/// 回收站子目录名。
pub const TRASH_DIR: &str = ".trash";
