use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::core::{Config, SessionInfo, ProjectInfo, ProjectWithSessions};
use crate::core::pty_session::{PtyInputMessage, PtyOutputMessage, GridUpdateMessage};

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
}

#[derive(Debug, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: String,
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
        self.client.get(format!("{}/api/projects", self.base_url))
            .timeout(Duration::from_secs(2))
            .send()
            .await.is_ok()
    }
    
    /// Create a new session on the server
    pub async fn create_session(
        &self,
        agent: String,
        args: Vec<String>,
        project_id: Option<String>,
    ) -> Result<SessionInfo> {
        let request = CreateSessionRequest {
            agent,
            args,
            project_id,
        };
        
        let response = self.client
            .post(format!("{}/api/sessions", self.base_url))
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Failed to create session: {}", response.status()));
        }
        
        let session: SessionInfo = response.json().await?;
        Ok(session)
    }
    
    /// Get session information
    pub async fn get_session(&self, session_id: &str) -> Result<SessionInfo> {
        let response = self.client
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
        let response = self.client
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
        
        let response = self.client
            .post(format!("{}/api/projects", self.base_url))
            .json(&request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Failed to create project: {}", response.status()));
        }
        
        let project: ProjectInfo = response.json().await?;
        Ok(project)
    }
    
    /// List all projects
    pub async fn list_projects(&self) -> Result<Vec<ProjectWithSessions>> {
        let response = self.client
            .get(format!("{}/api/projects", self.base_url))
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Failed to list projects: {}", response.status()));
        }
        
        let projects: Vec<ProjectWithSessions> = response.json().await?;
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
            if let (Some(canonical_input), Some(canonical_project)) = (&canonical_input, &canonical_project) {
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
        let ws_url = format!("ws://localhost:{}/ws/{}", 
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
}

/// WebSocket connection to a specific session
pub struct SessionConnection {
    ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Input(PtyInputMessage),
    Resize { rows: u16, cols: u16 },
    RequestKeyframe,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Output(PtyOutputMessage),
    Grid(GridUpdateMessage),
    Size { rows: u16, cols: u16 },
    Error(String),
}

impl SessionConnection {
    fn new(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
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
        self.send_message(ClientMessage::Input(input)).await
    }
    
    /// Send resize event to the session
    pub async fn send_resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.send_message(ClientMessage::Resize { rows, cols }).await
    }
    
    /// Request a keyframe (full terminal state)
    pub async fn request_keyframe(&mut self) -> Result<()> {
        self.send_message(ClientMessage::RequestKeyframe).await
    }
    
    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        use futures_util::SinkExt;
        
        self.ws_stream.send(Message::Close(None)).await?;
        Ok(())
    }
}