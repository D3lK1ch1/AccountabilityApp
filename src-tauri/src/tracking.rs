use crate::database::{AppSession, Database};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct ActiveWindowInfo {
    pub app_name: String,
    pub window_title: String,
}

pub struct ActivityTracker {
    db: Arc<Database>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    needs_reset: Arc<std::sync::atomic::AtomicBool>,
    polling_interval_secs: u64,
}

impl ActivityTracker {
    pub fn new(db: Arc<Database>, polling_interval_secs: u64) -> Self {
        Self {
            db,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            needs_reset: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            polling_interval_secs,
        }
    }

    pub fn start(&self, stop_tx: mpsc::Sender<()>) {
        let is_running = self.is_running.clone();
        let needs_reset = self.needs_reset.clone();
        let db = self.db.clone();
        let interval = self.polling_interval_secs;

        is_running.store(true, std::sync::atomic::Ordering::SeqCst);

        std::thread::spawn(move || {
            log::info!("Activity tracker started with {}s interval", interval);

            let mut current_session: Option<(i64, AppSession)> = None;

            while is_running.load(std::sync::atomic::Ordering::SeqCst) {
                let window_info = Self::get_active_window();

                if needs_reset.load(std::sync::atomic::Ordering::SeqCst) {
                    current_session = None;
                    needs_reset.store(false, std::sync::atomic::Ordering::SeqCst);

                }

                match window_info {
                    Some(info) => {
                        if info.app_name != "Accountability App" {
                            if let Some((session_id, session)) = &current_session {
                                if Self::should_start_new_session(session, &info) {
                                    let end_time = Utc::now().timestamp();
                                    let duration = end_time - session.start_time;

                                    if let Err(e) =
                                        db.update_session_end(*session_id, end_time, duration)
                                    {
                                        log::error!("Failed to update session: {}", e);
                                    }

                                    let new_session = AppSession {
                                        id: None,
                                        app_name: info.app_name.clone(),
                                        window_title: Some(info.window_title.clone()),
                                        start_time: Utc::now().timestamp(),
                                        end_time: None,
                                        duration_seconds: 0,
                                    };

                                    match db.insert_app_session(&new_session) {
                                        Ok(new_id) => {
                                            current_session = Some((new_id, new_session));
                                        }
                                        Err(e) => {
                                            log::error!("Failed to insert new session: {}", e);
                                        }
                                    }
                                }
                            } else {
                                let new_session = AppSession {
                                    id: None,
                                    app_name: info.app_name.clone(),
                                    window_title: Some(info.window_title.clone()),
                                    start_time: Utc::now().timestamp(),
                                    end_time: None,
                                    duration_seconds: 0,
                                };

                                match db.insert_app_session(&new_session) {
                                    Ok(id) => {
                                        current_session = Some((id, new_session));
                                    }
                                    Err(e) => {
                                        log::error!("Failed to insert session: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        if let Some((session_id, session)) = current_session.take() {
                            let end_time = Utc::now().timestamp();
                            let duration = end_time - session.start_time;

                            if let Err(e) = db.update_session_end(session_id, end_time, duration) {
                                log::error!("Failed to finalize session: {}", e);
                            }
                        }
                    }
                }

                std::thread::sleep(Duration::from_secs(interval));
            }

            if let Some((session_id, session)) = current_session {
                let end_time = Utc::now().timestamp();
                let duration = end_time - session.start_time;

                if let Err(e) = db.update_session_end(session_id, end_time, duration) {
                    log::error!("Failed to finalize session on stop: {}", e);
                }
            }

            log::info!("Activity tracker stopped");
            let _ = stop_tx.send(());
        });
    }


    pub fn stop(&self) {
        self.is_running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn reset_sessions(&self) {
        self.needs_reset
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn get_active_window() -> Option<ActiveWindowInfo> {
        use active_win_pos_rs::get_active_window;

        match get_active_window() {
            Ok(window) => Some(ActiveWindowInfo {
                app_name: window.app_name,
                window_title: window.title,
            }),
            Err(_e) => {
                log::warn!("Failed to get active window");
                None
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    fn get_active_window() -> Option<ActiveWindowInfo> {
        None
    }

    pub fn get_current_activity(&self) -> Option<ActiveWindowInfo> {
        Self::get_active_window()
    }

    fn should_start_new_session(session: &AppSession, info: &ActiveWindowInfo) -> bool {
        session.app_name != info.app_name
            || session.window_title.as_deref() != Some(info.window_title.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(app_name: &str, window_title: Option<&str>) -> AppSession {
        AppSession {
            id: Some(1),
            app_name: app_name.to_string(),
            window_title: window_title.map(str::to_string),
            start_time: 100,
            end_time: None,
            duration_seconds: 0,
        }
    }

    fn info(app_name: &str, window_title: &str) -> ActiveWindowInfo {
        ActiveWindowInfo {
            app_name: app_name.to_string(),
            window_title: window_title.to_string(),
        }
    }

    #[test]
    fn starts_new_session_when_app_changes() {
        assert!(ActivityTracker::should_start_new_session(
            &session("Chrome", Some("Docs")),
            &info("Code", "Docs")
        ));
    }

    #[test]
    fn starts_new_session_when_title_changes() {
        assert!(ActivityTracker::should_start_new_session(
            &session("Chrome", Some("Docs")),
            &info("Chrome", "Mail")
        ));
    }

    #[test]
    fn keeps_session_when_app_and_title_match() {
        assert!(!ActivityTracker::should_start_new_session(
            &session("Chrome", Some("Docs")),
            &info("Chrome", "Docs")
        ));
    }
}
