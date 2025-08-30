use anyhow::Result;
use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use super::{
    git::{get_git_diff, get_git_file_diff, get_git_status},
    projects::{add_project, list_projects},
    sessions::{
        create_session, delete_session, get_session, shutdown_server, stream_session_jsonl,
    },
    static_files::{react_spa_handler, server_index, session_page, static_handler},
    types::AppState,
    websocket::websocket_handler,
};
use crate::server::manager::SessionManagerHandle;

pub async fn start_web_server(port: u16, session_manager: SessionManagerHandle) -> Result<()> {
    let state = AppState { session_manager };

    let app = Router::new()
        .route("/", get(server_index))
        .route("/session/:session_id", get(session_page))
        .route("/ws/:session_id", get(websocket_handler))
        .route("/api/sessions", axum::routing::post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id", axum::routing::delete(delete_session))
        .route("/api/sessions/:id/stream", get(stream_session_jsonl))
        .route("/api/sessions/:id/git/status", get(get_git_status))
        .route("/api/sessions/:id/git/diff", get(get_git_diff))
        .route("/api/sessions/:id/git/diff/*path", get(get_git_file_diff))
        .route("/api/projects", get(list_projects))
        .route("/api/projects", axum::routing::post(add_project))
        .route("/api/shutdown", axum::routing::post(shutdown_server))
        .route("/_expo/static/*path", get(static_handler))
        .route("/*path", get(react_spa_handler))
        .layer(
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            ),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("CodeMux web server listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}
