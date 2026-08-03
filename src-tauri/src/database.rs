use chrono::{Local, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};
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
    #[error("Keyring error: {0}")]
    KeyringError(String),

}

pub type DbResult<T> = Result<T, DatabaseError>;

pub fn today_start_timestamp() -> i64 {
    let local_midnight = Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
    match Local.from_local_datetime(&local_midnight) {
        chrono::LocalResult::Single(dt) => dt.timestamp(),
        chrono::LocalResult::Ambiguous(dt, _) => dt.timestamp(),
        chrono::LocalResult::None => Utc::now().timestamp(),
    }
}

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
pub struct TabSession {
    pub id: Option<i64>,
    pub source: String,
    pub tab_url: String,
    pub tab_title: String,
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
pub struct BlockCategory {
    pub id: Option<i64>,
    pub name: String,
    pub daily_limit_minutes: i32,
    pub enabled: bool,
    pub manual_block_paused: bool,
    pub domain_keywords: Vec<String>,
    pub app_keywords: Vec<String>,
    pub display_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUsage {
    pub category_id: i64,
    pub category_name: String,
    pub used_seconds: i64,
    pub limit_seconds: i64,
    pub limit_exceeded: bool,
    pub enabled: bool,
    pub manual_block_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDecision {
    pub blocked: bool,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub used_seconds: i64,
    pub limit_seconds: i64,
    pub deterrent_mode: String,
    pub popup_interval_ms: i32,
}
pub struct Database {
    conn: Mutex<Connection>,
}

fn get_or_create_key(db_path: &std::path::Path) -> Result<String, DatabaseError> {
    let entry = keyring::Entry::new("accountability_app", "db_key")
    .map_err(|e| DatabaseError::KeyringError(e.to_string()))?;

    match entry.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => {
            if db_path.exists(){
                let timestamp = Utc::now().timestamp();
                let backup_path = db_path.with_extension(format!("db.unreadable.{}", timestamp));
                std::fs::rename(db_path, &backup_path).map_err(DatabaseError::Io)?;
                log::warn!("Encryption key missing - existing database moved to {:?}", backup_path);
            }
            let raw : [u8; 32] = rand::random();
            let key = hex::encode(raw);
            entry.set_password(&key).map_err(|e| DatabaseError::KeyringError(e.to_string()))?;
            Ok(key)
        }
        Err(e) => Err(DatabaseError::KeyringError(e.to_string())),
    }
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> DbResult<Self> {
        std::fs::create_dir_all(&app_data_dir)?;
        let db_path = app_data_dir.join("accountability.db");

        log::info!("Opening database at: {:?}", db_path);

        let db_key = get_or_create_key(&db_path)?;
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", db_key))?;
        let db = Database {
            conn: Mutex::new(conn),
        };

        db.initialize()?;
        db.seed_default_block_categories()?;
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
            "CREATE TABLE IF NOT EXISTS tab_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                tab_url TEXT NOT NULL,
                tab_title TEXT NOT NULL,
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

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tab_sessions_start_time ON tab_sessions(start_time)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tab_sessions_source ON tab_sessions(source)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS block_categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                daily_limit_minutes INTEGER NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                manual_block_paused BOOLEAN DEFAULT 0,
                domain_keywords TEXT NOT NULL,
                app_keywords TEXT NOT NULL,
                display_order INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        log::info!("Database tables initialized");
        Ok(())
    }

    fn seed_default_block_categories(&self) -> DbResult<()> {
        if !self.get_block_categories()?.is_empty() {
            return Ok(());
        }

        let defaults = [
            BlockCategory {
                id: None,
                name: "Social Media".to_string(),
                daily_limit_minutes: 60,
                enabled: true,
                manual_block_paused: false,
                domain_keywords: vec![
                    "x.com",
                    "twitter.com",
                    "instagram.com",
                    "facebook.com",
                    "tiktok.com",
                    "reddit.com",
                    "threads.net",
                    "snapchat.com",
                    "linkedin.com",
                    "pinterest.com",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
                app_keywords: vec![
                    "discord",
                    "instagram",
                    "facebook",
                    "tiktok",
                    "twitter",
                    "reddit",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
                display_order: 0,
            },
            BlockCategory {
                id: None,
                name: "Games".to_string(),
                daily_limit_minutes: 60,
                enabled: true,
                manual_block_paused: false,
                domain_keywords: vec![
                    "steampowered.com",
                    "steamcommunity.com",
                    "epicgames.com",
                    "itch.io",
                    "roblox.com",
                    "minecraft.net",
                    "battle.net",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
                app_keywords: vec![
                    "steam",
                    "epic games",
                    "battle.net",
                    "riot client",
                    "roblox",
                    "minecraft",
                    "game",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
                display_order: 1,
            },
        ];

        for category in defaults {
            self.upsert_block_category(&category)?;
        }

        Ok(())
    }

    pub fn insert_app_session(&self, session: &AppSession) -> DbResult<i64> {
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
        self.get_sessions_since(today_start_timestamp())
    }

    pub fn get_sessions_since(&self, since: i64) -> DbResult<Vec<AppSession>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, app_name, window_title, start_time, end_time, duration_seconds
             FROM app_sessions
             WHERE start_time >= ?1
             ORDER BY start_time DESC",
        )?;

        let sessions = stmt
            .query_map([since], |row| {
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

    pub fn insert_tab_session(&self, session: &TabSession) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "INSERT INTO tab_sessions (source, tab_url, tab_title, start_time, end_time, duration_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.source,
                session.tab_url,
                session.tab_title,
                session.start_time,
                session.end_time,
                session.duration_seconds,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn close_tab_session(
        &self,
        session_id: i64,
        end_time: i64,
        duration: i64,
    ) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "UPDATE tab_sessions SET end_time = ?1, duration_seconds = ?2 WHERE id = ?3",
            params![end_time, duration, session_id],
        )?;

        Ok(())
    }

    pub fn get_tab_sessions_today(&self) -> DbResult<Vec<TabSession>> {
        self.get_tab_sessions_since(today_start_timestamp())
    }

    pub fn get_tab_sessions_since(&self, since: i64) -> DbResult<Vec<TabSession>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, source, tab_url, tab_title, start_time, end_time, duration_seconds
             FROM tab_sessions
             WHERE start_time >= ?1
             ORDER BY start_time DESC",
        )?;

        let sessions = stmt
            .query_map([since], |row| {
                Ok(TabSession {
                    id: Some(row.get(0)?),
                    source: row.get(1)?,
                    tab_url: row.get(2)?,
                    tab_title: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                    duration_seconds: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn delete_all_tab_sessions(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute("DELETE FROM tab_sessions", [])?;

        Ok(())
    }

    pub fn get_open_tab_session_for_source(
        &self,
        source: &str,
    ) -> DbResult<Option<(i64, i64)>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let result = conn
            .query_row(
                "SELECT id, start_time FROM tab_sessions
                 WHERE source = ?1 AND end_time IS NULL
                 ORDER BY start_time DESC LIMIT 1",
                [source],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;

        Ok(result)
    }

    pub fn finalize_open_tab_sessions(&self, source: &str, now: i64) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        conn.execute(
            "UPDATE tab_sessions
             SET end_time = ?1, duration_seconds = (?1 - start_time)
             WHERE source = ?2 AND end_time IS NULL",
            params![now, source],
        )?;

        Ok(())
    }

    pub fn end_crash_tab_sessions(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE tab_sessions
             SET end_time = ?1, duration_seconds = (?1 - start_time)
             WHERE end_time IS NULL AND start_time <= ?1",
            [now],
        )?;

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

    pub fn get_block_categories(&self) -> DbResult<Vec<BlockCategory>> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;

        let mut stmt = conn.prepare(
            "SELECT id, name, daily_limit_minutes, enabled, manual_block_paused,
                domain_keywords, app_keywords, display_order
             FROM block_categories
             ORDER BY display_order ASC, name ASC",
        )?;

        let categories = stmt
            .query_map([], |row| {
                let domain_keywords: String = row.get(5)?;
                let app_keywords: String = row.get(6)?;
                Ok(BlockCategory {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    daily_limit_minutes: row.get(2)?,
                    enabled: row.get(3)?,
                    manual_block_paused: row.get(4)?,
                    domain_keywords: serde_json::from_str(&domain_keywords).unwrap_or_default(),
                    app_keywords: serde_json::from_str(&app_keywords).unwrap_or_default(),
                    display_order: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(categories)
    }

    pub fn upsert_block_category(&self, category: &BlockCategory) -> DbResult<i64> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;
        let domain_keywords = serde_json::to_string(&category.domain_keywords)
            .unwrap_or_else(|_| "[]".to_string());
        let app_keywords =
            serde_json::to_string(&category.app_keywords).unwrap_or_else(|_| "[]".to_string());

        if let Some(id) = category.id {
            conn.execute(
                "UPDATE block_categories
                 SET name = ?1, daily_limit_minutes = ?2, enabled = ?3,
                    manual_block_paused = ?4, domain_keywords = ?5, app_keywords = ?6,
                    display_order = ?7
                 WHERE id = ?8",
                params![
                    category.name,
                    category.daily_limit_minutes,
                    category.enabled,
                    category.manual_block_paused,
                    domain_keywords,
                    app_keywords,
                    category.display_order,
                    id,
                ],
            )?;
            Ok(id)
        } else {
            conn.execute(
                "INSERT INTO block_categories
                    (name, daily_limit_minutes, enabled, manual_block_paused,
                     domain_keywords, app_keywords, display_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(name) DO UPDATE SET
                    daily_limit_minutes = excluded.daily_limit_minutes,
                    enabled = excluded.enabled,
                    manual_block_paused = excluded.manual_block_paused,
                    domain_keywords = excluded.domain_keywords,
                    app_keywords = excluded.app_keywords,
                    display_order = excluded.display_order",
                params![
                    category.name,
                    category.daily_limit_minutes,
                    category.enabled,
                    category.manual_block_paused,
                    domain_keywords,
                    app_keywords,
                    category.display_order,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        }
    }

    pub fn set_block_category_enabled(&self, category_id: i64, enabled: bool) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;
        conn.execute(
            "UPDATE block_categories SET enabled = ?1 WHERE id = ?2",
            params![enabled, category_id],
        )?;
        Ok(())
    }

    pub fn set_block_category_paused(&self, category_id: i64, paused: bool) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DatabaseError::Lock)?;
        conn.execute(
            "UPDATE block_categories SET manual_block_paused = ?1 WHERE id = ?2",
            params![paused, category_id],
        )?;
        Ok(())
    }

    pub fn get_category_usage_today(&self) -> DbResult<Vec<CategoryUsage>> {
        let categories = self.get_block_categories()?;
        let app_sessions = self.get_sessions_today()?;
        let tab_sessions = self.get_tab_sessions_today()?;
        let now = Utc::now().timestamp();

        let usages = categories
            .iter()
            .filter_map(|category| {
                category.id.map(|category_id| {
                    let app_seconds = app_sessions
                        .iter()
                        .filter(|session| app_matches_category(session, category))
                        .map(|session| session_seconds(session.start_time, session.end_time, session.duration_seconds, now))
                        .sum::<i64>();
                    let tab_seconds = tab_sessions
                        .iter()
                        .filter(|session| tab_matches_category(session, category))
                        .map(|session| session_seconds(session.start_time, session.end_time, session.duration_seconds, now))
                        .sum::<i64>();
                    let used_seconds = app_seconds + tab_seconds;
                    let limit_seconds = i64::from(category.daily_limit_minutes.max(0)) * 60;

                    CategoryUsage {
                        category_id,
                        category_name: category.name.clone(),
                        used_seconds,
                        limit_seconds,
                        limit_exceeded: limit_seconds > 0 && used_seconds >= limit_seconds,
                        enabled: category.enabled,
                        manual_block_paused: category.manual_block_paused,
                    }
                })
            })
            .collect();

        Ok(usages)
    }

    pub fn evaluate_tab_block(&self, tab_url: &str, tab_title: &str) -> DbResult<BlockDecision> {
        let categories = self.get_block_categories()?;
        let usages = self.get_category_usage_today()?;
        let probe = TabSession {
            id: None,
            source: "probe".to_string(),
            tab_url: tab_url.to_string(),
            tab_title: tab_title.to_string(),
            start_time: Utc::now().timestamp(),
            end_time: None,
            duration_seconds: 0,
        };

        for category in categories {
            if !category.enabled || category.manual_block_paused {
                continue;
            }
            if !tab_matches_category(&probe, &category) {
                continue;
            }
            if let Some(category_id) = category.id {
                if let Some(usage) = usages.iter().find(|u| u.category_id == category_id) {
                    if usage.limit_exceeded {
                        return Ok(BlockDecision {
                            blocked: true,
                            category_id: Some(category_id),
                            category_name: Some(category.name),
                            used_seconds: usage.used_seconds,
                            limit_seconds: usage.limit_seconds,
                            deterrent_mode: "rotating_mix".to_string(),
                            popup_interval_ms: 5000,
                        });
                    }
                }
            }
        }

        Ok(BlockDecision {
            blocked: false,
            category_id: None,
            category_name: None,
            used_seconds: 0,
            limit_seconds: 0,
            deterrent_mode: "none".to_string(),
            popup_interval_ms: 0,
        })
    }
}

fn session_seconds(start_time: i64, end_time: Option<i64>, duration_seconds: i64, now: i64) -> i64 {
    match end_time {
        Some(_) => duration_seconds.max(0),
        None => (now - start_time).max(0),
    }
}

fn app_matches_category(session: &AppSession, category: &BlockCategory) -> bool {
    let mut haystack = session.app_name.to_lowercase();
    if let Some(title) = &session.window_title {
        haystack.push(' ');
        haystack.push_str(&title.to_lowercase());
    }
    contains_keyword(&haystack, &category.app_keywords)
}

fn tab_matches_category(session: &TabSession, category: &BlockCategory) -> bool {
    let title = session.tab_title.to_lowercase();
    let url = session.tab_url.to_lowercase();
    let host = host_from_url(&url);
    contains_domain_keyword(host.as_deref(), &category.domain_keywords)
        || contains_keyword(&title, &category.domain_keywords)
        || contains_keyword(&url, &category.domain_keywords)
}

fn contains_keyword(haystack: &str, keywords: &[String]) -> bool {
    keywords
        .iter()
        .map(|keyword| keyword.trim().to_lowercase())
        .filter(|keyword| !keyword.is_empty())
        .any(|keyword| haystack.contains(&keyword))
}

fn contains_domain_keyword(host: Option<&str>, keywords: &[String]) -> bool {
    let Some(host) = host else {
        return false;
    };
    keywords
        .iter()
        .map(|keyword| keyword.trim().trim_start_matches("*.").to_lowercase())
        .filter(|keyword| !keyword.is_empty())
        .any(|keyword| host == keyword || host.ends_with(&format!(".{}", keyword)))
}

fn host_from_url(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches("www.");

    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let id = db.insert_app_session(&session).unwrap();
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

        let id = db.insert_app_session(&session).unwrap();
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

        db.insert_app_session(&session1).unwrap();
        db.insert_app_session(&session2).unwrap();
        db.insert_app_session(&session3).unwrap();

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

        db.insert_app_session(&session).unwrap();

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

        db.insert_app_session(&session).unwrap();
        db.end_crash_session().unwrap();
        let sessions = db.get_sessions_today().unwrap();
        let ended_at = sessions[0].end_time.unwrap();

        assert!(ended_at >= now);
        assert!(ended_at <= chrono::Utc::now().timestamp());
        assert_eq!(sessions[0].duration_seconds, ended_at - session.start_time);
    }

    #[test]
    fn test_insert_close_and_get_tab_session() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        let session = TabSession {
            id: None,
            source: "chrome".to_string(),
            tab_url: "https://example.com".to_string(),
            tab_title: "Example".to_string(),
            start_time: now,
            end_time: None,
            duration_seconds: 0,
        };

        let id = db.insert_tab_session(&session).unwrap();
        let sessions = db.get_tab_sessions_today().unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].source, "chrome");
        assert_eq!(sessions[0].tab_title, "Example");

        db.close_tab_session(id, now + 30, 30).unwrap();
        let sessions = db.get_tab_sessions_today().unwrap();

        assert_eq!(sessions[0].end_time, Some(now + 30));
        assert_eq!(sessions[0].duration_seconds, 30);
    }

    #[test]
    fn test_delete_all_tab_sessions() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        let session = TabSession {
            id: None,
            source: "vscode".to_string(),
            tab_url: "file:///workspace/main.rs".to_string(),
            tab_title: "main.rs".to_string(),
            start_time: now,
            end_time: None,
            duration_seconds: 0,
        };

        db.insert_tab_session(&session).unwrap();
        db.delete_all_tab_sessions().unwrap();

        assert!(db.get_tab_sessions_today().unwrap().is_empty());
    }

    #[test]
    fn test_finalize_open_tab_sessions_by_source() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        let chrome_session = TabSession {
            id: None,
            source: "chrome".to_string(),
            tab_url: "https://example.com".to_string(),
            tab_title: "Example".to_string(),
            start_time: now,
            end_time: None,
            duration_seconds: 0,
        };
        let vscode_session = TabSession {
            id: None,
            source: "vscode".to_string(),
            tab_url: "file:///main.rs".to_string(),
            tab_title: "main.rs".to_string(),
            start_time: now,
            end_time: None,
            duration_seconds: 0,
        };

        db.insert_tab_session(&chrome_session).unwrap();
        db.insert_tab_session(&chrome_session).unwrap();
        db.insert_tab_session(&vscode_session).unwrap();

        db.finalize_open_tab_sessions("chrome", now + 60).unwrap();

        let sessions = db.get_tab_sessions_today().unwrap();
        let chrome_sessions: Vec<_> = sessions.iter().filter(|s| s.source == "chrome").collect();
        let vscode_sessions: Vec<_> = sessions.iter().filter(|s| s.source == "vscode").collect();

        assert!(chrome_sessions.iter().all(|s| s.end_time.is_some()));
        assert!(vscode_sessions.iter().all(|s| s.end_time.is_none()));
    }

    #[test]
    fn test_end_crash_tab_sessions() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        for source in &["chrome", "vscode"] {
            db.insert_tab_session(&TabSession {
                id: None,
                source: source.to_string(),
                tab_url: "https://example.com".to_string(),
                tab_title: "Example".to_string(),
                start_time: now,
                end_time: None,
                duration_seconds: 0,
            })
            .unwrap();
        }

        db.end_crash_tab_sessions().unwrap();

        let sessions = db.get_tab_sessions_today().unwrap();
        assert!(sessions.iter().all(|s| s.end_time.is_some()));
        assert!(sessions.iter().all(|s| s.duration_seconds >= 0));
    }

    #[test]
    fn test_source_isolation() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        db.insert_tab_session(&TabSession {
            id: None,
            source: "chrome".to_string(),
            tab_url: "https://example.com".to_string(),
            tab_title: "Chrome Tab".to_string(),
            start_time: now,
            end_time: None,
            duration_seconds: 0,
        })
        .unwrap();

        db.insert_tab_session(&TabSession {
            id: None,
            source: "vscode".to_string(),
            tab_url: "file:///main.rs".to_string(),
            tab_title: "main.rs".to_string(),
            start_time: now,
            end_time: None,
            duration_seconds: 0,
        })
        .unwrap();

        let chrome_open = db.get_open_tab_session_for_source("chrome").unwrap();
        let vscode_open = db.get_open_tab_session_for_source("vscode").unwrap();

        assert!(chrome_open.is_some());
        assert!(vscode_open.is_some());

        // closing chrome does not affect vscode
        let (chrome_id, _) = chrome_open.unwrap();
        db.close_tab_session(chrome_id, now + 10, 10).unwrap();

        assert!(db.get_open_tab_session_for_source("chrome").unwrap().is_none());
        assert!(db.get_open_tab_session_for_source("vscode").unwrap().is_some());
    }

