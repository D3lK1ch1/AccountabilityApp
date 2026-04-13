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
            "SELECT app_name, SUM(CASE WHEN            
            end_time IS NULL THEN (strftime('%s', 'now') - start_time)
            ELSE duration_seconds END) 
            as total_duration
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

    pub fn delete_all_sessions(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute("DELETE FROM app_sessions", [])?;

        Ok(())
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
            "SELECT COALESCE (SUM (CASE WHEN end_time IS NULL 
            THEN (strftime('%s', 'now') - start_time) ELSE duration_seconds END), 0)
             FROM app_sessions
             WHERE start_time >= ?1",
            [today_start],
            |row| row.get(0),  
        )?;

        Ok(total)
    }

    pub fn get_tracked_time_per_app(&self, app_name: &str) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let today_start = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        let app: i64 = conn.query_row(
            "SELECT COALESCE (SUM (CASE WHEN end_time IS NULL 
            THEN (strftime('%s', 'now') - start_time) ELSE duration_seconds END), 0)
             FROM app_sessions
             WHERE start_time >= ?1 AND app_name = ?2",
            params! [today_start, app_name],
            |row| row.get(0),  
        )?;

        Ok(app)
    }

        pub fn end_crash_session(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE app_sessions
             SET end_time = ?1, duration_seconds = (?1 - start_time)
             WHERE end_time IS NULL AND start_time <= ?1",
            [now],
        )?;

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = Database::new(temp_dir.path().to_path_buf()).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_insert_and_get_session() {
        let (db, _dir) = create_test_db();

        let session = AppSession {
            id: None,
            app_name: "TestApp".to_string(),
            window_title: Some("Test Window".to_string()),
            start_time: Utc::now().timestamp(),
            end_time: None,
            duration_seconds: 0,
        };

        let id = db.insert_session(&session).unwrap();
        assert!(id > 0);

        let sessions = db.get_sessions_today().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].app_name, "TestApp");
    }

    #[test]
    fn test_update_session_end() {
        let (db, _dir) = create_test_db();

        let session = AppSession {
            id: None,
            app_name: "TestApp".to_string(),
            window_title: None,
            start_time: Utc::now().timestamp(),
            end_time: None,
            duration_seconds: 0,
        };

        let id = db.insert_session(&session).unwrap();
        db.update_session_end(id, 2000, 1000).unwrap();

        let sessions = db.get_sessions_today().unwrap();
        assert_eq!(sessions[0].duration_seconds, 1000);
        assert_eq!(sessions[0].end_time, Some(2000));
    }

    #[test]
    fn test_blocked_apps_crud() {
        let (db, _dir) = create_test_db();

        let app = BlockedApp {
            id: None,
            app_name: "Discord".to_string(),
            block_duration_minutes: 10,
            enabled: true,
        };

        let id = db.add_blocked_app(&app).unwrap();
        assert!(id > 0);

        let apps = db.get_blocked_apps().unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].app_name, "Discord");

        db.remove_blocked_app("Discord").unwrap();

        let apps = db.get_blocked_apps().unwrap();
        assert!(apps.is_empty());
    }

    #[test]
    fn test_settings_crud() {
        let (db, _dir) = create_test_db();

        db.set_setting("theme", "dark").unwrap();
        db.set_setting("language", "en").unwrap();

        assert_eq!(db.get_setting("theme").unwrap(), Some("dark".to_string()));
        assert_eq!(db.get_setting("language").unwrap(), Some("en".to_string()));
        assert_eq!(db.get_setting("nonexistent").unwrap(), None);

        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.get_setting("theme").unwrap(), Some("light".to_string()));
    }

    #[test]
    fn test_app_usage_summary() {
        let (db, _dir) = create_test_db();

        let now = chrono::Utc::now().timestamp();

        let session1 = AppSession {
            id: None,
            app_name: "Chrome".to_string(),
            window_title: None,
            start_time: now,
            end_time: Some(now + 100),
            duration_seconds: 100,
        };
        let session2 = AppSession {
            id: None,
            app_name: "Chrome".to_string(),
            window_title: None,
            start_time: now,
            end_time: Some (now + 200),
            duration_seconds: 200,
        };
        let session3 = AppSession {
            id: None,
            app_name: "VSCode".to_string(),
            window_title: None,
            start_time: now,
            end_time: Some(now + 50),
            duration_seconds: 50,
        };

        db.insert_session(&session1).unwrap();
        db.insert_session(&session2).unwrap();
        db.insert_session(&session3).unwrap();

        let summary = db.get_app_usage_summary().unwrap();
        assert_eq!(summary.len(), 2);

        assert_eq!(summary[0].0, "Chrome");
        assert_eq!(summary[0].1, 300);

        assert_eq!(summary[1].0, "VSCode");
        assert_eq!(summary[1].1, 50);
    }

    #[test]
    fn test_total_tracked_time() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();
        assert_eq!(db.get_total_tracked_time_today().unwrap(), 0);

        let session = AppSession {
            id: None,
            app_name: "Chrome".to_string(),
            window_title: None,
            start_time: now,
            end_time: Some(now + 500),
            duration_seconds: 500,
        };

        db.insert_session(&session).unwrap();

        assert_eq!(db.get_total_tracked_time_today().unwrap(), 500);
    }

    #[test]
    fn test_end_crash_session() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        let session = AppSession {
            id: None,
            app_name: "TestApp".to_string(),
            window_title: None,
            start_time: now - 1000,
            end_time: None,
            duration_seconds: 0,
        };

        db.insert_session(&session).unwrap();
        db.end_crash_session().unwrap();
        let sessions = db.get_sessions_today().unwrap();

        assert_eq!(sessions[0].end_time, Some(now));
        assert_eq!(sessions[0].duration_seconds, 1000);
    }
}
