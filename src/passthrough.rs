use anyhow::{Context, Result, bail};
use clap::Parser;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "harness-hat",
    version,
    about = "Containerized command passthrough for local workspaces"
)]
struct WrapperOptions {
    /// Path to config file.
    #[arg(short, long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Image name resolved as <docker_dir>/<name>.dockerfile.
    #[arg(long, value_name = "NAME")]
    image: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedArgs {
    options: WrapperOptions,
    command: Vec<String>,
}

#[derive(Debug, Clone)]
struct PassthroughRuntime {
    container_name: String,
    image_stem: String,
    mount_target: PathBuf,
    mounts: Vec<crate::config::ContainerMount>,
    env_passthrough: Vec<String>,
    bypass_proxy: Vec<String>,
    agent: crate::config::AgentKind,
}

pub async fn run_and_get_exit_code() -> Result<i32> {
    let parsed = parse_args()?;

    if which::which("docker").is_err() {
        bail!("docker not found in PATH — harness-hat requires Docker to run containers");
    }

    let config_path =
        match crate::manager::resolve_or_prompt_config_path(parsed.options.config.clone())? {
            Some(path) => path,
            None => return Ok(0),
        };
    let config = crate::config::load(&config_path)?;

    crate::init::ensure_base_dockerfile(&config.docker_dir)?;
    crate::init::ensure_default_dockerfile(&config.docker_dir)?;
    crate::init::ensure_helper_scripts(&config.docker_dir)?;

    let cwd = std::env::current_dir().context("reading current directory")?;
    validate_rules(
        &config.manager.global_rules_file,
        &cwd.join("harness-rules.toml"),
    )?;

    let runtime = infer_passthrough_runtime(
        &config,
        parsed.options.image.as_deref(),
        parsed.command.first().map(String::as_str),
    );
    let image_name = runtime.image_stem.as_str();
    let dockerfile_path = config.docker_dir.join(format!("{image_name}.dockerfile"));
    if !dockerfile_path.exists() {
        bail!(
            "Looked for {} and didn't find it, please use a valid image name.",
            dockerfile_path.display()
        );
    }
    if !dockerfile_path.is_file() {
        bail!(
            "Found {}, but it is not a file. Please use a valid image name.",
            dockerfile_path.display()
        );
    }

    let image_tag = crate::config::image_tag_for_stem(image_name);
    ensure_image_built(&image_tag, &dockerfile_path, &config.docker_dir)?;

    let (term_cols, term_rows) = crossterm::terminal::size().unwrap_or((120, 40));
    let project_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "workspace".to_string());
    let config = Arc::new(config);
    let shared_config = crate::shared_config::SharedConfig::new(config.clone());
    let state = crate::state::StateManager::open(&config.logging.log_dir)?;
    let token = state.get_or_create_token()?;
    let session_registry = crate::server::SessionRegistry::default();

    let ca_dir = config.logging.log_dir.join("ca");
    let ca = Arc::new(crate::ca::CaStore::load_or_create(&ca_dir)?);
    let ca_cert_path = ca_dir.join("ca.crt");
    let ca_cert_path_str = ca_cert_path.display().to_string();

    let (exec_pending_tx, exec_pending_rx) = mpsc::channel::<crate::server::PendingItem>(64);
    let (stop_pending_tx, stop_pending_rx) = mpsc::channel::<crate::server::ContainerStopItem>(64);
    let (net_pending_tx, net_pending_rx) = mpsc::channel::<crate::proxy::PendingNetworkItem>(64);
    let (audit_tx, audit_rx) = mpsc::channel(256);

    let exec_bind_host = resolve_bind_host_for_container_access(
        &config.defaults.hostdo.server_host,
        "defaults.hostdo.server_host",
    );
    let proxy_bind_host = resolve_bind_host_for_container_access(
        &config.defaults.proxy.proxy_host,
        "defaults.proxy.proxy_host",
    );

