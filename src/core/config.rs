use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub whitelist: AgentWhitelist,
    pub server: ServerConfig,
    pub web: WebConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWhitelist {
    pub agents: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
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
            server: ServerConfig {
                port: default_server_port(),
                data_dir: data_dir.clone(),
                pid_file: data_dir.join("server.pid"),
            },
            web: WebConfig { static_dir: None },
        }
    }
}

/// Get the default server port based on build type
pub fn default_server_port() -> u16 {
    if cfg!(debug_assertions) { 18765 } else { 8765 }
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "codemux", "codemux") {
            let config_file = config_dir.config_dir().join("config.toml");
            if config_file.exists() {
                let content = std::fs::read_to_string(&config_file)?;

                // Try to load as new format first
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return Ok(config);
                }

                // Try to load legacy format and migrate
                if let Ok(legacy_config) = toml::from_str::<LegacyConfig>(&content) {
                    let migrated_config = Config::from_legacy(legacy_config);

                    // Save the migrated config
                    if let Err(e) = migrated_config.save() {
                        tracing::warn!("Failed to save migrated config: {}", e);
                    } else {
                        tracing::info!("Migrated legacy daemon config to server config");
                    }

                    return Ok(migrated_config);
                }
            }
        }
        Ok(Config::default())
    }

    pub fn save(&self) -> Result<()> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "codemux", "codemux") {
            std::fs::create_dir_all(config_dir.config_dir())?;
            let config_file = config_dir.config_dir().join("config.toml");
            let content = toml::to_string_pretty(self)?;
            std::fs::write(config_file, content)?;
        }
        Ok(())
    }

    fn from_legacy(legacy: LegacyConfig) -> Self {
        Config {
            whitelist: legacy.whitelist,
            server: ServerConfig {
                port: legacy.daemon.port,
                data_dir: legacy.daemon.data_dir,
                pid_file: legacy
                    .daemon
                    .pid_file
                    .parent()
                    .map(|p| p.join("server.pid"))
                    .unwrap_or_else(|| PathBuf::from("server.pid")),
            },
            web: legacy.web,
        }
    }

    pub fn is_agent_allowed(&self, agent: &str) -> bool {
        self.whitelist.agents.contains(agent)
    }
}

// Legacy config structures for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyConfig {
    pub whitelist: AgentWhitelist,
    pub daemon: LegacyDaemonConfig,
    pub web: WebConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyDaemonConfig {
    pub port: u16,
    pub data_dir: PathBuf,
    pub pid_file: PathBuf,
}
