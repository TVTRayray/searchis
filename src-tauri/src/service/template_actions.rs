//! SPEC-23：文件模板的带格式复制与导出。
//!
//! 带格式复制：先生成数据并成为剪贴板所有者，再计数；失败不计数。
//! 导出：改名成功后再计数。日志只记录操作名与错误码。

use std::path::PathBuf;

use super::{auto_paste_step, SnippetService};
use crate::{
    error::AppError,
    format_copy,
    model::{
        PasteOutcome, TemplateExportInput, TemplateExportOutcome, TemplateFormatCopyInput,
        TemplateFormatCopyOutcome,
    },
    platform::{x11_clipboard::FormatClipboard, WindowManager},
    storage::FileRow,
    templates::{export, MAX_FILE_BYTES},
};

fn check_operation_id(operation_id: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(operation_id)
        .map(|_| ())
        .map_err(|_| AppError::validation("operationId", "operationId 必须是 UUID。"))
}

impl SnippetService {
    /// 文件缺失时触发一次目录扫描（让列表收敛），再把错误交给调用方。
    fn live_template(&self, id: &str) -> Result<(FileRow, PathBuf), AppError> {
        let result = self.open_target(id);
        if matches!(&result, Err(error) if error.code == "TEMPLATE_FILE_MISSING") {
            let _ = self.template_scan();
        }
        result
    }

    pub fn template_format_copy(
        &self,
        input: TemplateFormatCopyInput,
        clipboard: &dyn FormatClipboard,
        window_manager: &dyn WindowManager,
    ) -> Result<TemplateFormatCopyOutcome, AppError> {
        check_operation_id(&input.operation_id)?;
        let (row, path) = self.live_template(&input.id)?;
        // 同一 operationId 只处理一次：重复投递在写剪贴板之前短路。
        if self.is_operation_processed(&input.operation_id)? {
            let snippet = self.get(&input.id)?;
            return Ok(TemplateFormatCopyOutcome {
                paste: PasteOutcome {
                    snippet: self.item_for(&snippet),
                    action: "copied".into(),
                    target_valid: false,
                    counted: false,
                    auto_paste_disabled: false,
                },
                notes: Vec::new(),
            });
        }
        let setting_enabled = self.repository.get_settings()?.0.auto_paste;
        // 只有 md 与 xlsx 需要读文件内容；其他类型只提供路径。
        let bytes = if matches!(row.ext.as_str(), "md" | "xlsx") {
            read_template(&path)?
        } else {
            Vec::new()
        };
        let set = format_copy::build(&row.ext, &bytes, &path)?;
        clipboard.set_formats(set.formats)?;
        let (action, target_valid) =
            auto_paste_step(input.auto_paste && setting_enabled, window_manager);
        let (updated, counted) = self
            .record_usage(&input.id, &input.operation_id)
            .map_err(|_| {
                AppError::new(
                    "DB_WRITE_FAILED",
                    "已复制到剪贴板，但使用统计未更新。请稍后重试。",
                )
            })?;
        Ok(TemplateFormatCopyOutcome {
            paste: PasteOutcome {
                snippet: self.item_for(&updated),
                action,
                target_valid,
                counted,
                auto_paste_disabled: input.auto_paste && !setting_enabled,
            },
            notes: set.notes,
        })
    }

    pub fn template_export(
        &self,
        input: TemplateExportInput,
    ) -> Result<TemplateExportOutcome, AppError> {
        check_operation_id(&input.operation_id)?;
        let (row, source) = self.live_template(&input.id)?;
        if self.is_operation_processed(&input.operation_id)? {
            return Ok(TemplateExportOutcome {
                path: String::new(),
                counted: false,
            });
        }
        let configured = self.repository.get_settings()?.0.export_dir;
        let dir = export::resolve_export_dir(configured.as_deref())?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let path = export::export_copy(&source, &dir, &row.title, &row.ext, &date)?;
        let (_, counted) = self
            .record_usage(&input.id, &input.operation_id)
            .map_err(|_| {
                AppError::new(
                    "DB_WRITE_FAILED",
                    "文件已导出，但使用统计未更新。请稍后重试。",
                )
            })?;
        Ok(TemplateExportOutcome {
            path: path.to_string_lossy().into_owned(),
            counted,
        })
    }
}