    let exec_listener = tokio::net::TcpListener::bind(format!("{exec_bind_host}:0"))
        .await
        .map_err(|e| anyhow::anyhow!("binding exec bridge to {exec_bind_host}:0: {e}"))?;
    let exec_port = exec_listener.local_addr()?.port();
    let exec_url = format!("http://{exec_bind_host}:{exec_port}");
    let server_state = crate::server::ServerState {
        config: shared_config.clone(),
        state: state.clone(),
        pending_tx: exec_pending_tx,
        stop_tx: stop_pending_tx,
        audit_tx,
        token: token.clone(),
        sessions: session_registry.clone(),
    };
    tokio::spawn(async move {
        if let Err(e) = crate::server::run_with_listener(server_state, exec_listener).await {
            eprintln!("exec server error: {e}");
        }
    });

    let proxy_state = crate::proxy::ProxyState::new(ca, shared_config.clone(), net_pending_tx)?;
    let scoped_proxy = crate::proxy::spawn_scoped_listener(
        &proxy_state,
        &proxy_bind_host,
        &project_name,
        &runtime.container_name,
    )?;
    let proxy_url = format!("http://{}", scoped_proxy.addr);

    let ctr = crate::config::ContainerDef {
        name: runtime.container_name.clone(),
        image: image_tag.clone(),
        image_stem: runtime.image_stem.clone(),
        profile: None,
        mount_target: runtime.mount_target.clone(),
        agent: runtime.agent,
        mounts: runtime.mounts.clone(),
        env_passthrough: runtime.env_passthrough.clone(),
        bypass_proxy: runtime.bypass_proxy.clone(),
    };

    let session_token = uuid::Uuid::new_v4().simple().to_string();
    session_registry.insert(
        session_token.clone(),
        crate::server::SessionIdentity {
            project: project_name.clone(),
            container_id: String::new(),
            mount_target: ctr.mount_target.display().to_string(),
        },
    );

    let (session, launch_notes) = crate::container::spawn(
        &ctr,
        Some(parsed.command.as_slice()),
        &project_name,
        &cwd,
        None,
        None,
        &session_token,
        &token,
        &exec_url,
        &proxy_url,
        &ca_cert_path_str,
        Some(scoped_proxy),
        config.defaults.proxy.strict_network,
        term_rows.max(6),
        term_cols.max(20),
    )?;
    session_registry.insert(
        session.session_token.clone(),
        crate::server::SessionIdentity {
            project: session.project.clone(),
            container_id: session.container_id.clone(),
            mount_target: session.mount_target.clone(),
        },
    );

    let mut app = crate::tui::App::new(
        shared_config,
        config_path,
        token,
        session_registry,
        exec_pending_rx,
        stop_pending_rx,
        net_pending_rx,
        audit_rx,
        state,
        proxy_state,
        proxy_url,
        ca_cert_path_str,
    )?;

    let exit_code_slot = Arc::new(AtomicI32::new(i32::MIN));
    app.enable_passthrough_mode(exit_code_slot.clone());
    for note in launch_notes {
        app.push_log(note, false);
    }
    app.sessions.push(session);
    app.active_session = Some(0);
    app.preview_session = Some(0);
    app.focus = crate::tui::Focus::Terminal;
    app.terminal_fullscreen = true;
    app.scroll_mode = false;
    app.terminal_scroll = 0;

    crate::tui::run(app).await?;
    let code = exit_code_slot.load(Ordering::SeqCst);
    Ok(if code == i32::MIN { 0 } else { code })
}

fn validate_rules(global_rules: &Path, local_rules: &Path) -> Result<()> {
    let global = crate::rules::load(global_rules)
        .with_context(|| format!("loading global rules from {}", global_rules.display()))?;
    let local = crate::rules::load(local_rules)
        .with_context(|| format!("loading local rules from {}", local_rules.display()))?;
    let _ = crate::rules::ComposedRules::compose(&global, &[local]);
    Ok(())
}

