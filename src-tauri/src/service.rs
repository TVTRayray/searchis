use chrono::Utc;

use crate::{
    clipboard::ClipboardWriter,
    error::AppError,
    model::{
        CopyInput, CopyOutcome, CreateSnippetInput, PrepareNewOutcome, SearchInput, SearchResponse,
        SearchResultItem, Settings, SettingsResponse, Snippet, UpdateSnippetInput,
    },
    storage::Repository,
    validation::{normalize_key, validate_create, validate_update},
};

pub struct SnippetService {
    repository: Repository,
}

impl SnippetService {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }

    pub fn create(&self, input: CreateSnippetInput) -> Result<Snippet, AppError> {
        let fields = validate_create(&input)?;
        self.repository.create(fields, &input.request_id)
    }

    pub fn get(&self, id: &str) -> Result<Snippet, AppError> {
        self.repository
            .get(id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "片段不存在。请重新载入列表。"))
    }

    pub fn list(&self) -> Result<Vec<Snippet>, AppError> {
        self.repository.list()
    }

    pub fn update(&self, input: UpdateSnippetInput) -> Result<Snippet, AppError> {
        let fields = validate_update(&input)?;
        self.repository.update(&input.id, input.revision, fields)
    }

    pub fn search(&self, input: SearchInput) -> Result<SearchResponse, AppError> {
        if !(5..=100).contains(&input.limit) {
            return Err(AppError::validation("limit", "结果数量必须在 5–100 之间。"));
        }
        self.repository.search(&input.query, input.limit)
    }

    pub fn copy(
        &self,
        input: CopyInput,
        clipboard: &dyn ClipboardWriter,
    ) -> Result<CopyOutcome, AppError> {
        if uuid::Uuid::parse_str(&input.operation_id).is_err() {
            return Err(AppError::validation(
                "operationId",
                "operationId 必须是 UUID。",
            ));
        }
        let snippet = self.get(&input.id)?;
        if snippet.deleted_at.is_some() {
            return Err(AppError::new(
                "NOT_FOUND",
                "片段不存在或已删除。请重新搜索后重试。",
            ));
        }
        // AC-14：同一 operationId 只处理一次。重复投递在写剪贴板之前即短路，
        // 避免覆盖用户两次投递之间复制的新内容。
        if self
            .repository
            .is_operation_processed(&input.operation_id)?
        {
            return Ok(CopyOutcome {
                snippet: to_search_item(&snippet),
                counted: false,
            });
        }
        clipboard.write_text(snippet.content.clone())?;
        let (updated, counted) = self
            .repository
            .record_usage(&input.id, &input.operation_id)
            .map_err(|_| {
                AppError::new(
                    "DB_WRITE_FAILED",
                    "已复制到剪贴板，但使用统计未更新。请稍后重试。",
                )
            })?;
        Ok(CopyOutcome {
            snippet: to_search_item(&updated),
            counted,
        })
    }

    pub fn prepare_new(&self, raw_query: &str) -> Result<PrepareNewOutcome, AppError> {
        let normalized_key = normalize_key(raw_query)?;
        Ok(PrepareNewOutcome { normalized_key })
    }

    /// 代理到 repository：检查 operationId 是否已处理（幂等）。
    pub fn is_operation_processed(&self, operation_id: &str) -> Result<bool, AppError> {
        self.repository.is_operation_processed(operation_id)
    }

    /// 代理到 repository：原子更新 usageCount/lastUsedAt。
    pub fn record_usage(
        &self,
        snippet_id: &str,
        operation_id: &str,
    ) -> Result<(Snippet, bool), AppError> {
        self.repository.record_usage(snippet_id, operation_id)
    }

    /// SPEC-03 完整粘贴事务：剪贴板写入 → 自动粘贴 → 统计更新。
    pub fn paste(
        &self,
        input: crate::model::PasteInput,
        clipboard: &dyn crate::clipboard::ClipboardWriter,
        window_manager: &dyn crate::platform::WindowManager,
    ) -> Result<crate::model::PasteOutcome, AppError> {
        // 1. 取片段
        let snippet = self.get(&input.snippet_id)?;
        if snippet.deleted_at.is_some() {
            return Err(AppError::new("NOT_FOUND", "片段不存在或已删除。"));
        }
        // 2. 幂等检查
        if self.is_operation_processed(&input.operation_id)? {
            return Ok(crate::model::PasteOutcome {
                snippet: to_search_item(&snippet),
                action: "copied".into(),
                target_valid: false,
                counted: false,
            });
        }
        // 3. 写剪贴板（失败直接返回，不计数不注入）
        clipboard.write_text(snippet.content.clone())?;
        // 4. 自动粘贴
        let mut action = "copied".into();
        let mut target_valid = false;
        if input.auto_paste {
            if let Ok(Some(target)) = window_manager.get_active_window() {
                if window_manager.activate_window(target).is_ok() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if window_manager.inject_ctrl_v(target).is_ok() {
                        action = "pasted".into();
                        target_valid = true;
                    } else {
                        action = "degraded".into();
                    }
                } else {
                    action = "degraded".into();
                }
            } else {
                action = "degraded".into();
            }
        }
        // 5. 原子统计更新
        let (updated, counted) = self.record_usage(&input.snippet_id, &input.operation_id)?;
        Ok(crate::model::PasteOutcome {
            snippet: to_search_item(&updated),
            action,
            target_valid,
            counted,
        })
    }

    // === SPEC-05: 回收站操作 ===

    pub fn trash_move(&self, id: &str) -> Result<Snippet, AppError> {
        self.repository.trash_move(id)
    }

    pub fn trash_restore(&self, id: &str) -> Result<Snippet, AppError> {
        self.repository.trash_restore(id)
    }

    pub fn trash_purge_one(&self, id: &str) -> Result<(), AppError> {
        self.repository.trash_purge_one(id)
    }

    pub fn trash_empty(&self) -> Result<i64, AppError> {
        self.repository.trash_empty()
    }

    pub fn auto_purge(&self, days: Option<i64>) -> Result<i64, AppError> {
        match days {
            None => Ok(0),
            Some(d) => {
                let threshold = (Utc::now() - chrono::Duration::days(d))
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                self.repository.auto_purge(&threshold)
            }
        }
    }

    // === SPEC-06: Settings ===

    pub fn get_settings(&self, autostart_actual: bool) -> Result<SettingsResponse, AppError> {
        let (settings, rev) = self.repository.get_settings()?;
        Ok(SettingsResponse {
            settings,
            revision: rev,
            autostart_actual,
        })
    }

    pub fn update_settings(
        &self,
        key: &str,
        value: &serde_json::Value,
        revision: i64,
    ) -> Result<Settings, AppError> {
        let (mut settings, current_rev) = self.repository.get_settings()?;
        if revision != current_rev {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "设置已被其他窗口修改。请重新加载。",
            ));
        }
        match key {
            "globalShortcut" => {
                settings.global_shortcut = value
                    .as_str()
                    .ok_or_else(|| AppError::validation("globalShortcut", "必须是字符串。"))?
                    .to_string();
            }
            "autoPaste" => {
                settings.auto_paste = value
                    .as_bool()
                    .ok_or_else(|| AppError::validation("autoPaste", "必须是布尔值。"))?;
            }
            "launchAtLogin" => {
                settings.launch_at_login = value
                    .as_bool()
                    .ok_or_else(|| AppError::validation("launchAtLogin", "必须是布尔值。"))?;
            }
            "theme" => {
                let t = value
                    .as_str()
                    .ok_or_else(|| AppError::validation("theme", "必须是字符串。"))?;
                if !matches!(t, "dark" | "light" | "system") {
                    return Err(AppError::validation(
                        "theme",
                        "必须是 dark、light 或 system。",
                    ));
                }
                settings.theme = t.to_string();
            }
            "maxResultsCount" => {
                let v = value
                    .as_i64()
                    .ok_or_else(|| AppError::validation("maxResultsCount", "必须是整数。"))?;
                if !(5..=100).contains(&v) {
                    return Err(AppError::validation(
                        "maxResultsCount",
                        "必须在 5–100 之间。",
                    ));
                }
                settings.max_results_count = v;
            }
            "trashAutoPurgeDays" => {
                settings.trash_auto_purge_days = match value {
                    serde_json::Value::Null => None,
                    serde_json::Value::Number(n) => {
                        let v = n.as_i64().ok_or_else(|| {
                            AppError::validation("trashAutoPurgeDays", "必须是整数。")
                        })?;
                        if !matches!(v, 7 | 30 | 90) {
                            return Err(AppError::validation(
                                "trashAutoPurgeDays",
                                "必须是 null、7、30 或 90。",
                            ));
                        }
                        Some(v)
                    }
                    _ => {
                        return Err(AppError::validation(
                            "trashAutoPurgeDays",
                            "必须是 null 或整数。",
                        ))
                    }
                };
            }
            "onboardingCompletedAt" => {
                settings.onboarding_completed_at = value.as_str().map(String::from);
            }
            "restoreClipboard" => {
                // PRD 硬约束：restoreClipboard 固定 false，不允许用户修改
                return Err(AppError::validation("restoreClipboard", "该字段不可修改。"));
            }
            _ => {
                return Err(AppError::validation(key, format!("未知设置字段: {key}")));
            }
        }
        let new_rev = self.repository.update_settings(&settings, revision)?;
        settings.schema_version = new_rev;
        Ok(settings)
    }

    // === SPEC-07: 导出 ===

    pub fn export_preview(&self, _path: &str) -> Result<crate::model::ExportPreview, AppError> {
        let snippets = self.list()?;
        let settings_resp = self.get_settings(false)?;
        let settings = settings_resp.settings;
        let sensitive_count = snippets.iter().filter(|s| s.sensitive).count() as i64;
        let snippet_count = snippets.len() as i64;
        // 估算 JSON 大小：直接序列化一次
        let backup = serde_json::json!({
            "schemaVersion": 1,
            "exportedAt": Utc::now().to_rfc3339(),
            "appVersion": "0.1.0",
            "snippets": &snippets,
            "settings": &settings,
        });
        let estimated_size = serde_json::to_vec(&backup)
            .map_err(|_| AppError::new("EXPORT_FAILED", "序列化估算失败。"))?
            .len() as u64;
        Ok(crate::model::ExportPreview {
            estimated_size_bytes: estimated_size,
            sensitive_count,
            snippet_count,
            has_settings: true,
        })
    }

    pub fn export_confirm(&self, path: &str) -> Result<crate::model::ExportOutcome, AppError> {
        use std::io::Write;
        let snippets = self.list()?;
        let settings_resp = self.get_settings(false)?;
        let settings = settings_resp.settings;
        let backup = serde_json::json!({
            "schemaVersion": 1,
            "exportedAt": Utc::now().to_rfc3339(),
            "appVersion": "0.1.0",
            "snippets": &snippets,
            "settings": &settings,
        });
        let json = serde_json::to_string_pretty(&backup)
            .map_err(|_| AppError::new("EXPORT_FAILED", "序列化备份数据失败。"))?;
        if json.len() > 100 * 1024 * 1024 {
            return Err(AppError::new(
                "EXPORT_FAILED",
                format!("备份文件超过 100 MB 上限（估算 {} 字节）。", json.len()),
            ));
        }
        let mut file = std::fs::File::create(path)
            .map_err(|e| AppError::new("EXPORT_FAILED", format!("无法创建文件: {e}")))?;
        file.write_all(json.as_bytes())
            .map_err(|e| AppError::new("EXPORT_FAILED", format!("写入文件失败: {e}")))?;
        file.sync_all().map_err(|_| {
            let _ = std::fs::remove_file(path);
            AppError::new("EXPORT_FAILED", "文件同步失败，已清理。")
        })?;
        Ok(crate::model::ExportOutcome {
            path: path.to_string(),
            size_bytes: json.len() as u64,
            snippet_count: snippets.len() as i64,
        })
    }

    // === SPEC-07: 导入 ===

    pub fn import_validate(&self, path: &str) -> Result<crate::model::ImportValidation, AppError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::new("IMPORT_FAILED", format!("无法读取文件: {e}")))?;
        if content.len() > 100 * 1024 * 1024 {
            return Err(AppError::new("IMPORT_TOO_LARGE", "导入文件超过 100 MB。"));
        }
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AppError::new("IMPORT_FAILED", format!("JSON 格式错误: {e}")))?;
        let obj = parsed
            .as_object()
            .ok_or_else(|| AppError::new("IMPORT_SCHEMA_INVALID", "导入文件必须是 JSON 对象。"))?;
        let mut errors = Vec::new();
        let schema_version = obj
            .get("schemaVersion")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if schema_version != 1 {
            errors.push(format!("schemaVersion 必须为 1，实际为 {schema_version}"));
        }
        let snippets_raw = obj.get("snippets").and_then(|v| v.as_array());
        let snippet_count = snippets_raw.as_ref().map_or(0, |a| a.len() as i64);
        let settings_present = obj.get("settings").is_some();
        // 校验每条 snippet 的必填字段
        if let Some(snippets_arr) = snippets_raw {
            for (i, item) in snippets_arr.iter().enumerate() {
                if item.get("id").is_none() {
                    errors.push(format!("片段 {i}: 缺少 id 字段"));
                }
                if item.get("key").is_none() {
                    errors.push(format!("片段 {i}: 缺少 key 字段"));
                }
                if item.get("content").is_none() {
                    errors.push(format!("片段 {i}: 缺少 content 字段"));
                }
            }
        }
        let import_token = uuid::Uuid::new_v4().to_string();
        Ok(crate::model::ImportValidation {
            valid: errors.is_empty(),
            snippet_count,
            settings_present,
            schema_version,
            errors,
            import_token,
        })
    }

    pub fn import_commit(
        &self,
        path: &str,
        import_token: &str,
    ) -> Result<crate::model::ImportOutcome, AppError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::new("IMPORT_FAILED", format!("无法读取文件: {e}")))?;
        let parsed: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AppError::new("IMPORT_FAILED", format!("JSON 格式错误: {e}")))?;
        let obj = parsed
            .as_object()
            .ok_or_else(|| AppError::new("IMPORT_SCHEMA_INVALID", "导入文件必须是 JSON 对象。"))?;
        let snippets_raw = obj
            .get("snippets")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AppError::new("IMPORT_SCHEMA_INVALID", "缺少 snippets 字段。"))?;
        // 解析并提交到存储
        self.repository.import_snippets(snippets_raw, import_token)
    }

    // === SPEC-07: 重置 ===

    pub fn reset_examples(&self) -> Result<i64, AppError> {
        self.repository.reset_to_examples()
    }

    // === SPEC-08: Onboarding ===

    pub fn environment_detect(&self) -> Result<crate::model::EnvironmentDetection, AppError> {
        // 检测发行版
        let distro = detect_distro();
        // 检测 KDE 版本
        let kde = detect_kde_version();
        // 检测 X11
        let x11 = detect_x11();
        // 检测 KGlobalAccel
        let kglobalaccel = detect_kglobalaccel();
        // 检测剪贴板
        let clipboard = detect_clipboard();
        // 检测 X11 注入
        let x11_inject = detect_x11_inject();
        Ok(crate::model::EnvironmentDetection {
            distro,
            kde_version: kde,
            x11,
            kglobalaccel,
            clipboard,
            x11_inject,
        })
    }

    pub fn onboarding_progress(&self, step: i32, revision: i64) -> Result<i64, AppError> {
        let mut settings = self.repository.get_settings()?.0;
        if revision != self.repository.get_settings()?.1 {
            return Err(AppError::new("REVISION_CONFLICT", "设置已被其他窗口修改。"));
        }
        // 在 settings 中保存 onboarding 进度（复用 onboardingCompletedAt 或新增字段）
        // 这里用 schema_version 字段的高 8 位存储步骤（临时方案）
        // 实际应该在 Settings 中添加 onboarding_step 字段
        // 为简化，用 onboardingCompletedAt 存储进度字符串 "step:N"
        settings.onboarding_completed_at = Some(format!("step:{step}"));
        self.repository.update_settings(&settings, revision)?;
        Ok(step as i64)
    }

    pub fn onboarding_complete(&self, action: &str) -> Result<Option<String>, AppError> {
        let (mut settings, rev) = self.repository.get_settings()?;
        match action {
            "finish" => {
                let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                settings.onboarding_completed_at = Some(completed_at.clone());
                self.repository.update_settings(&settings, rev)?;
                Ok(Some(completed_at))
            }
            "skip" => {
                let completed_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                settings.onboarding_completed_at = Some(completed_at.clone());
                self.repository.update_settings(&settings, rev)?;
                Ok(Some(completed_at))
            }
            _ => Err(AppError::new(
                "INVALID_ACTION",
                "action 必须是 finish 或 skip。",
            )),
        }
    }
}

