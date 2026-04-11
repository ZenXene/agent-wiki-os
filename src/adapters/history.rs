use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;
use super::sync_state::SyncState;
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

    pub fn get_watch_path(&self) -> anyhow::Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        
        match self.agent_name.as_str() {
            "cursor" => {
                #[cfg(target_os = "macos")]
                return Ok(home_dir.join("Library/Application Support/Cursor/User/workspaceStorage"));
                #[cfg(target_os = "linux")]
                return Ok(home_dir.join(".config/Cursor/User/workspaceStorage"));
                #[cfg(target_os = "windows")]
                return Ok(home_dir.join("AppData/Roaming/Cursor/User/workspaceStorage"));
            },
            "trae" => {
                #[cfg(target_os = "macos")]
                return Ok(home_dir.join("Library/Application Support/Trae/User/workspaceStorage"));
                #[cfg(target_os = "linux")]
                return Ok(home_dir.join(".config/Trae/User/workspaceStorage"));
                #[cfg(target_os = "windows")]
                return Ok(home_dir.join("AppData/Roaming/Trae/User/workspaceStorage"));
            },
            "trae-cn" => {
                #[cfg(target_os = "macos")]
                return Ok(home_dir.join("Library/Application Support/Trae CN/User/workspaceStorage"));
                #[cfg(target_os = "linux")]
                return Ok(home_dir.join(".config/Trae CN/User/workspaceStorage"));
                #[cfg(target_os = "windows")]
                return Ok(home_dir.join("AppData/Roaming/Trae CN/User/workspaceStorage"));
            },
            "claude-cli" => Ok(home_dir.join(".claude").join("history.jsonl")),
            "codex-cli" => Ok(home_dir.join(".codex").join("history.jsonl")),
            "gemini-cli" => Ok(home_dir.join(".gemini").join("history.jsonl")),
            "openclaw" => Ok(home_dir.join(".openclaw").join("history.jsonl")),
            "opencode" => Ok(home_dir.join(".opencode").join("history.jsonl")),
            _ => anyhow::bail!("Unsupported agent: {}", self.agent_name),
        }
    }

    pub fn fetch_grouped_by_project(&self) -> anyhow::Result<HashMap<String, String>> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let mut grouped_history: HashMap<String, String> = HashMap::new();

        // Check if it's an Electron-based SQLite DB (Cursor, Trae)
        let sqlite_path = match self.agent_name.as_str() {
            "cursor" => Some(home_dir.join("Library/Application Support/Cursor/User/workspaceStorage")),
            "trae" => Some(home_dir.join("Library/Application Support/Trae/User/workspaceStorage")),
            "trae-cn" => Some(home_dir.join("Library/Application Support/Trae CN/User/workspaceStorage")),
            _ => None,
        };

        if let Some(base_path) = sqlite_path {
            if base_path.exists() {
                // Electron databases handle multiple projects but their JSON structures
                // inside SQLite are opaque without deep parsing. For now, fallback to global.
                // In future, this can be enhanced to parse project names from workspaceStorage.
                let data = self.fetch()?;
                grouped_history.insert("global".to_string(), data);
                return Ok(grouped_history);
            }
        }

        // Handle JSON-based CLIs
        let path = match self.agent_name.as_str() {
            "claude-cli" => home_dir.join(".claude").join("history.jsonl"),
            "codex-cli" => home_dir.join(".codex").join("history.jsonl"),
            "gemini-cli" => home_dir.join(".gemini").join("history.jsonl"),
            "openclaw" => home_dir.join(".openclaw").join("history.jsonl"),
            "opencode" => home_dir.join(".opencode").join("history.jsonl"),
            _ => anyhow::bail!("Unsupported agent: {}", self.agent_name),
        };

        if !path.exists() {
            anyhow::bail!("History file not found at: {}", path.display());
        }

        let mut sync_state = SyncState::load();
        let path_str = path.to_string_lossy().to_string();
        let last_offset = sync_state.get_offset(&self.agent_name, &path_str);

        let mut file = File::open(&path)?;
        let file_len = file.metadata()?.len();

        if last_offset >= file_len {
            if last_offset > file_len {
                sync_state.update_offset(&self.agent_name, &path_str, 0);
                let _ = sync_state.save();
                file.seek(SeekFrom::Start(0))?;
            } else {
                grouped_history.insert("global".to_string(), "No chat history found in file.".to_string());
                return Ok(grouped_history);
            }
        } else {
            file.seek(SeekFrom::Start(last_offset))?;
        }

        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut current_offset = last_offset;
        
        while reader.read_line(&mut line)? > 0 {
            let bytes_read = line.len() as u64;
            current_offset += bytes_read;

            if line.trim().is_empty() {
                line.clear();
                continue;
            }
            if let Ok(item) = serde_json::from_str::<serde_json::Value>(&line) {
                // Extract Project Path
                let project_path = item.get("project").and_then(|v| v.as_str()).unwrap_or("global").to_string();

                // Handle different JSON structures
                let msg = item.get("message").and_then(|v| v.as_str())
                    .or_else(|| item.get("text").and_then(|v| v.as_str()))
                    .or_else(|| item.get("display").and_then(|v| v.as_str()))
                    .or_else(|| item.get("content").and_then(|v| v.as_str()))
                    .or_else(|| {
                        item.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|f| f.get("text"))
                            .and_then(|t| t.as_str())
                    });
                    
                if let Some(m) = msg {
                    let role = item.get("role").and_then(|v| v.as_str())
                        .or_else(|| item.get("message").and_then(|msg| msg.get("role")).and_then(|r| r.as_str()))
                        .unwrap_or("user");
                    
                    let entry = format!("**{}**: {}\n\n", role, m);
                    grouped_history.entry(project_path)
                        .and_modify(|h| h.push_str(&entry))
                        .or_insert(entry);
                }
            }
            line.clear();
        }
        
        // Update checkpoint
        sync_state.update_offset(&self.agent_name, &path_str, current_offset);
        let _ = sync_state.save(); // ignore save errors for now
        
        if grouped_history.is_empty() {
            grouped_history.insert("global".to_string(), "No chat history found in file.".to_string());
        }
        
        Ok(grouped_history)
    }

    fn fetch_electron_sqlite_history(home_dir: &std::path::PathBuf, app_name: &str) -> anyhow::Result<String> {
        let mut history_text = String::new();
        
        #[cfg(target_os = "macos")]
        let storage_dir = home_dir.join("Library/Application Support").join(app_name).join("User/workspaceStorage");
        #[cfg(target_os = "linux")]
        let storage_dir = home_dir.join(".config").join(app_name).join("User/workspaceStorage");
        #[cfg(target_os = "windows")]
        let storage_dir = home_dir.join("AppData/Roaming").join(app_name).join("User/workspaceStorage");

        if !storage_dir.exists() {
            anyhow::bail!("{} workspaceStorage not found at {}", app_name, storage_dir.display());
        }

        // Iterate over all workspace storage folders
        for entry in fs::read_dir(&storage_dir)? {
            let entry = entry?;
            let db_path = entry.path().join("state.vscdb");
            
            if db_path.exists() {
            // Open DB in read-only mode with exponential backoff retry for locked databases
            let mut retries = 0;
            let max_retries = 5;
            let mut conn_opt = None;
            
            while retries < max_retries {
                match Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                    Ok(conn) => {
                        conn_opt = Some(conn);
                        break;
                    }
                    Err(e) => {
                        let err_msg = e.to_string().to_lowercase();
                        if err_msg.contains("locked") || err_msg.contains("busy") {
                            std::thread::sleep(std::time::Duration::from_millis(200 * (1 << retries)));
                            retries += 1;
                        } else {
                            break;
                        }
                    }
                }
            }

            if let Some(conn) = conn_opt {
                let mut stmt = conn.prepare("SELECT value FROM ItemTable WHERE key LIKE '%chat%' OR key LIKE '%composer%'")?;
                    let rows = stmt.query_map([], |row| {
                        let val: String = row.get(0)?;
                        Ok(val)
                    });
                    
                    if let Ok(mapped_rows) = rows {
                        for row_result in mapped_rows {
                            if let Ok(json_str) = row_result {
                                history_text.push_str(&json_str);
                                history_text.push_str("\n\n---\n\n");
                            }
                        }
                    }
                }
            }
        }
        
        if history_text.is_empty() {
            Ok(format!("No chat history found in {}'s SQLite databases.", app_name))
        } else {
            Ok(history_text)
        }
    }
}

