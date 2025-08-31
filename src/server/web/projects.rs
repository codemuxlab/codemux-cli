use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

use super::json_api::{
    json_api_error_response_with_headers, json_api_response_with_headers, JsonApiResource,
};
use super::types::{AddProjectRequest, AppState};
use crate::core::session::{ProjectInfo, ProjectWithSessions, SessionInfo};

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

        // Get the 5 most recent historical sessions for this project from the cache
        let project_path = PathBuf::from(&project.path);
        let recent_sessions = state
            .session_manager
            .get_recent_project_sessions(project_path)
            .await;
        project_sessions.extend(recent_sessions);

        projects_with_sessions.push(ProjectWithSessions {
            id: project.id,
            name: project.name,
            path: project.path,
            sessions: project_sessions,
        });
    }

    // Sort projects by most recent session timestamp
    projects_with_sessions.sort_by(|a, b| {
        let a_latest = a
            .sessions
            .iter()
            .filter_map(|s| s.last_modified.as_ref())
            .filter_map(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .max();

        let b_latest = b
            .sessions
            .iter()
            .filter_map(|s| s.last_modified.as_ref())
            .filter_map(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .max();

        // Sort by most recent first (descending)
        match (b_latest, a_latest) {
            (Some(b_time), Some(a_time)) => b_time.cmp(&a_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name), // Fallback to name sorting
        }
    });

    // Convert to JSON API format
    let resources: Vec<JsonApiResource<ProjectWithSessions>> = projects_with_sessions
        .into_iter()
        .map(|project| project.into())
        .collect();

    json_api_response_with_headers(resources)
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
            let resource: JsonApiResource<ProjectInfo> = info.into();
            json_api_response_with_headers(resource)
        }
        Err(e) => json_api_error_response_with_headers(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Project Creation Failed".to_string(),
            e.to_string(),
        ),
    }
}
