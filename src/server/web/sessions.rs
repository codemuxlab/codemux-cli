use axum::{
    extract::{Path, State},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use futures::stream::Stream;
use std::convert::Infallible;

use super::types::{AppState, CreateSessionRequest};
use crate::core::session::SessionInfo;

pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, String> {
    tracing::debug!(
        "Creating session with agent: {}, args: {:?}",
        req.agent,
        req.args
    );
    match state
        .session_manager
        .create_session_with_path(req.agent, req.args, req.project_id, req.path)
        .await
    {
        Ok(info) => {
            tracing::info!("Session created successfully: {}", info.id);
            Ok(Json(info))
        }
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            Err(e.to_string())
        }
    }
}

pub async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionInfo>, String> {
    state
        .session_manager
        .get_session(&id)
        .await
        .map(Json)
        .ok_or_else(|| "Session not found".to_string())
}

pub async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<String, String> {
    state
        .session_manager
        .close_session(&id)
        .await
        .map(|_| "Session closed".to_string())
        .map_err(|e| e.to_string())
}

pub async fn stream_session_jsonl(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    use std::time::Duration;
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stream = async_stream::stream! {
        // Get session info to determine the agent
        let session_info = state.session_manager.get_session(&session_id).await;

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

pub async fn shutdown_server(State(state): State<AppState>) -> impl IntoResponse {
    use axum::Json;

    tracing::info!("Received shutdown request");

    // Gracefully shutdown all sessions
    tracing::info!("Shutting down all sessions...");
    state.session_manager.shutdown_all_sessions().await;

    // Spawn a task to exit the process after a short delay
    // This allows the HTTP response to be sent before the server shuts down
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        tracing::info!("Exiting server process");
        std::process::exit(0);
    });

    Json(serde_json::json!({"status": "shutdown initiated"})).into_response()
}
