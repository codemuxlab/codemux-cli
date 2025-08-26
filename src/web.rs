use anyhow::Result;
use axum::{
    body::Body,
    extract::{ws::WebSocketUpgrade, Path, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::get,
    Json, Router,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;

use crate::embedded_assets::ReactAssets;
use crate::pty_session::PtyInputMessage;
use crate::session::{ProjectInfo, SessionInfo, SessionManager};

#[derive(Clone)]
pub struct AppState {
    pub session_manager: Option<Arc<RwLock<SessionManager>>>,
    pub _is_daemon_mode: bool,
    pub grid_broadcast_tx: Option<tokio::sync::broadcast::Sender<String>>,
    pub pty_channels: Option<crate::pty_session::PtyChannels>,
}

pub async fn start_web_server(
    port: u16,
    session_manager: Option<Arc<RwLock<SessionManager>>>,
) -> Result<()> {
    let state = AppState {
        session_manager,
        _is_daemon_mode: true,
        grid_broadcast_tx: None,
        pty_channels: None,
    };

    let app = Router::new()
        .route("/", get(daemon_index))
        .route("/session/:session_id", get(session_page))
        .route("/ws/:session_id", get(websocket_handler))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions", axum::routing::post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id", axum::routing::delete(delete_session))
        .route("/api/sessions/:id/stream", get(stream_session_jsonl))
        .route("/api/projects", get(list_projects))
        .route("/api/projects", axum::routing::post(add_project))
        .route("/_expo/static/*path", get(static_handler))
        .route("/*path", get(react_spa_handler))
        .layer(ServiceBuilder::new())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Daemon web server listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn start_web_server_run_mode(
    port: u16,
    session_manager: Option<Arc<RwLock<SessionManager>>>,
    _agent: String,
    grid_rx: tokio::sync::broadcast::Receiver<String>,
    pty_channels: crate::pty_session::PtyChannels,
) -> Result<()> {
    // Store the broadcast sender for websocket handlers to subscribe
    let (grid_tx, _) = tokio::sync::broadcast::channel(1000);
    // Forward messages from the main grid receiver to our local sender
    let grid_tx_clone = grid_tx.clone();
    let mut grid_rx = grid_rx;
    tokio::spawn(async move {
        while let Ok(msg) = grid_rx.recv().await {
            let _ = grid_tx_clone.send(msg);
        }
    });

    let state = AppState {
        session_manager,
        _is_daemon_mode: false,
        grid_broadcast_tx: Some(grid_tx),
        pty_channels: Some(pty_channels),
    };

    let app = Router::new()
        .route("/", get(run_mode_session))
        .route("/ws/:session_id", get(websocket_handler))
        .route("/api/sessions/:id/stream", get(stream_session_jsonl))
        .route("/_expo/static/*path", get(static_handler))
        .route("/*path", get(react_spa_handler))
        .layer(ServiceBuilder::new())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Run mode web server listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn daemon_index() -> impl IntoResponse {
    serve_react_asset("index.html").await
}

async fn run_mode_session() -> impl IntoResponse {
    serve_react_asset("index.html").await
}

async fn session_page(State(_state): State<AppState>) -> impl IntoResponse {
    // For daemon mode, serve React app
    serve_react_asset("index.html").await
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

    tracing::info!(
        "WebSocket connection established for session: {}",
        session_id
    );

    // Get PTY channels from state
    let pty_channels = if let Some(channels) = &state.pty_channels {
        channels
    } else {
        tracing::error!(
            "WebSocket: No PTY channels available for session: {}",
            session_id
        );
        return;
    };

    // Send initial connection message
    let session_short = if session_id.len() >= 8 {
        &session_id[..8]
    } else {
        &session_id
    };
    let welcome_msg = serde_json::json!({
        "type": "output",
        "content": format!("Connected to session {} - Claude Code TUI starting...\r\n", session_short),
        "source": "system"
    });

    if socket
        .send(Message::Text(welcome_msg.to_string()))
        .await
        .is_err()
    {
        return;
    }

    // Subscribe to PTY size updates
    let mut size_rx = pty_channels.size_tx.subscribe();
    if let Ok(current_size) = size_rx.try_recv() {
        let size_msg = serde_json::json!({
            "type": "pty_size",
            "rows": current_size.rows,
            "cols": current_size.cols
        });

        if socket
            .send(Message::Text(size_msg.to_string()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Subscribe to PTY grid updates (our new primary channel)
    let mut grid_rx = pty_channels.grid_tx.subscribe();

    // Subscribe to PTY output for fallback/debug (raw bytes)
    let mut pty_output_rx = pty_channels.output_tx.subscribe();

    // Clone input channel for sending to PTY
    let pty_input_tx = pty_channels.input_tx.clone();

    // Request keyframe for new client (so they get current terminal state immediately)
    match pty_channels.request_keyframe().await {
        Ok(keyframe) => {
            tracing::debug!("Received keyframe for new WebSocket client");
            // Convert keyframe to JSON and send immediately
            let keyframe_json = match keyframe {
                crate::pty_session::GridUpdateMessage::Keyframe {
                    size,
                    cells,
                    cursor,
                    cursor_visible,
                    timestamp,
                } => {
                    let cells = cells
                        .into_iter()
                        .filter(|(_, cell)| !cell.is_empty_space())
                        .map(|((row, col), cell)| serde_json::json!([row, col, cell]))
                        .collect::<Vec<_>>();
                    tracing::debug!("Request keyframe: {}", cells.len());
                    serde_json::json!({
                        "type": "grid_update",
                        "update_type": "keyframe",
                        "size": {
                            "rows": size.rows,
                            "cols": size.cols
                        },
                        "cells": cells,
                        "cursor": {
                            "row": cursor.0,
                            "col": cursor.1
                        },
                        "cursor_visible": cursor_visible,
                        "timestamp": timestamp.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default().as_millis()
                    })
                }
                // This shouldn't happen for keyframe requests, but handle it
                crate::pty_session::GridUpdateMessage::Diff { .. } => {
                    tracing::warn!("Received diff instead of keyframe for new client request");
                    serde_json::json!({"type": "error", "message": "Expected keyframe, got diff"})
                }
            };

            if socket
                .send(Message::Text(keyframe_json.to_string()))
                .await
                .is_err()
            {
                tracing::warn!("Failed to send initial keyframe to new WebSocket client");
                return;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to request keyframe for new WebSocket client: {}", e);
        }
    }

    // Main WebSocket handling loop
    loop {
        tokio::select! {
            // Forward grid updates to WebSocket (primary channel)
            grid_update = grid_rx.recv() => {
                match grid_update {
                    Ok(update) => {
                        // Convert grid update to JSON format for frontend
                        let grid_json = match update {
                            crate::pty_session::GridUpdateMessage::Keyframe { size, cells, cursor, cursor_visible, timestamp } => {
                                serde_json::json!({
                                    "type": "grid_update",
                                    "update_type": "keyframe",
                                    "size": {
                                        "rows": size.rows,
                                        "cols": size.cols
                                    },
                                    "cells": cells.into_iter()
                                        .filter(|(_, cell)| !cell.is_empty_space())
                                        .map(|((row, col), cell)| {
                                            serde_json::json!([row, col, cell])
                                        }).collect::<Vec<_>>(),
                                    "cursor": {
                                        "row": cursor.0,
                                        "col": cursor.1
                                    },
                                    "cursor_visible": cursor_visible,
                                    "timestamp": timestamp.duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default().as_millis()
                                })
                            }
                            crate::pty_session::GridUpdateMessage::Diff { changes, cursor, cursor_visible, timestamp } => {
                                serde_json::json!({
                                    "type": "grid_update",
                                    "update_type": "diff",
                                    "cells": changes.into_iter().map(|(row, col, cell)| {
                                        serde_json::json!([row, col, cell])
                                    }).collect::<Vec<_>>(),
                                    "cursor": cursor.map(|(row, col)| serde_json::json!({
                                        "row": row,
                                        "col": col
                                    })),
                                    "cursor_visible": cursor_visible,
                                    "timestamp": timestamp.duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default().as_millis()
                                })
                            }
                        };

                        if socket.send(Message::Text(grid_json.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("PTY grid channel closed");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        tracing::warn!("WebSocket lagged behind grid updates");
                        // Continue processing
                    }
                }
            }

            // Optional: Forward raw PTY output for debugging
            pty_output = pty_output_rx.recv() => {
                match pty_output {
                    Ok(_output_msg) => {
                        // Skip raw output - we're using grid updates now
                        // Could optionally send for debugging:
                        // let output_json = serde_json::json!({
                        //     "type": "raw_output",
                        //     "content": String::from_utf8_lossy(&output_msg.data),
                        //     "source": "pty"
                        // });
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("PTY output channel closed");
                        // Don't break - we can continue with just grid updates
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        tracing::debug!("WebSocket lagged behind raw PTY output");
                    }
                }
            }

            // Forward PTY size updates to WebSocket
            size_update = size_rx.recv() => {
                match size_update {
                    Ok(size) => {
                        let size_msg = serde_json::json!({
                            "type": "pty_size",
                            "rows": size.rows,
                            "cols": size.cols
                        });

                        if socket.send(Message::Text(size_msg.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("PTY size channel closed");
                        // Don't break - we can continue without size updates
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        tracing::warn!("WebSocket lagged behind PTY size updates");
                        // Continue processing
                    }
                }
            }

            // Handle WebSocket messages from client
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            match parsed.get("type").and_then(|t| t.as_str()) {
                                Some("input") => {
                                    if let Some(data) = parsed.get("data").and_then(|d| d.as_str()) {
                                        // Web input is sent as-is, text and \r are sent separately
                                        let input_bytes = data.as_bytes().to_vec();

                                        let input_msg = PtyInputMessage {
                                            data: input_bytes,
                                            client_id: format!("websocket-{}", session_id),
                                        };

                                        tracing::debug!("WebSocket input: {:?} -> {:?}",
                                            data, String::from_utf8_lossy(&input_msg.data));

                                        if pty_input_tx.send(input_msg).is_err() {
                                            tracing::error!("Failed to send input to PTY");
                                            break;
                                        }
                                    }
                                }
                                Some("resize") => {
                                    // Web UI resize requests are ignored - PTY size controlled by terminal
                                    tracing::debug!("Ignoring resize request from web UI - PTY size follows terminal");
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

    tracing::info!("WebSocket connection closed for session: {}", session_id);
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    if let Some(session_manager) = state.session_manager {
        let manager = session_manager.read().await;
        let sessions = manager.list_sessions();
        Json(sessions)
    } else {
        Json(vec![])
    }
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
    let mut manager = match &state.session_manager {
        Some(sm) => sm.write().await,
        None => return Err("Session manager not available in run mode".to_string()),
    };
    match manager
        .create_session(req.agent, req.args, req.project_id)
        .await
    {
        Ok(info) => Ok(Json(info)),
        Err(e) => Err(e.to_string()),
    }
}

async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionInfo>, String> {
    let manager = match &state.session_manager {
        Some(sm) => sm.read().await,
        None => return Err("Session manager not available in run mode".to_string()),
    };
    manager
        .get_session(&id)
        .map(Json)
        .ok_or_else(|| "Session not found".to_string())
}

async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<String, String> {
    let mut manager = match &state.session_manager {
        Some(sm) => sm.write().await,
        None => return Err("Session manager not available in run mode".to_string()),
    };
    manager
        .close_session(&id)
        .await
        .map(|_| "Session closed".to_string())
        .map_err(|e| e.to_string())
}

async fn stream_session_jsonl(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    use std::time::Duration;
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stream = async_stream::stream! {
        // Get session info to determine the agent
        let manager = match &state.session_manager {
            Some(sm) => sm.read().await,
            None => {
                yield Ok(Event::default().data("Session manager not available"));
                return;
            }
        };
        let session_info = manager.get_session(&session_id);
        drop(manager);

        if let Some(info) = session_info {
            // Only process Claude sessions
            if info.agent.to_lowercase() == "claude" {
                // Get current working directory and convert to dash-case for project folder
                let cwd = std::env::current_dir().unwrap_or_default();
                let cwd_str = cwd.to_string_lossy();
                let project_name = if let Some(stripped) = cwd_str.strip_prefix('/') {
                    // Remove leading slash, then replace remaining slashes with dashes, then add prefix dash
                    format!("-{}", stripped.replace('/', "-"))
                } else {
                    // For relative paths or Windows paths, just replace slashes with dashes and add prefix
                    format!("-{}", cwd_str.replace('/', "-"))
                };

                // Build path to JSONL file
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                let jsonl_path = format!("{}/.claude/projects/{}/{}.jsonl",
                    home, project_name, session_id);

                // Try to open the file
                if let Ok(file) = File::open(&jsonl_path).await {
                    let mut reader = BufReader::new(file);
                    let mut line = String::new();
                    let mut last_position = 0u64;

                    // Read existing content first
                    loop {
                        line.clear();
                        match reader.read_line(&mut line).await {
                            Ok(0) => break, // EOF for now
                            Ok(_) => {
                                if !line.trim().is_empty() {
                                    yield Ok(Event::default().data(line.trim()));
                                }
                            }
                            Err(e) => {
                                yield Ok(Event::default().data(format!("Error reading: {}", e)));
                                break;
                            }
                        }
                    }

                    // Now tail the file for new content
                    yield Ok(Event::default().data("[STREAMING]"));

                    loop {
                        tokio::time::sleep(Duration::from_millis(500)).await;

                        // Re-open file to check for new content
                        if let Ok(mut file) = File::open(&jsonl_path).await {
                            use tokio::io::AsyncSeekExt;

                            // Seek to last position
                            let _ = file.seek(std::io::SeekFrom::Start(last_position)).await;
                            let mut reader = BufReader::new(file);

                            loop {
                                line.clear();
                                match reader.read_line(&mut line).await {
                                    Ok(0) => break, // No new content
                                    Ok(n) => {
                                        last_position += n as u64;
                                        if !line.trim().is_empty() {
                                            yield Ok(Event::default().data(line.trim()));
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                } else {
                    yield Ok(Event::default().data(format!("JSONL file not found: {}", jsonl_path)));
                }
            } else {
                yield Ok(Event::default().data(format!("Not a Claude session: {}", info.agent)));
            }
        } else {
            yield Ok(Event::default().data("Session not found"));
        }
    };

    Sse::new(stream)
}

async fn list_projects(State(state): State<AppState>) -> Json<Vec<ProjectInfo>> {
    let manager = match &state.session_manager {
        Some(sm) => sm.read().await,
        None => return Json(vec![]), // Return empty list in run mode
    };
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
    let mut manager = match &state.session_manager {
        Some(sm) => sm.write().await,
        None => return Err("Session manager not available in run mode".to_string()),
    };
    match manager.add_project(req.name, req.path).await {
        Ok(info) => Ok(Json(info)),
        Err(e) => Err(e.to_string()),
    }
}

async fn serve_react_asset(path: &str) -> impl IntoResponse {
    tracing::debug!("serve_react_asset called with path: '{}'", path);
    match ReactAssets::get(path) {
        Some(content) => {
            let body = Body::from(content.data.into_owned());
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            tracing::debug!("Found asset '{}', serving with mime: {}", path, mime);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        }
        None => {
            tracing::debug!("Asset '{}' not found, returning 404", path);
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not found"))
                .unwrap()
        }
    }
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let file_path = format!("_expo/static/{}", path);
    tracing::debug!(
        "Static handler requested path: '{}', serving file: '{}'",
        path,
        file_path
    );
    serve_react_asset(&file_path).await
}

async fn react_spa_handler(Path(_path): Path<String>) -> impl IntoResponse {
    // For SPA routing, always serve index.html for non-API routes
    serve_react_asset("index.html").await
}
