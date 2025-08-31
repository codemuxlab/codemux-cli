use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::core::pty_session::{GridUpdateMessage, PtyInputMessage};
use crate::core::{
    ClientMessage, Config, ProjectInfo, ProjectWithSessions, ServerMessage, SessionInfo,
};

#[derive(Debug, Clone)]
pub struct CodeMuxClient {
    base_url: String,
    client: Client,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionRequest {
    pub agent: String,
    pub args: Vec<String>,
    pub project_id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
struct JsonApiDocument<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct JsonApiResource<T> {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    resource_type: String,
    #[allow(dead_code)]
    id: String,
    attributes: Option<T>,
}

impl CodeMuxClient {
    /// Extract data from JSON API response
    #[allow(dead_code)]
    async fn extract_from_json_api_response<T>(response: reqwest::Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response_text = response.text().await?;

        // Try JSON API format first
        if let Ok(json_api) = serde_json::from_str::<JsonApiDocument<T>>(&response_text) {
            return Ok(json_api.data);
        }

        // If that failed, try direct format (backwards compatibility)
        let data: T = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        Ok(data)
    }

    /// Extract data from JSON API resource response
    async fn extract_from_json_api_resource<T>(response: reqwest::Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response_text = response.text().await?;

        // Try JSON API resource format first
        if let Ok(json_api) =
            serde_json::from_str::<JsonApiDocument<JsonApiResource<T>>>(&response_text)
        {
            if let Some(attributes) = json_api.data.attributes {
                return Ok(attributes);
            }
        }

        // If that failed, try direct format (backwards compatibility)
        let data: T = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        Ok(data)
    }

    /// Extract array data from JSON API resource array response
    async fn extract_from_json_api_resource_array<T>(response: reqwest::Response) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response_text = response.text().await?;

        // Try JSON API resource array format first
        if let Ok(json_api) =
            serde_json::from_str::<JsonApiDocument<Vec<JsonApiResource<T>>>>(&response_text)
        {
            let mut results = Vec::new();
            for resource in json_api.data {
                if let Some(attributes) = resource.attributes {
                    results.push(attributes);
                }
            }
            return Ok(results);
        }

        // If that failed, try direct format (backwards compatibility)
        let data: Vec<T> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        Ok(data)
    }
}

impl CodeMuxClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, client }
    }

    pub fn from_config(config: &Config) -> Self {
        let base_url = format!("http://localhost:{}", config.server.port);
        Self::new(base_url)
    }

    /// Check if server is running by trying to connect
    pub async fn is_server_running(&self) -> bool {
        self.client
            .get(format!("{}/api/projects", self.base_url))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok()
    }

    /// Create a new session on the server
    pub async fn create_session(
        &self,
        agent: String,
        args: Vec<String>,
        project_id: Option<String>,
    ) -> Result<SessionInfo> {
        let request = CreateSessionRequest {
            agent: agent.clone(),
            args: args.clone(),
            project_id: project_id.clone(),
            path: None,
        };

        tracing::debug!("POST /api/sessions request body: {:?}", request);
        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::debug!("POST /api/sessions JSON body:\n{}", json);
        }

        let url = format!("{}/api/sessions", self.base_url);
        tracing::debug!("Making POST request to: {}", url);

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        tracing::debug!("POST /api/sessions response status: {}", status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!(
                "Session creation failed with status {}: {}",
                status,
                error_text
            );
            return Err(anyhow!(
                "Failed to create session: {} - {}",
                status,
                error_text
            ));
        }

        let response_text = response.text().await?;
        tracing::debug!("POST /api/sessions response body: {}", response_text);

        let session: SessionInfo = serde_json::from_str(&response_text).map_err(|e| {
            anyhow!(
                "Failed to parse session response: {} - Response: {}",
                e,
                response_text
            )
        })?;

        tracing::debug!("Parsed session info: {:?}", session);
        Ok(session)
    }

    /// Create a new session on the server with explicit path
    pub async fn create_session_with_path(
        &self,
        agent: String,
        args: Vec<String>,
        path: String,
    ) -> Result<SessionInfo> {
        let request = CreateSessionRequest {
            agent: agent.clone(),
            args: args.clone(),
            project_id: None,
            path: Some(path.clone()),
        };

        tracing::debug!("POST /api/sessions request body: {:?}", request);
        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::debug!("POST /api/sessions JSON body:\n{}", json);
        }

        let url = format!("{}/api/sessions", self.base_url);
        tracing::debug!("Making POST request to: {}", url);

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        tracing::debug!("POST /api/sessions response status: {}", status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!(
                "Session creation failed with status {}: {}",
                status,
                error_text
            );
            return Err(anyhow!(
                "Failed to create session: {} - {}",
                status,
                error_text
            ));
        }

        let response_text = response.text().await?;
        tracing::debug!("POST /api/sessions response body: {}", response_text);

        let session: SessionInfo = serde_json::from_str(&response_text).map_err(|e| {
            anyhow!(
                "Failed to parse session response: {} - Response: {}",
                e,
                response_text
            )
        })?;

        tracing::debug!("Parsed session info: {:?}", session);
        Ok(session)
    }

    /// Get session information
    pub async fn get_session(&self, session_id: &str) -> Result<SessionInfo> {
        let response = self
            .client
            .get(format!("{}/api/sessions/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get session: {}", response.status()));
        }

        let session: SessionInfo = response.json().await?;
        Ok(session)
    }

    /// List all sessions (extracted from projects)
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let projects = self.list_projects().await?;

        // Extract all sessions from all projects
        let mut all_sessions = Vec::new();
        for project in projects {
            all_sessions.extend(project.sessions);
        }

        Ok(all_sessions)
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let response = self
            .client
            .delete(format!("{}/api/sessions/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to delete session: {}", response.status()));
        }

        Ok(())
    }

    /// Create a new project
    pub async fn create_project(&self, name: String, path: String) -> Result<ProjectInfo> {
        let request = CreateProjectRequest { name, path };

        let response = self
            .client
            .post(format!("{}/api/projects", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to create project: {}", response.status()));
        }

        let project = Self::extract_from_json_api_resource(response).await?;
        Ok(project)
    }

    /// List all projects
    pub async fn list_projects(&self) -> Result<Vec<ProjectWithSessions>> {
        let response = self
            .client
            .get(format!("{}/api/projects", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to list projects: {}", response.status()));
        }

        let projects = Self::extract_from_json_api_resource_array(response).await?;
        Ok(projects)
    }

    /// Resolve a directory path to a project ID
    /// Accepts both absolute paths and relative paths (resolved from current directory)
    /// Special case: "." resolves to current directory
    pub async fn resolve_project_path(&self, path_input: &str) -> Result<Option<String>> {
        use std::path::Path;

        // Handle special case for current directory
        let resolved_path_string;
        let path_input = if path_input == "." {
            resolved_path_string = std::env::current_dir()?.to_string_lossy().to_string();
            &resolved_path_string
        } else {
            path_input
        };

        // Convert input to absolute path
        let input_path = if Path::new(path_input).is_absolute() {
            std::path::PathBuf::from(path_input)
        } else {
            std::env::current_dir()?.join(path_input)
        };

        // Canonicalize to resolve symlinks and normalize
        let canonical_input = input_path.canonicalize().ok();

        // Get all projects and find matching path
        let projects = self.list_projects().await?;

        for project in projects {
            let project_path = std::path::PathBuf::from(&project.path);
            let canonical_project = project_path.canonicalize().ok();

            // Try exact match first
            if project.path == path_input {
                return Ok(Some(project.id));
            }

            // Try canonical path match (handles symlinks, .., etc.)
            if let (Some(canonical_input), Some(canonical_project)) =
                (&canonical_input, &canonical_project)
            {
                if canonical_input == canonical_project {
                    return Ok(Some(project.id));
                }
            }

            // Try path contains match (for subdirectories)
            if let Some(canonical_input) = &canonical_input {
                if let Some(canonical_project) = &canonical_project {
                    if canonical_input.starts_with(canonical_project) {
                        return Ok(Some(project.id));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Connect to a session via WebSocket
    pub async fn connect_to_session(&self, session_id: &str) -> Result<SessionConnection> {
        let ws_url = format!(
            "ws://localhost:{}/ws/{}",
            self.base_url.trim_start_matches("http://localhost:"),
            session_id
        );

        let (ws_stream, _) = connect_async(&ws_url).await?;

        Ok(SessionConnection::new(ws_stream, session_id.to_string()))
    }

    /// Get the web interface URL for a session
    pub fn get_session_url(&self, session_id: &str) -> String {
        format!("{}/session/{}", self.base_url, session_id)
    }

    /// Shutdown the server
    pub async fn shutdown_server(&self) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/api/shutdown", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to shutdown server: {}", response.status()));
        }

        Ok(())
    }
}

/// WebSocket connection to a specific session
pub struct SessionConnection {
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    session_id: String,
}

impl SessionConnection {
    fn new(
        ws_stream: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        session_id: String,
    ) -> Self {
        Self {
            ws_stream,
            session_id,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Convert WebSocket connection into PTY-like channels for TUI
    pub fn into_pty_channels(self) -> crate::core::pty_session::PtyChannels {
        use crate::core::pty_session::{PtyChannels, PtyControlMessage, PtyOutputMessage};
        use futures_util::{SinkExt, StreamExt};

        // Create channels for PTY communication
        let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<PtyInputMessage>();
        let (output_tx, _output_rx) = tokio::sync::broadcast::channel::<PtyOutputMessage>(100);
        let (grid_tx, _grid_rx) = tokio::sync::broadcast::channel::<GridUpdateMessage>(100);
        let (control_tx, mut control_rx) =
            tokio::sync::mpsc::unbounded_channel::<PtyControlMessage>();
        let (size_tx, _size_rx) = tokio::sync::broadcast::channel::<portable_pty::PtySize>(10);

        let mut ws_stream = self.ws_stream;
        let session_id = self.session_id.clone();

        // Clone the broadcast senders for use in the spawn task
        let output_tx_clone = output_tx.clone();
        let grid_tx_clone = grid_tx.clone();

        // Spawn task to handle WebSocket -> PTY channel forwarding
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle input from TUI -> WebSocket
                    Some(input_msg) = input_rx.recv() => {
                        // Handle both Key and Scroll events
                        let client_msg = match input_msg.input {
                            crate::core::pty_session::PtyInput::Key { event, .. } => {
                                ClientMessage::Key {
                                    code: event.code,
                                    modifiers: event.modifiers
                                }
                            }
                            crate::core::pty_session::PtyInput::Scroll { direction, lines, .. } => {
                                ClientMessage::Scroll { direction, lines }
                            }
                        };

                        if let Ok(json) = serde_json::to_string(&client_msg) {
                            tracing::trace!("Client WebSocket sending input: {} chars", json.len());
                            if ws_stream.send(Message::Text(json)).await.is_err() {
                                tracing::error!("Failed to send input via client WebSocket");
                                break;
                            }
                        }
                    }

                    // Handle control messages from TUI -> WebSocket
                    Some(control_msg) = control_rx.recv() => {
                        match control_msg {
                            PtyControlMessage::Resize { rows, cols } => {
                                let client_msg = ClientMessage::Resize { rows, cols };
                                if let Ok(json) = serde_json::to_string(&client_msg) {
                                    if ws_stream.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            PtyControlMessage::RequestKeyframe { response_tx } => {
                                // Client should not request keyframes - server sends them automatically
                                tracing::warn!("Client received RequestKeyframe - ignoring as server handles keyframes automatically");
                                drop(response_tx);
                            }
                            PtyControlMessage::Terminate => {
                                // Send close message and break
                                let _ = ws_stream.close(None).await;
                                break;
                            }
                        }
                    }

                    // Handle messages from WebSocket -> PTY channels
                    msg = ws_stream.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                tracing::trace!("Client WebSocket received message: {} chars", text.len());
                                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                    match server_msg {
                                        ServerMessage::Output { data, timestamp } => {
                                            tracing::debug!("Client WebSocket forwarding output to PTY channel");
                                            let output_msg = crate::core::pty_session::PtyOutputMessage { data, timestamp };
                                            let _ = output_tx_clone.send(output_msg);
                                        }
                                        ServerMessage::GridUpdate { update } => {
                                            tracing::debug!("Client WebSocket forwarding grid update to PTY channel");
                                            let _ = grid_tx_clone.send(update);
                                        }
                                        ServerMessage::PtySize { rows, cols } => {
                                            tracing::debug!("Client WebSocket received PTY size: {}x{}", cols, rows);
                                            // Forward size update if needed
                                        }
                                        ServerMessage::Error { message } => {
                                            tracing::error!("Server error: {}", message);
                                        }
                                    }
                                } else {
                                    tracing::warn!("Failed to parse WebSocket message: {}", text);
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                tracing::info!("WebSocket connection closed for session {}", session_id);
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {} // Ignore other message types
                        }
                    }
                }
            }
        });

        PtyChannels {
            input_tx,
            output_tx,
            control_tx,
            size_tx,
            grid_tx,
        }
    }

    /// Send a message to the server
    pub async fn send_message(&mut self, message: ClientMessage) -> Result<()> {
        use futures_util::SinkExt;

        let json = serde_json::to_string(&message)?;
        self.ws_stream.send(Message::Text(json)).await?;
        Ok(())
    }

    /// Receive a message from the server
    pub async fn receive_message(&mut self) -> Result<Option<ServerMessage>> {
        use futures_util::StreamExt;

        loop {
            match self.ws_stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let message: ServerMessage = serde_json::from_str(&text)?;
                    return Ok(Some(message));
                }
                Some(Ok(Message::Close(_))) => return Ok(None),
                Some(Ok(Message::Binary(_))) => {
                    // Skip binary messages, continue loop
                    continue;
                }
                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                    // Skip ping/pong messages, continue loop
                    continue;
                }
                Some(Ok(Message::Frame(_))) => {
                    // Skip frame messages, continue loop
                    continue;
                }
                Some(Err(e)) => return Err(anyhow!("WebSocket error: {}", e)),
                None => return Ok(None),
            }
        }
    }

    /// Send PTY input to the session
    pub async fn send_input(&mut self, input: PtyInputMessage) -> Result<()> {
        let client_msg = match input.input {
            crate::core::pty_session::PtyInput::Key { event, .. } => ClientMessage::Key {
                code: event.code,
                modifiers: event.modifiers,
            },
            crate::core::pty_session::PtyInput::Scroll {
                direction, lines, ..
            } => ClientMessage::Scroll { direction, lines },
        };
        self.send_message(client_msg).await
    }

    /// Send resize event to the session
    pub async fn send_resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.send_message(ClientMessage::Resize { rows, cols })
            .await
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        use futures_util::SinkExt;

        self.ws_stream.send(Message::Close(None)).await?;
        Ok(())
    }
}
