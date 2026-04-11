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

        /// Output generation mode: 'wiki' (knowledge base), 'skill' (AI skill prompt), 'persona', 'postmortem', 'spec', 'onboard'
        #[arg(short, long, default_value = "wiki")]
        mode: String,
        
        /// Explicitly specify the output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Analyze a public GitHub repository
    Github {
        /// The GitHub repository URL (e.g., https://github.com/user/repo)
        #[arg(value_name = "URL")]
        url: String,

        /// Output generation mode (e.g., 'persona', 'onboard')
        #[arg(short, long, default_value = "persona")]
        mode: String,

        /// Explicitly specify the output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Manage IDE/CLI skills for agent-wiki-os
    Skills {
        /// The action to perform (e.g., 'install')
        #[arg(value_name = "ACTION")]
        action: String,

        /// Target IDE or tool (e.g., 'trae', 'trae-cn', 'cursor', 'all')
        #[arg(value_name = "TARGET")]
        target: String,
    },
    /// Start the MCP Server
    Mcp {
        #[arg(short, long, default_value = "stdio")]
        mode: String,
    },
    /// Start the background daemon to auto-ingest history
    Daemon,
    /// Configure global settings (e.g., llm.enable, llm.model)
    Config {
        /// The sub-command for config (e.g., 'set', 'get')
        #[arg(value_name = "ACTION")]
        action: String,

        /// The key to set or get (e.g., 'llm.model')
        #[arg(value_name = "KEY")]
        key: String,

        /// The value to set (required if action is 'set')
        #[arg(value_name = "VALUE")]
        value: Option<String>,
    },
}