// === 环境检测辅助函数 ===

fn detect_distro() -> crate::model::EnvironmentCapability {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        if let Some(line) = content.lines().find(|l| l.starts_with("PRETTY_NAME=")) {
            let value = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
            return crate::model::EnvironmentCapability {
                name: "发行版".into(),
                detected: true,
                supported: value.to_lowercase().contains("arch"),
                detail: value.to_string(),
            };
        }
    }
    crate::model::EnvironmentCapability {
        name: "发行版".into(),
        detected: false,
        supported: false,
        detail: "无法检测 /etc/os-release".into(),
    }
}

fn detect_kde_version() -> crate::model::EnvironmentCapability {
    let output = std::process::Command::new("plasmashell")
        .arg("--version")
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let is_v6 = version.contains("6.");
            crate::model::EnvironmentCapability {
                name: "KDE Plasma".into(),
                detected: true,
                supported: is_v6,
                detail: version,
            }
        }
        _ => crate::model::EnvironmentCapability {
            name: "KDE Plasma".into(),
            detected: false,
            supported: false,
            detail: "无法检测或未安装 plasmashell".into(),
        },
    }
}

fn detect_x11() -> crate::model::EnvironmentCapability {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let display = std::env::var("DISPLAY").unwrap_or_default();
    let is_x11 = session_type == "x11" || (!display.is_empty() && session_type.is_empty());
    crate::model::EnvironmentCapability {
        name: "显示服务器".into(),
        detected: !session_type.is_empty() || !display.is_empty(),
        supported: is_x11,
        detail: if session_type.is_empty() && display.is_empty() {
            "未检测到显示会话环境变量".into()
        } else {
            format!("{session_type} (DISPLAY={display})")
        },
    }
}

