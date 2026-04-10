mod cli;
mod storage;
mod adapters;
mod engine;
mod mcp;
mod config;

use clap::Parser;
use cli::{Cli, Commands};
use storage::WikiStorage;
use adapters::{Adapter, HistoryAdapter};
use engine::ingest::{RefinementEngine, ProcessMode};
use config::AppConfig;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc::channel;
use std::collections::HashMap;
use std::path::PathBuf;

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
            }
        }
        Commands::Mcp { mode } => {
            println!("Starting MCP server in {} mode", mode);
            if mode == "stdio" {
                mcp::run_stdio_server().await?;
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