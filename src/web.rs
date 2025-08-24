use anyhow::Result;
use axum::{
    extract::{ws::WebSocketUpgrade, State, Path},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;

use crate::session::{SessionManager, SessionInfo, ProjectInfo};

#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub _is_daemon_mode: bool,
}

pub async fn start_web_server(port: u16, session_manager: Arc<RwLock<SessionManager>>) -> Result<()> {
    let state = AppState { 
        session_manager,
        _is_daemon_mode: true,
    };
    
    let app = Router::new()
        .route("/", get(daemon_index))
        .route("/session/:session_id", get(session_page))
        .route("/ws/:session_id", get(websocket_handler))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions", axum::routing::post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id", axum::routing::delete(delete_session))
        .route("/api/projects", get(list_projects))
        .route("/api/projects", axum::routing::post(add_project))
        .layer(ServiceBuilder::new())
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Daemon web server listening on http://0.0.0.0:{}", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn start_web_server_run_mode(port: u16, session_manager: Arc<RwLock<SessionManager>>, _agent: String) -> Result<()> {
    let state = AppState { 
        session_manager,
        _is_daemon_mode: false,
    };
    
    let app = Router::new()
        .route("/", get(run_mode_session))
        .route("/ws/:session_id", get(websocket_handler))
        .layer(ServiceBuilder::new())
        .with_state(state.clone());
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Run mode web server listening on http://0.0.0.0:{}", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn daemon_index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn run_mode_session() -> Html<&'static str> {
    Html(include_str!("../static/session.html"))
}

async fn session_page(State(_state): State<AppState>) -> Html<String> {
    // For daemon mode, include back button
    let html = include_str!("../static/session.html");
    let with_back_button = html.replace("<!-- DAEMON_MODE_NAV -->", r#"
        <a href="/" class="back-btn">
            <span>← Back to Sessions</span>
        </a>
    "#);
    Html(with_back_button)
}

async fn websocket_handler(
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, session_id, state))
}

async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    session_id: String,
    state: AppState,
) {
    use axum::extract::ws::Message;
    use std::io::Read;
    use std::io::Write;
    
    tracing::info!("WebSocket connection established for session: {}", session_id);
    
    // Get the PTY session
    let manager = state.session_manager.read().await;
    let pty_session = if let Some(session) = manager.sessions.get(&session_id) {
        session
    } else {
        tracing::error!("Session {} not found", session_id);
        return;
    };
    
    let pty = pty_session.pty.clone();
    let reader = pty_session.reader.clone();
    drop(manager); // Release the lock
    
    // Send initial connection message
    let session_short = if session_id.len() >= 8 { &session_id[..8] } else { &session_id };
    let welcome_msg = serde_json::json!({
        "type": "output",
        "content": format!("Connected to session {} - Claude Code TUI starting...\r\n", session_short),
        "source": "system"
    });
    
    if socket.send(Message::Text(welcome_msg.to_string())).await.is_err() {
        return;
    }
    
    // Give Claude Code a moment to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Create channels for communication
    let (pty_to_ws_tx, mut pty_to_ws_rx) = tokio::sync::mpsc::channel(100);
    let (ws_to_pty_tx, mut ws_to_pty_rx) = tokio::sync::mpsc::channel(100);
    
    // Task to read from PTY and send to channel
    let reader_clone = reader.clone();
    let pty_reader_task = tokio::spawn(async move {
        let mut buffer = [0u8; 1024];
        loop {
            let mut reader_guard = reader_clone.lock().await;
            match reader_guard.read(&mut buffer) {
                Ok(0) => {
                    tracing::info!("PTY reader reached EOF");
                    break;
                }
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                    tracing::debug!("PTY output ({} bytes): {:?}", n, data);
                    if pty_to_ws_tx.send(data).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading from PTY: {}", e);
                    break;
                }
            }
            drop(reader_guard);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });
    
    // Task to write to PTY from channel
    let pty_clone = pty.clone();
    let pty_writer_task = tokio::spawn(async move {
        while let Some(data) = ws_to_pty_rx.recv().await {
            let input = format!("{}\n", data);
            let pty_guard = pty_clone.lock().await;
            if let Ok(mut writer) = pty_guard.take_writer() {
                if let Err(e) = writer.write_all(input.as_bytes()) {
                    tracing::error!("Failed to write to PTY: {}", e);
                    break;
                }
                let _ = writer.flush();
            }
        }
    });
    
    // Main WebSocket handling loop
    loop {
        tokio::select! {
            // Send PTY output to WebSocket
            pty_data = pty_to_ws_rx.recv() => {
                if let Some(data) = pty_data {
                    let msg = serde_json::json!({
                        "type": "output",
                        "content": data,
                        "source": "pty"
                    });
                    
                    if socket.send(Message::Text(msg.to_string())).await.is_err() {
                        break;
                    }
                } else {
                    break; // Channel closed
                }
            }
            
            // Handle WebSocket messages
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            match parsed.get("type").and_then(|t| t.as_str()) {
                                Some("input") => {
                                    if let Some(data) = parsed.get("data").and_then(|d| d.as_str()) {
                                        if ws_to_pty_tx.send(data.to_string()).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                Some("resize") => {
                                    if let (Some(cols), Some(rows)) = (
                                        parsed.get("cols").and_then(|c| c.as_u64()),
                                        parsed.get("rows").and_then(|r| r.as_u64())
                                    ) {
                                        let pty_guard = pty.lock().await;
                                        let _ = pty_guard.resize(portable_pty::PtySize {
                                            rows: rows as u16,
                                            cols: cols as u16,
                                            pixel_width: 0,
                                            pixel_height: 0,
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }
    
    // Clean up background tasks
    pty_reader_task.abort();
    pty_writer_task.abort();
    
    tracing::info!("WebSocket connection closed for session: {}", session_id);
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    let manager = state.session_manager.read().await;
    let sessions = manager.list_sessions();
    Json(sessions)
}

#[derive(Deserialize)]
struct CreateSessionRequest {
    agent: String,
    args: Vec<String>,
    project_id: Option<String>,
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, String> {
    let mut manager = state.session_manager.write().await;
    match manager.create_session(req.agent, req.args, req.project_id).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => Err(e.to_string()),
    }
}

async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionInfo>, String> {
    let manager = state.session_manager.read().await;
    manager.get_session(&id)
        .map(|info| Json(info))
        .ok_or_else(|| "Session not found".to_string())
}

async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<String, String> {
    let mut manager = state.session_manager.write().await;
    manager.close_session(&id).await
        .map(|_| "Session closed".to_string())
        .map_err(|e| e.to_string())
}

async fn list_projects(State(state): State<AppState>) -> Json<Vec<ProjectInfo>> {
    let manager = state.session_manager.read().await;
    let projects = manager.list_projects();
    Json(projects)
}

#[derive(Deserialize)]
struct AddProjectRequest {
    name: String,
    path: String,
}

async fn add_project(
    State(state): State<AppState>,
    Json(req): Json<AddProjectRequest>,
) -> Result<Json<ProjectInfo>, String> {
    let mut manager = state.session_manager.write().await;
    match manager.add_project(req.name, req.path).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => Err(e.to_string()),
    }
}