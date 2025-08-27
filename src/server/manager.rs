use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::core::{
    pty_session::{PtyChannels, PtySession},
    session::{ProjectInfo, SessionInfo},
    Config,
};

// Commands that can be sent to the SessionManager actor
pub enum SessionCommand {
    CreateSession {
        agent: String,
        args: Vec<String>,
        project_id: Option<String>,
        path: Option<String>,
        response_tx: oneshot::Sender<Result<SessionInfo>>,
    },
    GetSession {
        session_id: String,
        response_tx: oneshot::Sender<Option<SessionInfo>>,
    },
    GetSessionChannels {
        session_id: String,
        response_tx: oneshot::Sender<Option<PtyChannels>>,
    },
    ListSessions {
        response_tx: oneshot::Sender<Vec<SessionInfo>>,
    },
    CloseSession {
        session_id: String,
        response_tx: oneshot::Sender<Result<()>>,
    },
    CreateProject {
        name: String,
        path: String,
        response_tx: oneshot::Sender<Result<ProjectInfo>>,
    },
    ListProjects {
        response_tx: oneshot::Sender<Vec<ProjectInfo>>,
    },
    ShutdownAllSessions {
        response_tx: oneshot::Sender<()>,
    },
}

// Actor handle for communicating with SessionManager
#[derive(Clone)]
pub struct SessionManagerHandle {
    command_tx: mpsc::UnboundedSender<SessionCommand>,
}

// Internal session manager state (runs in its own task)
struct SessionManagerActor {
    config: Config,
    sessions: HashMap<String, SessionState>,
    projects: HashMap<String, Project>,
    command_rx: mpsc::UnboundedReceiver<SessionCommand>,
}

struct SessionState {
    id: String,
    agent: String,
    channels: PtyChannels,
    project_id: Option<String>,
}

struct Project {
    id: String,
    name: String,
    path: PathBuf,
}

impl SessionManagerHandle {
    pub fn new(config: Config) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let actor = SessionManagerActor {
            config,
            sessions: HashMap::new(),
            projects: HashMap::new(),
            command_rx,
        };

        // Spawn the actor task
        tokio::spawn(actor.run());

        Self { command_tx }
    }

    pub async fn create_session_with_path(
        &self,
        agent: String,
        args: Vec<String>,
        project_id: Option<String>,
        path: Option<String>,
    ) -> Result<SessionInfo> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::CreateSession {
            agent,
            args,
            project_id,
            path,
            response_tx,
        };

        self.command_tx
            .send(command)
            .map_err(|_| anyhow!("SessionManager actor is not running"))?;

        response_rx
            .await
            .map_err(|_| anyhow!("SessionManager actor did not respond"))?
    }

    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::GetSession {
            session_id: session_id.to_string(),
            response_tx,
        };

        if self.command_tx.send(command).is_err() {
            return None;
        }

        response_rx.await.unwrap_or(None)
    }

    pub async fn get_session_channels(&self, session_id: &str) -> Option<PtyChannels> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::GetSessionChannels {
            session_id: session_id.to_string(),
            response_tx,
        };

        if self.command_tx.send(command).is_err() {
            return None;
        }

        response_rx.await.unwrap_or(None)
    }

    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::ListSessions { response_tx };

        if self.command_tx.send(command).is_err() {
            return vec![];
        }

        response_rx.await.unwrap_or_else(|_| vec![])
    }

    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::CloseSession {
            session_id: session_id.to_string(),
            response_tx,
        };

        self.command_tx
            .send(command)
            .map_err(|_| anyhow!("SessionManager actor is not running"))?;

        response_rx
            .await
            .map_err(|_| anyhow!("SessionManager actor did not respond"))?
    }

    pub async fn create_project(&self, name: String, path: String) -> Result<ProjectInfo> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::CreateProject {
            name,
            path,
            response_tx,
        };

        self.command_tx
            .send(command)
            .map_err(|_| anyhow!("SessionManager actor is not running"))?;

        response_rx
            .await
            .map_err(|_| anyhow!("SessionManager actor did not respond"))?
    }

    pub async fn list_projects(&self) -> Vec<ProjectInfo> {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::ListProjects { response_tx };

        if self.command_tx.send(command).is_err() {
            return vec![];
        }

        response_rx.await.unwrap_or_else(|_| vec![])
    }

    pub async fn shutdown_all_sessions(&self) {
        let (response_tx, response_rx) = oneshot::channel();

        let command = SessionCommand::ShutdownAllSessions { response_tx };

        if self.command_tx.send(command).is_ok() {
            let _ = response_rx.await;
        }
    }
}

