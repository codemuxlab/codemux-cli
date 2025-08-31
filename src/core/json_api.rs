use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonApiDocument<T> {
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiResource<T, R = ()> {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: Option<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationships: Option<R>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[ts(export)]
pub struct ProjectRelationships {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_sessions: Option<Vec<SessionResourceTS>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonApiResourceRef {
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

/// JSON API error document
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JsonApiErrorDocument {
    pub errors: Vec<JsonApiError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub meta: Option<serde_json::Value>,
}

// Helper functions to create JSON API responses

/// Create a successful JSON API response
pub fn json_api_response<T>(data: T) -> JsonApiDocument<T> {
    JsonApiDocument { data }
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

// HTTP Response helpers
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

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

// Type aliases for common JSON API resources
pub type ProjectResource = JsonApiResource<crate::core::session::ProjectAttributes, ProjectRelationships>;
pub type SessionResource = JsonApiResource<crate::core::session::SessionAttributes, ()>;

// TypeScript-exported versions for frontend
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectResourceTS {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: Option<crate::core::session::ProjectAttributes>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationships: Option<ProjectRelationships>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionResourceTS {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: Option<crate::core::session::SessionAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectListResponse {
    pub data: Vec<ProjectResourceTS>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionResponse {
    pub data: SessionResourceTS,
}