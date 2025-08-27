use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub project: Option<String>,
    pub status: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectWithSessions {
    pub id: String,
    pub name: String,
    pub path: String,
    pub sessions: Vec<SessionInfo>,
}
