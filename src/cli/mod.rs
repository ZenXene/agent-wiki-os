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
        /// Optional target (can be a URL starting with http:// or https://, or a local directory path)
        #[arg(value_name = "TARGET")]
        target: Option<String>,
        
        /// Explicitly specify a directory path
        #[arg(short, long)]
        dir: Option<String>,
        
        /// Explicitly specify a URL
        #[arg(short, long)]
        url: Option<String>,
    },
    /// Start the MCP Server
    Mcp {
        #[arg(short, long, default_value = "stdio")]
        mode: String,
    },
    /// Start the background daemon to auto-ingest history
    Daemon,
}
