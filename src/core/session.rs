use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub project: Option<String>,
    pub status: String,
    pub session_type: SessionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SessionType {
    Active,
    Historical,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectWithSessions {
    pub id: String,
    pub name: String,
    pub path: String,
    pub sessions: Vec<SessionInfo>,
}

