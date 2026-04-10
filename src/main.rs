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
    }

    Ok(())
}