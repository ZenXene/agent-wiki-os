use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Pull history from a specific agent
    Pull { agent: String },
    /// Ingest from a specific directory or URL
    Ingest { 
        #[arg(short, long)]
        dir: Option<String>,
        #[arg(short, long)]
        url: Option<String>,
    },
    /// Start the MCP Server
    Mcp {
        #[arg(short, long, default_value = "stdio")]
        mode: String,
    }
}
