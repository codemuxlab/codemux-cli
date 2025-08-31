// Re-export core JSON API types and functions for backward compatibility
pub use crate::core::{
    json_api_error, json_api_error_response_with_headers, json_api_response,
    json_api_response_with_headers, JsonApiDocument, JsonApiError, JsonApiErrorDocument,
    JsonApiResource, JsonApiResourceRef,
};
