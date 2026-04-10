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
use engine::ingest::RefinementEngine;
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
            match adapter.fetch() {
                Ok(data) => {
                    if let Err(e) = RefinementEngine::process(&data, &wiki_root).await {
                        eprintln!("Failed to process history: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching history for {}: {}", agent, e);
                }
            }
        }
        Commands::Ingest { dir, url: _url } => {
            if let Some(d) = dir {
                println!("Ingesting directory: {}", d);
                let fs_adapter = adapters::FsAdapter::new(d);
                if let Ok(files_content) = fs_adapter.fetch_all() {
                    for content in files_content {
                        RefinementEngine::process(&content, &wiki_root).await?;
                    }
                }
            } else {
                println!("No directory specified for ingest.");
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
                        match adapter.fetch() {
                            Ok(data) => {
                                // Only process if it actually found data and not the mock fallback message
                                if !data.contains("No chat history found") {
                                    if let Err(e) = RefinementEngine::process(&data, &wiki_root).await {
                                        eprintln!("[Daemon] Failed to process {}: {}", agent, e);
                                    }
                                } else {
                                    println!("[Daemon] No new data for {}", agent);
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
                                        if let Ok(data) = adapter.fetch() {
                                            if !data.contains("No chat history found") {
                                                // Note: In a real app we'd spawn this to avoid blocking the watcher thread,
                                                // but for MVP blocking is okay.
                                                if let Err(e) = tokio::runtime::Handle::current().block_on(RefinementEngine::process(&data, &wiki_root)) {
                                                    eprintln!("[Watcher] Failed to process {}: {}", agent, e);
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