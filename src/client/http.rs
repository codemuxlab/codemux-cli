use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::core::pty_session::{GridUpdateMessage, PtyInputMessage};
use crate::core::{
    ClientMessage, Config, JsonApiDocument, ProjectResource, ServerMessage, SessionResource,
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

#[derive(Debug, Clone)]
pub struct ReconnectionConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            base_delay_ms: 5000, // Start at 5 seconds
            max_delay_ms: 30000, // Max 30 seconds
            backoff_factor: 2.0, // Power of 2
        }
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
    ) -> Result<SessionResource> {
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

        tracing::debug!("POST /api/sessions response status: {}", response.status());

        let response_text = response.text().await?;
        let json_api: JsonApiDocument<SessionResource> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse session response: {}", e))?;
        let session_resource = json_api.data;

        tracing::debug!("Parsed session resource: {:?}", session_resource);
        Ok(session_resource)
    }

    /// Create a new session on the server with explicit path
    pub async fn create_session_with_path(
        &self,
        agent: String,
        args: Vec<String>,
        path: String,
    ) -> Result<SessionResource> {
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

        tracing::debug!("POST /api/sessions response status: {}", response.status());

        let response_text = response.text().await?;
        let json_api: JsonApiDocument<SessionResource> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse session response: {}", e))?;
        let session_resource = json_api.data;

        tracing::debug!("Parsed session resource: {:?}", session_resource);
        Ok(session_resource)
    }

    /// Get session information
    pub async fn get_session(&self, session_id: &str) -> Result<SessionResource> {
        let response = self
            .client
            .get(format!("{}/api/sessions/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get session: {}", response.status()));
        }

        let response_text = response.text().await?;
        let json_api: JsonApiDocument<SessionResource> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse session response: {}", e))?;
        let session_resource = json_api.data;
        Ok(session_resource)
    }

    /// List all sessions (extracted from project relationships)
    pub async fn list_sessions(&self) -> Result<Vec<SessionResource>> {
        let projects = self.list_projects().await?;

        // Collect all session IDs from project relationships
        let mut session_ids = Vec::new();
        for project_resource in projects {
            if let Some(relationships) = &project_resource.relationships {
                if let Some(recent_sessions) = &relationships.recent_sessions {
                    for session_ref in recent_sessions {
                        session_ids.push(session_ref.id.clone());
                    }
                }
            }
        }

        // Fetch each session individually
        let mut all_sessions = Vec::new();
        for session_id in session_ids {
            match self.get_session(&session_id).await {
                Ok(session) => all_sessions.push(session),
                Err(e) => {
                    tracing::warn!("Failed to fetch session {}: {}", session_id, e);
                    // Continue with other sessions
                }
            }
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
    pub async fn create_project(&self, name: String, path: String) -> Result<ProjectResource> {
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

        let response_text = response.text().await?;
        let json_api: JsonApiDocument<ProjectResource> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse project response: {}", e))?;
        let project_resource = json_api.data;
        Ok(project_resource)
    }

    /// List all projects
    pub async fn list_projects(&self) -> Result<Vec<ProjectResource>> {
        let response = self
            .client
            .get(format!("{}/api/projects", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to list projects: {}", response.status()));
        }

        let response_text = response.text().await?;
        let json_api: JsonApiDocument<Vec<ProjectResource>> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse JSON API resource array response: {}", e))?;

        Ok(json_api.data)
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

        for project_resource in projects {
            if let Some(project_attrs) = project_resource.attributes {
                let project_path = std::path::PathBuf::from(&project_attrs.path);
                let canonical_project = project_path.canonicalize().ok();

                // Try exact match first
                if project_attrs.path == path_input {
                    return Ok(Some(project_resource.id));
                }

                // Try canonical path match (handles symlinks, .., etc.)
                if let (Some(canonical_input), Some(canonical_project)) =
                    (&canonical_input, &canonical_project)
                {
                    if canonical_input == canonical_project {
                        return Ok(Some(project_resource.id));
                    }
                }

                // Try path contains match (for subdirectories)
                if let Some(canonical_input) = &canonical_input {
                    if let Some(canonical_project) = &canonical_project {
                        if canonical_input.starts_with(canonical_project) {
                            return Ok(Some(project_resource.id));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Connect to a session via WebSocket
    pub async fn connect_to_session(&self, session_id: &str) -> Result<SessionConnection> {
        let config = ReconnectionConfig::default();
        self.connect_to_session_with_config(session_id, config)
            .await
    }

    /// Connect to a session via WebSocket with custom reconnection configuration
    pub async fn connect_to_session_with_config(
        &self,
        session_id: &str,
        config: ReconnectionConfig,
    ) -> Result<SessionConnection> {
        let ws_url = format!(
            "ws://localhost:{}/ws/{}",
            self.base_url.trim_start_matches("http://localhost:"),
            session_id
        );

        // Try to connect with exponential backoff
        for attempt in 0..=config.max_attempts {
            match connect_async(&ws_url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!(
                        "WebSocket connected to session {} (attempt {})",
                        session_id,
                        attempt + 1
                    );
                    return Ok(SessionConnection::new(ws_stream, session_id.to_string()));
                }
                Err(e) => {
                    if attempt < config.max_attempts {
                        let delay_ms = (config.base_delay_ms as f64
                            * config.backoff_factor.powi(attempt as i32))
                        .min(config.max_delay_ms as f64)
                            as u64;

                        // Add jitter to prevent thundering herd
                        let jitter = (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                            % 1000) as u64;
                        let delay_with_jitter = Duration::from_millis(delay_ms + jitter);

                        tracing::warn!(
                            "WebSocket connection attempt {} failed: {}. Retrying in {:.1}s (attempt {}/{})",
                            attempt + 1,
                            e,
                            delay_with_jitter.as_secs_f64(),
                            attempt + 1,
                            config.max_attempts
                        );

                        sleep(delay_with_jitter).await;
                    } else {
                        tracing::error!(
                            "WebSocket connection failed after {} attempts: {}",
                            config.max_attempts + 1,
                            e
                        );
                        return Err(anyhow!(
                            "Failed to connect to WebSocket after {} attempts: {}",
                            config.max_attempts + 1,
                            e
                        ));
                    }
                }
            }
        }

        unreachable!()
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
        use crate::core::pty_session::{
            ConnectionStatus, PtyChannels, PtyControlMessage, PtyOutputMessage,
        };
        use futures_util::{SinkExt, StreamExt};

        // Create channels for PTY communication
        let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<PtyInputMessage>();
        let (output_tx, _output_rx) = tokio::sync::broadcast::channel::<PtyOutputMessage>(100);
        let (grid_tx, _grid_rx) = tokio::sync::broadcast::channel::<GridUpdateMessage>(100);
        let (control_tx, mut control_rx) =
            tokio::sync::mpsc::unbounded_channel::<PtyControlMessage>();
        let (size_tx, _size_rx) = tokio::sync::broadcast::channel::<portable_pty::PtySize>(10);
        let (connection_status_tx, _connection_status_rx) =
            tokio::sync::broadcast::channel::<ConnectionStatus>(10);

        let ws_stream = self.ws_stream;
        let session_id = self.session_id.clone();

        // Clone the broadcast senders for use in the spawn task
        let output_tx_clone = output_tx.clone();
        let grid_tx_clone = grid_tx.clone();
        let connection_status_tx_clone = connection_status_tx.clone();

        // Spawn task to handle WebSocket -> PTY channel forwarding with auto-reconnection
        tokio::spawn(async move {
            let reconnect_config = ReconnectionConfig::default();
            let mut current_ws = ws_stream;
            let mut reconnect_attempt = 0u32;
            let should_reconnect = true;

            // Send initial connected status
            let _ = connection_status_tx_clone.send(ConnectionStatus::Connected);

            // Helper function to attempt reconnection
            async fn attempt_reconnect(
                attempt: u32,
                session_id: &str,
                reconnect_config: &ReconnectionConfig,
                status_tx: &tokio::sync::broadcast::Sender<ConnectionStatus>,
            ) -> Option<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            > {
                if attempt >= reconnect_config.max_attempts {
                    tracing::error!(
                        "Max reconnection attempts reached for session {}",
                        session_id
                    );
                    let _ = status_tx.send(ConnectionStatus::Disconnected);
                    return None;
                }

                // Send reconnecting status
                let _ = status_tx.send(ConnectionStatus::Reconnecting {
                    attempt: attempt + 1,
                    max_attempts: reconnect_config.max_attempts,
                });

                let delay_ms = (reconnect_config.base_delay_ms as f64
                    * reconnect_config.backoff_factor.powi(attempt as i32))
                .min(reconnect_config.max_delay_ms as f64) as u64;

                // Add jitter to prevent thundering herd
                let jitter = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    % 1000) as u64;
                let delay_with_jitter = Duration::from_millis(delay_ms + jitter);

                tracing::warn!(
                    "WebSocket disconnected for session {}. Reconnecting in {:.1}s (attempt {}/{})",
                    session_id,
                    delay_with_jitter.as_secs_f64(),
                    attempt + 1,
                    reconnect_config.max_attempts
                );

                sleep(delay_with_jitter).await;

                let ws_url = format!("ws://localhost:{}/ws/{}", crate::core::config::default_server_port(), session_id);
                match connect_async(&ws_url).await {
                    Ok((new_ws, _)) => {
                        tracing::info!(
                            "WebSocket reconnected to session {} (attempt {})",
                            session_id,
                            attempt + 1
                        );
                        let _ = status_tx.send(ConnectionStatus::Connected);
                        Some(new_ws)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Reconnection attempt {} failed for session {}: {}",
                            attempt + 1,
                            session_id,
                            e
                        );
                        None
                    }
                }
            }

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
                            if current_ws.send(Message::Text(json)).await.is_err() {
                                tracing::error!("Failed to send input via client WebSocket - connection lost");
                                // Trigger reconnection
                                if should_reconnect {
                                    if let Some(new_ws) = attempt_reconnect(reconnect_attempt, &session_id, &reconnect_config, &connection_status_tx_clone).await {
                                        current_ws = new_ws;
                                        reconnect_attempt = 0; // Reset counter on successful reconnection
                                        continue;
                                    } else {
                                        reconnect_attempt += 1;
                                        if reconnect_attempt >= reconnect_config.max_attempts {
                                            break;
                                        }
                                    }
                                } else {
                                    break;
                                }
                            } else {
                                // Reset reconnection attempt counter on successful send
                                reconnect_attempt = 0;
                            }
                        }
                    }

                    // Handle control messages from TUI -> WebSocket
                    Some(control_msg) = control_rx.recv() => {
                        match control_msg {
                            PtyControlMessage::Resize { rows, cols } => {
                                let client_msg = ClientMessage::Resize { rows, cols };
                                if let Ok(json) = serde_json::to_string(&client_msg) {
                                    if current_ws.send(Message::Text(json)).await.is_err() {
                                        // Trigger reconnection on control message failure
                                        if should_reconnect {
                                            if let Some(new_ws) = attempt_reconnect(reconnect_attempt, &session_id, &reconnect_config, &connection_status_tx_clone).await {
                                                current_ws = new_ws;
                                                reconnect_attempt = 0;
                                                continue;
                                            } else {
                                                reconnect_attempt += 1;
                                                if reconnect_attempt >= reconnect_config.max_attempts {
                                                    break;
                                                }
                                            }
                                        } else {
                                            break;
                                        }
                                    } else {
                                        reconnect_attempt = 0;
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
                                let _ = current_ws.close(None).await;
                                break;
                            }
                        }
                    }

                    // Handle messages from WebSocket -> PTY channels
                    msg = current_ws.next() => {
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
                                // Reset reconnection counter on successful message receive
                                reconnect_attempt = 0;
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                tracing::info!("WebSocket connection closed for session {}", session_id);
                                // Attempt to reconnect unless explicitly terminated
                                if should_reconnect {
                                    if let Some(new_ws) = attempt_reconnect(reconnect_attempt, &session_id, &reconnect_config, &connection_status_tx_clone).await {
                                        current_ws = new_ws;
                                        reconnect_attempt = 0;
                                        tracing::info!("Successfully reconnected to session {}", session_id);
                                        continue;
                                    } else {
                                        reconnect_attempt += 1;
                                        if reconnect_attempt >= reconnect_config.max_attempts {
                                            tracing::error!("Max reconnection attempts reached, giving up on session {}", session_id);
                                            break;
                                        }
                                    }
                                } else {
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("WebSocket error for session {}: {}", session_id, e);
                                // Attempt to reconnect on error
                                if should_reconnect {
                                    if let Some(new_ws) = attempt_reconnect(reconnect_attempt, &session_id, &reconnect_config, &connection_status_tx_clone).await {
                                        current_ws = new_ws;
                                        reconnect_attempt = 0;
                                        tracing::info!("Successfully reconnected after error to session {}", session_id);
                                        continue;
                                    } else {
                                        reconnect_attempt += 1;
                                        if reconnect_attempt >= reconnect_config.max_attempts {
                                            tracing::error!("Max reconnection attempts reached after error, giving up on session {}", session_id);
                                            break;
                                        }
                                    }
                                } else {
                                    break;
                                }
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
            connection_status_tx,
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