    #[test]
    fn test_default_block_categories_seeded() {
        let (db, _dir) = create_test_db();
        let categories = db.get_block_categories().unwrap();

        assert!(categories.iter().any(|c| c.name == "Social Media"));
        assert!(categories.iter().any(|c| c.name == "Games"));
    }

    #[test]
    fn test_category_usage_counts_tabs_and_apps() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();

        db.insert_tab_session(&TabSession {
            id: None,
            source: "chrome".to_string(),
            tab_url: "https://instagram.com/direct".to_string(),
            tab_title: "Instagram".to_string(),
            start_time: now,
            end_time: Some(now + 120),
            duration_seconds: 120,
        })
        .unwrap();
        db.insert_app_session(&AppSession {
            id: None,
            app_name: "Discord".to_string(),
            window_title: Some("Friends".to_string()),
            start_time: now,
            end_time: Some(now + 60),
            duration_seconds: 60,
        })
        .unwrap();

        let usages = db.get_category_usage_today().unwrap();
        let social = usages
            .iter()
            .find(|usage| usage.category_name == "Social Media")
            .unwrap();

        assert_eq!(social.used_seconds, 180);
    }

    #[test]
    fn test_evaluate_tab_block_after_limit_exceeded() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();
        let mut social = db
            .get_block_categories()
            .unwrap()
            .into_iter()
            .find(|category| category.name == "Social Media")
            .unwrap();
        social.daily_limit_minutes = 1;
        db.upsert_block_category(&social).unwrap();

