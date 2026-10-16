//! 模板路径安全与分组目录映射。

use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

use crate::{error::AppError, model::Group};

fn fs_error(message: &str) -> AppError {
    AppError::new("TEMPLATE_FS_FAILED", message)
}

/// 校验相对路径：非空、非绝对、无 `..`、无空段与 NUL。
pub fn validate_rel_path(rel: &str) -> Result<(), AppError> {
    let invalid = rel.is_empty()
        || rel.contains('\0')
        || rel.starts_with('/')
        || rel
            .split('/')
            .any(|segment| segment.is_empty() || segment == "..");
    if invalid
        || Path::new(rel)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(fs_error("模板路径不合法。"));
    }
    Ok(())
}

/// 校验 `path`（必须位于 `root` 之下）：自 `root` 起逐段检查每个已存在的组成部分（含 `path` 自身），
/// 任何符号链接都拒绝，且已存在的最深部分规范化后必须仍在 canonical root 内。
pub fn verify_under(root: &Path, path: &Path) -> Result<(), AppError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| fs_error("模板路径超出模板目录。"))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|_| fs_error("无法访问模板目录。"))?;
    let mut current = root.to_path_buf();
    for component in rel.components() {
        let Component::Normal(part) = component else {
            return Err(fs_error("模板路径不合法。"));
        };
        current.push(part);
        match current.symlink_metadata() {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(fs_error("模板路径包含符号链接。"));
            }
            Ok(_) => {
                let canonical = current
                    .canonicalize()
                    .map_err(|_| fs_error("无法访问模板目录。"))?;
                if !canonical.starts_with(&canonical_root) {
                    return Err(fs_error("模板路径超出模板目录。"));
                }
            }
            // 之后的组成部分都不存在，无需继续。
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err(fs_error("无法访问模板目录。")),
        }
    }
    Ok(())
}

/// `root` + 校验后的相对路径（见 [`verify_under`]）。
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    validate_rel_path(rel)?;
    let joined = root.join(rel);
    verify_under(root, &joined)?;
    Ok(joined)
}

/// 相对目录 → 绝对路径；空串表示模板目录根。
pub fn dir_join(root: &Path, dir: &str) -> Result<PathBuf, AppError> {
    if dir.is_empty() {
        Ok(root.to_path_buf())
    } else {
        safe_join(root, dir)
    }
}

pub fn join_rel(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// 相对路径的目录部分（根目录为空串）。
pub fn parent_dir(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(dir, _)| dir)
}

pub fn base_name(rel: &str) -> &str {
    rel.rsplit_once('/').map_or(rel, |(_, name)| name)
}

/// 分组 id → 相对目录（`a/b/c`）。
pub fn group_dirs(groups: &[Group]) -> HashMap<String, String> {
    let by_id: HashMap<&str, &Group> = groups.iter().map(|g| (g.id.as_str(), g)).collect();
    let mut result = HashMap::new();
    for group in groups {
        let mut parts = vec![group.name.as_str()];
        let mut cursor = group.parent_id.as_deref();
        let mut guard = 0;
        while let Some(parent) = cursor {
            let Some(node) = by_id.get(parent) else { break };
            parts.push(node.name.as_str());
            cursor = node.parent_id.as_deref();
            guard += 1;
            if guard > 8 {
                break;
            }
        }
        parts.reverse();
        result.insert(group.id.clone(), parts.join("/"));
    }
    result
}

/// 删除分组时，各子分组上移后的最终名称（与存储层的 `(2)` 后缀规则一致）。
pub fn plan_group_delete_names(groups: &[Group], id: &str) -> Vec<(String, String)> {
    let Some(target) = groups.iter().find(|g| g.id == id) else {
        return Vec::new();
    };
    let parent = target.parent_id.clone();
    let mut taken: Vec<String> = groups
        .iter()
        .filter(|g| g.parent_id == parent && g.id != id)
        .map(|g| g.name.to_lowercase())
        .collect();
    let mut children: Vec<&Group> = groups
        .iter()
        .filter(|g| g.parent_id.as_deref() == Some(id))
        .collect();
    children.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let mut result = Vec::new();
    for child in children {
        let mut candidate = child.name.clone();
        let mut suffix = 2;
        while taken.contains(&candidate.to_lowercase()) {
            let tail = format!(" ({suffix})");
            let keep = 64usize.saturating_sub(tail.chars().count());
            candidate = format!(
                "{}{tail}",
                child.name.chars().take(keep).collect::<String>()
            );
            suffix += 1;
        }
        taken.push(candidate.to_lowercase());
        result.push((child.id.clone(), candidate));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_absolute_and_empty_segments() {
        for rel in ["", "/etc/passwd", "../x", "a/../b", "a//b", "a/\0b"] {
            assert!(validate_rel_path(rel).is_err(), "{rel:?}");
        }
        assert!(validate_rel_path("行政/报销单.xlsx").is_ok());
    }

    #[test]
    fn safe_join_rejects_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("evil")).unwrap();
        assert!(safe_join(root.path(), "evil/a.md").is_err());
        assert!(safe_join(root.path(), "ok/a.md").is_ok());
    }

    #[test]
    fn safe_join_rejects_leaf_and_intermediate_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("evil")).unwrap();
        std::fs::create_dir_all(root.path().join("a")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("a/mid")).unwrap();
        for rel in ["evil", "a/mid", "a/mid/deeper/x.md"] {
            assert!(safe_join(root.path(), rel).is_err(), "{rel}");
            assert!(dir_join(root.path(), rel).is_err(), "{rel}");
        }
        // 根内的链接同样拒绝。
        std::os::unix::fs::symlink(root.path().join("a"), root.path().join("inner")).unwrap();
        assert!(safe_join(root.path(), "inner").is_err());
    }

    fn group(id: &str, name: &str, parent: Option<&str>) -> Group {
        Group {
            id: id.into(),
            name: name.into(),
            parent_id: parent.map(String::from),
            depth: 1,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
            revision: 1,
            snippet_count: 0,
        }
    }

    #[test]
    fn group_dirs_build_nested_paths() {
        let groups = vec![
            group("a", "运维", None),
            group("b", "数据库", Some("a")),
            group("c", "慢查询", Some("b")),
        ];
        let dirs = group_dirs(&groups);
        assert_eq!(dirs["c"], "运维/数据库/慢查询");
        assert_eq!(dirs["a"], "运维");
    }

    #[test]
    fn delete_names_suffix_conflicting_children() {
        let groups = vec![
            group("p", "行政", None),
            group("q", "报销", Some("p")),
            group("r", "报销", None),
        ];
        let names = plan_group_delete_names(&groups, "p");
        assert_eq!(names, vec![("q".to_owned(), "报销 (2)".to_owned())]);
    }
}
