use crate::models::*;
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("lock poisoned")]
    Lock,
}

pub type AppResult<T> = Result<T, AppError>;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_dir: &Path) -> AppResult<Self> {
        let path = app_dir.join("app.db");
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              created_at TEXT DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS teams (
              id TEXT PRIMARY KEY,
              json TEXT NOT NULL,
              updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS matches (
              id TEXT PRIMARY KEY,
              json TEXT NOT NULL,
              updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS favorite_teams (
              team_id TEXT PRIMARY KEY,
              created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS predictions (
              match_id TEXT PRIMARY KEY,
              json TEXT NOT NULL,
              updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS data_cache (
              source TEXT PRIMARY KEY,
              payload TEXT NOT NULL,
              fetched_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS license_cache (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.seed_defaults()?;
        Ok(db)
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let value = self.get_setting("app_settings")?;
        Ok(value
            .map(|json| serde_json::from_str(&json))
            .transpose()?
            .unwrap_or_else(default_settings))
    }

    pub fn set_settings(&self, settings: &AppSettings) -> AppResult<AppSettings> {
        self.set_setting("app_settings", &serde_json::to_string(settings)?)?;
        Ok(settings.clone())
    }

    pub fn get_predictions(&self) -> AppResult<Vec<Prediction>> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        let mut stmt = conn.prepare("SELECT json FROM predictions ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut items: Vec<Prediction> = Vec::new();
        for row in rows {
            items.push(serde_json::from_str(&row?)?);
        }
        Ok(items)
    }

    pub fn save_prediction(&self, prediction: &Prediction) -> AppResult<Vec<Prediction>> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        conn.execute(
            "INSERT INTO predictions(match_id, json, updated_at) VALUES(?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(match_id) DO UPDATE SET json=excluded.json, updated_at=CURRENT_TIMESTAMP",
            params![prediction.match_id, serde_json::to_string(prediction)?],
        )?;
        drop(conn);
        self.get_predictions()
    }

    pub fn toggle_favorite(&self, team_id: &str) -> AppResult<Vec<String>> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorite_teams WHERE team_id=?1)",
            [team_id],
            |row| row.get(0),
        )?;
        if exists {
            conn.execute("DELETE FROM favorite_teams WHERE team_id=?1", [team_id])?;
        } else {
            conn.execute("INSERT OR IGNORE INTO favorite_teams(team_id) VALUES(?1)", [team_id])?;
        }
        let mut stmt = conn.prepare("SELECT team_id FROM favorite_teams ORDER BY created_at")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    pub fn cache_matches(&self, source: &str, payload: &str, matches: &[Match]) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        let fetched_at = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO data_cache(source, payload, fetched_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(source) DO UPDATE SET payload=excluded.payload, fetched_at=excluded.fetched_at",
            params![source, payload, fetched_at],
        )?;
        for item in matches {
            conn.execute(
                "INSERT INTO matches(id, json, updated_at) VALUES(?1, ?2, CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET json=excluded.json, updated_at=CURRENT_TIMESTAMP",
                params![item.id, serde_json::to_string(item)?],
            )?;
        }
        Ok(())
    }

    pub fn get_cached_matches(&self) -> AppResult<Vec<Match>> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        let mut stmt = conn.prepare("SELECT json FROM matches")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut items: Vec<Match> = Vec::new();
        for row in rows {
            items.push(serde_json::from_str(&row?)?);
        }
        items.sort_by(|a, b| (a.date.as_str(), a.time.as_str(), a.id.as_str()).cmp(&(b.date.as_str(), b.time.as_str(), b.id.as_str())));
        Ok(items)
    }

    pub fn get_latest_refresh_label(&self) -> AppResult<Option<String>> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        let result = conn.query_row(
            "SELECT fetched_at FROM data_cache ORDER BY fetched_at DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(value) => Ok(format_beijing_minute_from_rfc3339(&value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn seed_defaults(&self) -> AppResult<()> {
        if self.get_setting("app_settings")?.is_none() {
            self.set_settings(&default_settings())?;
        }
        Ok(())
    }

    fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        let result = conn.query_row("SELECT value FROM settings WHERE key=?1", [key], |row| row.get(0));
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let conn = self.conn.lock().map_err(|_| AppError::Lock)?;
        conn.execute(
            "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=CURRENT_TIMESTAMP",
            params![key, value],
        )?;
        Ok(())
    }
}

pub fn default_settings() -> AppSettings {
    AppSettings::default()
}

pub fn default_license() -> LicenseState {
    LicenseState {
        status: "trial".to_string(),
        remaining_days: 30,
        expires_at: "2026-07-16".to_string(),
    }
}

fn format_beijing_minute_from_rfc3339(value: &str) -> Option<String> {
    let instant = chrono::DateTime::parse_from_rfc3339(value).ok()?;
    let beijing = instant.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600)?);
    Some(beijing.format("%H:%M").to_string())
}
