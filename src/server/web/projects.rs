use axum::{extract::State, response::IntoResponse, Json};
use std::path::PathBuf;
use tokio::fs;

use super::json_api::{json_api_response, json_api_error, JsonApiResource};
use super::types::{AddProjectRequest, AppState};
use crate::core::session::{ProjectWithSessions, SessionInfo, SessionType};

/// Find all JSONL sessions for a specific project path
async fn find_project_jsonl_sessions(project_path: &str) -> Result<Vec<SessionInfo>, std::io::Error> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let claude_projects_path = PathBuf::from(&home).join(".claude").join("projects");
    
    if !claude_projects_path.exists() {
        return Ok(Vec::new());
    }

    // Convert project path to dash-case folder name
    let project_name = if let Some(stripped) = project_path.strip_prefix('/') {
        format!("-{}", stripped.replace('/', "-"))
    } else {
        format!("-{}", project_path.replace('/', "-"))
    };
    
    let project_dir = claude_projects_path.join(&project_name);
    if !project_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut sessions = Vec::new();
    let mut entries = fs::read_dir(&project_dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let file_path = entry.path();
        if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Some(session_id) = file_path
                .file_stem()
                .and_then(|s| s.to_str()) 
            {
                sessions.push(SessionInfo {
                    id: session_id.to_string(),
                    agent: "claude".to_string(), // JSONL files are currently only for Claude
                    project: None, // Will be set by caller
                    status: "completed".to_string(),
                    session_type: SessionType::Historical,
                });
            }
        }
    }
    
    Ok(sessions)
}

pub async fn list_projects(State(state): State<AppState>) -> impl IntoResponse {
    // Return actual projects with their sessions
    let projects = state.session_manager.list_projects().await;
    let active_sessions = state.session_manager.list_sessions().await;

    let mut projects_with_sessions = Vec::new();
    
    for project in projects {
        // Get active sessions for this project
        let mut project_sessions: Vec<SessionInfo> = active_sessions
            .iter()
            .filter(|session| session.project.as_deref() == Some(&project.id))
            .cloned()
            .collect();

        // Get historical JSONL sessions for this project
        if let Ok(jsonl_sessions) = find_project_jsonl_sessions(&project.path).await {
            // Set the project field for JSONL sessions
            let jsonl_sessions_with_project: Vec<SessionInfo> = jsonl_sessions
                .into_iter()
                .map(|session| {
                    SessionInfo {
                        id: session.id,
                        agent: session.agent,
                        project: Some(project.id.clone()),
                        status: session.status,
                        session_type: session.session_type,
                    }
                })
                .collect();
            project_sessions.extend(jsonl_sessions_with_project);
        }

        projects_with_sessions.push(ProjectWithSessions {
            id: project.id,
            name: project.name,
            path: project.path,
            sessions: project_sessions,
        });
    }

    // Convert to JSON API format
    let resources: Vec<JsonApiResource> = projects_with_sessions
        .into_iter()
        .map(|project| project.into())
        .collect();
    
    Json(json_api_response(resources))
}

pub async fn add_project(
    State(state): State<AppState>,
    Json(req): Json<AddProjectRequest>,
) -> impl IntoResponse {
    match state
        .session_manager
        .create_project(req.name, req.path)
        .await
    {
        Ok(info) => {
            let resource: JsonApiResource = info.into();
            Json(json_api_response(resource)).into_response()
        }
        Err(e) => {
            Json(json_api_error(
                "500".to_string(),
                "Project Creation Failed".to_string(),
                e.to_string(),
            ))
            .into_response()
        }
    }
}
