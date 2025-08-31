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

use crate::core::{
    json_api_error_response_with_headers, json_api_response_with_headers,
};
use super::types::{AppState, CreateSessionRequest};
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;

/// Check if a specific session ID exists in ~/.claude/projects
async fn session_exists(session_id: &str) -> Result<bool, std::io::Error> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let claude_projects_path = PathBuf::from(&home).join(".claude").join("projects");

    if !claude_projects_path.exists() {
        return Ok(false);
    }

    let mut entries = fs::read_dir(&claude_projects_path).await?;

    while let Some(project_dir) = entries.next_entry().await? {
        if !project_dir.file_type().await?.is_dir() {
            continue;
        }

        let project_path = project_dir.path();
        let session_file = project_path.join(format!("{}.jsonl", session_id));
        
        if session_file.exists() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Find the most recent JSONL file in ~/.claude/projects
async fn find_most_recent_jsonl() -> Result<Option<String>, std::io::Error> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let claude_projects_path = PathBuf::from(&home).join(".claude").join("projects");

    if !claude_projects_path.exists() {
        return Ok(None);
    }

    let mut most_recent: Option<(SystemTime, String)> = None;
    let mut entries = fs::read_dir(&claude_projects_path).await?;

    while let Some(project_dir) = entries.next_entry().await? {
        if !project_dir.file_type().await?.is_dir() {
            continue;
        }

        let project_path = project_dir.path();
        let mut project_entries = fs::read_dir(&project_path).await?;

        while let Some(entry) = project_entries.next_entry().await? {
            let file_path = entry.path();
            if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let session_id = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if most_recent.is_none() || modified > most_recent.as_ref().unwrap().0 {
                            most_recent = Some((modified, session_id));
                        }
                    }
                }
            }
        }
    }

    Ok(most_recent.map(|(_, session_id)| session_id))
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(mut req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        "Creating session with agent: {}, args: {:?}",
        req.agent,
        req.args
    );

    // Handle --continue flag for Claude agent
    let resume_session_id = if req.agent.to_lowercase() == "claude" {
        if let Some(continue_idx) = req.args.iter().position(|arg| arg == "--continue") {
            tracing::info!("Server: Processing --continue flag for Claude");

            // Remove --continue from args
            req.args.remove(continue_idx);

            // Find the most recent JSONL session
            match find_most_recent_jsonl().await {
                Ok(Some(session_id)) => {
                    tracing::info!("Server: Found most recent session: {}", session_id);
                    // Replace with --resume <session_id>
                    req.args.push("--resume".to_string());
                    req.args.push(session_id.clone());
                    Some(session_id)
                }
                Ok(None) => {
                    tracing::info!(
                        "Server: No previous JSONL sessions found, starting new session"
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(
                        "Server: Error finding recent JSONL: {}, starting new session",
                        e
                    );
                    None
                }
            }
        } else if let Some(resume_idx) = req.args.iter().position(|arg| arg == "--resume") {
            // Handle explicit --resume flag
            if let Some(session_id) = req.args.get(resume_idx + 1) {
                tracing::info!("Server: Processing --resume flag with session: {}", session_id);
                
                // Validate that the session exists
                match session_exists(session_id).await {
                    Ok(true) => {
                        tracing::info!("Server: Session {} exists, proceeding with resume", session_id);
                        Some(session_id.clone())
                    }
                    Ok(false) => {
                        tracing::warn!("Server: Session {} does not exist", session_id);
                        return json_api_error_response_with_headers(
                            axum::http::StatusCode::NOT_FOUND,
                            "Session Not Found".to_string(),
                            format!(
                                "Session '{}' does not exist. Use --continue to resume the most recent session, \
                                or check available sessions with 'codemux list'.",
                                session_id
                            ),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Server: Error checking if session exists: {}", e);
                        return json_api_error_response_with_headers(
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            "Session Validation Failed".to_string(),
                            "Unable to validate session existence".to_string(),
                        );
                    }
                }
            } else {
                tracing::warn!("Server: --resume flag found but no session ID provided");
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    match state
        .session_manager
        .create_session_with_path(req.agent, req.args, req.project_id, req.path, resume_session_id)
        .await
    {
        Ok(info) => {
            tracing::info!("Session created successfully: {}", info.id);
            json_api_response_with_headers(info)
        }
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            json_api_error_response_with_headers(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Session Creation Failed".to_string(),
                e.to_string(),
            )
        }
    }
}

pub async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.session_manager.get_session(&id).await {
        Some(info) => {
            json_api_response_with_headers(info)
        }
        None => json_api_error_response_with_headers(
            axum::http::StatusCode::NOT_FOUND,
            "Session Not Found".to_string(),
            format!("Session with id '{}' not found", id),
        ),
    }
}

pub async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.session_manager.close_session(&id).await {
        Ok(_) => json_api_response_with_headers(serde_json::json!({
            "message": "Session closed successfully"
        })),
        Err(e) => json_api_error_response_with_headers(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Session Deletion Failed".to_string(),
            e.to_string(),
        ),
    }
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
            if let Some(attrs) = &info.attributes {
                if attrs.agent.to_lowercase() == "claude" {
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
                    yield Ok(Event::default().data(format!("Not a Claude session: {}", attrs.agent)));
                }
            } else {
                yield Ok(Event::default().data("Session missing attributes"));
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