        db.insert_tab_session(&TabSession {
            id: None,
            source: "chrome".to_string(),
            tab_url: "https://x.com/home".to_string(),
            tab_title: "X".to_string(),
            start_time: now,
            end_time: Some(now + 60),
            duration_seconds: 60,
        })
        .unwrap();

        let decision = db
            .evaluate_tab_block("https://instagram.com/reels", "Instagram")
            .unwrap();

        assert!(decision.blocked);
        assert_eq!(decision.category_name, Some("Social Media".to_string()));
    }

    #[test]
    fn test_evaluate_tab_block_respects_manual_pause() {
        let (db, _dir) = create_test_db();
        let now = chrono::Utc::now().timestamp();
        let mut social = db
            .get_block_categories()
            .unwrap()
            .into_iter()
            .find(|category| category.name == "Social Media")
            .unwrap();
        social.daily_limit_minutes = 1;
        db.upsert_block_category(&social).unwrap();
        db.set_block_category_paused(social.id.unwrap(), true).unwrap();

        db.insert_tab_session(&TabSession {
            id: None,
            source: "chrome".to_string(),
            tab_url: "https://reddit.com/r/rust".to_string(),
            tab_title: "Reddit".to_string(),
            start_time: now,
            end_time: Some(now + 120),
            duration_seconds: 120,
        })
        .unwrap();

        let decision = db
            .evaluate_tab_block("https://reddit.com/r/games", "Reddit")
            .unwrap();

        assert!(!decision.blocked);
    }

    #[test]
    fn test_domain_keyword_matches_subdomain() {
        assert!(contains_domain_keyword(
            Some("mobile.twitter.com"),
            &[String::from("twitter.com")]
        ));
        assert!(!contains_domain_keyword(
            Some("notwitter.com"),
            &[String::from("twitter.com")]
        ));
    }
}
