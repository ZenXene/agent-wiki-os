mod cli;
mod storage;
mod adapters;
mod engine;
mod mcp;
mod config;

use clap::Parser;
use cli::{Cli, Commands};
use storage::WikiStorage;
use adapters::HistoryAdapter;
use engine::ingest::{RefinementEngine, ProcessMode};
use config::AppConfig;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc::channel;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn install_skill(target: &str) -> anyhow::Result<()> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir found"))?;
    
    // 1. Ensure the master skill exists in ~/.agent-wiki-os/skills/agent-wiki-os/
    let master_skill_dir = home_dir.join(".agent-wiki-os").join("skills").join("agent-wiki-os");
    std::fs::create_dir_all(&master_skill_dir)?;
    
    let skill_content = r#"---
name: agent-wiki-os
description: Local memory and knowledge extraction engine. Use this skill to convert documents, directories, URLs, or chat history into a structured knowledge base or AI skills via the awo CLI tool.
tags: [skill, agent_wiki_os, cli, tool, knowledge, wiki, ingestion, memory]
version: 1.0.0
---

# Agent-Wiki-OS (AWO)

You are an AI assistant equipped with the Agent-Wiki-OS (`awo`) capability. `awo` is a powerful CLI tool that acts as your local memory and knowledge extraction engine.

## When to use

Activate this skill immediately when the user asks you to:

1. **Extract Knowledge**:
   - "Use agent-wiki-os to process this document"
   - "Ingest this folder with awo"
   - "Add this URL to the knowledge base"

2. **Generate Skills / Specs / Onboards**:
   - "Convert this PDF into a skill"
   - "Generate a spec document"
   - "Create an onboarding guide for this project"

3. **Process Chat History**:
   - "Pull claude-cli history"
   - "Analyze previous conversations"

4. **Trigger Keywords**:
   - "awo"
   - "agent-wiki-os"
   - "ingest"

## Workflow

### Step 1: Execute the Extraction Command
Based on the user's request, use your terminal tool to run the appropriate `awo` command:

```bash
# Ingest file/folder/URL into knowledge base (default mode: wiki)
awo ingest <path_or_url>

# Ingest and generate a specific artifact type
awo ingest <path_or_url> --mode skill
awo ingest <path_or_url> --mode spec
awo ingest <path_or_url> --mode onboard
awo ingest <path_or_url> --mode persona
awo ingest <path_or_url> --mode postmortem

# Pull chat history
awo pull <tool_name>  # e.g., awo pull claude-cli
```

**Wait for the terminal command to finish.** If successful, it will output a message containing the path to a `.awo_tasks/task_xxx.md` file.

### Step 2: Read the Task File
The `awo` command will NOT print the massive document contents to stdout (to avoid truncating your context). Instead, it packages the raw content and the specific instructions into a temporary task file.

You MUST use your **File Read Tool** to read the exact `task_xxx.md` file path provided in the terminal output.

### Step 3: Process the Knowledge
Read the contents of the `task_xxx.md` file carefully. It contains a specific "System Prompt" instructing you on exactly how to format the output, followed by the raw data.

Process this information in your mind. Do NOT output the raw data back to the user.

### Step 4: Write the Final Artifact
Generate the final Markdown document exactly as instructed by the task file.

Use your **File Write Tool** to save this document into the `.wiki/` directory under the appropriate subfolder. **Crucially, you must infer the correct subfolder based on the requested mode**:
- `--mode wiki` -> `.wiki/concepts/` or `.wiki/entities/`
- `--mode skill` -> `.wiki/skills/` (use `.skill` or `.md` extension as appropriate)
- `--mode spec` -> `.wiki/specs/`
- `--mode onboard` -> `.wiki/onboards/`
- `--mode persona` -> `.wiki/personas/`
- `--mode postmortem` -> `.wiki/postmortems/`

