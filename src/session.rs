use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::config::Config;
use crate::pty::PtySession;

pub struct SessionManager {
    config: Config,
    pub sessions: HashMap<String, PtySession>,
    projects: HashMap<String, Project>,
}

struct Project {
    id: String,
    name: String,
    path: PathBuf,
}

impl SessionManager {
    pub fn new(config: Config) -> Self {
        SessionManager {
            config,
            sessions: HashMap::new(),
            projects: HashMap::new(),
        }
    }

    pub async fn create_session(
        &mut self,
        agent: String,
        args: Vec<String>,
        project_id: Option<String>,
    ) -> Result<SessionInfo> {
        if !self.config.is_agent_allowed(&agent) {
            return Err(anyhow!("Code agent '{}' is not whitelisted", agent));
        }

        let session_id = Uuid::new_v4().to_string();

        // Add session ID to args if the agent is Claude
        let mut final_args = args.clone();
        if agent.to_lowercase() == "claude" {
            // Add --session-id flag for Claude to continue or start a specific session
            final_args.push("--session-id".to_string());
            final_args.push(session_id.clone());
        }
        
        if let Some(proj_id) = &project_id {
            if let Some(project) = self.projects.get(proj_id) {
                std::env::set_current_dir(&project.path)?;
            }
        }

        let session = PtySession::new(session_id.clone(), agent.clone(), final_args)?;
        self.sessions.insert(session_id.clone(), session);

        Ok(SessionInfo {
            id: session_id,
            agent,
            project: project_id,
            status: "running".to_string(),
        })
    }

    pub async fn close_session(&mut self, session_id: &str) -> Result<()> {
        self.sessions
            .remove(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.get(session_id).map(|s| SessionInfo {
            id: session_id.to_string(),
            agent: s.agent.clone(),
            project: None,
            status: "running".to_string(),
        })
    }

    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|(id, session)| SessionInfo {
                id: id.clone(),
                agent: session.agent.clone(),
                project: None,
                status: "running".to_string(),
            })
            .collect()
    }

    pub async fn add_project(&mut self, name: String, path: String) -> Result<ProjectInfo> {
        let project_id = Uuid::new_v4().to_string();
        let project_path = PathBuf::from(path);

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

    pub fn list_projects(&self) -> Vec<ProjectInfo> {
        self.projects
            .values()
            .map(|p| ProjectInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                path: p.path.to_string_lossy().to_string(),
            })
            .collect()
    }
}

#[derive(Clone, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub project: Option<String>,
    pub status: String,
}

#[derive(Clone, Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}