impl SessionManagerActor {
    async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            self.handle_command(command).await;
        }
    }

    async fn handle_command(&mut self, command: SessionCommand) {
        match command {
            SessionCommand::CreateSession {
                agent,
                args,
                project_id,
                path,
                response_tx,
            } => {
                let result = self
                    .create_session_with_path(agent, args, project_id, path)
                    .await;
                let _ = response_tx.send(result);
            }
            SessionCommand::GetSession {
                session_id,
                response_tx,
            } => {
                let result = self.get_session(&session_id);
                let _ = response_tx.send(result);
            }
            SessionCommand::GetSessionChannels {
                session_id,
                response_tx,
            } => {
                let result = self.get_session_channels(&session_id);
                let _ = response_tx.send(result);
            }
            SessionCommand::ListSessions { response_tx } => {
                let result = self.list_sessions();
                let _ = response_tx.send(result);
            }
            SessionCommand::CloseSession {
                session_id,
                response_tx,
            } => {
                let result = self.close_session(&session_id).await;
                let _ = response_tx.send(result);
            }
            SessionCommand::CreateProject {
                name,
                path,
                response_tx,
            } => {
                let result = self.create_project(name, path);
                let _ = response_tx.send(result);
            }
            SessionCommand::ListProjects { response_tx } => {
                let result = self.list_projects();
                let _ = response_tx.send(result);
            }
            SessionCommand::ShutdownAllSessions { response_tx } => {
                self.shutdown_all_sessions().await;
                let _ = response_tx.send(());
            }
        }
    }

    async fn create_session_with_path(
        &mut self,
        agent: String,
        args: Vec<String>,
        project_id: Option<String>,
        path: Option<String>,
    ) -> Result<SessionInfo> {
        if !self.config.is_agent_allowed(&agent) {
            return Err(anyhow!("Code agent '{}' is not whitelisted", agent));
        }

        let session_id = Uuid::new_v4().to_string();

        // Add session ID to args if the agent is Claude
        let mut final_args = args.clone();
        if agent.to_lowercase() == "claude" {
            final_args.push("--session-id".to_string());
            final_args.push(session_id.clone());
        }

        // Handle project association and working directory
        let resolved_project_id = if let Some(proj_id) = project_id {
            // Use provided project ID
            if let Some(project) = self.projects.get(&proj_id) {
                std::env::set_current_dir(&project.path)?;
            }
            Some(proj_id)
        } else if let Some(current_path) = path {
            // Try to find existing project for this path
            let mut found_project_id = None;
            for (pid, project) in &self.projects {
                if project.path.to_string_lossy() == current_path {
                    found_project_id = Some(pid.clone());
                    break;
                }
            }

            if let Some(existing_id) = found_project_id {
                // Found existing project
                std::env::set_current_dir(&current_path)?;
                Some(existing_id)
            } else {
                // Create temporary project for this path
                let path_buf = std::path::PathBuf::from(&current_path);
                let project_name = path_buf
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();

                let temp_project_id = Uuid::new_v4().to_string();
                self.projects.insert(
                    temp_project_id.clone(),
                    Project {
                        id: temp_project_id.clone(),
                        name: format!("{} (temporary)", project_name),
                        path: path_buf.clone(),
                    },
                );

                std::env::set_current_dir(&current_path)?;
                Some(temp_project_id)
            }
        } else {
            // No project or path specified
            None
        };

        tracing::debug!(
            "SessionManager - Creating PTY session with ID: {}, agent: {}",
            session_id,
            agent
        );
        let (session, channels) = PtySession::new(session_id.clone(), agent.clone(), final_args)?;
        tracing::debug!(
            "SessionManager - PTY session created, channels available, spawning start task"
        );

        // Clone channels for storage
        let channels_clone = channels.clone();

        // Spawn the PTY session start task to actually begin reading from the PTY
        let session_id_clone = session_id.clone();
        tokio::spawn(async move {
            tracing::info!(
                "SessionManager - Starting PTY session tasks for {}",
                session_id_clone
            );
            if let Err(e) = session.start().await {
                tracing::error!(
                    "SessionManager - PTY session {} failed: {}",
                    session_id_clone,
                    e
                );
            }
            tracing::info!(
                "SessionManager - PTY session {} completed",
                session_id_clone
            );
        });

        // Store the session state
        let session_state = SessionState {
            id: session_id.clone(),
            agent: agent.clone(),
            channels: channels_clone,
            project_id: resolved_project_id.clone(),
        };
        self.sessions.insert(session_id.clone(), session_state);
        tracing::info!(
            "SessionManager - Session {} stored successfully, channels ready for use",
            session_id
        );

        Ok(SessionInfo {
            id: session_id,
            agent,
            project: resolved_project_id,
            status: "running".to_string(),
        })
    }

    fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.get(session_id).map(|state| SessionInfo {
            id: state.id.clone(),
            agent: state.agent.clone(),
            project: state.project_id.clone(),
            status: "running".to_string(),
        })
    }

    fn get_session_channels(&self, session_id: &str) -> Option<PtyChannels> {
        tracing::debug!(
            "SessionManager - Looking for session channels: {}, total sessions: {}",
            session_id,
            self.sessions.len()
        );
        let result = self
            .sessions
            .get(session_id)
            .map(|state| state.channels.clone());
        if result.is_some() {
            tracing::debug!(
                "SessionManager - Found channels for session: {}",
                session_id
            );
        } else {
            tracing::warn!(
                "SessionManager - No channels found for session: {}",
                session_id
            );
            // Log all available session IDs for debugging
            let session_ids: Vec<_> = self.sessions.keys().collect();
            tracing::debug!("SessionManager - Available session IDs: {:?}", session_ids);
        }
        result
    }

    fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|(_, state)| SessionInfo {
                id: state.id.clone(),
                agent: state.agent.clone(),
                project: state.project_id.clone(),
                status: "running".to_string(),
            })
            .collect()
    }

    async fn close_session(&mut self, session_id: &str) -> Result<()> {
        if let Some(state) = self.sessions.remove(session_id) {
            // Send terminate signal
            if let Err(e) = state
                .channels
                .control_tx
                .send(crate::core::pty_session::PtyControlMessage::Terminate)
            {
                tracing::warn!(
                    "Failed to send terminate signal to session {}: {}",
                    session_id,
                    e
                );
            }
            Ok(())
        } else {
            Err(anyhow!("Session not found"))
        }
    }

    fn create_project(&mut self, name: String, path: String) -> Result<ProjectInfo> {
        let project_id = Uuid::new_v4().to_string();
        let project_path = std::path::PathBuf::from(&path);

        if !project_path.exists() {
            return Err(anyhow!("Project path does not exist"));
        }

        self.projects.insert(
            project_id.clone(),
            Project {
                id: project_id.clone(),
                name: name.clone(),
                path: project_path.clone(),
            },
        );

        Ok(ProjectInfo {
            id: project_id,
            name,
            path: project_path.to_string_lossy().to_string(),
        })
    }

    fn list_projects(&self) -> Vec<ProjectInfo> {
        self.projects
            .values()
            .map(|p| ProjectInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                path: p.path.to_string_lossy().to_string(),
            })
            .collect()
    }

    async fn shutdown_all_sessions(&mut self) {
        tracing::info!("Shutting down {} sessions", self.sessions.len());

        // Send terminate signal to all sessions
        for (session_id, state) in &self.sessions {
            tracing::info!("Terminating session: {}", session_id);

            // Send terminate control message
            if let Err(e) = state
                .channels
                .control_tx
                .send(crate::core::pty_session::PtyControlMessage::Terminate)
            {
                tracing::warn!(
                    "Failed to send terminate signal to session {}: {}",
                    session_id,
                    e
                );
            }
        }

        // Clear the sessions map
        self.sessions.clear();
        tracing::info!("All sessions terminated");
    }
}
