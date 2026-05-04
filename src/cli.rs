use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "harness-hat-manager",
    version,
    about = "LLM agent workspace manager — safely exposes filtered workspaces to AI coding agents"
)]
pub struct Cli {
    /// Path to config file. Starts the interactive workspace manager.
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Generate a sample config file. Defaults to ./harness-hat.toml if no path is given.
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        default_missing_value = "harness-hat.toml"
    )]
    pub init: Option<PathBuf>,
}
