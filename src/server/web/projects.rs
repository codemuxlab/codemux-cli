use axum::{extract::State, Json};

use super::types::{AddProjectRequest, AppState};
use crate::core::session::{ProjectInfo, ProjectWithSessions};

pub async fn list_projects(State(state): State<AppState>) -> Json<Vec<ProjectWithSessions>> {
    // Return actual projects with their sessions
    let projects = state.session_manager.list_projects().await;
    let sessions = state.session_manager.list_sessions().await;

    let projects_with_sessions = projects
        .into_iter()
        .map(|project| {
            let project_sessions = sessions
                .iter()
                .filter(|session| session.project.as_deref() == Some(&project.id))
                .cloned()
                .collect();

            ProjectWithSessions {
                id: project.id,
                name: project.name,
                path: project.path,
                sessions: project_sessions,
            }
        })
        .collect();

    Json(projects_with_sessions)
}

pub async fn add_project(
    State(state): State<AppState>,
    Json(req): Json<AddProjectRequest>,
) -> Result<Json<ProjectInfo>, String> {
    match state
        .session_manager
        .create_project(req.name, req.path)
        .await
    {
        Ok(info) => Ok(Json(info)),
        Err(e) => Err(e.to_string()),
    }
}
