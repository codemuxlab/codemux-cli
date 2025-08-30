use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// JSON API success document with data
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonApiDocument<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub meta: Option<serde_json::Value>,
}

/// JSON API error document
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonApiErrorDocument {
    pub errors: Vec<JsonApiError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub meta: Option<serde_json::Value>,
}

/// JSON API resource object
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonApiResource<T: TS> {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<HashMap<String, JsonApiRelationship>>,
}

/// JSON API relationship - supports both to-one and to-many relationships
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum JsonApiRelationship {
    ToOne { data: JsonApiResourceIdentifier },
    ToMany { data: Vec<JsonApiResourceIdentifier> },
}

/// JSON API resource identifier
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonApiResourceIdentifier {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
}

/// JSON API error object
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

impl From<ProjectWithSessions> for JsonApiResource<ProjectWithSessions> {
    fn from(project: ProjectWithSessions) -> Self {
        let mut relationships = HashMap::new();
        
        // Add sessions relationship
        if !project.sessions.is_empty() {
            let session_identifiers: Vec<JsonApiResourceIdentifier> = project.sessions
                .iter()
                .map(|s| JsonApiResourceIdentifier {
                    resource_type: "session".to_string(),
                    id: s.id.clone(),
                })
                .collect();
            
            relationships.insert(
                "sessions".to_string(),
                JsonApiRelationship::ToMany {
                    data: session_identifiers,
                },
            );
        }
        
        JsonApiResource {
            resource_type: "project".to_string(),
            id: project.id.clone(),
            attributes: Some(project.clone()),
            relationships: if relationships.is_empty() { None } else { Some(relationships) },
        }
    }
}

impl From<SessionInfo> for JsonApiResource<SessionInfo> {
    fn from(session: SessionInfo) -> Self {
        let mut relationships = HashMap::new();
        
        // Add project relationship if exists
        if let Some(project_id) = &session.project {
            relationships.insert(
                "project".to_string(),
                JsonApiRelationship::ToOne {
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
            attributes: Some(session.clone()),
            relationships: if relationships.is_empty() { None } else { Some(relationships) },
        }
    }
}

impl From<ProjectInfo> for JsonApiResource<ProjectInfo> {
    fn from(project: ProjectInfo) -> Self {
        JsonApiResource {
            resource_type: "project".to_string(),
            id: project.id.clone(),
            attributes: Some(project.clone()),
            relationships: None,
        }
    }
}

/// Create a successful JSON API response
pub fn json_api_response<T>(data: T) -> JsonApiDocument<T> {
    JsonApiDocument {
        data,
        meta: None,
    }
}

/// Create an error JSON API response
pub fn json_api_error(status: String, title: String, detail: String) -> JsonApiErrorDocument {
    JsonApiErrorDocument {
        errors: vec![JsonApiError {
            status: Some(status),
            title: Some(title),
            detail: Some(detail),
        }],
        meta: None,
    }
}

/// Create a JSON API response with proper Content-Type header
pub fn json_api_response_with_headers<T>(data: T) -> Response
where
    T: Serialize,
{
    let document = json_api_response(data);
    let mut response = Json(document).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/vnd.api+json".parse().unwrap(),
    );
    response
}

/// Create a JSON API error response with proper Content-Type header
pub fn json_api_error_response_with_headers(
    status: StatusCode,
    title: String,
    detail: String,
) -> Response {
    let document = json_api_error(status.as_u16().to_string(), title, detail);
    let mut response = (status, Json(document)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/vnd.api+json".parse().unwrap(),
    );
    response
}