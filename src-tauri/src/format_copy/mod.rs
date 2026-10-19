//! SPEC-23：文件模板带格式复制的格式转换（纯函数，与 X11 和数据库解耦）。
//!
//! 输入是文件字节，输出是“剪贴板格式名 → 字节”的列表。日志不得记录文件名、路径与内容。

pub mod markdown;
pub mod uri;
pub mod xlsx;

use crate::error::AppError;

/// xlsx 活动工作表已用区域的单元格上限。
pub const MAX_XLSX_CELLS: u64 = 50_000;

/// 一次带格式复制要提供的全部剪贴板格式。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormatSet {
    pub formats: Vec<(String, Vec<u8>)>,
    /// 给用户的提示，如“公式已转为数值”。
    pub notes: Vec<String>,
}

impl FormatSet {
    pub fn push(&mut self, name: &str, bytes: Vec<u8>) {
        self.formats.push((name.to_owned(), bytes));
    }
}

/// 按扩展名选择转换器。`path` 只用于“其他文件”的 URI / 路径格式。
pub fn build(ext: &str, bytes: &[u8], path: &std::path::Path) -> Result<FormatSet, AppError> {
    match ext {
        "md" => Ok(markdown::convert(bytes)),
        "xlsx" => xlsx::convert(bytes),
        _ => uri::convert(path),
    }
}

pub fn copy_failed(message: &str) -> AppError {
    AppError::new("FORMAT_COPY_FAILED", message)
}

pub fn too_large() -> AppError {
    AppError::new(
        "FORMAT_COPY_TOO_LARGE",
        "表格过大，无法带格式复制。请使用“导出”（Shift+Enter）。",
    )
}