fn resolve_bind_host_for_container_access(configured: &str, key: &str) -> String {
    let host = configured.trim();
    if host.is_empty() {
        eprintln!("warning: {key} is empty; using 0.0.0.0 for passthrough runtime");
        return "0.0.0.0".to_string();
    }
    if matches!(host, "127.0.0.1" | "localhost" | "::1") {
        eprintln!(
            "warning: {key}='{}' is loopback; using 0.0.0.0 so containers can reach host services",
            host
        );
        return "0.0.0.0".to_string();
    }
    host.to_string()
}

fn infer_agent_kind(command: Option<&str>) -> crate::config::AgentKind {
    match normalize_command_name(command).as_deref() {
        Some("codex") => crate::config::AgentKind::Codex,
        Some("claude") => crate::config::AgentKind::Claude,
        Some("gemini") => crate::config::AgentKind::Gemini,
        Some("opencode") => crate::config::AgentKind::Opencode,
        _ => crate::config::AgentKind::None,
    }
}

fn normalize_command_name(command: Option<&str>) -> Option<String> {
    let raw = command?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(
        Path::new(raw)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(raw)
            .to_ascii_lowercase(),
    )
}

fn inferred_profile_for_command<'a>(
    config: &'a crate::config::Config,
    command: Option<&str>,
) -> Option<&'a crate::config::ContainerDef> {
    let normalized = normalize_command_name(command)?;

    if let Some(ctr) = config
        .containers
        .iter()
        .find(|ctr| ctr.name.eq_ignore_ascii_case(&normalized))
    {
        return Some(ctr);
    }

    let inferred_agent = infer_agent_kind(command);
    if inferred_agent == crate::config::AgentKind::None {
        return None;
    }
    config
        .containers
        .iter()
        .find(|ctr| ctr.agent == inferred_agent)
}

fn infer_passthrough_runtime(
    config: &crate::config::Config,
    explicit_image: Option<&str>,
    command: Option<&str>,
) -> PassthroughRuntime {
    let explicit_image = explicit_image.map(str::trim).filter(|s| !s.is_empty());
    let inferred_profile = inferred_profile_for_command(config, command);
    let container_name = inferred_profile
        .map(|ctr| ctr.name.clone())
        .or_else(|| normalize_command_name(command))
        .unwrap_or_else(|| "passthrough".to_string());

    let image_stem = explicit_image
        .map(ToOwned::to_owned)
        .or_else(|| inferred_profile.map(|ctr| ctr.image_stem.clone()))
        .unwrap_or_else(|| "default".to_string());

    let mount_target = inferred_profile
        .map(|ctr| ctr.mount_target.clone())
        .or_else(|| config.defaults.containers.mount_target.clone())
        .unwrap_or_else(crate::config::default_mount_target);

    let mounts = inferred_profile
        .map(|ctr| ctr.mounts.clone())
        .unwrap_or_else(|| config.defaults.containers.mounts.clone());

    let env_passthrough = inferred_profile
        .map(|ctr| ctr.env_passthrough.clone())
        .unwrap_or_else(|| config.defaults.containers.env_passthrough.clone());

    let bypass_proxy = inferred_profile
        .map(|ctr| ctr.bypass_proxy.clone())
        .unwrap_or_else(|| config.defaults.containers.bypass_proxy.clone());

    let inferred_agent = infer_agent_kind(command);
    let agent = if inferred_agent != crate::config::AgentKind::None {
        inferred_agent
    } else {
        inferred_profile
            .map(|ctr| ctr.agent.clone())
            .unwrap_or(crate::config::AgentKind::None)
    };

    PassthroughRuntime {
        container_name,
        image_stem,
        mount_target,
        mounts,
        env_passthrough,
        bypass_proxy,
        agent,
    }
}

