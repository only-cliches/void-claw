use alacritty_terminal::event::WindowSize;
use alacritty_terminal::event_loop::{EventLoop, Notifier};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Config as TermConfig;
use alacritty_terminal::term::Term;
use alacritty_terminal::tty;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::time::Instant;
use tracing::info;
use tracing::instrument;

use crate::config::{AgentKind, ContainerDef};
use crate::container::core::{
    TermSize, append_codex_home_args, append_gemini_home_args, extract_claude_keychain_credential,
    find_codex_home_container_path, find_gemini_home_container_path, loopback_to_host_docker,
    mount_mode_arg, mounts_include_codex_session_state, mounts_include_gemini_session_state,
    read_claude_setup_token, sanitize_docker_name,
};
use crate::container::helpers::detect_default_colors;
use crate::container::{ContainerSession, SessionEventProxy, compose_no_proxy, read_container_id};

/// Launch `docker run` for a container definition and wire it to a PTY-backed
/// terminal session.
#[instrument(skip(
    ctr,
    workspace_path,
    codex_home_host_path,
    gemini_home_host_path,
    scoped_proxy
))]
pub fn spawn(
    ctr: &ContainerDef,
    project_name: &str,
    workspace_path: &Path,
    codex_home_host_path: Option<&Path>,
    gemini_home_host_path: Option<&Path>,
    session_token: &str,
    token: &str,
    exec_url: &str,
    proxy_url: &str,
    ca_cert_host_path: &str,
    scoped_proxy: Option<crate::proxy::ScopedProxyListener>,
    strict_network: bool,
    rows: u16,
    cols: u16,
) -> Result<(ContainerSession, Vec<String>)> {
    let ca_env_path = "/usr/local/share/ca-certificates/void-claw-ca.crt";
    let no_proxy = if strict_network {
        compose_no_proxy(&[])
    } else {
        compose_no_proxy(&ctr.bypass_proxy)
    };
    let mount_str = ctr.mount_target.display().to_string();

    let cidfile =
        std::env::temp_dir().join(format!("void-claw-cid-{}.txt", uuid::Uuid::new_v4()));
    let docker_run_name = format!(
        "void-claw-{}-{}",
        sanitize_docker_name(&ctr.name),
        uuid::Uuid::new_v4().simple()
    );

    let container_exec_url = loopback_to_host_docker(exec_url);
    let container_proxy_url = loopback_to_host_docker(proxy_url);
    let container_proxy_addr = container_proxy_url
        .strip_prefix("http://")
        .or_else(|| container_proxy_url.strip_prefix("https://"))
        .unwrap_or(&container_proxy_url)
        .to_string();
    let mut launch_notes = Vec::new();

    let mut docker_args: Vec<String> = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-it".to_string(),
        "--name".to_string(),
        docker_run_name.clone(),
        "--cidfile".to_string(),
        cidfile.display().to_string(),
    ];

    #[cfg(target_os = "linux")]
    docker_args.push("--add-host=host.docker.internal:host-gateway".to_string());

    #[cfg(target_os = "linux")]
    if !strict_network {
        docker_args.extend_from_slice(&["--user".to_string(), "1000:1000".to_string()]);
    }

    if strict_network {
        docker_args.extend_from_slice(&["--user".to_string(), "0:0".to_string()]);
        #[cfg(target_os = "macos")]
        {
            // Docker Desktop exposes `/dev/net/tun` for strict mode only when the
            // container is privileged.
            docker_args.push("--privileged".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            docker_args.extend_from_slice(&["--cap-add".to_string(), "NET_ADMIN".to_string()]);
            if Path::new("/dev/net/tun").exists() {
                docker_args.extend_from_slice(&[
                    "--device".to_string(),
                    "/dev/net/tun:/dev/net/tun".to_string(),
                ]);
            } else {
                anyhow::bail!(
                    "Strict network mode requires /dev/net/tun on the host. Cannot safely fallback to --privileged."
                );
            }
        }
    }

    docker_args.extend_from_slice(&[
        "-v".to_string(),
        format!("{}:{}:rw", workspace_path.display(), mount_str),
        "-v".to_string(),
        format!("{ca_cert_host_path}:{ca_env_path}:ro"),
        "-w".to_string(),
        mount_str.clone(),
    ]);

    // Prepare secure env file to prevent token leakage via `ps`
    let mut env_file = tempfile::Builder::new()
        .prefix("void-claw-env-")
        .tempfile()
        .context("failed to create temp env file")?;

    let ca_env_vars = [
        "SSL_CERT_FILE",
        "CURL_CA_BUNDLE",
        "NODE_EXTRA_CA_CERTS",
        "DENO_CERT",
        "REQUESTS_CA_BUNDLE",
        "AWS_CA_BUNDLE",
        "GIT_SSL_CAINFO",
        "GRPC_DEFAULT_SSL_ROOTS_FILE_PATH",
    ];
    for var in ca_env_vars {
        writeln!(env_file, "{var}={ca_env_path}")?;
    }

    writeln!(env_file, "VOID_CLAW_TOKEN={token}")?;
    writeln!(env_file, "VOID_CLAW_SESSION_TOKEN={session_token}")?;
    writeln!(env_file, "VOID_CLAW_PROJECT={project_name}")?;
    writeln!(env_file, "VOID_CLAW_MOUNT_TARGET={mount_str}")?;
    writeln!(env_file, "VOID_CLAW_URL={container_exec_url}")?;
    writeln!(
        env_file,
        "VOID_CLAW_STRICT_NETWORK={}",
        if strict_network { "1" } else { "0" }
    )?;
    writeln!(
        env_file,
        "VOID_CLAW_SCOPED_PROXY_ADDR={container_proxy_addr}"
    )?;

    if !strict_network {
        writeln!(env_file, "HTTP_PROXY={container_proxy_url}")?;
        writeln!(env_file, "HTTPS_PROXY={container_proxy_url}")?;
        writeln!(env_file, "ALL_PROXY={container_proxy_url}")?;
        writeln!(env_file, "NO_PROXY={no_proxy}")?;
        writeln!(env_file, "http_proxy={container_proxy_url}")?;
        writeln!(env_file, "https_proxy={container_proxy_url}")?;
        writeln!(env_file, "all_proxy={container_proxy_url}")?;
        writeln!(env_file, "no_proxy={no_proxy}")?;
    }

    docker_args.push("--env-file".to_string());
    docker_args.push(env_file.path().display().to_string());

    if ctr.agent == AgentKind::Codex && !ctr.env_passthrough.iter().any(|v| v == "CODEX_HOME") {
        // Prefer a real host-mounted Codex home when the container already has
        // one; otherwise create a project-scoped cache directory so sessions
        // survive container restarts without leaking across projects.
        if let Some(container_codex_home) = find_codex_home_container_path(&ctr.mounts) {
            let note = format!(
                "Codex session data imported from mounted CODEX_HOME at {}",
                container_codex_home.display()
            );
            info!("{note}");
            launch_notes.push(note);
            docker_args.push("-e".to_string());
            docker_args.push(format!("CODEX_HOME={}", container_codex_home.display()));
        } else if mounts_include_codex_session_state(&ctr.mounts) {
            let note =
                "Codex session data is already mounted in the container; leaving existing Codex state paths untouched"
                    .to_string();
            info!("{note}");
            launch_notes.push(note);
        } else if let Some(host_path) = codex_home_host_path {
            // No existing host-state mounts — use per-project persistence.
            let note = format!(
                "Codex session data imported from host cache at {}",
                host_path.join(".codex").display()
            );
            info!("{note}");
            launch_notes.push(note);
            append_codex_home_args(&mut docker_args, host_path)?;
        }
    }

    if ctr.agent == AgentKind::Gemini {
        if let Some(container_gemini_home) = find_gemini_home_container_path(&ctr.mounts) {
            let note = format!(
                "Gemini session data imported from mounted .gemini at {}",
                container_gemini_home.display()
            );
            info!("{note}");
            launch_notes.push(note);
        } else if mounts_include_gemini_session_state(&ctr.mounts) {
            let note =
                "Gemini session data is already mounted in the container; leaving existing Gemini state paths untouched"
                    .to_string();
            info!("{note}");
            launch_notes.push(note);
        } else if let Some(host_path) = gemini_home_host_path {
            let note = format!(
                "Gemini session data imported from host cache at {}",
                host_path.join(".gemini").display()
            );
            info!("{note}");
            launch_notes.push(note);
            append_gemini_home_args(&mut docker_args, host_path)?;
        }
    }

    for mount in &ctr.mounts {
        if ctr.agent == crate::config::AgentKind::Claude {
            if mount.container == PathBuf::from("/home/ubuntu/.claude.json") {
                if let Ok(meta) = std::fs::metadata(&mount.host) {
                    if meta.is_dir() {
                        anyhow::bail!(
                            "invalid Claude mount: host path '{}' is a directory, but '{}' must be a file; fix by replacing ~/.claude.json with the credential file",
                            mount.host.display(),
                            mount.container.display()
                        );
                    } else {
                        let note = format!(
                            "Claude session data imported via mount {} -> {}",
                            mount.host.display(),
                            mount.container.display()
                        );
                        info!("{note}");
                        launch_notes.push(note);
                    }
                }
            }
            if mount.container == PathBuf::from("/home/ubuntu/.claude") {
                if let Ok(meta) = std::fs::metadata(&mount.host) {
                    if meta.is_file() {
                        anyhow::bail!(
                            "invalid Claude mount: host path '{}' is a file, but '{}' must be a directory",
                            mount.host.display(),
                            mount.container.display()
                        );
                    } else {
                        let note = format!(
                            "Claude session data imported via mount {} -> {}",
                            mount.host.display(),
                            mount.container.display()
                        );
                        info!("{note}");
                        launch_notes.push(note);
                    }
                }
            }
        }
        docker_args.push("-v".to_string());
        docker_args.push(format!(
            "{}:{}:{}",
            mount.host.display(),
            mount.container.display(),
            mount_mode_arg(&mount.mode),
        ));
    }

    for name in &ctr.env_passthrough {
        docker_args.push("-e".to_string());
        docker_args.push(name.to_string());
    }

    let mut _cred_tempfile = None;
    if ctr.agent == crate::config::AgentKind::Claude {
        if let Some((setup_token, source)) = read_claude_setup_token() {
            let note = format!(
                "Claude session data imported from {:?} and exported as CLAUDE_CODE_OAUTH_TOKEN",
                source
            );
            info!("{note}");
            launch_notes.push(note);
            docker_args.push("-e".to_string());
            docker_args.push(format!("CLAUDE_CODE_OAUTH_TOKEN={setup_token}"));
        } else {
            _cred_tempfile = extract_claude_keychain_credential().and_then(|cred_json| {
                let access_token: Option<String> =
                    serde_json::from_str::<serde_json::Value>(&cred_json)
                        .ok()
                        .and_then(|v| {
                            v.get("claudeAiOauth")?
                                .get("accessToken")?
                                .as_str()
                                .map(String::from)
                        });

                if let Some(ref tok) = access_token {
                    let note = "Claude session data imported from the macOS keychain credential and exported as CLAUDE_CODE_OAUTH_TOKEN".to_string();
                    info!("{note}");
                    launch_notes.push(note);
                    docker_args.push("-e".to_string());
                    docker_args.push(format!("CLAUDE_CODE_OAUTH_TOKEN={tok}"));
                }

                let staging_path = "/tmp/.zc-claude-credentials.json";
                tempfile::Builder::new()
                    .prefix("void-claw-claude-cred-")
                    .suffix(".json")
                    .tempfile()
                    .ok()
                    .and_then(|mut tf| {
                        tf.write_all(cred_json.as_bytes()).ok()?;
                        let host_path = tf.path().display().to_string();
                        docker_args.push("-v".to_string());
                        docker_args.push(format!("{host_path}:{staging_path}:ro"));
                        Some(tf)
                    })
            });
        }
    }

    docker_args.push(ctr.image.clone());

    info!(
        "launching container: docker {}",
        docker_args
            .iter()
            .map(|a| if a.contains(' ') || a.contains('=') {
                format!("'{a}'")
            } else {
                a.clone()
            })
            .collect::<Vec<_>>()
            .join(" ")
    );

    let (fg, bg) = detect_default_colors();
    let default_fg = alacritty_terminal::vte::ansi::Rgb {
        r: fg.0,
        g: fg.1,
        b: fg.2,
    };
    let default_bg = alacritty_terminal::vte::ansi::Rgb {
        r: bg.0,
        g: bg.1,
        b: bg.2,
    };

    let window_size = WindowSize {
        num_lines: rows,
        num_cols: cols,
        cell_width: 0,
        cell_height: 0,
    };
    let window_size_arc = Arc::new(Mutex::new(window_size));

    let exited = Arc::new(AtomicBool::new(false));
    let has_bell = Arc::new(AtomicBool::new(false));

    let proxy = SessionEventProxy {
        sender: Arc::new(Mutex::new(None)),
        window_size: Arc::clone(&window_size_arc),
        exited: Arc::clone(&exited),
        has_bell: Arc::clone(&has_bell),
        default_fg,
        default_bg,
        grayscale_palette: ctr.agent == crate::config::AgentKind::Codex,
    };

    let mut term_cfg = TermConfig::default();
    term_cfg.scrolling_history = 50_000;
    let term_size = TermSize {
        cols: cols as usize,
        lines: rows as usize,
    };
    let term = Arc::new(FairMutex::new(Term::new(
        term_cfg,
        &term_size,
        proxy.clone(),
    )));

    let mut options = tty::Options::default();
    options.shell = Some(tty::Shell::new("docker".to_string(), docker_args));
    options.working_directory = None;
    options.drain_on_exit = false;
    options.env = HashMap::new();

    let pty = tty::new(&options, window_size, 0).context("open PTY")?;
    let event_loop = EventLoop::new(Arc::clone(&term), proxy.clone(), pty, false, false)
        .context("event loop")?;
    let sender = event_loop.channel();
    let notifier = Notifier(sender.clone());
    if let Ok(mut s) = proxy.sender.lock() {
        *s = Some(sender);
    }
    let _handle = event_loop.spawn();

    let container_id =
        read_container_id(&cidfile, &docker_run_name).context("reading docker container id")?;
    let docker_name = docker_run_name.clone();
    let _ = std::fs::remove_file(&cidfile);

    Ok((
        ContainerSession {
            container_name: ctr.name.clone(),
            container_id,
            docker_name,
            project: project_name.to_owned(),
            session_token: session_token.to_string(),
            mount_target: mount_str,
            launched_at: Instant::now(),
            term,
            notifier,
            window_size: window_size_arc,
            exited,
            has_bell,
            exit_reported: false,
            _scoped_proxy: scoped_proxy,
            _cred_tempfile,
            _env_tempfile: Some(env_file),
        },
        launch_notes,
    ))
}
