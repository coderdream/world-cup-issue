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
        self.write("INFO", module, action, message.as_ref(), None);
    }

    pub fn error(&self, module: &str, action: &str, message: impl AsRef<str>, detail: impl AsRef<str>) {
        self.write("ERROR", module, action, message.as_ref(), Some(detail.as_ref()));
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
                  detail TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_operate_log_created_at ON operate_log(created_at);
                CREATE INDEX IF NOT EXISTS idx_operate_log_module_action ON operate_log(module, action);
                "#,
            );
        }
    }

    fn write(&self, level: &str, module: &str, action: &str, message: &str, detail: Option<&str>) {
        let now = Local::now();
        let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let date = now.format("%Y-%m-%d");
        let time = now.format("%H:%M:%S");
        let log_line = match detail {
            Some(detail) if !detail.is_empty() => format!("[{}][{}][{}][{}] {}: {}\n", date, time, module, level, message, detail),
            _ => format!("[{}][{}][{}][{}] {}\n", date, time, module, level, message),
        };

        let log_path = self.log_dir.join(format!("info_{}.log", now.format("%Y_%m_%d")));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = file.write_all(log_line.as_bytes());
        }

        if let Ok(connection) = Connection::open(&self.db_path) {
            let _ = connection.execute(
                "INSERT INTO operate_log (created_at, level, module, action, message, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![created_at, level, module, action, message, detail],
            );
        }
    }
}
