use std::collections::HashSet;

use crate::{
    error::AppError,
    model::{CreateSnippetInput, UpdateSnippetInput, ValidatedFields},
};

const MAX_CONTENT_BYTES: usize = 100 * 1024;

pub fn normalize_key(value: &str) -> Result<String, AppError> {
    let mut normalized = String::new();
    let mut pending_separator = false;

    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            pending_separator = !normalized.is_empty();
            continue;
        }
        if pending_separator {
            normalized.push('-');
            pending_separator = false;
        }
        for lower in ch.to_lowercase() {
            normalized.push(lower);
        }
    }

    let length = normalized.chars().count();
    if !(1..=64).contains(&length) {
        return Err(AppError::validation("key", "Key 长度必须为 1–64 个字符。"));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(AppError::validation(
            "key",
            "Key 只能包含 Unicode 字母、数字、-、_ 或 .。",
        ));
    }
    Ok(normalized)
}

fn normalize_list(
    values: &[String],
    field: &str,
    max_length: usize,
) -> Result<Vec<String>, AppError> {
    if values.len() > 20 {
        return Err(AppError::validation(
            field,
            format!("{field} 最多包含 20 项。"),
        ));
    }
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        let length = trimmed.chars().count();
        if !(1..=max_length).contains(&length) {
            return Err(AppError::validation(
                field,
                format!("{field} 每项长度必须为 1–{max_length} 个字符。"),
            ));
        }
        let folded = trimmed.to_lowercase();
        if seen.insert(folded) {
            result.push(trimmed.to_owned());
        }
    }
    Ok(result)
}

fn validate_fields(
    key: &str,
    title: &str,
    content: &str,
    aliases: &[String],
    tags: &[String],
    sensitive: bool,
    pinned: bool,
) -> Result<ValidatedFields, AppError> {
    let normalized_key = normalize_key(key)?;
    let title = title.trim();
    if !(1..=100).contains(&title.chars().count()) {
        return Err(AppError::validation(
            "title",
            "标题长度必须为 1–100 个字符。",
        ));
    }
    if content.is_empty() || content.len() > MAX_CONTENT_BYTES {
        return Err(AppError::validation(
            "content",
            "正文 UTF-8 大小必须为 1 byte–100 KB。",
        ));
    }
    Ok(ValidatedFields {
        key: normalized_key.clone(),
        normalized_key,
        title: title.to_owned(),
        content: content.to_owned(),
        aliases: normalize_list(aliases, "aliases", 64)?,
        tags: normalize_list(tags, "tags", 32)?,
        sensitive,
        pinned,
    })
}

pub fn validate_create(input: &CreateSnippetInput) -> Result<ValidatedFields, AppError> {
    if uuid::Uuid::parse_str(&input.request_id).is_err() {
        return Err(AppError::validation("requestId", "requestId 必须是 UUID。"));
    }
    validate_fields(
        &input.key,
        &input.title,
        &input.content,
        &input.aliases,
        &input.tags,
        input.sensitive,
        input.pinned,
    )
}

pub fn validate_update(input: &UpdateSnippetInput) -> Result<ValidatedFields, AppError> {
    if uuid::Uuid::parse_str(&input.id).is_err() {
        return Err(AppError::validation("id", "片段 ID 无效。"));
    }
    if input.revision < 1 {
        return Err(AppError::validation(
            "revision",
            "revision 必须大于或等于 1。",
        ));
    }
    validate_fields(
        &input.key,
        &input.title,
        &input.content,
        &input.aliases,
        &input.tags,
        input.sensitive,
        input.pinned,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_unicode_key_and_internal_whitespace() {
        assert_eq!(normalize_key("  ÄBC  中文\tKey ").unwrap(), "äbc-中文-key");
    }

    #[test]
    fn rejects_invalid_or_empty_keys() {
        assert!(normalize_key("   ").is_err());
        assert!(normalize_key("hello/world").is_err());
        assert!(normalize_key(&"a".repeat(65)).is_err());
    }

    #[test]
    fn content_limit_counts_utf8_bytes() {
        let input = CreateSnippetInput {
            key: "emoji".into(),
            title: "Emoji".into(),
            content: "😀".repeat(25_601),
            aliases: vec![],
            tags: vec![],
            sensitive: false,
            pinned: false,
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        assert_eq!(
            validate_create(&input).unwrap_err().field.as_deref(),
            Some("content")
        );
    }

    #[test]
    fn accepts_exact_content_byte_limit() {
        let input = CreateSnippetInput {
            key: "limit".into(),
            title: "Limit".into(),
            content: "😀".repeat(25_600),
            aliases: vec![],
            tags: vec![],
            sensitive: false,
            pinned: false,
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        assert_eq!(input.content.len(), MAX_CONTENT_BYTES);
        assert!(validate_create(&input).is_ok());
    }

    #[test]
    fn lists_are_trimmed_and_case_insensitively_deduplicated() {
        let input = CreateSnippetInput {
            key: "key".into(),
            title: "Title".into(),
            content: "body".into(),
            aliases: vec![" Home ".into(), "home".into()],
            tags: vec!["工作".into(), "工作".into()],
            sensitive: false,
            pinned: false,
            request_id: uuid::Uuid::new_v4().to_string(),
        };
        let fields = validate_create(&input).unwrap();
        assert_eq!(fields.aliases, vec!["Home"]);
        assert_eq!(fields.tags, vec!["工作"]);
    }

    #[test]
    fn lists_enforce_count_and_item_length_limits() {
        assert!(normalize_list(&vec!["alias".into(); 21], "aliases", 64).is_err());
        assert!(normalize_list(&["a".repeat(65)], "aliases", 64).is_err());
        assert!(normalize_list(&["t".repeat(33)], "tags", 32).is_err());
    }
}
