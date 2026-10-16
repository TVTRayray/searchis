//! 文件名清洗、重名后缀、Key 自动生成。

const ILLEGAL: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
/// 文件名主干的字节上限（NAME_MAX 为 255，预留扩展名与 ` (n)` 后缀）。
const MAX_STEM_BYTES: usize = 180;

/// 清洗标题为文件名主干：非法字符换成 `-`，去掉首尾空白与点，空结果为 `file`。
pub fn sanitize_stem(input: &str) -> String {
    let replaced: String = input
        .chars()
        .map(|ch| {
            if ch.is_control() || ILLEGAL.contains(&ch) {
                '-'
            } else {
                ch
            }
        })
        .collect();
    let trimmed = replaced
        .trim()
        .trim_start_matches('.')
        .trim_end_matches(|ch: char| ch == '.' || ch.is_whitespace());
    if trimmed.is_empty() {
        return "file".to_owned();
    }
    truncate_bytes(trimmed, MAX_STEM_BYTES)
}

fn truncate_bytes(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_owned();
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].trim_end().to_owned()
}

/// 把文件名拆成（主干，小写扩展名）。无扩展名时扩展名为空串。
pub fn split_name(name: &str) -> (String, String) {
    match name.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty()
                && !ext.is_empty()
                && ext.chars().count() <= 16
                && ext.chars().all(|ch| ch.is_alphanumeric()) =>
        {
            (stem.to_owned(), ext.to_lowercase())
        }
        _ => (name.to_owned(), String::new()),
    }
}

pub fn file_name(stem: &str, ext: &str) -> String {
    if ext.is_empty() {
        stem.to_owned()
    } else {
        format!("{stem}.{ext}")
    }
}

/// 在 `exists` 判定的目录里找一个可用主干：`stem`、`stem (2)`、`stem (3)`…
pub fn dedupe_stem(stem: &str, ext: &str, exists: impl Fn(&str) -> bool) -> String {
    let mut candidate = stem.to_owned();
    let mut suffix = 2;
    while exists(&file_name(&candidate, ext)) {
        candidate = format!("{stem} ({suffix})");
        suffix += 1;
    }
    candidate
}

/// 标题最长 100 个字符。
pub fn title_from_stem(stem: &str) -> String {
    stem.chars().take(100).collect::<String>().trim().to_owned()
}

/// 由文件名主干生成 Key 基底：保留中文与字母数字，非法字符换成 `-`，空结果为 `file`。
pub fn key_from_stem(stem: &str) -> String {
    let mut key = String::new();
    for ch in stem.chars() {
        if ch.is_alphanumeric() || matches!(ch, '_' | '.') {
            for lower in ch.to_lowercase() {
                key.push(lower);
            }
        } else if !key.ends_with('-') {
            key.push('-');
        }
    }
    let trimmed: String = key.trim_matches('-').chars().take(60).collect();
    let trimmed = trimmed.trim_matches('-');
    if trimmed.is_empty() {
        "file".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// 冲突时加 `-2`、`-3`… 后缀，保证总长不超过 64 个字符。
pub fn unique_key(base: &str, mut taken: impl FnMut(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_owned();
    }
    let mut n = 2;
    loop {
        let suffix = format!("-{n}");
        let keep = 64 - suffix.chars().count();
        let candidate = format!("{}{suffix}", base.chars().take(keep).collect::<String>());
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// 扫描忽略规则：隐藏文件、编辑器/Office 临时与锁文件、系统元数据。
pub fn is_ignored_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    name.starts_with('.')
        || name.starts_with("~$")
        || name.ends_with('~')
        || matches!(lower.as_str(), "thumbs.db" | "desktop.ini")
        || [
            ".tmp",
            ".temp",
            ".swp",
            ".swx",
            ".part",
            ".crdownload",
            ".bak",
        ]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_illegal_and_defaults_to_file() {
        assert_eq!(sanitize_stem("a/b:c*d"), "a-b-c-d");
        assert_eq!(sanitize_stem("  报销单 "), "报销单");
        assert_eq!(sanitize_stem("..."), "file");
        assert_eq!(sanitize_stem(".hidden"), "hidden");
        assert_eq!(sanitize_stem(""), "file");
        assert!(sanitize_stem(&"字".repeat(100)).len() <= 180);
    }

    #[test]
    fn split_name_handles_dotfiles_and_odd_extensions() {
        assert_eq!(split_name("报销单.XLSX"), ("报销单".into(), "xlsx".into()));
        assert_eq!(split_name("a.tar.gz"), ("a.tar".into(), "gz".into()));
        assert_eq!(split_name("README"), ("README".into(), "".into()));
        assert_eq!(split_name(".bashrc"), (".bashrc".into(), "".into()));
        assert_eq!(split_name("v1. final"), ("v1. final".into(), "".into()));
    }

    #[test]
    fn dedupe_adds_numeric_suffix() {
        let taken = ["月报.md".to_owned(), "月报 (2).md".to_owned()];
        let stem = dedupe_stem("月报", "md", |name| taken.iter().any(|t| t == name));
        assert_eq!(stem, "月报 (3)");
        assert_eq!(dedupe_stem("周报", "md", |_| false), "周报");
    }

    #[test]
    fn key_generation_keeps_chinese_and_handles_edge_cases() {
        assert_eq!(key_from_stem("报销单"), "报销单");
        assert_eq!(key_from_stem("Weekly Report (v2)"), "weekly-report-v2");
        assert_eq!(key_from_stem("***"), "file");
        assert_eq!(key_from_stem(""), "file");
        assert_eq!(key_from_stem("a__b.c"), "a__b.c");
        assert!(key_from_stem(&"长".repeat(100)).chars().count() <= 60);
    }

    #[test]
    fn unique_key_appends_numeric_suffix_within_limit() {
        let taken = ["报销单".to_owned(), "报销单-2".to_owned()];
        assert_eq!(
            unique_key("报销单", |k| taken.iter().any(|t| t == k)),
            "报销单-3"
        );
        let long = "x".repeat(60);
        let result = unique_key(&long, |k| k == long);
        assert!(result.chars().count() <= 64 && result.ends_with("-2"));
    }

    #[test]
    fn ignore_rules() {
        for name in [
            ".hidden",
            "~$a.docx",
            "a.md~",
            "x.tmp",
            "a.swp",
            "Thumbs.db",
            ".~lock.a.odt#",
        ] {
            assert!(is_ignored_name(name), "{name}");
        }
        for name in ["报销单.xlsx", "a.md", "notes"] {
            assert!(!is_ignored_name(name), "{name}");
        }
    }
}
