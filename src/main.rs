mod cli;
mod storage;
mod adapters;
mod engine;
mod mcp;

use clap::Parser;
use cli::{Cli, Commands};
use storage::WikiStorage;
use adapters::{Adapter, CursorAdapter};
use engine::ingest::RefinementEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize storage
    let storage = WikiStorage::new(Some(std::path::PathBuf::from("./.wiki")));

    match &cli.command {
        Commands::Pull { agent } => {
            println!("Pulling history for agent: {}", agent);
            if agent == "cursor" {
                let adapter = CursorAdapter;
                if let Ok(data) = adapter.fetch() {
                    RefinementEngine::process(&data).await?;
                }
            }
        }
        Commands::Ingest { dir, url } => {
            println!("Ingesting data...");
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