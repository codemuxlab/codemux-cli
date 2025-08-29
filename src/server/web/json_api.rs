use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON API top-level document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiDocument<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<JsonApiError>>,
}

/// JSON API resource object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiResource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<HashMap<String, JsonApiRelationship>>,
}

/// JSON API relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiRelationship {
    pub data: JsonApiResourceIdentifier,
}

/// JSON API resource identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiResourceIdentifier {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
}

/// JSON API error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// Helper functions to create JSON API responses

use crate::core::session::{ProjectWithSessions, ProjectInfo, SessionInfo};

impl From<ProjectWithSessions> for JsonApiResource {
    fn from(project: ProjectWithSessions) -> Self {
        let mut relationships = HashMap::new();
        
        // Add sessions relationship
        if !project.sessions.is_empty() {
            let _session_identifiers: Vec<JsonApiResourceIdentifier> = project.sessions
                .iter()
                .map(|s| JsonApiResourceIdentifier {
                    resource_type: "session".to_string(),
                    id: s.id.clone(),
                })
                .collect();
            
            relationships.insert(
                "sessions".to_string(),
                JsonApiRelationship {
                    data: JsonApiResourceIdentifier {
                        resource_type: "sessions".to_string(),
                        id: format!("{}_sessions", project.id),
                    },
                },
            );
        }
        
        JsonApiResource {
            resource_type: "project".to_string(),
            id: project.id.clone(),
            attributes: Some(serde_json::json!({
                "name": project.name,
                "path": project.path,
                "sessions": project.sessions,
            })),
            relationships: if relationships.is_empty() { None } else { Some(relationships) },
        }
    }
}

impl From<SessionInfo> for JsonApiResource {
    fn from(session: SessionInfo) -> Self {
        let mut relationships = HashMap::new();
        
        // Add project relationship if exists
        if let Some(project_id) = &session.project {
            relationships.insert(
                "project".to_string(),
                JsonApiRelationship {
                    data: JsonApiResourceIdentifier {
                        resource_type: "project".to_string(),
                        id: project_id.clone(),
                    },
                },
            );
        }
        
        JsonApiResource {
            resource_type: "session".to_string(),
            id: session.id.clone(),
            attributes: Some(serde_json::json!({
                "agent": session.agent,
                "status": session.status,
                "sessionType": session.session_type,
            })),
            relationships: if relationships.is_empty() { None } else { Some(relationships) },
        }
    }
}

impl From<ProjectInfo> for JsonApiResource {
    fn from(project: ProjectInfo) -> Self {
        JsonApiResource {
            resource_type: "project".to_string(),
            id: project.id.clone(),
            attributes: Some(serde_json::json!({
                "name": project.name,
                "path": project.path,
            })),
            relationships: None,
        }
    }
}

/// Create a successful JSON API response
pub fn json_api_response<T>(data: T) -> JsonApiDocument<T> {
    JsonApiDocument {
        data,
        meta: None,
        errors: None,
    }
}

/// Create an error JSON API response
pub fn json_api_error(status: String, title: String, detail: String) -> JsonApiDocument<()> {
    JsonApiDocument {
        data: (),
        meta: None,
        errors: Some(vec![JsonApiError {
            status: Some(status),
            title: Some(title),
            detail: Some(detail),
        }]),
    }
}