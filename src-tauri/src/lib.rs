use crate::database::{AppSession, BlockedApp, Database};
use crate::models::{DashboardStats, TrackerStatus, UsageData};
use crate::tracking::ActivityTracker;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WindowEvent,
};
use tokio::sync::Mutex;

mod database;
mod models;
mod tracking;

struct AppState {
    db: Arc<Database>,
    tracker: Arc<Mutex<Option<ActivityTracker>>>,
    stop_tracker_tx: Arc<Mutex<Option<tokio::sync::mpsc::Sender<()>>>>,
}

#[tauri::command]
async fn start_tracking(state: State<'_, AppState>) -> Result<(), String> {
    let tracker = ActivityTracker::new(state.db.clone(), 3);
    let (tx, _rx) = tokio::sync::mpsc::channel::<()>(1);
    
    {
        let mut stop_tx = state.stop_tracker_tx.lock().await;
        *stop_tx = Some(tx.clone());
    }

    tracker.start(tx);

    let mut tracker_lock = state.tracker.lock().await;
    *tracker_lock = Some(tracker);

    log::info!("Tracking started");
    Ok(())
}

#[tauri::command]
async fn stop_tracking(state: State<'_, AppState>) -> Result<(), String> {
    let mut tracker_lock = state.tracker.lock().await;
    if let Some(tracker) = tracker_lock.take() {
        tracker.stop();
    }
    
    let mut stop_tx = state.stop_tracker_tx.lock().await;
    *stop_tx = None;
    
    log::info!("Tracking stopped");
    Ok(())
}

#[tauri::command]
async fn get_tracker_status(state: State<'_, AppState>) -> Result<TrackerStatus, String> {
    let tracker_lock = state.tracker.lock().await;
    
    if let Some(tracker) = tracker_lock.as_ref() {
        let is_tracking = tracker.is_running();
        let current = tracker.get_current_activity();
        
        Ok(TrackerStatus {
            is_tracking,
            current_app: current.as_ref().map(|c| c.app_name.clone()),
            current_window_title: current.as_ref().map(|c| c.window_title.clone()),
        })
    } else {
        Ok(TrackerStatus {
            is_tracking: false,
            current_app: None,
            current_window_title: None,
        })
    }
}

#[tauri::command]
async fn get_dashboard_stats(state: State<'_, AppState>) -> Result<DashboardStats, String> {
    let db = state.db.clone();
    
    let sessions = db.get_sessions_today().map_err(|e| e.to_string())?;
    let total = db.get_total_tracked_time_today().map_err(|e| e.to_string())?;
    let summary = db.get_app_usage_summary().map_err(|e| e.to_string())?;
    
    let usage_by_app: Vec<UsageData> = summary
        .into_iter()
        .map(|(app_name, total_seconds)| {
            let percentage = if total > 0 {
                (total_seconds as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            UsageData {
                app_name,
                total_seconds,
                percentage,
            }
        })
        .collect();

    let most_used_app = usage_by_app.first().map(|u| u.app_name.clone());

    Ok(DashboardStats {
        total_tracked_seconds: total,
        most_used_app,
        usage_by_app,
        sessions_count: sessions.len(),
    })
}

#[tauri::command]
async fn get_sessions_today(state: State<'_, AppState>) -> Result<Vec<AppSession>, String> {
    state.db.get_sessions_today().map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_blocked_app(
    state: State<'_, AppState>,
    app_name: String,
    block_duration_minutes: i32,
) -> Result<i64, String> {
    let app = BlockedApp {
        id: None,
        app_name,
        block_duration_minutes,
        enabled: true,
    };
    state.db.add_blocked_app(&app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_blocked_apps(state: State<'_, AppState>) -> Result<Vec<BlockedApp>, String> {
    state.db.get_blocked_apps().map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_blocked_app(state: State<'_, AppState>, app_name: String) -> Result<(), String> {
    state.db.remove_blocked_app(&app_name).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state.db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn show_dashboard(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn hide_to_tray(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Hide to Tray", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    let _ = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Accountability App")
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn setup_global_shortcut(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA);
    
    app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, _event| {
        if let Some(window) = app.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    })?;

    log::info!("Global shortcut Ctrl+Shift+A registered");
    Ok(())
}

fn position_window_bottom_right(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(monitor) = window.primary_monitor() {
            if let Some(monitor) = monitor {
                let screen_size = monitor.size();
                let window_size = window.outer_size().unwrap_or(tauri::PhysicalSize::new(400, 300));
                
                let x = screen_size.width.saturating_sub(window_size.width + 20);
                let y = screen_size.height.saturating_sub(window_size.height.saturating_add(40));
                
                let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("Starting Accountability App");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            log::info!("App data directory: {:?}", app_data_dir);

            let db = Arc::new(Database::new(app_data_dir).expect("Failed to initialize database"));
            
            app.manage(AppState {
                db: db.clone(),
                tracker: Arc::new(Mutex::new(None)),
                stop_tracker_tx: Arc::new(Mutex::new(None)),
            });

            setup_tray(app.handle())?;
            setup_global_shortcut(app.handle())?;
            
            position_window_bottom_right(app.handle());

            let main_window = app.get_webview_window("main").unwrap();
            let app_handle = app.handle().clone();
            
            main_window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            });

            log::info!("App setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_tracking,
            stop_tracking,
            get_tracker_status,
            get_dashboard_stats,
            get_sessions_today,
            add_blocked_app,
            get_blocked_apps,
            remove_blocked_app,
            set_setting,
            get_setting,
            toggle_window,
            show_dashboard,
            hide_to_tray,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
