use std::fs;
use std::path::PathBuf;
use serde_json::Value;
use rusqlite::{Connection, OpenFlags};
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

    fn fetch_cursor_history(home_dir: &PathBuf) -> anyhow::Result<String> {
        let mut history_text = String::new();
        
        #[cfg(target_os = "macos")]
        let cursor_dir = home_dir.join("Library/Application Support/Cursor/User/workspaceStorage");
        #[cfg(target_os = "linux")]
        let cursor_dir = home_dir.join(".config/Cursor/User/workspaceStorage");
        #[cfg(target_os = "windows")]
        let cursor_dir = home_dir.join("AppData/Roaming/Cursor/User/workspaceStorage");

        if !cursor_dir.exists() {
            anyhow::bail!("Cursor workspaceStorage not found at {}", cursor_dir.display());
        }

        // Iterate over all workspace storage folders
        for entry in fs::read_dir(&cursor_dir)? {
            let entry = entry?;
            let db_path = entry.path().join("state.vscdb");
            
            if db_path.exists() {
                // Open DB in read-only mode
                if let Ok(conn) = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                    // Look for keys containing chat history (this key might vary, "workbench.panel.aichat.view.history" is an example)
                    let mut stmt = conn.prepare("SELECT value FROM ItemTable WHERE key LIKE '%chat%' OR key LIKE '%composer%'")?;
                    let rows = stmt.query_map([], |row| {
                        let val: String = row.get(0)?;
                        Ok(val)
                    });
                    
                    if let Ok(mapped_rows) = rows {
                        for row_result in mapped_rows {
                            if let Ok(json_str) = row_result {
                                // Just append the raw json string for now, or attempt to format it
                                history_text.push_str(&json_str);
                                history_text.push_str("\n\n---\n\n");
                            }
                        }
                    }
                }
            }
        }
        
        if history_text.is_empty() {
            Ok("No chat history found in Cursor's SQLite databases.".to_string())
        } else {
            Ok(history_text)
        }
    }
}

impl Adapter for HistoryAdapter {
    fn fetch(&self) -> anyhow::Result<String> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        
        if self.agent_name == "cursor" {
            return Self::fetch_cursor_history(&home_dir);
        }
        
        let path = match self.agent_name.as_str() {
            "claude-cli" => home_dir.join(".claude").join("history.json"),
            "codex-cli" => home_dir.join(".codex").join("history.json"),
            "gemini-cli" => home_dir.join(".gemini").join("history.json"),
            "openclaw" => home_dir.join(".openclaw").join("history.json"),
            "opencode" => home_dir.join(".opencode").join("history.json"),
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