use crate::database::{Database, TabSession};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Deserialize)]
struct TabEvent {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    client_event_id: Option<String>,
    source: String,
    #[serde(default)]
    tab_id: Option<i64>,
    tab_title: String,
    tab_url: String,
    timestamp: i64,
}

#[derive(Serialize)]
struct BlockDecisionResponse {
    r#type: String,
    client_event_id: Option<String>,
    tab_id: Option<i64>,
    blocked: bool,
    category_id: Option<i64>,
    category_name: Option<String>,
    used_seconds: i64,
    limit_seconds: i64,
    deterrent_mode: String,
    popup_interval_ms: i32,
}

pub async fn run_ws_server(db: Arc<Database>) {
    let listener = match TcpListener::bind("127.0.0.1:7734").await {
        Ok(l) => {
            log::info!("WebSocket server listening on 127.0.0.1:7734");
            l
        }
        Err(e) => {
            log::error!("WebSocket server failed to bind: {}", e);
            return;
        }
    };
    accept_loop(db, listener).await;
}

pub(crate) async fn accept_loop(db: Arc<Database>, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                log::info!("WebSocket client connected");
                let db = db.clone();
                tokio::spawn(handle_connection(db, stream));
            }
            Err(e) => {
                log::error!("WebSocket accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(db: Arc<Database>, stream: tokio::net::TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let mut ws = ws_stream;
    let mut last_source: Option<String> = None;

    while let Some(Ok(msg)) = ws.next().await {
        if let Message::Text(text) = msg {
            if let Ok(event) = serde_json::from_str::<TabEvent>(&text) {
                log::debug!("Tab event: source={}, url={}", event.source, event.tab_url);
                let now = event.timestamp;

                // Close the previously open session for this source before recording the new one
                if let Ok(Some((prev_id, prev_start))) =
                    db.get_open_tab_session_for_source(&event.source)
                {
                    let duration = (now - prev_start).max(0);
                    let _ = db.close_tab_session(prev_id, now, duration);
                }

                let session = TabSession {
                    id: None,
                    source: event.source.clone(),
                    tab_title: event.tab_title.clone(),
                    tab_url: event.tab_url.clone(),
                    start_time: now,
                    end_time: None,
                    duration_seconds: 0,
                };
                let _ = db.insert_tab_session(&session);

                if event.r#type.as_deref().unwrap_or("tab_event") == "tab_event" {
                    if let Ok(decision) = db.evaluate_tab_block(&event.tab_url, &event.tab_title) {
                        let response = BlockDecisionResponse {
                            r#type: "block_decision".to_string(),
                            client_event_id: event.client_event_id.clone(),
                            tab_id: event.tab_id,
                            blocked: decision.blocked,
                            category_id: decision.category_id,
                            category_name: decision.category_name,
                            used_seconds: decision.used_seconds,
                            limit_seconds: decision.limit_seconds,
                            deterrent_mode: decision.deterrent_mode,
                            popup_interval_ms: decision.popup_interval_ms,
                        };
                        if let Ok(text) = serde_json::to_string(&response) {
                            let _ = ws.send(Message::Text(text.into())).await;
                        }
                    }
                }

                last_source = Some(event.source);
            }
        }
    }

    // Client disconnected — finalize any still-open session for this source
    if let Some(source) = last_source {
        log::info!("WebSocket client disconnected (source={})", source);
        let now = chrono::Utc::now().timestamp();
        let _ = db.finalize_open_tab_sessions(&source, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::SinkExt;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::time::{sleep, Duration};
    use tokio_tungstenite::connect_async;

    fn make_test_db() -> (Arc<Database>, TempDir) {
        let dir = TempDir::new().unwrap();
        let db = Arc::new(Database::new(dir.path().to_path_buf()).unwrap());
        (db, dir)
    }

    fn tab_event(source: &str, title: &str, url: &str, timestamp: i64) -> String {
        serde_json::json!({
            "source": source,
            "tab_title": title,
            "tab_url": url,
            "timestamp": timestamp
        })
        .to_string()
    }

    async fn start_server(db: Arc<Database>) -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(accept_loop(db, listener));
        addr
    }

    #[tokio::test]
    async fn test_accepts_event_and_creates_session() {
        let (db, _dir) = make_test_db();
        let addr = start_server(db.clone()).await;

        let (mut ws, _) = connect_async(format!("ws://{}", addr)).await.unwrap();
        let now = chrono::Utc::now().timestamp();
        ws.send(Message::Text(tab_event("chrome", "Google", "https://google.com", now).into()))
            .await
            .unwrap();

        sleep(Duration::from_millis(50)).await;

        let open = db.get_open_tab_session_for_source("chrome").unwrap();
        assert!(open.is_some());
    }

    #[tokio::test]
    async fn test_switches_session_on_second_event() {
        let (db, _dir) = make_test_db();
        let addr = start_server(db.clone()).await;

        let (mut ws, _) = connect_async(format!("ws://{}", addr)).await.unwrap();
        let now = chrono::Utc::now().timestamp();

        ws.send(Message::Text(tab_event("chrome", "Tab A", "https://a.com", now).into()))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        ws.send(Message::Text(
            tab_event("chrome", "Tab B", "https://b.com", now + 10).into(),
        ))
        .await
        .unwrap();
        sleep(Duration::from_millis(50)).await;

        let sessions = db.get_tab_sessions_today().unwrap();
        let tab_a = sessions.iter().find(|s| s.tab_url == "https://a.com").unwrap();
        let tab_b = sessions.iter().find(|s| s.tab_url == "https://b.com").unwrap();

        assert!(tab_a.end_time.is_some(), "tab A should be closed");
        assert_eq!(tab_a.duration_seconds, 10);
        assert!(tab_b.end_time.is_none(), "tab B should still be open");
    }

    #[tokio::test]
    async fn test_finalizes_on_disconnect() {
        let (db, _dir) = make_test_db();
        let addr = start_server(db.clone()).await;

        {
            let (mut ws, _) = connect_async(format!("ws://{}", addr)).await.unwrap();
            let now = chrono::Utc::now().timestamp();
            ws.send(Message::Text(
                tab_event("chrome", "Google", "https://google.com", now).into(),
            ))
            .await
            .unwrap();
            sleep(Duration::from_millis(50)).await;
            // ws drops here, closing the connection
        }

        sleep(Duration::from_millis(100)).await;

        let open = db.get_open_tab_session_for_source("chrome").unwrap();
        assert!(open.is_none(), "session should be finalized after disconnect");
    }

    #[tokio::test]
    async fn test_ignores_malformed_json() {
        let (db, _dir) = make_test_db();
        let addr = start_server(db.clone()).await;

        let (mut ws, _) = connect_async(format!("ws://{}", addr)).await.unwrap();

        ws.send(Message::Text("not valid json {{{{".into()))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        // Server must still be alive — valid event after bad one should work
        let now = chrono::Utc::now().timestamp();
        ws.send(Message::Text(tab_event("chrome", "Google", "https://google.com", now).into()))
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;

        let open = db.get_open_tab_session_for_source("chrome").unwrap();
        assert!(open.is_some(), "server should still accept events after bad JSON");
    }

    #[tokio::test]
    async fn test_multiple_sources_independent() {
        let (db, _dir) = make_test_db();
        let addr = start_server(db.clone()).await;

        let (mut chrome_ws, _) = connect_async(format!("ws://{}", addr)).await.unwrap();
        let (mut vscode_ws, _) = connect_async(format!("ws://{}", addr)).await.unwrap();

        let now = chrono::Utc::now().timestamp();

        chrome_ws
            .send(Message::Text(
                tab_event("chrome", "Chrome Tab", "https://example.com", now).into(),
            ))
            .await
            .unwrap();
        vscode_ws
            .send(Message::Text(
                tab_event("vscode", "main.rs", "file:///main.rs", now).into(),
            ))
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        assert!(
            db.get_open_tab_session_for_source("chrome").unwrap().is_some(),
            "chrome session should be open"
        );
        assert!(
            db.get_open_tab_session_for_source("vscode").unwrap().is_some(),
            "vscode session should be open"
        );
    }
}
