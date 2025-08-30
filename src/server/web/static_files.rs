use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

use super::types::AppState;
use crate::assets::embedded::ReactAssets;

pub async fn server_index() -> impl IntoResponse {
    serve_react_asset("index.html").await
}

pub async fn session_page(State(_state): State<AppState>) -> impl IntoResponse {
    // For server mode, serve React app
    serve_react_asset("index.html").await
}

pub async fn serve_react_asset(path: &str) -> impl IntoResponse {
    tracing::debug!("serve_react_asset called with path: '{}'", path);
    match ReactAssets::get(path) {
        Some(content) => {
            let body = Body::from(content.data.into_owned());
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            tracing::debug!("Found asset '{}', serving with mime: {}", path, mime);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        }
        None => {
            tracing::debug!("Asset '{}' not found, returning 404", path);
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not found"))
                .unwrap()
        }
    }
}

pub async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let file_path = format!("_expo/static/{}", path);
    tracing::debug!(
        "Static handler requested path: '{}', serving file: '{}'",
        path,
        file_path
    );
    serve_react_asset(&file_path).await
}

pub async fn react_spa_handler(Path(_path): Path<String>) -> impl IntoResponse {
    // For SPA routing, always serve index.html for non-API routes
    serve_react_asset("index.html").await
}