fn ensure_image_built(image: &str, dockerfile_path: &Path, docker_dir: &Path) -> Result<()> {
    if docker_image_exists(image)? {
        return Ok(());
    }

    let base_image = "harness-hat-base:local";
    let base_dockerfile = docker_dir.join("harness-hat-base.dockerfile");
    if !docker_image_exists(base_image)? && !base_dockerfile.exists() {
        bail!(
            "Looked for {} and didn't find it, please run setup to restore the base dockerfile.",
            base_dockerfile.display()
        );
    }
    if !docker_image_exists(base_image)? && base_dockerfile.exists() {
        eprintln!(
            "Building base image '{base_image}' from {} ...",
            base_dockerfile.display()
        );
        let base_dockerfile_arg = base_dockerfile.display().to_string();
        let docker_dir_arg = docker_dir.display().to_string();
        let base_status = std::process::Command::new("docker")
            .args([
                "build",
                "-t",
                base_image,
                "-f",
                &base_dockerfile_arg,
                &docker_dir_arg,
            ])
            .status()
            .context("starting base docker build")?;
        if !base_status.success() {
            bail!(
                "docker build failed for base image '{}' using {}",
                base_image,
                base_dockerfile.display()
            );
        }
    }

    eprintln!(
        "Building image '{image}' from {} ...",
        dockerfile_path.display()
    );
    let dockerfile_arg = dockerfile_path.display().to_string();
    let docker_dir_arg = docker_dir.display().to_string();
    let status = std::process::Command::new("docker")
        .args(["build", "-t", image, "-f", &dockerfile_arg, &docker_dir_arg])
        .status()
        .context("starting docker build")?;
    if !status.success() {
        bail!(
            "docker build failed for image '{}' using {}",
            image,
            dockerfile_path.display()
        );
    }
    Ok(())
}

fn docker_image_exists(image: &str) -> Result<bool> {
    let status = std::process::Command::new("docker")
        .args(["image", "inspect", image])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("checking docker image")?;
    Ok(status.success())
}

fn parse_args() -> Result<ParsedArgs> {
    let raw: Vec<OsString> = std::env::args_os().collect();
    parse_args_from(raw)
}

fn parse_args_from(raw: Vec<OsString>) -> Result<ParsedArgs> {
    const USAGE: &str = "Usage: harness-hat [--image NAME] -- <command ...>";

    let Some(bin_name) = raw.first().cloned() else {
        bail!("missing argv[0]. {USAGE}");
    };

    if let Some(sep_idx) = raw.iter().position(|arg| arg == std::ffi::OsStr::new("--")) {
        if sep_idx + 1 >= raw.len() {
            bail!("missing command after '--'. {USAGE}");
        }

        let mut option_argv = Vec::with_capacity(sep_idx);
        option_argv.push(bin_name);
        option_argv.extend(raw[1..sep_idx].iter().cloned());
        let options = WrapperOptions::try_parse_from(option_argv)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let command = raw[sep_idx + 1..]
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        return Ok(ParsedArgs { options, command });
    }

    // Compatibility mode for invocations like:
    //   cargo run --bin harness-hat -- codex
    // where Cargo consumes the first `--` and forwards only `codex`.
    let mut option_argv = vec![bin_name];
    let mut idx = 1usize;
    while idx < raw.len() {
        let token = raw[idx].to_string_lossy();
        let token = token.as_ref();
        let consumes_next = match token {
            "-c" | "--config" | "--image" => Some(true),
            _ => {
                if token.starts_with("--config=")
                    || token.starts_with("--image=")
                    || (token.starts_with("-c") && token.len() > 2)
                {
                    Some(false)
                } else {
                    None
                }
            }
        };

        let Some(consumes_next) = consumes_next else {
            break;
        };
        option_argv.push(raw[idx].clone());
        if consumes_next {
            if idx + 1 >= raw.len() {
                bail!("missing value for option '{token}'. {USAGE}");
            }
            option_argv.push(raw[idx + 1].clone());
            idx += 2;
        } else {
            idx += 1;
        }
    }

    let options =
        WrapperOptions::try_parse_from(option_argv).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let command = raw[idx..]
        .iter()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if command.is_empty() {
        bail!("missing command. {USAGE}");
    }
    if command[0].starts_with('-') {
        bail!("command starts with '-'; use '--' before command to disambiguate. {USAGE}");
    }

    Ok(ParsedArgs { options, command })
}

