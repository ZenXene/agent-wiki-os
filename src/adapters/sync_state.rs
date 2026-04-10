use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SyncState {
    pub agents: HashMap<String, HashMap<String, u64>>,
}

impl SyncState {
    pub fn get_path() -> anyhow::Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
        let state_dir = home_dir.join(".agent-wiki-os");
        fs::create_dir_all(&state_dir)?;
        Ok(state_dir.join("sync_state.json"))
    }

    pub fn load() -> Self {
        if let Ok(path) = Self::get_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str(&content) {
                    return state;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::get_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_offset(&self, agent: &str, project_or_file: &str) -> u64 {
        self.agents
            .get(agent)
            .and_then(|p| p.get(project_or_file))
            .copied()
            .unwrap_or(0)
    }

    pub fn update_offset(&mut self, agent: &str, project_or_file: &str, offset: u64) {
        self.agents
            .entry(agent.to_string())
            .or_default()
            .insert(project_or_file.to_string(), offset);
    }
}