impl Adapter for HistoryAdapter {
    fn fetch(&self) -> anyhow::Result<String> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        
        // Handle SQLite-based IDEs
        match self.agent_name.as_str() {
            "cursor" => return Self::fetch_electron_sqlite_history(&home_dir, "Cursor"),
            "trae" => return Self::fetch_electron_sqlite_history(&home_dir, "Trae"),
            "trae-cn" => return Self::fetch_electron_sqlite_history(&home_dir, "Trae CN"),
            _ => {}
        }
        
        // Handle JSON-based CLIs
        let path = match self.agent_name.as_str() {
            "claude-cli" => home_dir.join(".claude").join("history.jsonl"),
            "codex-cli" => home_dir.join(".codex").join("history.jsonl"),
            "gemini-cli" => home_dir.join(".gemini").join("history.jsonl"),
            "openclaw" => home_dir.join(".openclaw").join("history.jsonl"),
            "opencode" => home_dir.join(".opencode").join("history.jsonl"),
            _ => anyhow::bail!("Unsupported agent: {}", self.agent_name),
        };

        if !path.exists() {
            anyhow::bail!("History file not found at: {}", path.display());
        }

        let content = fs::read_to_string(&path)?;
        let mut formatted_history = String::new();
        
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(item) = serde_json::from_str::<serde_json::Value>(line) {
                // Handle different JSON structures (Claude Code uses "message", "text", "display", some use "content")
                let msg = item.get("message").and_then(|v| v.as_str())
                    .or_else(|| item.get("text").and_then(|v| v.as_str()))
                    .or_else(|| item.get("display").and_then(|v| v.as_str()))
                    .or_else(|| item.get("content").and_then(|v| v.as_str()))
                    .or_else(|| {
                        // For Claude's deeply nested structure
                        item.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|f| f.get("text"))
                            .and_then(|t| t.as_str())
                    });
                    
                if let Some(m) = msg {
                    let role = item.get("role").and_then(|v| v.as_str())
                        .or_else(|| item.get("message").and_then(|msg| msg.get("role")).and_then(|r| r.as_str()))
                        .unwrap_or("user"); // Default to user if no role is found
                    formatted_history.push_str(&format!("**{}**: {}\n\n", role, m));
                }
            }
        }
        
        if formatted_history.is_empty() {
            Ok("No chat history found in file.".to_string())
        } else {
            Ok(formatted_history)
        }
    }
}