### Step 5: Clean Up
Use your terminal tool to delete the temporary task file:
```bash
rm .wiki/.awo_tasks/task_xxx.md
```

## Important Rules

1. **NEVER ask the user to copy-paste output.** You must read the file and write the result yourself.
2. **NEVER run `cat` on the task file in the terminal.** Always use your built-in File Read Tool to avoid context truncation.
3. **Strictly follow the task file instructions.** Do not deviate from the requested format.
4. **Clean up after yourself.** Always delete the `.awo_tasks/task_xxx.md` file when finished.
"#;

    std::fs::write(master_skill_dir.join("SKILL.md"), skill_content)?;
    println!("✅ Master skill generated at: {}", master_skill_dir.display());

    // 2. Link to target IDEs
    let targets = if target == "all" {
        vec!["trae", "trae-cn", "cursor"]
    } else {
        vec![target]
    };

    for t in targets {
        let target_dir = match t {
            "trae" => home_dir.join(".trae").join("skills"),
            "trae-cn" => home_dir.join(".trae-cn").join("skills"),
            "cursor" => home_dir.join(".cursor").join("skills"),
            _ => {
                eprintln!("⚠️  Unsupported target: {}", t);
                continue;
            }
        };

        if target_dir.exists() {
            let symlink_path = target_dir.join("agent-wiki-os");
            // Remove if exists
            if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
                let _ = std::fs::remove_file(&symlink_path);
                let _ = std::fs::remove_dir_all(&symlink_path);
            }
            
            // Create symlink (Unix only for now)
            #[cfg(unix)]
            {
                if let Err(e) = std::os::unix::fs::symlink(&master_skill_dir, &symlink_path) {
                    eprintln!("❌ Failed to symlink for {}: {}", t, e);
                } else {
                    println!("🔗 Successfully linked skill to: {}", symlink_path.display());
                }
            }
            #[cfg(not(unix))]
            {
                println!("⚠️  Symlinking is currently only supported on Unix/macOS.");
            }
        } else {
            println!("ℹ️  Directory not found, skipping: {}", target_dir.display());
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize storage
    let storage = WikiStorage::new(Some(std::path::PathBuf::from("./.wiki")));
    let wiki_root = storage.local_path.clone().unwrap_or(storage.global_path.clone());

    match &cli.command {
        Commands::Pull { agent } => {
            println!("Pulling history for agent: {}", agent);
            let adapter = HistoryAdapter::new(agent);
            match adapter.fetch_grouped_by_project() {
                Ok(grouped_data) => {
                    for (project_path, data) in grouped_data {
                        if data.contains("No chat history found") {
                            continue;
                        }
                        
                        println!("Processing history for project path: {}", project_path);
                        
                        // Determine the correct wiki_root
                        let current_wiki_root = if project_path == "global" {
                            storage.global_path.clone()
                        } else {
                            let p = PathBuf::from(&project_path);
                            if p.exists() {
                                p.join(".wiki")
                            } else {
                                storage.global_path.clone()
                            }
                        };
                        
                        if let Err(e) = RefinementEngine::process(&data, &current_wiki_root, agent, ProcessMode::WorkingMemory).await {
                            eprintln!("Failed to process history for {}: {}", project_path, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching history for {}: {}", agent, e);
                }
            }
        }
        Commands::Ingest { target, dir, url, mode } => {
            let mut final_url = url.clone();
            let mut final_dir = dir.clone();

            // Auto-resolve positional target if provided
            if let Some(t) = target {
                if t.starts_with("http://") || t.starts_with("https://") {
                    final_url = Some(t.clone());
                } else {
                    final_dir = Some(t.clone());
                }
            }

            let process_mode = ProcessMode::from_str(&mode);

            if let Some(u) = final_url {
                println!("Ingesting URL via Web Clipper: {} (Mode: {:?})", u, process_mode);
                let web_adapter = adapters::WebAdapter::new(&u);
                match web_adapter.fetch().await {
                    Ok(content) => {
                        if let Err(e) = RefinementEngine::process(&content, &wiki_root, "web_clipper", process_mode).await {
                            eprintln!("Failed to process URL content: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch URL: {}", e);
                    }
                }
            } else if let Some(d) = final_dir {
                println!("Ingesting directory/file: {} (Mode: {:?})", d, process_mode);
                let fs_adapter = adapters::FsAdapter::new(&d);
                if let Ok(files_content) = fs_adapter.fetch_all() {
                    for content in files_content {
                        RefinementEngine::process(&content, &wiki_root, "local_fs", process_mode).await?;
                    }
                }
            } else {
                println!("Error: No target specified for ingest. Please provide a URL or directory path.");
                println!("Usage:");
                println!("  awo ingest https://example.com");
                println!("  awo ingest /path/to/folder");
                println!("  awo ingest /path/to/file.pdf --mode skill");
                println!("  Available modes: wiki, skill, persona, postmortem, spec, onboard");
            }
        }
        Commands::Mcp { mode } => {
            println!("Starting MCP server in {} mode", mode);
            if mode == "stdio" {
                mcp::run_stdio_server().await?;
            }
        }
        Commands::Skills { action, target } => {
            if action == "install" {
                if let Err(e) = install_skill(&target) {
                    eprintln!("❌ Failed to install skill: {}", e);
                }
            } else {
                eprintln!("Unknown action: {}. Use 'install'.", action);
            }
        }
        Commands::Config { action, key, value } => {
            let config_dir = dirs::home_dir().unwrap_or_default().join(".agent-wiki-os");
            let mut config = AppConfig::load_or_create(&config_dir).unwrap_or_default();
            
            match action.as_str() {
                "set" => {
                    if let Some(v) = value {
                        let mut success = true;
                        match key.as_str() {
                            "llm.enable" => {
                                config.llm.enable = v == "1" || v.to_lowercase() == "true";
                            }
                            "llm.model" => config.llm.model = v.clone(),
                            "llm.api_key" => config.llm.api_key = v.clone(),
                            "llm.base_url" => config.llm.base_url = v.clone(),
                            "llm.mock" => {
                                config.llm.mock = v == "1" || v.to_lowercase() == "true";
                            }
                            _ => {
                                eprintln!("Unknown config key: {}", key);
                                success = false;
                            }
                        }
                        if success {
                            if let Err(e) = config.save(&config_dir) {
                                eprintln!("Failed to save config: {}", e);
                            } else {
                                println!("Successfully set {} = '{}'", key, v);
                            }
                        }
                    } else {
                        eprintln!("Error: 'set' action requires a value. (e.g. awo config set llm.model GLM-5)");
                    }
                }
                "get" => {
                    match key.as_str() {
                        "llm.enable" => println!("{}", config.llm.enable),
                        "llm.model" => println!("{}", config.llm.model),
                        "llm.api_key" => println!("{}", config.llm.api_key),
                        "llm.base_url" => println!("{}", config.llm.base_url),
                        "llm.mock" => println!("{}", config.llm.mock),
                        _ => eprintln!("Unknown config key: {}", key),
                    }
                }
                _ => {
                    eprintln!("Unknown action: {}. Use 'set' or 'get'.", action);
                }
            }
        }
        Commands::Daemon => {
            let config = AppConfig::load_or_create(&storage.global_path)?;
            println!("Starting Agent-Wiki-OS Daemon...");
            println!("Mode: {}", config.daemon.mode);
            println!("Monitoring Agents: {:?}", config.agents.enabled);

            if config.daemon.mode == "polling" {
                println!("Interval: {} seconds", config.daemon.interval_seconds);
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(config.daemon.interval_seconds));

                loop {
                    interval.tick().await;
                    println!("[Daemon] Waking up to poll agents...");
                    
                    for agent in &config.agents.enabled {
                        println!("[Daemon] Polling {}...", agent);
                        let adapter = adapters::HistoryAdapter::new(agent);
                        match adapter.fetch_grouped_by_project() {
                            Ok(grouped_data) => {
                                for (project_path, data) in grouped_data {
                                    if data.contains("No chat history found") {
                                        continue;
                                    }
                                    
                                    let current_wiki_root = if project_path == "global" {
                                        storage.global_path.clone()
                                    } else {
                                        let p = PathBuf::from(&project_path);
                                        if p.exists() {
                                            p.join(".wiki")
                                        } else {
                                            storage.global_path.clone()
                                        }
                                    };
                                    
                                    if let Err(e) = RefinementEngine::process(&data, &current_wiki_root, agent, ProcessMode::WorkingMemory).await {
                                        eprintln!("[Daemon] Failed to process {} for {}: {}", agent, project_path, e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[Daemon] Error fetching {}: {}", agent, e);
                            }
                        }
                    }
                    println!("[Daemon] Sleep cycle started.");
                }
            } else if config.daemon.mode == "watcher" {
                let (tx, rx) = channel();
                // Use the recommended watcher
                let mut watcher = notify::recommended_watcher(tx)?;
                
                // Map paths back to agent names so we know who to pull when a file changes
                let mut path_to_agent: HashMap<PathBuf, String> = HashMap::new();

                for agent in &config.agents.enabled {
                    let adapter = adapters::HistoryAdapter::new(agent);
                    if let Ok(watch_path) = adapter.get_watch_path() {
                        if watch_path.exists() {
                            println!("[Watcher] Watching path for {}: {}", agent, watch_path.display());
                            if let Err(e) = watcher.watch(&watch_path, RecursiveMode::Recursive) {
                                eprintln!("[Watcher] Failed to watch {}: {}", watch_path.display(), e);
                            } else {
                                path_to_agent.insert(watch_path, agent.clone());
                            }
                        } else {
                            println!("[Watcher] Path does not exist for {}, skipping: {}", agent, watch_path.display());
                        }
                    }
                }

                println!("[Watcher] Listening for file changes...");
                
                // Basic debounce/event loop
                for res in rx {
                    match res {
                        Ok(Event { kind: EventKind::Modify(_), paths, .. }) => {
                            for path in paths {
                                // Find which agent this path belongs to
                                for (watch_path, agent) in &path_to_agent {
                                    if path.starts_with(watch_path) {
                                        println!("[Watcher] Change detected for {}. Triggering ingest...", agent);
                                        let adapter = adapters::HistoryAdapter::new(agent);
                                        if let Ok(grouped_data) = adapter.fetch_grouped_by_project() {
                                            for (project_path, data) in grouped_data {
                                                if data.contains("No chat history found") {
                                                    continue;
                                                }
                                                
                                                let current_wiki_root = if project_path == "global" {
                                                    storage.global_path.clone()
                                                } else {
                                                    let p = PathBuf::from(&project_path);
                                                    if p.exists() {
                                                        p.join(".wiki")
                                                    } else {
                                                        storage.global_path.clone()
                                                    }
                                                };

                                                // Note: In a real app we'd spawn this to avoid blocking the watcher thread,
                                                // but for MVP blocking is okay.
                                                if let Err(e) = tokio::runtime::Handle::current().block_on(RefinementEngine::process(&data, &current_wiki_root, agent, ProcessMode::WorkingMemory)) {
                                                    eprintln!("[Watcher] Failed to process {} for {}: {}", agent, project_path, e);
                                                }
                                            }
                                        }
                                        break; // Only trigger once per event
                                    }
                                }
                            }
                        },
                        Err(e) => eprintln!("[Watcher] watch error: {:?}", e),
                        _ => {} // Ignore other events
                    }
                }
            } else {
                eprintln!("Unknown daemon mode: {}. Use 'polling' or 'watcher'.", config.daemon.mode);
            }
        }
    }
    
    Ok(())
}

