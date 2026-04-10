use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub daemon: DaemonConfig,
    pub agents: AgentsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DaemonConfig {
    pub mode: String,
    pub interval_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentsConfig {
    pub enabled: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig {
                mode: "polling".to_string(),
                interval_seconds: 3600, // 1 hour default
            },
            agents: AgentsConfig {
                enabled: vec!["trae".to_string(), "cursor".to_string()],
            },
        }
    }
}

impl AppConfig {
    pub fn load_or_create(global_dir: &PathBuf) -> anyhow::Result<Self> {
        let config_path = global_dir.join("config.toml");
        
        if !config_path.exists() {
            let default_config = Self::default();
            let toml_string = toml::to_string_pretty(&default_config)?;
            fs::write(&config_path, toml_string)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&config_path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }
}