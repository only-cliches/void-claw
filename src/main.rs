#![allow(
    clippy::bind_instead_of_map,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::double_ended_iterator_last,
    clippy::doc_lazy_continuation,
    clippy::field_reassign_with_default,
    clippy::match_like_matches_macro,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::while_let_loop
)]

mod agents;
mod ca;
mod cli;
mod config;
mod container;
mod exec;
mod init;
mod new_project;
mod proxy;
mod rules;
mod server;
mod shared_config;
mod state;
mod telemetry;
mod tui;

use anyhow::Result;
use clap::Parser;
use crossterm::style::Stylize;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use cli::Cli;

// current_thread keeps all async tasks on one thread, which allows
// ContainerSession (containing Box<dyn MasterPty>, which is !Send) to be
// held in App across await points in the TUI event loop.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(init_path) = cli.init {
        init::write_sample_config(&init_path)?;
        info!("config written to: {}", init_path.display());
        info!(
            "edit it, then run: void-claw --config {}",
            init_path.display()
        );
        return Ok(());
    }

    let config_path = match cli.config {
        Some(path) => path,
        None => match discover_default_config_path() {
            Some(path) => path,
            None => match create_config_from_prompt()? {
                Some(path) => path,
                None => return Ok(()),
            },
        },
    };

    let config = config::load(&config_path)?;
    init::ensure_docker_assets(&config.docker_dir)?;

    // Bail early if docker is not available.
    if which::which("docker").is_err() {
        anyhow::bail!(
            "docker not found in PATH — void-claw requires Docker to run containers"
        );
    }

    // Initialise tracing (+ optional OTel export) before anything else logs.
    let telemetry_handle = telemetry::init(&config)?;
    info!("loaded config from {}", config_path.display());

    let config = Arc::new(config);

    let shared_config = shared_config::SharedConfig::new(config.clone());

    // Initialize file-backed runtime state.
    let state = state::StateManager::open(&config.logging.log_dir)?;
    let token = state.get_or_create_token()?;

    // Initialize (or load) the proxy CA certificate.
    let ca_dir = config.logging.log_dir.join("ca");
    let ca = Arc::new(ca::CaStore::load_or_create(&ca_dir)?);

    // Print CA setup instructions on first run (when ca.crt didn't exist before).
    let ca_cert_path = ca_dir.join("ca.crt");
    info!(
        "{}",
        agents::ca_setup_instructions(&ca.cert_pem, &ca_cert_path.display().to_string())
    );

    // Communication channels.
    let (exec_pending_tx, exec_pending_rx) = mpsc::channel::<server::PendingItem>(64);
    let (stop_pending_tx, stop_pending_rx) = mpsc::channel::<server::ContainerStopItem>(64);
    let (net_pending_tx, net_pending_rx) = mpsc::channel::<proxy::PendingNetworkItem>(64);
    let (audit_tx, audit_rx) = mpsc::channel(256);

    let session_registry = server::SessionRegistry::default();

    // Start the hostdo HTTP server.
    let exec_port = config.defaults.hostdo.server_port;
    let exec_host = config.defaults.hostdo.server_host.clone();
    let exec_addr = format!("{exec_host}:{exec_port}");
    let server_state = server::ServerState {
        config: shared_config.clone(),
        state: state.clone(),
        pending_tx: exec_pending_tx,
        stop_tx: stop_pending_tx,
        audit_tx,
        token: token.clone(),
        sessions: session_registry.clone(),
    };
    let exec_listener = tokio::net::TcpListener::bind(&exec_addr)
        .await
        .map_err(|e| anyhow::anyhow!("binding exec bridge to {exec_addr}: {e}"))?;
    info!("exec bridge listening on {}", exec_addr);
    tokio::spawn(async move {
        if let Err(e) = server::run_with_listener(server_state, exec_listener).await {
            error!("exec server error: {e}");
        }
    });

    // Start the MITM proxy.
    let proxy_port = config.defaults.proxy.proxy_port;
    let proxy_host = config.defaults.proxy.proxy_host.clone();
    let proxy_addr = format!("{proxy_host}:{proxy_port}");
    let proxy_state = proxy::ProxyState::new(ca.clone(), shared_config.clone(), net_pending_tx)?;
    let proxy_addr_display = proxy_addr.clone();
    let proxy_state_for_server = proxy_state.clone();
    tokio::spawn(async move {
        if let Err(e) = proxy::run(proxy_state_for_server, proxy_addr).await {
            error!("proxy error: {e}");
        }
    });

    // Build and run the TUI.
    let ca_cert_path_str = ca_cert_path.display().to_string();
    let app = tui::App::new(
        shared_config,
        config_path.clone(),
        token,
        session_registry,
        exec_pending_rx,
        stop_pending_rx,
        net_pending_rx,
        audit_rx,
        state,
        proxy_state,
        proxy_addr_display,
        ca_cert_path_str,
    )?;
    tui::run(app).await?;

    // Flush any buffered OTel spans before exit.
    telemetry_handle.shutdown()?;

    Ok(())
}

fn discover_default_config_path() -> Option<PathBuf> {
    let cwd_candidate = PathBuf::from("void-claw.toml");
    if cwd_candidate.exists() {
        return Some(cwd_candidate);
    }
    let home_candidate = default_home_config_path().ok()?;
    if home_candidate.exists() {
        return Some(home_candidate);
    }
    None
}

fn default_home_config_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".config/void-claw/void-claw.toml"))
}

enum ConfigCreationChoice {
    CreateCwd,
    CreateHome,
    Cancel,
}

fn create_config_from_prompt() -> Result<Option<PathBuf>> {
    match prompt_config_creation_choice()? {
        ConfigCreationChoice::CreateHome => {
            let path = default_home_config_path()?;
            init::write_sample_config(&path)?;
            println!("created config: {}", path.display());
            Ok(Some(path))
        }
        ConfigCreationChoice::CreateCwd => {
            let path = PathBuf::from("void-claw.toml");
            init::write_sample_config(&path)?;
            println!("created config: {}", path.display());
            Ok(Some(path))
        }
        ConfigCreationChoice::Cancel => {
            println!("cancelled");
            Ok(None)
        }
    }
}

fn prompt_config_creation_choice() -> Result<ConfigCreationChoice> {
    let cwd = std::env::current_dir()?;
    println!("No config file found.");
    println!(
        "1. Create default config at ~/.config/void-claw/void-claw.toml {}",
        "(Recommended)".dark_grey()
    );
    println!(
        "2. Create default config at {}/void-claw.toml",
        cwd.display()
    );
    println!("3. Cancel and close");
    print!("Select an option [1-3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = match input.trim() {
        "1" => ConfigCreationChoice::CreateHome,
        "2" => ConfigCreationChoice::CreateCwd,
        _ => ConfigCreationChoice::Cancel,
    };
    Ok(choice)
}

#[cfg(test)]
mod tests {
    use super::{ConfigCreationChoice, create_config_from_prompt};
    use std::path::PathBuf;

    #[test]
    fn config_creation_choice_variants_are_stable() {
        assert!(matches!(
            ConfigCreationChoice::CreateCwd,
            ConfigCreationChoice::CreateCwd
        ));
        assert!(matches!(
            ConfigCreationChoice::CreateHome,
            ConfigCreationChoice::CreateHome
        ));
        assert!(matches!(
            ConfigCreationChoice::Cancel,
            ConfigCreationChoice::Cancel
        ));
    }

    #[test]
    fn prompt_creation_helper_not_used_directly_in_tests() {
        let _ = create_config_from_prompt as fn() -> anyhow::Result<Option<PathBuf>>;
    }
}
