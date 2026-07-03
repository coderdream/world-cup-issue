use chrono::Local;
use rusqlite::{params, Connection};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct OperationLogger {
    db_path: PathBuf,
    log_dir: PathBuf,
}

impl OperationLogger {
    pub fn new(db_path: PathBuf, log_dir: PathBuf) -> Self {
        let logger = Self { db_path, log_dir };
        logger.init();
        logger
    }

    pub fn info(&self, module: &str, action: &str, message: impl AsRef<str>) {
        self.log("INFO", module, action, message, None, None);
    }

    #[allow(dead_code)]
    pub fn debug(&self, module: &str, action: &str, message: impl AsRef<str>, detail: impl AsRef<str>, trace_id: impl AsRef<str>) {
        self.log("DEBUG", module, action, message, Some(detail.as_ref()), Some(trace_id.as_ref()));
    }

    #[allow(dead_code)]
    pub fn trace_info(&self, module: &str, action: &str, message: impl AsRef<str>, detail: impl AsRef<str>, trace_id: impl AsRef<str>) {
        self.log("INFO", module, action, message, Some(detail.as_ref()), Some(trace_id.as_ref()));
    }

    #[allow(dead_code)]
    pub fn warn(&self, module: &str, action: &str, message: impl AsRef<str>, detail: impl AsRef<str>, trace_id: impl AsRef<str>) {
        self.log("WARN", module, action, message, Some(detail.as_ref()), Some(trace_id.as_ref()));
    }

    #[allow(dead_code)]
    pub fn trace_error(&self, module: &str, action: &str, message: impl AsRef<str>, detail: impl AsRef<str>, trace_id: impl AsRef<str>) {
        self.log("ERROR", module, action, message, Some(detail.as_ref()), Some(trace_id.as_ref()));
    }

    pub fn error(&self, module: &str, action: &str, message: impl AsRef<str>, detail: impl AsRef<str>) {
        self.log("ERROR", module, action, message, Some(detail.as_ref()), None);
    }

    pub fn log(
        &self,
        level: &str,
        module: &str,
        action: &str,
        message: impl AsRef<str>,
        detail: Option<&str>,
        trace_id: Option<&str>,
    ) {
        self.write(level, module, action, message.as_ref(), detail, trace_id);
    }

    fn init(&self) {
        if let Some(parent) = self.db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::create_dir_all(&self.log_dir);

        if let Ok(connection) = Connection::open(&self.db_path) {
            let _ = connection.execute_batch(
                r#"
                PRAGMA journal_mode = WAL;
                CREATE TABLE IF NOT EXISTS operate_log (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  created_at TEXT NOT NULL,
                  level TEXT NOT NULL,
                  module TEXT NOT NULL,
                  action TEXT NOT NULL,
                  message TEXT NOT NULL,
                  detail TEXT,
                  trace_id TEXT
                );
                "#,
            );
            let _ = connection.execute("ALTER TABLE operate_log ADD COLUMN trace_id TEXT", []);
            let _ = connection.execute_batch(
                r#"
                CREATE INDEX IF NOT EXISTS idx_operate_log_created_at ON operate_log(created_at);
                CREATE INDEX IF NOT EXISTS idx_operate_log_module_action ON operate_log(module, action);
                CREATE INDEX IF NOT EXISTS idx_operate_log_trace_id ON operate_log(trace_id);
                "#,
            );
        }
    }

    fn write(&self, level: &str, module: &str, action: &str, message: &str, detail: Option<&str>, trace_id: Option<&str>) {
        let now = Local::now();
        let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let date = now.format("%Y-%m-%d");
        let time = now.format("%H:%M:%S");
        let trace_part = trace_id
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("[{}]", value.trim()))
            .unwrap_or_default();
        let log_line = match detail {
            Some(detail) if !detail.is_empty() => format!("[{}][{}]{}[{}][{}] {}: {}\n", date, time, trace_part, module, level, message, detail),
            _ => format!("[{}][{}]{}[{}][{}] {}\n", date, time, trace_part, module, level, message),
        };

        let log_path = self.log_dir.join(format!("info_{}.log", now.format("%Y_%m_%d")));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = file.write_all(log_line.as_bytes());
        }

        if let Ok(connection) = Connection::open(&self.db_path) {
            let _ = connection.execute(
                "INSERT INTO operate_log (created_at, level, module, action, message, detail, trace_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![created_at, level, module, action, message, detail, trace_id],
            );
        }
    }
}
