use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

use crate::core::{
    json_api_error_response_with_headers, json_api_response_with_headers,
};
use super::types::{AddProjectRequest, AppState};

pub async fn list_projects(State(state): State<AppState>) -> impl IntoResponse {
    // Return actual projects with their sessions
    let mut projects = state.session_manager.list_projects().await;
    let active_sessions = state.session_manager.list_sessions().await;

    for project in &mut projects {
        // Get active sessions for this project
        let project_sessions: Vec<crate::core::json_api::SessionResourceTS> = active_sessions
            .iter()
            .filter(|session| {
                if let Some(attrs) = &session.attributes {
                    attrs.project.as_deref() == Some(&project.id)
                } else {
                    false
                }
            })
            .map(|session| crate::core::json_api::SessionResourceTS {
                resource_type: "session".to_string(),
                id: session.id.clone(),
                attributes: session.attributes.clone(),
            })
            .collect();

        // Get the 5 most recent historical sessions for this project from the cache
        if let Some(attrs) = &project.attributes {
            let project_path = PathBuf::from(&attrs.path);
            let recent_sessions = state
                .session_manager
                .get_recent_project_sessions(project_path)
                .await;
            
            // Add recent sessions to relationships
            let mut all_sessions = project_sessions;
            all_sessions.extend(recent_sessions.into_iter().map(|session| crate::core::json_api::SessionResourceTS {
                resource_type: "session".to_string(),
                id: session.id.clone(),
                attributes: session.attributes.clone(),
            }));

            // Update relationships
            if let Some(ref mut relationships) = project.relationships {
                relationships.recent_sessions = if all_sessions.is_empty() { None } else { Some(all_sessions) };
            } else {
                project.relationships = Some(crate::core::ProjectRelationships {
                    recent_sessions: if all_sessions.is_empty() { None } else { Some(all_sessions) },
                });
            }
        }
    }

    // Sort projects by most recent session timestamp
    projects.sort_by(|a, b| {
        let a_latest = if let Some(relationships) = &a.relationships {
            relationships.recent_sessions.as_ref().and_then(|sessions| {
                active_sessions
                    .iter()
                    .filter(|s| sessions.iter().any(|ref_s| ref_s.id == s.id))
                    .filter_map(|s| s.attributes.as_ref()?.last_modified.as_ref())
                    .filter_map(|ts| DateTime::parse_from_rfc3339(ts).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .max()
            })
        } else {
            None
        };

        let b_latest = if let Some(relationships) = &b.relationships {
            relationships.recent_sessions.as_ref().and_then(|sessions| {
                active_sessions
                    .iter()
                    .filter(|s| sessions.iter().any(|ref_s| ref_s.id == s.id))
                    .filter_map(|s| s.attributes.as_ref()?.last_modified.as_ref())
                    .filter_map(|ts| DateTime::parse_from_rfc3339(ts).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .max()
            })
        } else {
            None
        };

        // Sort by most recent first (descending)
        match (b_latest, a_latest) {
            (Some(b_time), Some(a_time)) => b_time.cmp(&a_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => {
                // Fallback to name sorting
                if let (Some(a_attrs), Some(b_attrs)) = (&a.attributes, &b.attributes) {
                    a_attrs.name.cmp(&b_attrs.name)
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        }
    });

    json_api_response_with_headers(projects)
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
            json_api_response_with_headers(info)
        }
        Err(e) => json_api_error_response_with_headers(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Project Creation Failed".to_string(),
            e.to_string(),
        ),
    }
}
