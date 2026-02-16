use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "dkr", about = "Docker container monitor for NAS")]
#[command(version, propagate_version = true)]
pub struct Cli {
    /// Docker host URI (overrides config and `DOCKER_HOST`)
    #[arg(long, global = true)]
    pub host: Option<String>,

    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show container status
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Continuously refresh
        #[arg(long)]
        watch: bool,

        /// Show a specific container by name
        name: Option<String>,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Print loaded configuration
    Show,
    /// Print config file path
    Path,
}
