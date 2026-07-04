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

    pub fn debug(
        &self,
        module: &str,
        action: &str,
        message: impl AsRef<str>,
        detail: impl AsRef<str>,
        trace_id: impl AsRef<str>,
    ) {
        self.log(
            "DEBUG",
            module,
            action,
            message,
            Some(detail.as_ref()),
            Some(trace_id.as_ref()),
        );
    }

    pub fn trace_info(
        &self,
        module: &str,
        action: &str,
        message: impl AsRef<str>,
        detail: impl AsRef<str>,
        trace_id: impl AsRef<str>,
    ) {
        self.log(
            "INFO",
            module,
            action,
            message,
            Some(detail.as_ref()),
            Some(trace_id.as_ref()),
        );
    }

    pub fn warn(
        &self,
        module: &str,
        action: &str,
        message: impl AsRef<str>,
        detail: impl AsRef<str>,
        trace_id: impl AsRef<str>,
    ) {
        self.log(
            "WARN",
            module,
            action,
            message,
            Some(detail.as_ref()),
            Some(trace_id.as_ref()),
        );
    }

    pub fn trace_error(
        &self,
        module: &str,
        action: &str,
        message: impl AsRef<str>,
        detail: impl AsRef<str>,
        trace_id: impl AsRef<str>,
    ) {
        self.log(
            "ERROR",
            module,
            action,
            message,
            Some(detail.as_ref()),
            Some(trace_id.as_ref()),
        );
    }

    pub fn error(
        &self,
        module: &str,
        action: &str,
        message: impl AsRef<str>,
        detail: impl AsRef<str>,
    ) {
        self.log(
            "ERROR",
            module,
            action,
            message,
            Some(detail.as_ref()),
            None,
        );
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

    fn write(
        &self,
        level: &str,
        module: &str,
        action: &str,
        message: &str,
        detail: Option<&str>,
        trace_id: Option<&str>,
    ) {
        let clean_message = clean_log_text(message, module, action);
        let clean_detail = detail.map(|value| clean_log_text(value, module, action));
        let message = clean_message.as_str();
        let detail = clean_detail.as_deref();
        let now = Local::now();
        let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let date = now.format("%Y-%m-%d");
        let time = now.format("%H:%M:%S");
        let trace_part = trace_id
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("[{}]", value.trim()))
            .unwrap_or_default();
        let log_line = match detail {
            Some(detail) if !detail.is_empty() => format!(
                "[{}][{}]{}[{}][{}] {}: {}\n",
                date, time, trace_part, module, level, message, detail
            ),
            _ => format!(
                "[{}][{}]{}[{}][{}] {}\n",
                date, time, trace_part, module, level, message
            ),
        };

        let log_path = self
            .log_dir
            .join(format!("info_{}.log", now.format("%Y_%m_%d")));
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

fn clean_log_text(value: &str, module: &str, action: &str) -> String {
    if !looks_like_mojibake(value) {
        return value.to_string();
    }
    if let Some(message) = fallback_log_message(module, action) {
        return message.to_string();
    }
    format!("{} completed.", action.replace('.', " "))
}

fn looks_like_mojibake(value: &str) -> bool {
    if value.contains('\u{fffd}')
        || value.contains("\u{951f}\u{fffd}")
        || value.contains("\u{003f}\u{003f}\u{003f}")
    {
        return true;
    }
    let suspicious = value
        .chars()
        .filter(|ch| {
            matches!(
                *ch,
                '\u{95c1}' | '\u{95c2}' | '\u{95bb}' | '\u{95ba}' | '\u{95bf}'
                    | '\u{5a75}' | '\u{6fde}' | '\u{7f02}' | '\u{940e}'
                    | '\u{9420}' | '\u{9207}' | '\u{93c9}' | '\u{7d8b}' | '\u{20ac}'
            ) || ('\u{e000}'..='\u{f8ff}').contains(ch)
        })
        .count();
    suspicious >= 2
}
fn fallback_log_message(module: &str, action: &str) -> Option<&'static str> {
    match (module, action) {
        ("materials", "generate.start") => Some("Start generating book materials."),
        ("materials", "settings.snapshot") => Some("Loaded AI settings for material generation."),
        ("materials", "source.file") => Some("Source file resolved."),
        ("materials", "source.read") => Some("Reading source book."),
        ("materials", "source.read.done") => Some("Source book read successfully."),
        ("materials", "source.read.timeout") => Some("Source book read timed out."),
        ("materials", "prompt.build") => Some("AI prompt built."),
        ("materials", "ai.request") => Some("Requesting AI material generation JSON."),
        ("materials", "ai.response") => Some("AI material generation response received."),
        ("materials", "ai.request.failed") => Some("AI material generation request failed."),
        ("materials", "ai.local_initial_fallback") => Some("Using local material fallback."),
        ("materials", "ai.parse") => Some("AI material JSON parsed."),
        ("materials", "ai.parse.failed") => Some("AI material JSON parse failed."),
        ("materials", "ai.repair.skip") => Some("Narration length is within target range."),
        ("materials", "ai.repair.required") => Some("Narration length repair is required."),
        ("materials", "ai.repair.request") => Some("Requesting AI narration repair."),
        ("materials", "ai.repair.response") => Some("AI narration repair response received."),
        ("materials", "ai.repair.done") => Some("Narration repair completed."),
        ("materials", "ai.repair.parse_failed") => Some("Narration repair response could not be parsed."),
        ("materials", "ai.repair.failed") => Some("Narration repair request failed."),
        ("materials", "ai.repair.local_fallback") => Some("Using local narration repair fallback."),
        ("materials", "ai.repair.out_of_range") => Some("Narration length is still outside target range."),
        ("materials", "subtitle.split") => Some("Subtitles split from narration."),
        ("materials", "generate.done") => Some("Book materials generated."),
        ("materials", "generate.auto_export.done") => Some("Book materials exported."),
        ("materials", "generate.auto_export.failed") => Some("Book materials export failed."),
        ("audio", "generate.start") => Some("Start generating narration audio."),
        ("audio", "generate.plan") => Some("Audio generation plan built."),
        ("audio", "speech.request") => Some("Requesting speech synthesis chunk."),
        ("audio", "speech.response") => Some("Speech synthesis chunk completed."),
        ("audio", "speech.request.failed") => Some("Speech synthesis chunk failed."),
        ("audio", "ffmpeg.concat") => Some("Concatenating audio chunks."),
        ("audio", "ffmpeg.concat.done") => Some("Audio chunks concatenated."),
        ("audio", "duration.probe") => Some("Audio duration probed."),
        ("audio", "generate.done") => Some("Narration audio generated."),
        ("app", "get_app_state") => Some("已读取应用状态。"),
        ("settings", "load") => Some("配置已从数据库加载。"),
        _ => None,
    }
}

