use alacritty_terminal::event::{Event, EventListener, Notify, OnResize, WindowSize};
use alacritty_terminal::event_loop::Msg;
use alacritty_terminal::event_loop::{EventLoop, Notifier};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::tty;
/// Container session management.
///
/// Each running container gets a `ContainerSession` that owns a PTY process
/// (`docker run -it …`) and a `vt100::Parser` screen buffer updated in
/// real-time by a background reader thread.
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::info;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tempfile::NamedTempFile;

use crate::config::{AgentKind, ContainerDef, ContainerMount, MountMode};

/// Live container session state and PTY plumbing for a launched container.
pub struct ContainerSession {
    pub container_name: String,
    pub container_id: String,
    pub docker_name: String,
    pub project: String,
    pub session_token: String,
    pub mount_target: String,
    pub launched_at: Instant,
    pub term: Arc<FairMutex<Term<SessionEventProxy>>>,
    notifier: Notifier,
    window_size: Arc<Mutex<WindowSize>>,
    pub exited: Arc<AtomicBool>,
    pub has_bell: Arc<AtomicBool>,
    pub exit_reported: bool,
    _scoped_proxy: Option<crate::proxy::ScopedProxyListener>,
    _cred_tempfile: Option<NamedTempFile>,
    _env_tempfile: Option<NamedTempFile>,
}

/// Event sink that keeps the Alacritty-backed PTY state synchronized with the
/// event loop and the UI.
#[derive(Clone)]
pub struct SessionEventProxy {
    sender: Arc<Mutex<Option<alacritty_terminal::event_loop::EventLoopSender>>>,
    window_size: Arc<Mutex<WindowSize>>,
    exited: Arc<AtomicBool>,
    has_bell: Arc<AtomicBool>,
    default_fg: alacritty_terminal::vte::ansi::Rgb,
    default_bg: alacritty_terminal::vte::ansi::Rgb,
    grayscale_palette: bool,
}