#[cfg(test)]
mod tests {
    use super::{
        infer_agent_kind, infer_passthrough_runtime, inferred_profile_for_command,
        normalize_command_name, parse_args_from, resolve_bind_host_for_container_access,
    };
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn sample_container(
        name: &str,
        image_stem: &str,
        agent: crate::config::AgentKind,
        mount_target: &str,
        env: &[&str],
        mount_host: &str,
        mount_container: &str,
    ) -> crate::config::ContainerDef {
        crate::config::ContainerDef {
            name: name.to_string(),
            image: crate::config::image_tag_for_stem(image_stem),
            image_stem: image_stem.to_string(),
            profile: None,
            mount_target: PathBuf::from(mount_target),
            agent,
            mounts: vec![crate::config::ContainerMount {
                host: PathBuf::from(mount_host),
                container: PathBuf::from(mount_container),
                mode: crate::config::MountMode::Rw,
            }],
            env_passthrough: env.iter().map(|v| (*v).to_string()).collect(),
            bypass_proxy: vec![],
        }
    }

    #[test]
    fn infer_agent_kind_from_command_name() {
        assert_eq!(
            infer_agent_kind(Some("codex")),
            crate::config::AgentKind::Codex
        );
        assert_eq!(
            infer_agent_kind(Some("claude")),
            crate::config::AgentKind::Claude
        );
        assert_eq!(
            infer_agent_kind(Some("gemini")),
            crate::config::AgentKind::Gemini
        );
        assert_eq!(
            infer_agent_kind(Some("opencode")),
            crate::config::AgentKind::Opencode
        );
        assert_eq!(
            infer_agent_kind(Some("anything-else")),
            crate::config::AgentKind::None
        );
        assert_eq!(
            infer_agent_kind(Some("/usr/local/bin/codex")),
            crate::config::AgentKind::Codex
        );
    }

