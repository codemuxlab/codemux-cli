use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionAttributes {
    pub agent: String,
    pub project: Option<String>,
    pub status: String,
    pub session_type: SessionType,
    pub last_modified: Option<String>, // ISO 8601 timestamp string
    pub last_message: Option<String>,  // Most recent message from session
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SessionType {
    Active,
    Historical,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectAttributes {
    pub name: String,
    pub path: String,
}