fn detect_kglobalaccel() -> crate::model::EnvironmentCapability {
    let has_kglobalaccel = std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase().contains("kde"))
        .unwrap_or(false);
    crate::model::EnvironmentCapability {
        name: "KGlobalAccel 全局快捷键".into(),
        detected: has_kglobalaccel,
        supported: has_kglobalaccel,
        detail: if has_kglobalaccel {
            "KDE 桌面环境已检测，支持全局快捷键注册".into()
        } else {
            "未检测到 KDE 桌面环境，仅支持应用内快捷键".into()
        },
    }
}

fn detect_clipboard() -> crate::model::EnvironmentCapability {
    crate::model::EnvironmentCapability {
        name: "系统剪贴板".into(),
        detected: true,
        supported: true,
        detail: "通过 Tauri 剪贴板插件支持".into(),
    }
}

fn detect_x11_inject() -> crate::model::EnvironmentCapability {
    let has_xdotool = std::process::Command::new("xdotool")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    crate::model::EnvironmentCapability {
        name: "X11 键盘事件注入".into(),
        detected: has_xdotool,
        supported: has_xdotool,
        detail: if has_xdotool {
            "xdotool 已安装，支持键盘事件注入".into()
        } else {
            "xdotool 未安装，自动粘贴将降级为仅复制".into()
        },
    }
}

