use anyhow::Result;
use clap::Parser;
use crossterm::style::Stylize;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::cli::Cli;

/// Run the interactive workspace manager (current TUI behavior).
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    if let Some(init_path) = cli.init {
        crate::init::write_sample_config(&init_path)?;
        info!("config written to: {}", init_path.display());
        info!(
            "edit it, then run: harness-hat-manager --config {}",
            init_path.display()
        );
        return Ok(());
    }

    let config_path = match resolve_or_prompt_config_path(cli.config)? {
        Some(path) => path,
        None => return Ok(()),
    };

    let config = crate::config::load(&config_path)?;
    crate::init::ensure_base_dockerfile(&config.docker_dir)?;
    crate::init::ensure_default_dockerfile(&config.docker_dir)?;
    crate::init::ensure_docker_assets(&config.docker_dir)?;

    if which::which("docker").is_err() {
        anyhow::bail!(
            "docker not found in PATH — harness-hat-manager requires Docker to run containers"
        );
    }

    let telemetry_handle = crate::telemetry::init(&config)?;
    info!("loaded config from {}", config_path.display());

    let config = Arc::new(config);
    let shared_config = crate::shared_config::SharedConfig::new(config.clone());
    let state = crate::state::StateManager::open(&config.logging.log_dir)?;
    let token = state.get_or_create_token()?;

    let ca_dir = config.logging.log_dir.join("ca");
    let ca = Arc::new(crate::ca::CaStore::load_or_create(&ca_dir)?);

    let ca_cert_path = ca_dir.join("ca.crt");
    info!(
        "{}",
        crate::agents::ca_setup_instructions(&ca.cert_pem, &ca_cert_path.display().to_string())
    );

    let (exec_pending_tx, exec_pending_rx) = mpsc::channel::<crate::server::PendingItem>(64);
    let (stop_pending_tx, stop_pending_rx) = mpsc::channel::<crate::server::ContainerStopItem>(64);
    let (net_pending_tx, net_pending_rx) = mpsc::channel::<crate::proxy::PendingNetworkItem>(64);
    let (audit_tx, audit_rx) = mpsc::channel(256);

    let session_registry = crate::server::SessionRegistry::default();

    let exec_port = config.defaults.hostdo.server_port;
    let exec_host = config.defaults.hostdo.server_host.clone();
    let exec_addr = format!("{exec_host}:{exec_port}");
    let server_state = crate::server::ServerState {
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
        if let Err(e) = crate::server::run_with_listener(server_state, exec_listener).await {
            error!("exec server error: {e}");
        }
    });

    let proxy_port = config.defaults.proxy.proxy_port;
    let proxy_host = config.defaults.proxy.proxy_host.clone();
    let proxy_addr = format!("{proxy_host}:{proxy_port}");
    let proxy_state =
        crate::proxy::ProxyState::new(ca.clone(), shared_config.clone(), net_pending_tx)?;
    let proxy_addr_display = proxy_addr.clone();
    let proxy_state_for_server = proxy_state.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::proxy::run(proxy_state_for_server, proxy_addr).await {
            error!("proxy error: {e}");
        }
    });

    let ca_cert_path_str = ca_cert_path.display().to_string();
    let app = crate::tui::App::new(
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
    crate::tui::run(app).await?;

    telemetry_handle.shutdown()?;
    Ok(())
}

pub fn resolve_or_prompt_config_path(explicit: Option<PathBuf>) -> Result<Option<PathBuf>> {
    match explicit {
        Some(path) => Ok(Some(path)),
        None => match discover_default_config_path() {
            Some(path) => Ok(Some(path)),
            None => create_config_from_prompt(),
        },
    }
}

pub fn discover_default_config_path() -> Option<PathBuf> {
    let cwd_candidate = PathBuf::from("harness-hat.toml");
    if cwd_candidate.exists() {
        return Some(cwd_candidate);
    }
    let home_candidate = default_home_config_path().ok()?;
    if home_candidate.exists() {
        return Some(home_candidate);
    }
    None
}

pub fn default_home_config_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".config/harness-hat/harness-hat.toml"))
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
            crate::init::write_sample_config(&path)?;
            println!("created config: {}", path.display());
            Ok(Some(path))
        }
        ConfigCreationChoice::CreateCwd => {
            let path = PathBuf::from("harness-hat.toml");
            crate::init::write_sample_config(&path)?;
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
        "1. Create default config at ~/.config/harness-hat/harness-hat.toml {}",
        "(Recommended)".dark_grey()
    );
    println!(
        "2. Create default config at {}/harness-hat.toml",
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
    use super::{ConfigCreationChoice, resolve_or_prompt_config_path};
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
    fn resolve_or_prompt_config_path_returns_explicit_without_prompting() {
        let explicit = PathBuf::from("/tmp/explicit-harness-hat.toml");
        let resolved = resolve_or_prompt_config_path(Some(explicit.clone())).expect("resolve path");
        assert_eq!(resolved, Some(explicit));
    }
}
