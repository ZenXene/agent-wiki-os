use std::fs;
use serde_json::Value;
use super::Adapter;

pub struct HistoryAdapter {
    agent_name: String,
}

impl HistoryAdapter {
    pub fn new(agent_name: &str) -> Self {
        Self {
            agent_name: agent_name.to_string(),
        }
    }
}

impl Adapter for HistoryAdapter {
    fn fetch(&self) -> anyhow::Result<String> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        
        let path = match self.agent_name.as_str() {
            "claude-cli" => home_dir.join(".claude").join("history.json"),
            "codex-cli" => home_dir.join(".codex").join("history.json"),
            "gemini-cli" => home_dir.join(".gemini").join("history.json"),
            "openclaw" => home_dir.join(".openclaw").join("history.json"),
            "opencode" => home_dir.join(".opencode").join("history.json"),
            "cursor" => {
                // Mocking cursor for now since it requires SQLite parsing
                return Ok("Mock Cursor History - SQLite parsing pending".to_string());
            }
            _ => anyhow::bail!("Unsupported agent: {}", self.agent_name),
        };

        if !path.exists() {
            anyhow::bail!("History file not found at: {}", path.display());
        }

        let content = fs::read_to_string(&path)?;
        
        // Try parsing as JSON array
        if let Ok(json_array) = serde_json::from_str::<Vec<Value>>(&content) {
            let mut formatted = String::new();
            for item in json_array {
                if let Some(msg) = item.get("message").and_then(|v| v.as_str()) {
                    let role = item.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
                    formatted.push_str(&format!("**{}**: {}\n\n", role, msg));
                }
            }
            if !formatted.is_empty() {
                return Ok(formatted);
            }
        }
        
        // Fallback: just return the raw text if parsing fails
        Ok(content)
    }
}