impl EventListener for SessionEventProxy {
    fn send_event(&self, event: Event) {
        let sender = self.sender.lock().ok().and_then(|s| s.clone());
        match event {
            Event::Bell => {
                self.has_bell.store(true, Ordering::Relaxed);
            }
            Event::Exit | Event::ChildExit(_) => {
                self.exited.store(true, Ordering::Relaxed);
            }
            Event::PtyWrite(s) => {
                if let Some(tx) = sender {
                    let _ = tx.send(Msg::Input(s.into_bytes().into()));
                }
            }
            Event::TextAreaSizeRequest(formatter) => {
                if let Some(tx) = sender {
                    let size = self.window_size.lock().map(|s| *s).unwrap_or(WindowSize {
                        num_lines: 24,
                        num_cols: 80,
                        cell_width: 0,
                        cell_height: 0,
                    });
                    let _ = tx.send(Msg::Input(formatter(size).into_bytes().into()));
                }
            }
            Event::ColorRequest(index, formatter) => {
                if let Some(tx) = sender {
                    let rgb = if index == 10 {
                        self.default_fg
                    } else if index == 11 {
                        self.default_bg
                    } else {
                        let (r, g, b) = xterm_256_index_to_rgb(index as u8);
                        let (r, g, b) = if self.grayscale_palette {
                            let y = luma_u8((r, g, b));
                            (y, y, y)
                        } else {
                            (r, g, b)
                        };
                        let blend_weight = if self.grayscale_palette { 0.45 } else { 0.35 };
                        let (r, g, b) = blend_toward_bg(
                            (r, g, b),
                            (self.default_bg.r, self.default_bg.g, self.default_bg.b),
                            blend_weight,
                        );
                        alacritty_terminal::vte::ansi::Rgb { r, g, b }
                    };
                    let _ = tx.send(Msg::Input(formatter(rgb).into_bytes().into()));
                }
            }
            Event::ClipboardLoad(_ty, formatter) => {
                if let Some(tx) = sender {
                    let _ = tx.send(Msg::Input(formatter("").into_bytes().into()));
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TermSize {
    cols: usize,
    lines: usize,
}

impl Dimensions for TermSize {
    fn columns(&self) -> usize {
        self.cols
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn last_column(&self) -> alacritty_terminal::index::Column {
        alacritty_terminal::index::Column(self.cols.saturating_sub(1))
    }
    fn topmost_line(&self) -> alacritty_terminal::index::Line {
        alacritty_terminal::index::Line(0)
    }
    fn bottommost_line(&self) -> alacritty_terminal::index::Line {
        alacritty_terminal::index::Line(self.lines.saturating_sub(1) as i32)
    }
    fn history_size(&self) -> usize {
        0
    }
}

impl ContainerSession {
    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }
    pub fn has_bell(&self) -> bool {
        self.has_bell.load(Ordering::Relaxed)
    }
    pub fn clear_bell(&self) {
        self.has_bell.store(false, Ordering::Relaxed);
    }
    pub fn send_input(&self, bytes: Vec<u8>) {
        self.notifier.notify(bytes);
    }

    pub fn terminate(&self) {
        let target = if !self.container_id.is_empty() {
            &self.container_id
        } else {
            &self.docker_name
        };
        if target.is_empty() || target == "unknown" {
            return;
        }
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", target])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> anyhow::Result<()> {
        if let Ok(size) = self.window_size.lock() {
            if size.num_lines == rows && size.num_cols == cols {
                return Ok(());
            }
        }
        let ws = WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 0,
            cell_height: 0,
        };
        if let Ok(mut s) = self.window_size.lock() {
            *s = ws;
        }
        self.notifier.on_resize(ws);
        let mut term = self.term.lock();
        term.resize(TermSize {
            cols: cols as usize,
            lines: rows as usize,
        });
        Ok(())
    }

    pub fn tab_label(&self) -> String {
        format!("{} @ {}", self.container_name, self.project)
    }
}

fn loopback_to_host_docker(url: &str) -> String {
    url.replace("127.0.0.1", "host.docker.internal")
        .replace("localhost", "host.docker.internal")
        .replace("0.0.0.0", "host.docker.internal")
}

/// Convert an arbitrary project or container name into a Docker-safe name.
pub fn sanitize_docker_name(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    if out.is_empty() {
        "container".to_string()
    } else {
        out
    }
}

fn mount_mode_arg(mode: &MountMode) -> &'static str {
    match mode {
        MountMode::Ro => "ro",
        MountMode::Rw => "rw",
    }
}

fn find_codex_home_container_path(mounts: &[ContainerMount]) -> Option<&Path> {
    mounts.iter().find_map(|mount| {
        (mount.container == PathBuf::from("/home/ubuntu/.codex")
            || mount.container == PathBuf::from("/root/.codex"))
            .then_some(mount.container.as_path())
    })
}

fn mounts_include_codex_session_state(mounts: &[ContainerMount]) -> bool {
    mounts.iter().any(|mount| {
        let container = mount.container.to_string_lossy();
        container.contains(".codex")
            || container.contains(".config/codex")
            || container.contains("codex")
    })
}

fn append_codex_home_args(docker_args: &mut Vec<String>, host_path: &Path) -> Result<()> {
    let codex_home = host_path.join(".codex");
    std::fs::create_dir_all(&codex_home).with_context(|| {
        format!(
            "failed to create codex home directory at {}",
            codex_home.display()
        )
    })?;
    let container_path = "/home/ubuntu/.codex";
    docker_args.push("-e".to_string());
    docker_args.push(format!("CODEX_HOME={container_path}"));
    docker_args.push("-v".to_string());
    docker_args.push(format!("{}:{container_path}:rw", codex_home.display()));
    Ok(())
}

fn find_gemini_home_container_path(mounts: &[ContainerMount]) -> Option<&Path> {
    mounts.iter().find_map(|mount| {
        (mount.container == PathBuf::from("/home/ubuntu/.gemini")
            || mount.container == PathBuf::from("/root/.gemini"))
            .then_some(mount.container.as_path())
    })
}

fn mounts_include_gemini_session_state(mounts: &[ContainerMount]) -> bool {
    mounts.iter().any(|mount| {
        let container = mount.container.to_string_lossy();
        container.contains(".gemini") || container.contains(".config/gemini")
    })
}

fn append_gemini_home_args(docker_args: &mut Vec<String>, host_path: &Path) -> Result<()> {
    let gemini_home = host_path.join(".gemini");
    std::fs::create_dir_all(&gemini_home).with_context(|| {
        format!(
            "failed to create gemini home directory at {}",
            gemini_home.display()
        )
    })?;
    for container_path in ["/home/ubuntu/.gemini", "/root/.gemini"] {
        docker_args.push("-v".to_string());
        docker_args.push(format!("{}:{container_path}:rw", gemini_home.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::append_gemini_home_args;

    #[test]
    fn gemini_home_args_mounts_both_possible_homes() {
        let root = std::env::temp_dir().join(format!("void-claw-gemini-home-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let mut args = Vec::new();
        append_gemini_home_args(&mut args, &root).expect("append gemini args");

        let mounts: Vec<String> = args
            .chunks_exact(2)
            .filter_map(|chunk| {
                if chunk[0] == "-v" {
                    Some(chunk[1].clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(mounts.iter().any(|m| m.ends_with(":/home/ubuntu/.gemini:rw")));
        assert!(mounts.iter().any(|m| m.ends_with(":/root/.gemini:rw")));
    }

    #[test]
    fn compose_no_proxy_handles_empty_and_duplicates() {
        use super::compose_no_proxy;
        let bypass = vec!["google.com".to_string(), "  ".to_string(), "localhost".to_string()];
        let out = compose_no_proxy(&bypass);
        // Default: localhost,127.0.0.1,host.docker.internal
        assert!(out.contains("localhost"));
        assert!(out.contains("127.0.0.1"));
        assert!(out.contains("host.docker.internal"));
        assert!(out.contains("google.com"));
        // "localhost" was already there, "  " should be ignored
        assert_eq!(out.split(',').filter(|&s| s == "localhost").count(), 1);
        assert!(!out.contains("  "));
    }

    #[test]
    fn sanitize_docker_name_works() {
        use super::sanitize_docker_name;
        assert_eq!(sanitize_docker_name("my project"), "my-project");
        assert_eq!(sanitize_docker_name("my@proj!ect"), "my-proj-ect");
        assert_eq!(sanitize_docker_name(""), "container");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClaudeSessionSource {
    SetupTokenFile,
    #[cfg(target_os = "macos")]
    SetupTokenKeychain,
}

#[cfg(target_os = "macos")]
fn read_keychain_value(service: &str) -> Option<String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", service, "-w"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let val = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if val.is_empty() { None } else { Some(val) }
}

#[cfg(target_os = "macos")]
fn extract_claude_keychain_credential() -> Option<String> {
    read_keychain_value("Claude Code-credentials")
}

#[cfg(not(target_os = "macos"))]
fn extract_claude_keychain_credential() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn read_claude_setup_token() -> Option<(String, ClaudeSessionSource)> {
    if let Some(token) = read_keychain_value("void-claw-claude-setup-token") {
        return Some((token, ClaudeSessionSource::SetupTokenKeychain));
    }
    read_setup_token_file().map(|token| (token, ClaudeSessionSource::SetupTokenFile))
}

#[cfg(not(target_os = "macos"))]
fn read_claude_setup_token() -> Option<(String, ClaudeSessionSource)> {
    read_setup_token_file().map(|token| (token, ClaudeSessionSource::SetupTokenFile))
}

fn read_setup_token_file() -> Option<String> {
    let path = dirs::config_dir()?.join("void-claw").join("claude-setup-token");
    let contents = std::fs::read_to_string(path).ok()?;
    let token = contents.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

/// Launch `docker run` for a container definition and wire it to a PTY-backed
/// terminal session.
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

    let cidfile = std::env::temp_dir().join(format!("void-claw-cid-{}.txt", uuid::Uuid::new_v4()));
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

    let container_id = read_container_id(&cidfile, &docker_run_name)
        .context("reading docker container id")?;
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

fn read_container_id(cidfile: &Path, docker_name: &str) -> Result<String> {
    for _ in 0..400 {
        if let Ok(contents) = std::fs::read_to_string(cidfile) {
            let id = contents.trim().to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
        if let Some(id) = inspect_container_id(docker_name)? {
            return Ok(id);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    anyhow::bail!(
        "failed to read docker container id from {} or inspect container {}",
        cidfile.display(),
        docker_name
    );
}

fn inspect_container_id(docker_name: &str) -> Result<Option<String>> {
    let output = std::process::Command::new("docker")
        .args(["inspect", "--format", "{{.Id}}", docker_name])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("running docker inspect")?;

    if !output.status.success() {
        return Ok(None);
    }

    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(id))
    }
}

pub fn inspect_container_exit(docker_name: &str) -> Result<Option<(Option<i32>, String)>> {
    let output = std::process::Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{.State.ExitCode}}|{{.State.Error}}",
            docker_name,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("running docker inspect")?;

    if !output.status.success() {
        return Ok(None);
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut parts = raw.trim().splitn(2, '|');
    let exit_code = parts
        .next()
        .and_then(|s| s.trim().parse::<i32>().ok());
    let error = parts.next().unwrap_or("").trim().to_string();
    Ok(Some((exit_code, error)))
}

fn compose_no_proxy(bypass_proxy: &[String]) -> String {
    let mut entries = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "host.docker.internal".to_string(),
    ];
    for host in bypass_proxy {
        let host = host.trim();
        if host.is_empty() {
            continue;
        }
        if !entries.iter().any(|e| e == host) {
            entries.push(host.to_string());
        }
    }
    entries.join(",")
}

fn detect_default_colors() -> ((u8, u8, u8), (u8, u8, u8)) {
    parse_colorfgbg(env::var("COLORFGBG").ok().as_deref())
}

fn parse_colorfgbg(colorfgbg: Option<&str>) -> ((u8, u8, u8), (u8, u8, u8)) {
    let fallback = (ansi_index_to_rgb(15), ansi_index_to_rgb(0));
    let Some(val) = colorfgbg else {
        return fallback;
    };
    let parts: Vec<u8> = val
        .split(';')
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .collect();
    if parts.len() < 2 {
        return fallback;
    }
    let fg_idx = parts[parts.len().saturating_sub(2)];
    let bg_idx = parts[parts.len().saturating_sub(1)];
    if fg_idx == bg_idx {
        return fallback;
    }
    let fg = ansi_index_to_rgb(fg_idx);
    let bg = ansi_index_to_rgb(bg_idx);
    if fg == bg {
        return fallback;
    }
    (fg, bg)
}

fn ansi_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0x00, 0x00, 0x00),
        1 => (0xcd, 0x00, 0x00),
        2 => (0x00, 0xcd, 0x00),
        3 => (0xcd, 0xcd, 0x00),
        4 => (0x00, 0x00, 0xee),
        5 => (0xcd, 0x00, 0xcd),
        6 => (0x00, 0xcd, 0xcd),
        7 => (0xe5, 0xe5, 0xe5),
        8 => (0x7f, 0x7f, 0x7f),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x5c, 0x5c, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        _ => (0xff, 0xff, 0xff),
    }
}

fn xterm_256_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ansi_index_to_rgb(idx),
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i / 6) % 6;
            let b = i % 6;
            (xterm_cube(r), xterm_cube(g), xterm_cube(b))
        }
        232..=255 => {
            let shade = 8 + (idx - 232) * 10;
            (shade, shade, shade)
        }
    }
}

fn xterm_cube(v: u8) -> u8 {
    match v {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

fn blend_toward_bg(fg: (u8, u8, u8), bg: (u8, u8, u8), fg_weight: f32) -> (u8, u8, u8) {
    let fg_weight = fg_weight.clamp(0.0, 1.0);
    let bg_weight = 1.0 - fg_weight;
    let blend = |f: u8, b: u8| -> u8 {
        ((f as f32) * fg_weight + (b as f32) * bg_weight)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (blend(fg.0, bg.0), blend(fg.1, bg.1), blend(fg.2, bg.2))
}

fn luma_u8((r, g, b): (u8, u8, u8)) -> u8 {
    let y = 0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32);
    y.round().clamp(0.0, 255.0) as u8
}