fn to_search_item(snippet: &Snippet) -> SearchResultItem {
    SearchResultItem {
        id: snippet.id.clone(),
        key: snippet.key.clone(),
        title: snippet.title.clone(),
        aliases: snippet.aliases.clone(),
        tags: snippet.tags.clone(),
        pinned: snippet.pinned,
        sensitive: snippet.sensitive,
        usage_count: snippet.usage_count,
        last_used_at: snippet.last_used_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Repository;

    struct FakeClipboard {
        fail: bool,
        written: std::sync::Mutex<Vec<String>>,
    }

    impl ClipboardWriter for FakeClipboard {
        fn write_text(&self, text: String) -> Result<(), AppError> {
            if self.fail {
                return Err(AppError::new("CLIPBOARD_WRITE_FAILED", "模拟剪贴板失败"));
            }
            self.written.lock().unwrap().push(text);
            Ok(())
        }
    }

    fn create_snippet(service: &SnippetService, key: &str, content: &str) -> Snippet {
        let request_id = uuid::Uuid::new_v4().to_string();
        service
            .create(CreateSnippetInput {
                key: key.into(),
                title: "标题".into(),
                content: content.into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: request_id.clone(),
            })
            .unwrap()
    }

    #[test]
    fn copy_writes_content_to_clipboard_and_counts_once_per_operation() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let snippet = create_snippet(&service, "copy-key", "需要复制的内容 😀");
        let clipboard = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };

        let operation = uuid::Uuid::new_v4().to_string();
        let first = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: operation.clone(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap();
        assert!(first.counted);
        assert_eq!(first.snippet.usage_count, 1);
        assert_eq!(
            clipboard.written.lock().unwrap().as_slice(),
            &["需要复制的内容 😀".to_string()]
        );

        let retry = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: operation.clone(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap();
        assert!(!retry.counted);
        assert_eq!(retry.snippet.usage_count, 1);
        // AC-14：重复投递不再写剪贴板，避免覆盖用户新复制的内容。
        assert_eq!(clipboard.written.lock().unwrap().len(), 1);
    }

    #[test]
    fn copy_clipboard_success_but_stats_failure_reports_database_error() {
        let directory = tempfile::tempdir().unwrap();
        let repository = Repository::open(directory.path()).unwrap();
        repository
            .raw_connection()
            .execute_batch(
                "CREATE TRIGGER reject_operation BEFORE INSERT ON operation_requests
                 BEGIN SELECT RAISE(ABORT, 'simulated stats failure'); END;",
            )
            .unwrap();
        let service = SnippetService::new(repository);
        let snippet = create_snippet(&service, "stats-fail", "正文");
        let clipboard = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };

        let error = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(error.code, "DB_WRITE_FAILED");
        assert!(
            error.message.contains("已复制"),
            "提示必须说明已复制、统计未更新"
        );
        assert_eq!(clipboard.written.lock().unwrap().len(), 1);
        let after = service.get(&snippet.id).unwrap();
        assert_eq!(after.usage_count, 0, "统计失败不得计数");
        assert_eq!(after.revision, 1);
    }

    #[test]
    fn copy_clipboard_failure_does_not_count() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let snippet = create_snippet(&service, "fail-key", "正文");
        let clipboard = FakeClipboard {
            fail: true,
            written: std::sync::Mutex::new(vec![]),
        };

        let error = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(error.code, "CLIPBOARD_WRITE_FAILED");
        let after = service.get(&snippet.id).unwrap();
        assert_eq!(after.usage_count, 0);
        assert_eq!(after.revision, 1);
    }

    #[test]
    fn copy_rejects_invalid_operation_id_and_deleted_snippet() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let snippet = create_snippet(&service, "edge-key", "正文");
        let clipboard = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };

        let bad_operation = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: "not-a-uuid".into(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(bad_operation.code, "VALIDATION_FAILED");

        service.repository.mark_deleted(&snippet.id).unwrap();
        let deleted = service
            .copy(
                CopyInput {
                    id: snippet.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    keep_open: true,
                },
                &clipboard,
            )
            .unwrap_err();
        assert_eq!(deleted.code, "NOT_FOUND");
        assert_eq!(clipboard.written.lock().unwrap().len(), 0);
    }

    #[test]
    fn prepare_new_normalizes_key_and_rejects_invalid_input() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let outcome = service.prepare_new("  Hello   World ").unwrap();
        assert_eq!(outcome.normalized_key, "hello-world");
        assert!(service.prepare_new("   ").is_err());
        assert!(service.prepare_new("bad/key!").is_err());
    }

    #[test]
    fn search_validates_limit_bounds() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        assert!(service
            .search(SearchInput {
                query: "x".into(),
                limit: 3
            })
            .is_err());
        assert!(service
            .search(SearchInput {
                query: "x".into(),
                limit: 101
            })
            .is_err());
        assert!(service
            .search(SearchInput {
                query: "x".into(),
                limit: 20
            })
            .is_ok());
    }

    // === SPEC-03: paste transaction tests ===

    use crate::model::PasteInput;
    use crate::platform::{WindowHandle, WindowManager};

    /// 测试替身：模拟 X11 窗口管理。
    struct FakeWindowManager {
        inject_available: bool,
        activate_succeeds: bool,
        inject_succeeds: bool,
    }
    impl WindowManager for FakeWindowManager {
        fn get_active_window(&self) -> Result<Option<WindowHandle>, AppError> {
            Ok(Some(WindowHandle(12345)))
        }
        fn activate_window(&self, _handle: WindowHandle) -> Result<(), AppError> {
            if self.activate_succeeds {
                Ok(())
            } else {
                Err(AppError::new("X11_AUTOMATION_UNAVAILABLE", "模拟激活失败"))
            }
        }
        fn inject_ctrl_v(&self, _handle: WindowHandle) -> Result<(), AppError> {
            if self.inject_succeeds {
                Ok(())
            } else {
                Err(AppError::new("X11_AUTOMATION_UNAVAILABLE", "模拟注入失败"))
            }
        }
        fn injection_available(&self) -> bool {
            self.inject_available
        }
    }

    #[test]
    fn paste_with_autopaste_and_inject_succeeds() {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::open(directory.path()).unwrap();
        let service = SnippetService::new(repo);
        let req_id = uuid::Uuid::new_v4().to_string();
        let created = service
            .create(CreateSnippetInput {
                key: "test-paste".into(),
                title: "T".into(),
                content: "hello".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: req_id,
            })
            .unwrap();
        let wm = FakeWindowManager {
            inject_available: true,
            activate_succeeds: true,
            inject_succeeds: true,
        };
        let cb = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };
        let outcome = service
            .paste(
                PasteInput {
                    snippet_id: created.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    auto_paste: true,
                },
                &cb,
                &wm,
            )
            .unwrap();
        assert_eq!(outcome.action, "pasted");
        assert!(outcome.target_valid);
        assert!(outcome.counted);
        assert_eq!(outcome.snippet.usage_count, 1);
    }

    #[test]
    fn paste_autopaste_false_copies_only() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let req_id = uuid::Uuid::new_v4().to_string();
        let created = service
            .create(CreateSnippetInput {
                key: "copy-only".into(),
                title: "T".into(),
                content: "text".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: req_id,
            })
            .unwrap();
        let wm = FakeWindowManager {
            inject_available: true,
            activate_succeeds: true,
            inject_succeeds: true,
        };
        let cb = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };
        let outcome = service
            .paste(
                PasteInput {
                    snippet_id: created.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    auto_paste: false,
                },
                &cb,
                &wm,
            )
            .unwrap();
        assert_eq!(outcome.action, "copied");
        assert!(!outcome.target_valid);
        assert!(outcome.counted);
    }

    #[test]
    fn paste_clipboard_failure_no_count_no_inject() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let req_id = uuid::Uuid::new_v4().to_string();
        let created = service
            .create(CreateSnippetInput {
                key: "clip-fail".into(),
                title: "T".into(),
                content: "x".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: req_id,
            })
            .unwrap();
        let wm = FakeWindowManager {
            inject_available: true,
            activate_succeeds: true,
            inject_succeeds: true,
        };
        let cb = FakeClipboard {
            fail: true,
            written: std::sync::Mutex::new(vec![]),
        };
        let err = service
            .paste(
                PasteInput {
                    snippet_id: created.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    auto_paste: true,
                },
                &cb,
                &wm,
            )
            .unwrap_err();
        assert_eq!(err.code, "CLIPBOARD_WRITE_FAILED");
        let after = service.get(&created.id).unwrap();
        assert_eq!(after.usage_count, 0);
    }

    #[test]
    fn paste_inject_failure_degrades() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let req_id = uuid::Uuid::new_v4().to_string();
        let created = service
            .create(CreateSnippetInput {
                key: "inject-fail".into(),
                title: "T".into(),
                content: "y".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: req_id,
            })
            .unwrap();
        let wm = FakeWindowManager {
            inject_available: true,
            activate_succeeds: true,
            inject_succeeds: false,
        };
        let cb = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };
        let outcome = service
            .paste(
                PasteInput {
                    snippet_id: created.id.clone(),
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    auto_paste: true,
                },
                &cb,
                &wm,
            )
            .unwrap();
        assert_eq!(outcome.action, "degraded");
        assert!(outcome.counted);
    }

    #[test]
    fn paste_idempotent_on_duplicate_operation() {
        let directory = tempfile::tempdir().unwrap();
        let service = SnippetService::new(Repository::open(directory.path()).unwrap());
        let req_id = uuid::Uuid::new_v4().to_string();
        let created = service
            .create(CreateSnippetInput {
                key: "idempotent".into(),
                title: "T".into(),
                content: "z".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                request_id: req_id,
            })
            .unwrap();
        let wm = FakeWindowManager {
            inject_available: true,
            activate_succeeds: true,
            inject_succeeds: true,
        };
        let cb = FakeClipboard {
            fail: false,
            written: std::sync::Mutex::new(vec![]),
        };
        let op_id = uuid::Uuid::new_v4().to_string();
        service
            .paste(
                PasteInput {
                    snippet_id: created.id.clone(),
                    operation_id: op_id.clone(),
                    auto_paste: true,
                },
                &cb,
                &wm,
            )
            .unwrap();
        let retry = service
            .paste(
                PasteInput {
                    snippet_id: created.id.clone(),
                    operation_id: op_id.clone(),
                    auto_paste: true,
                },
                &cb,
                &wm,
            )
            .unwrap();
        assert!(!retry.counted);
        assert_eq!(retry.snippet.usage_count, 1);
    }
}
