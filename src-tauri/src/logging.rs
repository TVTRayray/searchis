use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
    sync::Mutex,
};

use crate::error::AppError;

#[derive(Debug)]
pub struct DiagnosticLogger {
    file: Option<Mutex<File>>,
}

impl DiagnosticLogger {
    pub fn open(log_dir: &Path) -> Self {
        let file = fs::create_dir_all(log_dir)
            .and_then(|_| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .mode(0o600)
                    .open(log_dir.join("searchis.log"))
            })
            .ok();
        if let Some(file) = &file {
            let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
        }
        Self {
            file: file.map(Mutex::new),
        }
    }

    pub fn record(&self, operation: &str, code: &str) {
        let line = format!(
            "{} app={} operation={} code={}\n",
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            env!("CARGO_PKG_VERSION"),
            operation,
            code,
        );
        eprint!("{line}");
        if let Some(file) = &self.file {
            if let Ok(mut file) = file.lock() {
                let _ = file.write_all(line.as_bytes()).and_then(|_| file.flush());
            }
        }
    }

    pub fn failure(&self, operation: &str, error: &AppError) {
        self.record(operation, error.code);
    }
}
