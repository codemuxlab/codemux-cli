use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub whitelist: AgentWhitelist,
    pub daemon: DaemonConfig,
    pub web: WebConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWhitelist {
    pub agents: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub port: u16,
    pub data_dir: PathBuf,
    pub pid_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub static_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        let mut agents = HashSet::new();
        agents.insert("claude".to_string());
        agents.insert("gemini".to_string());
        agents.insert("aider".to_string());
        agents.insert("cursor".to_string());
        agents.insert("continue".to_string());

        let data_dir = directories::ProjectDirs::from("com", "codemux", "codemux")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".codemux"));

        Config {
            whitelist: AgentWhitelist { agents },
            daemon: DaemonConfig {
                port: 8080,
                data_dir: data_dir.clone(),
                pid_file: data_dir.join("daemon.pid"),
            },
            web: WebConfig { static_dir: None },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "codemux", "codemux") {
            let config_file = config_dir.config_dir().join("config.toml");
            if config_file.exists() {
                let content = std::fs::read_to_string(config_file)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        Ok(Config::default())
    }

    pub fn _save(&self) -> Result<()> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "codemux", "codemux") {
            std::fs::create_dir_all(config_dir.config_dir())?;
            let config_file = config_dir.config_dir().join("config.toml");
            let content = toml::to_string_pretty(self)?;
            std::fs::write(config_file, content)?;
        }
        Ok(())
    }

    pub fn is_agent_allowed(&self, agent: &str) -> bool {
        self.whitelist.agents.contains(agent)
    }
}
