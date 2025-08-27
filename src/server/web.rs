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
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use crate::assets::embedded::ReactAssets;
use crate::core::pty_session::PtyInputMessage;
use crate::core::session::{ProjectInfo, ProjectWithSessions, SessionInfo};
use crate::server::SessionManager;

#[derive(Clone)]
pub struct AppState {
    pub session_manager: Option<Arc<RwLock<SessionManager>>>,
    pub _is_daemon_mode: bool,
    pub grid_broadcast_tx: Option<tokio::sync::broadcast::Sender<String>>,
    pub pty_channels: Option<crate::pty_session::PtyChannels>,
    pub run_mode_session_id: Option<String>, // For run mode, stores the actual session ID
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
        run_mode_session_id: None,
    };

    let app = Router::new()
        .route("/", get(daemon_index))
        .route("/session/:session_id", get(session_page))
        .route("/ws/:session_id", get(websocket_handler))
        .route("/api/sessions", axum::routing::post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id", axum::routing::delete(delete_session))
        .route("/api/sessions/:id/stream", get(stream_session_jsonl))
        .route("/api/sessions/:id/git/status", get(get_git_status))
        .route("/api/sessions/:id/git/diff", get(get_git_diff))
        .route("/api/sessions/:id/git/diff/*path", get(get_git_file_diff))
        .route("/api/projects", get(list_projects))
        .route("/api/projects", axum::routing::post(add_project))
        .route("/_expo/static/*path", get(static_handler))
        .route("/*path", get(react_spa_handler))
        .layer(
            ServiceBuilder::new()
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any)
                )
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Daemon web server listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn daemon_index() -> impl IntoResponse {
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
                                        // Legacy input: raw text data
                                        let input_bytes = data.as_bytes().to_vec();

                                        let input_msg = PtyInputMessage {
                                            input: crate::pty_session::PtyInput::Raw {
                                                data: input_bytes,
                                                client_id: format!("websocket-{}", session_id),
                                            },
                                        };

                                        tracing::debug!("WebSocket raw input: {:?}", data);

                                        if pty_input_tx.send(input_msg).is_err() {
                                            tracing::error!("Failed to send input to PTY");
                                            break;
                                        }
                                    }
                                }
                                Some("key") => {
                                    // New key event input
                                    if let Ok(key_event) = serde_json::from_value::<crate::pty_session::KeyEvent>(parsed.clone()) {
                                        tracing::debug!("WebSocket key event: {:?}", key_event);

                                        let input_msg = PtyInputMessage {
                                            input: crate::pty_session::PtyInput::Key {
                                                event: key_event,
                                                client_id: format!("websocket-{}", session_id),
                                            },
                                        };

                                        if pty_input_tx.send(input_msg).is_err() {
                                            tracing::error!("Failed to send key event to PTY");
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

async fn list_projects(State(state): State<AppState>) -> Json<Vec<ProjectWithSessions>> {
    match &state.session_manager {
        Some(sm) => {
            // Daemon mode: return actual projects with their sessions
            let manager = sm.read().await;
            let projects = manager.list_projects();
            let sessions = manager.list_sessions();
            
            let projects_with_sessions = projects.into_iter().map(|project| {
                let project_sessions = sessions.iter()
                    .filter(|session| session.project.as_deref() == Some(&project.id))
                    .cloned()
                    .collect();
                    
                ProjectWithSessions {
                    id: project.id,
                    name: project.name,
                    path: project.path,
                    sessions: project_sessions,
                }
            }).collect();
            
            Json(projects_with_sessions)
        },
        None => {
            // Quick mode: return a default project with current directory and default session
            let current_dir = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string();
            
            let project_name = std::path::Path::new(&current_dir)
                .file_name()
                .unwrap_or_else(|| "unknown".as_ref())
                .to_string_lossy()
                .to_string();
            
            // Create default session if PTY channels exist
            let sessions = if let Some(_pty_channels) = &state.pty_channels {
                vec![SessionInfo {
                    id: state.run_mode_session_id.clone().unwrap_or_else(|| "default-session".to_string()),
                    agent: "claude".to_string(),
                    project: Some("default".to_string()),
                    status: "running".to_string(),
                }]
            } else {
                vec![]
            };
            
            let default_project = ProjectWithSessions {
                id: "default".to_string(),
                name: project_name,
                path: current_dir,
                sessions,
            };
            
            Json(vec![default_project])
        }
    }
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

// Git-related data structures
#[derive(Serialize)]
struct GitFileStatus {
    path: String,
    status: String, // "modified", "added", "deleted", "renamed", "untracked"
    additions: Option<u32>,
    deletions: Option<u32>,
}

#[derive(Serialize)]
struct GitStatus {
    files: Vec<GitFileStatus>,
    branch: Option<String>,
    clean: bool,
}

#[derive(Serialize)]
struct GitDiff {
    files: Vec<GitFileDiff>,
}

#[derive(Serialize)]
struct GitFileDiff {
    path: String,
    old_path: Option<String>, // For renamed files
    status: String,
    additions: u32,
    deletions: u32,
    diff: String, // The actual diff content
}

// Git API handlers
async fn get_git_status(Path(session_id): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let working_dir = match get_session_working_dir(&session_id, &state).await {
        Some(dir) => dir,
        None => return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Session not found"))
            .unwrap(),
    };

    match execute_git_status(&working_dir).await {
        Ok(status) => Json(status).into_response(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Git error: {}", e)))
            .unwrap(),
    }
}

async fn get_git_diff(Path(session_id): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let working_dir = match get_session_working_dir(&session_id, &state).await {
        Some(dir) => dir,
        None => return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Session not found"))
            .unwrap(),
    };

    match execute_git_diff(&working_dir).await {
        Ok(diff) => Json(diff).into_response(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Git error: {}", e)))
            .unwrap(),
    }
}

async fn get_git_file_diff(Path((session_id, file_path)): Path<(String, String)>, State(state): State<AppState>) -> impl IntoResponse {
    let working_dir = match get_session_working_dir(&session_id, &state).await {
        Some(dir) => dir,
        None => return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Session not found"))
            .unwrap(),
    };

    match execute_git_file_diff(&working_dir, &file_path).await {
        Ok(diff) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(diff))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Git error: {}", e)))
            .unwrap(),
    }
}

// Helper functions
async fn get_session_working_dir(session_id: &str, state: &AppState) -> Option<String> {
    // For run mode, use current directory
    if state.session_manager.is_none() {
        return Some(std::env::current_dir().ok()?.to_string_lossy().to_string());
    }

    // For daemon mode, get session working directory
    let manager = state.session_manager.as_ref()?.read().await;
    let _session_info = manager.get_session(session_id)?;
    // TODO: Get actual working directory from session info
    // For now, return current directory
    Some(std::env::current_dir().ok()?.to_string_lossy().to_string())
}

async fn execute_git_status(working_dir: &str) -> Result<GitStatus, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("git")
        .args(&["status", "--porcelain", "-b", "--untracked-files=all"])
        .current_dir(working_dir)
        .output()?;

    if !output.status.success() {
        return Err("Not a git repository or git command failed".into());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    let mut branch = None;

    for line in output_str.lines() {
        if line.starts_with("##") {
            // Branch information
            let branch_info = line.strip_prefix("## ").unwrap_or("");
            branch = Some(branch_info.split("...").next().unwrap_or(branch_info).to_string());
        } else if line.len() >= 3 {
            let status_chars = &line[0..2];
            let file_path = line[3..].to_string();
            
            let status = match status_chars {
                " M" | "M " | "MM" => "modified",
                "A " | "AM" => "added",
                " D" | "D " => "deleted",
                "R " => "renamed",
                "??" => "untracked",
                _ => "unknown",
            };

            files.push(GitFileStatus {
                path: file_path,
                status: status.to_string(),
                additions: None, // TODO: Get from git diff --numstat
                deletions: None,
            });
        }
    }

    let is_clean = files.is_empty();
    Ok(GitStatus {
        files,
        branch,
        clean: is_clean,
    })
}

async fn execute_git_diff(working_dir: &str) -> Result<GitDiff, Box<dyn std::error::Error + Send + Sync>> {
    let mut files = Vec::new();

    // Get tracked file changes
    let output = Command::new("git")
        .args(&["diff", "--name-status"])
        .current_dir(working_dir)
        .output()?;

    if !output.status.success() {
        return Err("Git diff failed".into());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        if let Some((status_char, file_path)) = line.split_once('\t') {
            let status = match status_char {
                "M" => "modified",
                "A" => "added", 
                "D" => "deleted",
                "R" => "renamed",
                _ => "unknown",
            };

            // Get detailed diff for this file
            let diff_output = Command::new("git")
                .args(&["diff", file_path])
                .current_dir(working_dir)
                .output()?;

            let diff_content = String::from_utf8_lossy(&diff_output.stdout).to_string();
            
            // Parse additions/deletions from diff
            let (additions, deletions) = parse_diff_stats(&diff_content);

            files.push(GitFileDiff {
                path: file_path.to_string(),
                old_path: None, // TODO: Handle renamed files
                status: status.to_string(),
                additions,
                deletions,
                diff: diff_content,
            });
        }
    }

    // Add untracked files (show full content as "added")
    let untracked_output = Command::new("git")
        .args(&["status", "--porcelain", "--untracked-files=all"])
        .current_dir(working_dir)
        .output()?;

    if untracked_output.status.success() {
        let untracked_str = String::from_utf8_lossy(&untracked_output.stdout);
        
        for line in untracked_str.lines() {
            if line.starts_with("??") && line.len() >= 3 {
                let file_path = &line[3..];
                
                // Read the full content of the untracked file
                let file_content = std::fs::read_to_string(
                    std::path::Path::new(working_dir).join(file_path)
                ).unwrap_or_else(|_| String::from("Binary file or read error"));

                // Create a fake diff showing the entire file as added
                let fake_diff = if file_content.is_empty() {
                    format!("diff --git a/{} b/{}\nnew file mode 100644\nindex 0000000..0000000\n--- /dev/null\n+++ b/{}\n", file_path, file_path, file_path)
                } else {
                    let mut diff_lines = vec![
                        format!("diff --git a/{} b/{}", file_path, file_path),
                        "new file mode 100644".to_string(),
                        "index 0000000..0000000".to_string(),
                        "--- /dev/null".to_string(),
                        format!("+++ b/{}", file_path),
                    ];
                    
                    // Add each line of the file as an addition
                    for line in file_content.lines() {
                        diff_lines.push(format!("+{}", line));
                    }
                    
                    diff_lines.join("\n")
                };

                let line_count = file_content.lines().count() as u32;

                files.push(GitFileDiff {
                    path: file_path.to_string(),
                    old_path: None,
                    status: "untracked".to_string(),
                    additions: line_count,
                    deletions: 0,
                    diff: fake_diff,
                });
            }
        }
    }

    Ok(GitDiff { files })
}

async fn execute_git_file_diff(working_dir: &str, file_path: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("git")
        .args(&["diff", file_path])
        .current_dir(working_dir)
        .output()?;

    if output.status.success() {
        let diff_content = String::from_utf8_lossy(&output.stdout).to_string();
        if !diff_content.trim().is_empty() {
            return Ok(diff_content);
        }
    }

    // If git diff returns empty or fails, check if it's an untracked file
    let status_output = Command::new("git")
        .args(&["status", "--porcelain", file_path])
        .current_dir(working_dir)
        .output()?;

    if status_output.status.success() {
        let status_str = String::from_utf8_lossy(&status_output.stdout);
        if status_str.starts_with("??") {
            // It's an untracked file, show full content as additions
            let file_content = std::fs::read_to_string(
                std::path::Path::new(working_dir).join(file_path)
            ).unwrap_or_else(|_| String::from("Binary file or read error"));

            let fake_diff = if file_content.is_empty() {
                format!("diff --git a/{} b/{}\nnew file mode 100644\nindex 0000000..0000000\n--- /dev/null\n+++ b/{}\n", file_path, file_path, file_path)
            } else {
                let mut diff_lines = vec![
                    format!("diff --git a/{} b/{}", file_path, file_path),
                    "new file mode 100644".to_string(),
                    "index 0000000..0000000".to_string(),
                    "--- /dev/null".to_string(),
                    format!("+++ b/{}", file_path),
                ];
                
                // Add each line of the file as an addition
                for line in file_content.lines() {
                    diff_lines.push(format!("+{}", line));
                }
                
                diff_lines.join("\n")
            };

            return Ok(fake_diff);
        }
    }

    Err("No diff found for file".into())
}

fn parse_diff_stats(diff_content: &str) -> (u32, u32) {
    let mut additions = 0;
    let mut deletions = 0;

    for line in diff_content.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    (additions, deletions)
}
