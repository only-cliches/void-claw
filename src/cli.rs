use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "agent-zero",
    version,
    about = "Agent workspace manager — safely exposes filtered project workspaces to AI coding agents"
)]
pub struct Cli {
    /// Path to config file. Starts the interactive workspace manager.
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Generate a sample config file. Defaults to ./agent-zero.toml if no path is given.
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        default_missing_value = "agent-zero.toml"
    )]
    pub init: Option<PathBuf>,
}
