use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub daemon: DaemonConfig,
    pub agents: AgentsConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DaemonConfig {
    pub mode: String,
    pub interval_seconds: u64,
    #[serde(default)]
    pub custom_watch_dirs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentsConfig {
    pub enabled: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LlmConfig {
    pub enable: bool,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub mock: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enable: false,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            mock: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig {
                mode: "watcher".to_string(),
                interval_seconds: 3600, // 1 hour default for polling fallback
                custom_watch_dirs: vec![],
            },
            agents: AgentsConfig {
                enabled: vec![
                    "trae".to_string(), 
                    "trae-cn".to_string(), 
                    "cursor".to_string(),
                    "claude-cli".to_string(),
                    "codex-cli".to_string(),
                    "gemini-cli".to_string(),
                    "openclaw".to_string(),
                    "opencode".to_string()
                ],
            },
            llm: LlmConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load_or_create(global_dir: &Path) -> anyhow::Result<Self> {
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

    pub fn save(&self, global_dir: &Path) -> anyhow::Result<()> {
        let config_path = global_dir.join("config.toml");
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(&config_path, toml_str)?;
        Ok(())
    }
}