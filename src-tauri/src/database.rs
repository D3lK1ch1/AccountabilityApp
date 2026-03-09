use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Lock error")]
    Lock,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type DbResult<T> = Result<T, DatabaseError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSession {
    pub id: Option<i64>,
    pub app_name: String,
    pub window_title: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedApp {
    pub id: Option<i64>,
    pub app_name: String,
    pub block_duration_minutes: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> DbResult<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        let db_path = app_data_dir.join("accountability.db");

        log::info!("Opening database at: {:?}", db_path);

        let conn = Connection::open(&db_path)?;
        let db = Database {
            conn: Mutex::new(conn),
        };

        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name TEXT NOT NULL,
                window_title TEXT,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration_seconds INTEGER DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocked_apps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name TEXT NOT NULL UNIQUE,
                block_duration_minutes INTEGER DEFAULT 5,
                enabled BOOLEAN DEFAULT 1
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON app_sessions(start_time)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_app_name ON app_sessions(app_name)",
            [],
        )?;

        log::info!("Database tables initialized");
        Ok(())
    }

    pub fn insert_session(&self, session: &AppSession) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "INSERT INTO app_sessions (app_name, window_title, start_time, end_time, duration_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session.app_name,
                session.window_title,
                session.start_time,
                session.end_time,
                session.duration_seconds,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update_session_end(
        &self,
        session_id: i64,
        end_time: i64,
        duration: i64,
    ) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "UPDATE app_sessions SET end_time = ?1, duration_seconds = ?2 WHERE id = ?3",
            params![end_time, duration, session_id],
        )?;

        Ok(())
    }

    pub fn get_sessions_today(&self) -> DbResult<Vec<AppSession>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        let mut stmt = conn.prepare(
            "SELECT id, app_name, window_title, start_time, end_time, duration_seconds
             FROM app_sessions
             WHERE start_time >= ?1
             ORDER BY start_time DESC",
        )?;

        let sessions = stmt
            .query_map([today_start], |row| {
                Ok(AppSession {
                    id: Some(row.get(0)?),
                    app_name: row.get(1)?,
                    window_title: row.get(2)?,
                    start_time: row.get(3)?,
                    end_time: row.get(4)?,
                    duration_seconds: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn get_app_usage_summary(&self) -> DbResult<Vec<(String, i64)>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        let mut stmt = conn.prepare(
            "SELECT app_name, SUM(duration_seconds) as total_duration
             FROM app_sessions
             WHERE start_time >= ?1
             GROUP BY app_name
             ORDER BY total_duration DESC",
        )?;

        let summary = stmt
            .query_map([today_start], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(summary)
    }

    pub fn get_total_tracked_time_today(&self) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0) FROM app_sessions WHERE start_time >= ?1",
            [today_start],
            |row| row.get(0),
        )?;

        Ok(total)
    }

    pub fn add_blocked_app(&self, app: &BlockedApp) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "INSERT OR REPLACE INTO blocked_apps (app_name, block_duration_minutes, enabled)
             VALUES (?1, ?2, ?3)",
            params![app.app_name, app.block_duration_minutes, app.enabled],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_blocked_apps(&self) -> DbResult<Vec<BlockedApp>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let mut stmt =
            conn.prepare("SELECT id, app_name, block_duration_minutes, enabled FROM blocked_apps")?;

        let apps = stmt
            .query_map([], |row| {
                Ok(BlockedApp {
                    id: Some(row.get(0)?),
                    app_name: row.get(1)?,
                    block_duration_minutes: row.get(2)?,
                    enabled: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(apps)
    }

    pub fn remove_blocked_app(&self, app_name: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute("DELETE FROM blocked_apps WHERE app_name = ?1", [app_name])?;

        Ok(())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;

        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> DbResult<Option<String>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let result = conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        });

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
