use serde::{Deserialize, Serialize};

use crate::server::manager::SessionManagerHandle;

#[derive(Clone)]
pub struct AppState {
    pub session_manager: SessionManagerHandle,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub agent: String,
    pub args: Vec<String>,
    pub project_id: Option<String>,
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct AddProjectRequest {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct GitStatus {
    pub files: Vec<GitFileStatus>,
    pub branch: Option<String>,
    pub clean: bool,
}

#[derive(Debug, Serialize)]
pub struct GitDiff {
    pub files: Vec<GitFileDiff>,
}

#[derive(Debug, Serialize)]
pub struct GitFileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub diff: String,
}