    fn os(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    #[test]
    fn parse_args_accepts_explicit_separator() {
        let parsed = parse_args_from(os(&[
            "harness-hat",
            "--image",
            "default",
            "--",
            "codex",
            "--approval-mode",
            "full-auto",
        ]))
        .expect("parse args");
        assert_eq!(parsed.options.image.as_deref(), Some("default"));
        assert_eq!(
            parsed.command,
            vec!["codex", "--approval-mode", "full-auto"]
        );
    }

    #[test]
    fn parse_args_accepts_no_separator_for_simple_command() {
        let parsed = parse_args_from(os(&["harness-hat", "codex"])).expect("parse args");
        assert_eq!(parsed.options.image, None);
        assert_eq!(parsed.command, vec!["codex"]);
    }

    #[test]
    fn parse_args_accepts_no_separator_with_options() {
        let parsed = parse_args_from(os(&["harness-hat", "--image", "default", "codex"]))
            .expect("parse args");
        assert_eq!(parsed.options.image.as_deref(), Some("default"));
        assert_eq!(parsed.command, vec!["codex"]);
    }

    #[test]
    fn parse_args_requires_separator_for_hyphenated_command() {
        let err = parse_args_from(os(&["harness-hat", "--foo"]))
            .expect_err("hyphen-prefixed command should require separator");
        assert!(err.to_string().contains("use '--' before command"));
    }

    #[test]
    fn normalize_command_name_uses_basename_and_lowercase() {
        assert_eq!(
            normalize_command_name(Some("/opt/bin/CoDeX")),
            Some("codex".to_string())
        );
    }

    #[test]
    fn inferred_profile_prefers_exact_profile_name() {
        let mut cfg = crate::config::Config::default();
        cfg.containers = vec![
            sample_container(
                "assistant",
                "default",
                crate::config::AgentKind::Codex,
                "/workspace-a",
                &["A_ENV"],
                "/a",
                "/ca",
            ),
            sample_container(
                "codex",
                "special",
                crate::config::AgentKind::None,
                "/workspace-codex",
                &["CODEX_ENV"],
                "/c",
                "/cc",
            ),
        ];

        let picked =
            inferred_profile_for_command(&cfg, Some("codex")).expect("expected exact profile");
        assert_eq!(picked.name, "codex");
        assert_eq!(picked.image_stem, "special");
    }

    #[test]
    fn infer_passthrough_runtime_uses_inferred_profile_without_image_override() {
        let mut cfg = crate::config::Config::default();
        cfg.defaults.containers.mount_target = Some(PathBuf::from("/workspace-default"));
        cfg.defaults.containers.env_passthrough = vec!["DEFAULT_ENV".to_string()];
        cfg.defaults.containers.mounts = vec![crate::config::ContainerMount {
            host: PathBuf::from("/default"),
            container: PathBuf::from("/c-default"),
            mode: crate::config::MountMode::Rw,
        }];
        cfg.containers = vec![sample_container(
            "codex",
            "custom-codex",
            crate::config::AgentKind::Codex,
            "/workspace-codex",
            &["CODEX_ENV"],
            "/codex",
            "/c-codex",
        )];

        let runtime = infer_passthrough_runtime(&cfg, None, Some("codex"));
        assert_eq!(runtime.container_name, "codex");
        assert_eq!(runtime.image_stem, "custom-codex");
        assert_eq!(runtime.mount_target, PathBuf::from("/workspace-codex"));
        assert_eq!(runtime.env_passthrough, vec!["CODEX_ENV".to_string()]);
        assert_eq!(runtime.agent, crate::config::AgentKind::Codex);
        assert_eq!(runtime.mounts.len(), 1);
        assert_eq!(runtime.mounts[0].host, PathBuf::from("/codex"));
    }

    #[test]
    fn infer_passthrough_runtime_keeps_profile_runtime_with_explicit_image_override() {
        let mut cfg = crate::config::Config::default();
        cfg.containers = vec![sample_container(
            "codex",
            "default",
            crate::config::AgentKind::Codex,
            "/workspace-codex",
            &["CODEX_ENV"],
            "/codex",
            "/c-codex",
        )];

        let runtime = infer_passthrough_runtime(&cfg, Some("rust"), Some("codex"));
        assert_eq!(runtime.container_name, "codex");
        assert_eq!(runtime.image_stem, "rust");
        assert_eq!(runtime.mount_target, PathBuf::from("/workspace-codex"));
        assert_eq!(runtime.env_passthrough, vec!["CODEX_ENV".to_string()]);
        assert_eq!(runtime.agent, crate::config::AgentKind::Codex);
    }

    #[test]
    fn infer_passthrough_runtime_falls_back_to_defaults_for_unknown_command() {
        let mut cfg = crate::config::Config::default();
        cfg.defaults.containers.mount_target = Some(PathBuf::from("/workspace-default"));
        cfg.defaults.containers.env_passthrough = vec!["DEFAULT_ENV".to_string()];
        cfg.defaults.containers.mounts = vec![crate::config::ContainerMount {
            host: PathBuf::from("/default"),
            container: PathBuf::from("/c-default"),
            mode: crate::config::MountMode::Rw,
        }];

        let runtime = infer_passthrough_runtime(&cfg, None, Some("unknown-cmd"));
        assert_eq!(runtime.container_name, "unknown-cmd");
        assert_eq!(runtime.image_stem, "default");
        assert_eq!(runtime.mount_target, PathBuf::from("/workspace-default"));
        assert_eq!(runtime.env_passthrough, vec!["DEFAULT_ENV".to_string()]);
        assert_eq!(runtime.agent, crate::config::AgentKind::None);
        assert_eq!(runtime.mounts.len(), 1);
        assert_eq!(runtime.mounts[0].host, PathBuf::from("/default"));
    }

    #[test]
    fn resolve_bind_host_rewrites_loopback_for_passthrough() {
        assert_eq!(
            resolve_bind_host_for_container_access("127.0.0.1", "defaults.hostdo.server_host"),
            "0.0.0.0"
        );
        assert_eq!(
            resolve_bind_host_for_container_access("localhost", "defaults.proxy.proxy_host"),
            "0.0.0.0"
        );
        assert_eq!(
            resolve_bind_host_for_container_access("192.168.1.10", "defaults.proxy.proxy_host"),
            "192.168.1.10"
        );
    }
}