fn read_template(path: &std::path::Path) -> Result<Vec<u8>, AppError> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            super::templates::missing_error()
        } else {
            format_copy::copy_failed("无法读取模板文件。")
        }
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format_copy::copy_failed("无法读取模板文件。"))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format_copy::copy_failed("模板文件过大，无法带格式复制。"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    };

    use super::*;
    use crate::{
        model::{Snippet, TemplateImportInput},
        platform::WindowHandle,
        storage::Repository,
    };

    #[derive(Default)]
    struct FakeFormats {
        fail: bool,
        calls: Mutex<Vec<Vec<(String, Vec<u8>)>>>,
    }

    impl FormatClipboard for FakeFormats {
        fn set_formats(&self, formats: Vec<(String, Vec<u8>)>) -> Result<(), AppError> {
            if self.fail {
                return Err(AppError::new("FORMAT_COPY_FAILED", "模拟失败"));
            }
            self.calls.lock().unwrap().push(formats);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeWm {
        injected: AtomicUsize,
    }

    impl WindowManager for FakeWm {
        fn get_active_window(&self) -> Result<Option<WindowHandle>, AppError> {
            Ok(Some(WindowHandle(1)))
        }
        fn activate_window(&self, _: WindowHandle) -> Result<(), AppError> {
            Ok(())
        }
        fn inject_ctrl_v(&self, _: WindowHandle) -> Result<(), AppError> {
            self.injected.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn injection_available(&self) -> bool {
            true
        }
    }

    struct Env {
        _dir: tempfile::TempDir,
        service: SnippetService,
        root: PathBuf,
        scratch: PathBuf,
        exports: PathBuf,
    }

    fn env() -> Env {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        let root = dir.path().join("templates");
        let scratch = dir.path().join("scratch");
        let exports = dir.path().join("exports");
        fs::create_dir_all(&scratch).unwrap();
        fs::create_dir_all(&exports).unwrap();
        let service = SnippetService::new(repo).with_templates(root.clone());
        let (mut settings, revision) = service.repository.get_settings().unwrap();
        settings.export_dir = Some(exports.to_string_lossy().into_owned());
        service.repository.update_settings(&settings, revision).unwrap();
        Env {
            _dir: dir,
            service,
            root,
            scratch,
            exports,
        }
    }

    impl Env {
        fn import(&self, name: &str, bytes: &[u8]) -> Snippet {
            let path = self.scratch.join(name);
            fs::write(&path, bytes).unwrap();
            self.service
                .template_import(TemplateImportInput {
                    source_paths: vec![path.to_string_lossy().into_owned()],
                    group_id: None,
                    request_id: uuid::Uuid::new_v4().to_string(),
                })
                .unwrap()
                .created
                .remove(0)
        }

        fn copy(
            &self,
            id: &str,
            auto_paste: bool,
            clipboard: &FakeFormats,
            wm: &FakeWm,
        ) -> Result<TemplateFormatCopyOutcome, AppError> {
            self.copy_with(id, &uuid::Uuid::new_v4().to_string(), auto_paste, clipboard, wm)
        }

        fn copy_with(
            &self,
            id: &str,
            operation_id: &str,
            auto_paste: bool,
            clipboard: &FakeFormats,
            wm: &FakeWm,
        ) -> Result<TemplateFormatCopyOutcome, AppError> {
            self.service.template_format_copy(
                TemplateFormatCopyInput {
                    id: id.into(),
                    operation_id: operation_id.into(),
                    auto_paste,
                },
                clipboard,
                wm,
            )
        }

        fn usage(&self, id: &str) -> i64 {
            self.service.get(id).unwrap().usage_count
        }

        fn set_auto_paste(&self, value: bool) {
            let (mut settings, revision) = self.service.repository.get_settings().unwrap();
            settings.auto_paste = value;
            self.service.repository.update_settings(&settings, revision).unwrap();
        }

        fn export(&self, id: &str, operation_id: &str) -> Result<TemplateExportOutcome, AppError> {
            self.service.template_export(TemplateExportInput {
                id: id.into(),
                operation_id: operation_id.into(),
            })
        }
    }

    fn format_names(call: &[(String, Vec<u8>)]) -> Vec<&str> {
        call.iter().map(|(name, _)| name.as_str()).collect()
    }

    fn xlsx_bytes(cells: u32) -> Vec<u8> {
        let mut book = umya_spreadsheet::new_file();
        let sheet = book.sheet_mut(0).unwrap();
        sheet.cell_mut("A1").set_value("名称");
        sheet.cell_mut((cells, 1)).set_value_number(1);
        let mut buffer = std::io::Cursor::new(Vec::new());
        umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buffer).unwrap();
        buffer.into_inner()
    }

    #[test]
    fn markdown_copy_provides_html_and_plain_and_counts_once_per_operation() {
        let env = env();
        let template = env.import("说明.md", "# 标题\n\n<script>x</script>\n".as_bytes());
        let clipboard = FakeFormats::default();
        let wm = FakeWm::default();
        let operation = uuid::Uuid::new_v4().to_string();

        let first = env.copy_with(&template.id, &operation, false, &clipboard, &wm).unwrap();
        assert!(first.paste.counted);
        assert_eq!(first.paste.action, "copied");
        {
            let calls = clipboard.calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(format_names(&calls[0]), ["text/html", "text/plain", "UTF8_STRING"]);
            let html = String::from_utf8(calls[0][0].1.clone()).unwrap();
            assert!(html.contains("<h1>标题</h1>") && !html.contains("<script>"));
        }
        assert_eq!(env.usage(&template.id), 1);

        // 重复投递：不再写剪贴板，不再计数。
        let again = env.copy_with(&template.id, &operation, false, &clipboard, &wm).unwrap();
        assert!(!again.paste.counted);
        assert_eq!(clipboard.calls.lock().unwrap().len(), 1);
        assert_eq!(env.usage(&template.id), 1);
    }

    #[test]
    fn xlsx_copy_provides_spreadsheet_html_and_tsv() {
        let env = env();
        let template = env.import("预算.xlsx", &xlsx_bytes(3));
        let clipboard = FakeFormats::default();
        env.copy(&template.id, false, &clipboard, &FakeWm::default()).unwrap();
        let calls = clipboard.calls.lock().unwrap();
        assert_eq!(
            format_names(&calls[0]),
            ["XML Spreadsheet", "text/html", "text/plain", "UTF8_STRING"]
        );
    }

    #[test]
    fn other_file_types_provide_uri_list_gnome_copied_files_and_path() {
        let env = env();
        let template = env.import("合同.pdf", b"%PDF-1.4");
        let clipboard = FakeFormats::default();
        env.copy(&template.id, false, &clipboard, &FakeWm::default()).unwrap();
        let calls = clipboard.calls.lock().unwrap();
        assert_eq!(
            format_names(&calls[0]),
            ["text/uri-list", "x-special/gnome-copied-files", "text/plain", "UTF8_STRING"]
        );
        let path = String::from_utf8(calls[0][2].1.clone()).unwrap();
        assert!(path.starts_with(env.root.to_str().unwrap()) && path.ends_with(".pdf"), "{path}");
        let gnome = String::from_utf8(calls[0][1].1.clone()).unwrap();
        assert!(gnome.starts_with("copy\nfile:///"));
    }

    #[test]
    fn failures_keep_the_window_state_and_never_count() {
        let env = env();
        let wm = FakeWm::default();
        // 剪贴板失败
        let md = env.import("a.md", b"# a");
        let failing = FakeFormats {
            fail: true,
            ..Default::default()
        };
        assert_eq!(env.copy(&md.id, true, &failing, &wm).unwrap_err().code, "FORMAT_COPY_FAILED");
        assert_eq!(env.usage(&md.id), 0);
        assert_eq!(wm.injected.load(Ordering::SeqCst), 0);
        // 损坏的 xlsx
        let broken = env.import("b.xlsx", b"not a zip");
        let ok = FakeFormats::default();
        assert_eq!(env.copy(&broken.id, false, &ok, &wm).unwrap_err().code, "FORMAT_COPY_FAILED");
        // 超过 50,000 单元格
        let huge = env.import("c.xlsx", &xlsx_bytes(300));
        let mut book = umya_spreadsheet::new_file();
        book.sheet_mut(0).unwrap().cell_mut((300, 200)).set_value("x");
        let mut buffer = std::io::Cursor::new(Vec::new());
        umya_spreadsheet::writer::xlsx::write_writer(&book, &mut buffer).unwrap();
        fs::write(env.root.join(&env.service.repository.file_row(&huge.id).unwrap().unwrap().rel_path), buffer.into_inner()).unwrap();
        assert_eq!(env.copy(&huge.id, false, &ok, &wm).unwrap_err().code, "FORMAT_COPY_TOO_LARGE");
        assert_eq!(env.usage(&broken.id) + env.usage(&huge.id), 0);
        assert!(ok.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn auto_paste_is_requested_flag_and_setting() {
        let env = env();
        let template = env.import("a.md", b"# a");
        let clipboard = FakeFormats::default();
        let wm = FakeWm::default();

        // 请求 + 设置开 → 注入
        let outcome = env.copy(&template.id, true, &clipboard, &wm).unwrap();
        assert_eq!((outcome.paste.action.as_str(), outcome.paste.auto_paste_disabled), ("pasted", false));
        assert_eq!(wm.injected.load(Ordering::SeqCst), 1);

        // 未请求 → 仅复制
        let outcome = env.copy(&template.id, false, &clipboard, &wm).unwrap();
        assert_eq!((outcome.paste.action.as_str(), outcome.paste.auto_paste_disabled), ("copied", false));
        assert_eq!(wm.injected.load(Ordering::SeqCst), 1);

        // 请求 + 设置关 → 仅复制，并标记“设置中已关闭”
        env.set_auto_paste(false);
        let outcome = env.copy(&template.id, true, &clipboard, &wm).unwrap();
        assert_eq!((outcome.paste.action.as_str(), outcome.paste.auto_paste_disabled), ("copied", true));
        assert!(outcome.paste.counted);
        assert_eq!(wm.injected.load(Ordering::SeqCst), 1, "autoPaste=false must not inject");
    }

    #[test]
    fn missing_file_reports_template_file_missing_and_marks_the_row() {
        let env = env();
        let template = env.import("a.md", b"# a");
        let rel = env.service.repository.file_row(&template.id).unwrap().unwrap().rel_path;
        fs::remove_file(env.root.join(rel)).unwrap();
        let clipboard = FakeFormats::default();
        let error = env.copy(&template.id, false, &clipboard, &FakeWm::default()).unwrap_err();
        assert_eq!(error.code, "TEMPLATE_FILE_MISSING");
        assert!(clipboard.calls.lock().unwrap().is_empty());
        assert!(env.service.repository.file_row(&template.id).unwrap().unwrap().missing);
        assert_eq!(env.export(&template.id, &uuid::Uuid::new_v4().to_string()).unwrap_err().code, "TEMPLATE_FILE_MISSING");
    }

    #[test]
    fn operation_id_must_be_uuid() {
        let env = env();
        let template = env.import("a.md", b"# a");
        let error = env
            .copy_with(&template.id, "nope", false, &FakeFormats::default(), &FakeWm::default())
            .unwrap_err();
        assert_eq!(error.code, "VALIDATION_FAILED");
        assert_eq!(env.export(&template.id, "nope").unwrap_err().code, "VALIDATION_FAILED");
    }

    #[test]
    fn export_writes_dated_copy_counts_once_and_is_idempotent() {
        let env = env();
        let template = env.import("报销单.xlsx", &xlsx_bytes(2));
        let operation = uuid::Uuid::new_v4().to_string();
        let outcome = env.export(&template.id, &operation).unwrap();
        assert!(outcome.counted);
        let date = chrono::Local::now().format("%Y-%m-%d");
        let expected = env.exports.join(format!("报销单_{date}.xlsx"));
        assert_eq!(PathBuf::from(&outcome.path), expected);
        let source = fs::read(env.root.join(&env.service.repository.file_row(&template.id).unwrap().unwrap().rel_path)).unwrap();
        assert_eq!(fs::read(&expected).unwrap(), source);
        assert_eq!(env.usage(&template.id), 1);

        let again = env.export(&template.id, &operation).unwrap();
        assert_eq!((again.path.as_str(), again.counted), ("", false));
        assert_eq!(fs::read_dir(&env.exports).unwrap().count(), 1);

        // 新操作：同名加后缀，原文件不被覆盖。
        let second = env.export(&template.id, &uuid::Uuid::new_v4().to_string()).unwrap();
        assert_eq!(PathBuf::from(&second.path), env.exports.join(format!("报销单_{date} (2).xlsx")));
        assert_eq!(env.usage(&template.id), 2);
    }

    #[test]
    fn export_failure_does_not_count_and_leaves_nothing_behind() {
        let env = env();
        let template = env.import("a.md", b"# a");
        let (mut settings, revision) = env.service.repository.get_settings().unwrap();
        settings.export_dir = Some(env.exports.join("missing").to_string_lossy().into_owned());
        env.service.repository.update_settings(&settings, revision).unwrap();
        let error = env.export(&template.id, &uuid::Uuid::new_v4().to_string()).unwrap_err();
        assert_eq!(error.code, "EXPORT_FILE_FAILED");
        assert!(!env.exports.join("missing").exists(), "must not auto-create the directory");
        assert_eq!(env.usage(&template.id), 0);
        assert_eq!(fs::read_dir(&env.exports).unwrap().count(), 0);
    }

    #[test]
    fn text_snippets_are_not_file_templates() {
        let env = env();
        let snippet = env
            .service
            .create(crate::model::CreateSnippetInput {
                key: "txt".into(),
                title: "T".into(),
                content: "x".into(),
                aliases: vec![],
                tags: vec![],
                sensitive: false,
                pinned: false,
                group_id: None,
                language: None,
                request_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        let error = env
            .copy(&snippet.id, false, &FakeFormats::default(), &FakeWm::default())
            .unwrap_err();
        assert_eq!(error.code, "NOT_FOUND");
        assert_eq!(env.usage(&snippet.id), 0);
    }
}
