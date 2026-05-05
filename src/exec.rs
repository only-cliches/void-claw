use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;

use crate::config::{self, Config, WorkspaceConfig};
use crate::rules::{ComposedRules, RuleCommand};

#[derive(Debug)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecTarget {
    Host,
    DockerImage(String),
}

impl ExecTarget {
    pub fn image(&self) -> Option<&str> {
        match self {
            Self::Host => None,
            Self::DockerImage(image) => Some(image.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedExec {
    pub target: ExecTarget,
    pub argv: Vec<String>,
}

#[derive(Debug)]
pub enum CommandMatch<'a> {
    /// Matched an explicit rule command.
    Explicit(&'a RuleCommand),
    /// Not in the allowlist — falls back to composed rules default_policy.
    Unlisted,
}

#[derive(Debug)]
pub enum DenyReason {
    DeniedExecutable(String),
    DeniedArgumentFragment(String),
    EmptyArgv,
    InvalidImage(String),
}

impl std::fmt::Display for DenyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeniedExecutable(exe) => write!(f, "executable '{exe}' is on the deny list"),
            Self::DeniedArgumentFragment(frag) => {
                write!(f, "argument contains denied fragment '{frag}'")
            }
            Self::EmptyArgv => write!(f, "argv must not be empty"),
            Self::InvalidImage(reason) => write!(f, "{reason}"),
        }
    }
}

/// Split hostdo's argv into an execution target and the command argv.
///
/// `hostdo --image node:20 npm test` targets a short-lived Docker runner,
/// while every other argv stays on the default host execution path.
pub fn parse_exec_target(argv: &[String]) -> Result<ParsedExec, DenyReason> {
    if argv.is_empty() {
        return Err(DenyReason::EmptyArgv);
    }

    let Some(first) = argv.first() else {
        return Err(DenyReason::EmptyArgv);
    };

    if first == "--image" {
        let Some(image) = argv.get(1) else {
            return Err(DenyReason::InvalidImage(
                "--image requires an image name".to_string(),
            ));
        };
        validate_image_name(image)?;
        if argv.len() < 3 {
            return Err(DenyReason::EmptyArgv);
        }
        return Ok(ParsedExec {
            target: ExecTarget::DockerImage(image.clone()),
            argv: argv[2..].to_vec(),
        });
    }

    if let Some(image) = first.strip_prefix("--image=") {
        validate_image_name(image)?;
        if argv.len() < 2 {
            return Err(DenyReason::EmptyArgv);
        }
        return Ok(ParsedExec {
            target: ExecTarget::DockerImage(image.to_string()),
            argv: argv[1..].to_vec(),
        });
    }

    Ok(ParsedExec {
        target: ExecTarget::Host,
        argv: argv.to_vec(),
    })
}

pub fn parse_exec_target_with_image(
    image: &str,
    argv: &[String],
) -> Result<ParsedExec, DenyReason> {
    validate_image_name(image)?;
    if argv.is_empty() {
        return Err(DenyReason::EmptyArgv);
    }
    if argv
        .first()
        .is_some_and(|arg| arg == "--image" || arg.starts_with("--image="))
    {
        return Err(DenyReason::InvalidImage(
            "image field must not be combined with --image argv prefix".to_string(),
        ));
    }
    Ok(ParsedExec {
        target: ExecTarget::DockerImage(image.to_string()),
        argv: argv.to_vec(),
    })
}

fn validate_image_name(image: &str) -> Result<(), DenyReason> {
    if image.trim().is_empty() {
        return Err(DenyReason::InvalidImage(
            "--image requires a non-empty image name".to_string(),
        ));
    }
    if image.starts_with('-') {
        return Err(DenyReason::InvalidImage(
            "docker image name must not start with '-'".to_string(),
        ));
    }
    if !image
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | ':' | '@'))
    {
        return Err(DenyReason::InvalidImage(format!(
            "docker image name contains unsupported characters: {image}"
        )));
    }
    Ok(())
}

/// Check whether the request should be hard-denied before any approval flow.
/// Checks executable denylist, argument fragment denylist, and blocks shell metacharacters.
pub fn check_denied(
    argv: &[String],
    proj: &WorkspaceConfig,
    config: &Config,
) -> Option<DenyReason> {
    if argv.is_empty() {
        return Some(DenyReason::EmptyArgv);
    }

    let denied_exes = config::effective_denied_executables(proj, &config.defaults);
    let exe = argv[0].as_str();
    let exe_base = Path::new(exe)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(exe);

    if denied_exes.iter().any(|d| d == exe || d == exe_base) {
        return Some(DenyReason::DeniedExecutable(exe.to_string()));
    }

    // Hard-block shell metacharacters to prevent injections (e.g., `cargo test; cat /etc/shadow`)
    let shell_chars = ['|', '&', ';', '$', '>', '<', '`', '\n'];
    for arg in argv {
        if arg.contains(&shell_chars[..]) {
            return Some(DenyReason::DeniedArgumentFragment(
                "shell metacharacter".into(),
            ));
        }
    }

    let denied_frags = config::effective_denied_fragments(proj, &config.defaults);
    for arg in argv {
        for frag in &denied_frags {
            if arg.contains(frag.as_str()) {
                return Some(DenyReason::DeniedArgumentFragment(frag.clone()));
            }
        }
    }

    None
}

/// Find the first rule command that exactly matches argv.
pub fn find_matching_command<'a>(
    argv: &[String],
    target: &ExecTarget,
    rules: &'a ComposedRules,
) -> CommandMatch<'a> {
    match rules.find_hostdo_command_for_target(argv, target.image()) {
        Some(cmd) => CommandMatch::Explicit(cmd),
        None => CommandMatch::Unlisted,
    }
}

/// Resolve env vars for a named profile (empty map if profile not found).
pub fn resolve_env(profile_name: Option<&str>, config: &Config) -> HashMap<String, String> {
    profile_name
        .and_then(|name| config.env_profiles.get(name))
        .map(|p| p.vars.clone())
        .unwrap_or_default()
}

/// Execute a command and return its output. Runs the real host-side process.
pub async fn run_command(
    argv: &[String],
    cwd: &Path,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
) -> Result<ExecResult> {
    anyhow::ensure!(!argv.is_empty(), "argv must not be empty");

    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.current_dir(cwd);
    cmd.envs(env_vars);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();

    let output = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| anyhow::anyhow!("command timed out after {timeout_secs}s"))??;

    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(ExecResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        duration_ms,
    })
}

pub async fn run_command_streaming<F>(
    argv: &[String],
    cwd: &Path,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
    cancel_flag: Arc<AtomicBool>,
    mut on_output: F,
) -> Result<ExecResult>
where
    F: FnMut(bool, String) + Send,
{
    anyhow::ensure!(!argv.is_empty(), "argv must not be empty");

    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.current_dir(cwd);
    cmd.envs(env_vars);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let mut child = cmd.spawn()?;
    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<(bool, String)>();
    let mut reader_tasks = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        reader_tasks.push(tokio::spawn(read_process_progress(
            stdout,
            false,
            line_tx.clone(),
        )));
    }
    if let Some(stderr) = child.stderr.take() {
        reader_tasks.push(tokio::spawn(read_process_progress(stderr, true, line_tx)));
    }

    let mut stdout = String::new();
    let mut stderr = String::new();
    let status = loop {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            anyhow::bail!("command cancelled");
        }
        if started.elapsed() >= Duration::from_secs(timeout_secs) {
            let _ = child.kill().await;
            anyhow::bail!("command timed out after {timeout_secs}s");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }

        tokio::select! {
            line = line_rx.recv() => {
                if let Some((is_stderr, line)) = line {
                    if is_stderr {
                        stderr.push_str(&line);
                        stderr.push('\n');
                    } else {
                        stdout.push_str(&line);
                        stdout.push('\n');
                    }
                    on_output(is_stderr, line);
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                // Poll process status and cancellation on the next loop.
            }
        }
    };

    for task in reader_tasks {
        let _ = task.await;
    }
    while let Ok((is_stderr, line)) = line_rx.try_recv() {
        if is_stderr {
            stderr.push_str(&line);
            stderr.push('\n');
        } else {
            stdout.push_str(&line);
            stdout.push('\n');
        }
        on_output(is_stderr, line);
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    Ok(ExecResult {
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerPullProgress {
    pub message: String,
    pub id: Option<String>,
    pub status: Option<String>,
    pub detail: Option<String>,
}

pub async fn docker_image_present(image: &str) -> Result<bool> {
    validate_image_name(image).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let output = tokio::time::timeout(Duration::from_secs(15), async {
        tokio::process::Command::new("docker")
            .arg("image")
            .arg("inspect")
            .arg(image)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
    })
    .await
    .map_err(|_| anyhow::anyhow!("docker image inspect timed out after 15s"))??;

    if output.status.success() {
        return Ok(true);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stderr_lower = stderr.to_ascii_lowercase();
    if stderr_lower.contains("no such image") || stderr_lower.contains("no such object") {
        return Ok(false);
    }

    let reason = stderr.trim();
    if reason.is_empty() {
        anyhow::bail!(
            "docker image inspect failed with exit code {:?}",
            output.status.code()
        );
    }
    anyhow::bail!("docker image inspect failed: {reason}");
}

pub async fn pull_docker_image<F>(
    image: &str,
    timeout_secs: u64,
    mut on_progress: F,
) -> Result<ExecResult>
where
    F: FnMut(DockerPullProgress) + Send,
{
    pull_docker_image_cancelable(
        image,
        timeout_secs,
        Arc::new(AtomicBool::new(false)),
        move |progress| on_progress(progress),
    )
    .await
}

pub async fn pull_docker_image_cancelable<F>(
    image: &str,
    timeout_secs: u64,
    cancel_flag: Arc<AtomicBool>,
    mut on_progress: F,
) -> Result<ExecResult>
where
    F: FnMut(DockerPullProgress) + Send,
{
    validate_image_name(image).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("pull")
        .arg(image)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let mut child = cmd.spawn()?;

    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<(bool, String)>();
    let mut reader_tasks = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        reader_tasks.push(tokio::spawn(read_process_progress(
            stdout,
            false,
            line_tx.clone(),
        )));
    }
    if let Some(stderr) = child.stderr.take() {
        reader_tasks.push(tokio::spawn(read_process_progress(stderr, true, line_tx)));
    }

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut last_progress_emit = Instant::now() - Duration::from_secs(1);
    let status = loop {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            anyhow::bail!("docker pull cancelled");
        }
        if started.elapsed() >= Duration::from_secs(timeout_secs) {
            let _ = child.kill().await;
            anyhow::bail!("docker pull timed out after {timeout_secs}s");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }

        tokio::select! {
            maybe_line = line_rx.recv() => match maybe_line {
                Some((is_stderr, line)) => {
                    let capture = if is_stderr { &mut stderr } else { &mut stdout };
                    append_capped(capture, &line);
                    if let Some(progress) = parse_docker_pull_progress_line(&line)
                        && should_emit_pull_progress(&progress, &mut last_progress_emit)
                    {
                        on_progress(progress);
                    }
                }
                None => {}
            },
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    };

    for task in reader_tasks {
        let _ = task.await;
    }
    while let Ok((is_stderr, line)) = line_rx.try_recv() {
        append_capped(if is_stderr { &mut stderr } else { &mut stdout }, &line);
        if let Some(progress) = parse_docker_pull_progress_line(&line) {
            on_progress(progress);
        }
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    Ok(ExecResult {
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms,
    })
}

fn should_emit_pull_progress(progress: &DockerPullProgress, last_emit: &mut Instant) -> bool {
    let now = Instant::now();
    if now.duration_since(*last_emit) >= Duration::from_millis(500)
        || is_notable_pull_progress(progress)
    {
        *last_emit = now;
        return true;
    }
    false
}

fn is_notable_pull_progress(progress: &DockerPullProgress) -> bool {
    let status = progress.status.as_deref().unwrap_or("");
    let text = format!("{} {}", status, progress.message).to_ascii_lowercase();
    text.contains("complete")
        || text.contains("already exists")
        || text.contains("downloaded newer image")
        || text.contains("image is up to date")
        || text.starts_with("digest:")
        || text.starts_with("status:")
}

async fn read_process_progress<R>(
    mut reader: R,
    is_stderr: bool,
    tx: mpsc::UnboundedSender<(bool, String)>,
) where
    R: AsyncRead + Unpin,
{
    let mut pending = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        for byte in &buf[..n] {
            if *byte == b'\n' || *byte == b'\r' {
                if !pending.is_empty() {
                    let line = String::from_utf8_lossy(&pending).into_owned();
                    if tx.send((is_stderr, line)).is_err() {
                        return;
                    }
                    pending.clear();
                }
            } else {
                pending.push(*byte);
            }
        }
    }

    if !pending.is_empty() {
        let line = String::from_utf8_lossy(&pending).into_owned();
        if tx.send((is_stderr, line)).is_err() {
            return;
        }
    }
}

fn append_capped(buf: &mut String, line: &str) {
    const MAX_CAPTURE_BYTES: usize = 64 * 1024;
    if buf.len() >= MAX_CAPTURE_BYTES {
        return;
    }
    let remaining = MAX_CAPTURE_BYTES - buf.len();
    if line.len() + 1 <= remaining {
        buf.push_str(line);
        buf.push('\n');
    } else {
        buf.push_str(&line[..remaining.min(line.len())]);
    }
}

pub fn parse_docker_pull_progress_line(line: &str) -> Option<DockerPullProgress> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let status = value
            .get("status")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let id = value.get("id").and_then(|v| v.as_str()).map(str::to_string);
        let detail = value
            .get("progress")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| format_progress_detail(value.get("progressDetail")));

        let mut parts = Vec::new();
        if let Some(status) = &status {
            parts.push(status.clone());
        }
        if let Some(id) = &id {
            parts.push(id.clone());
        }
        if let Some(detail) = &detail {
            parts.push(detail.clone());
        }
        let message = if parts.is_empty() {
            trimmed.to_string()
        } else {
            parts.join(" ")
        };

        return Some(DockerPullProgress {
            message,
            id,
            status,
            detail,
        });
    }

    Some(DockerPullProgress {
        message: trimmed.to_string(),
        id: None,
        status: None,
        detail: None,
    })
}

fn format_progress_detail(value: Option<&serde_json::Value>) -> Option<String> {
    let detail = value?;
    let current = detail.get("current").and_then(|v| v.as_u64())?;
    let total = detail.get("total").and_then(|v| v.as_u64());
    Some(match total {
        Some(total) => format!("{} / {}", format_bytes(current), format_bytes(total)),
        None => format_bytes(current),
    })
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub async fn run_target_command(
    target: &ExecTarget,
    argv: &[String],
    host_cwd: &Path,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
    docker_workspace_host_path: &Path,
    docker_workspace_container_path: &Path,
    docker_container_cwd: &Path,
) -> Result<ExecResult> {
    match target {
        ExecTarget::Host => run_command(argv, host_cwd, env_vars, timeout_secs).await,
        ExecTarget::DockerImage(image) => {
            run_docker_command(
                image,
                argv,
                docker_workspace_host_path,
                docker_workspace_container_path,
                docker_container_cwd,
                env_vars,
                timeout_secs,
            )
            .await
        }
    }
}

pub async fn run_target_command_streaming<F>(
    target: &ExecTarget,
    argv: &[String],
    host_cwd: &Path,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
    workspace_host_path: &Path,
    workspace_container_path: &Path,
    runner_cwd: &Path,
    cancel_flag: Arc<AtomicBool>,
    on_output: F,
) -> Result<ExecResult>
where
    F: FnMut(bool, String) + Send,
{
    match target {
        ExecTarget::Host => {
            run_command_streaming(
                argv,
                host_cwd,
                env_vars,
                timeout_secs,
                cancel_flag,
                on_output,
            )
            .await
        }
        ExecTarget::DockerImage(image) => {
            run_docker_command_streaming(
                image,
                argv,
                env_vars,
                timeout_secs,
                workspace_host_path,
                workspace_container_path,
                runner_cwd,
                cancel_flag,
                on_output,
            )
            .await
        }
    }
}

/// Execute a command in a short-lived Docker container and return its output.
pub async fn run_docker_command(
    image: &str,
    argv: &[String],
    workspace_host_path: &Path,
    workspace_container_path: &Path,
    container_cwd: &Path,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
) -> Result<ExecResult> {
    anyhow::ensure!(!argv.is_empty(), "argv must not be empty");
    anyhow::ensure!(
        workspace_host_path.is_absolute(),
        "workspace host path must be absolute: {}",
        workspace_host_path.display()
    );
    anyhow::ensure!(
        workspace_container_path.is_absolute(),
        "workspace container path must be absolute: {}",
        workspace_container_path.display()
    );
    anyhow::ensure!(
        container_cwd.is_absolute(),
        "container cwd must be absolute: {}",
        container_cwd.display()
    );

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("run")
        .arg("--rm")
        .arg("--pull=never")
        .arg("-v")
        .arg(format!(
            "{}:{}",
            workspace_host_path.display(),
            workspace_container_path.display()
        ))
        .arg("-w")
        .arg(container_cwd);

    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        cmd.arg("--user").arg(format!("{uid}:{gid}"));
    }

    for (key, value) in env_vars {
        cmd.arg("-e").arg(format!("{key}={value}"));
    }

    cmd.arg(image);
    cmd.args(argv);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();

    let output = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| anyhow::anyhow!("docker command timed out after {timeout_secs}s"))??;

    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(ExecResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        duration_ms,
    })
}

pub async fn run_docker_command_streaming<F>(
    image: &str,
    argv: &[String],
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
    workspace_host_path: &Path,
    workspace_container_path: &Path,
    container_cwd: &Path,
    cancel_flag: Arc<AtomicBool>,
    mut on_output: F,
) -> Result<ExecResult>
where
    F: FnMut(bool, String) + Send,
{
    validate_image_name(image).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    anyhow::ensure!(!argv.is_empty(), "argv must not be empty");
    anyhow::ensure!(
        workspace_host_path.is_absolute(),
        "workspace host path must be absolute: {}",
        workspace_host_path.display()
    );
    anyhow::ensure!(
        workspace_container_path.is_absolute(),
        "workspace container path must be absolute: {}",
        workspace_container_path.display()
    );
    anyhow::ensure!(
        container_cwd.is_absolute(),
        "container cwd must be absolute: {}",
        container_cwd.display()
    );

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("run")
        .arg("--rm")
        .arg("--pull=never")
        .arg("-v")
        .arg(format!(
            "{}:{}",
            workspace_host_path.display(),
            workspace_container_path.display()
        ))
        .arg("-w")
        .arg(container_cwd);

    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        cmd.arg("--user").arg(format!("{uid}:{gid}"));
    }

    for (key, value) in env_vars {
        cmd.arg("-e").arg(format!("{key}={value}"));
    }

    cmd.arg(image);
    cmd.args(argv);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let mut child = cmd.spawn()?;
    let (line_tx, mut line_rx) = mpsc::unbounded_channel::<(bool, String)>();
    let mut reader_tasks = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        reader_tasks.push(tokio::spawn(read_process_progress(
            stdout,
            false,
            line_tx.clone(),
        )));
    }
    if let Some(stderr) = child.stderr.take() {
        reader_tasks.push(tokio::spawn(read_process_progress(stderr, true, line_tx)));
    }

    let mut stdout = String::new();
    let mut stderr = String::new();

    let status = loop {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            anyhow::bail!("docker command cancelled");
        }
        if started.elapsed() >= Duration::from_secs(timeout_secs) {
            let _ = child.kill().await;
            anyhow::bail!("docker command timed out after {timeout_secs}s");
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }

        tokio::select! {
            line = line_rx.recv() => {
                if let Some((is_stderr, line)) = line {
                    if is_stderr {
                        stderr.push_str(&line);
                        stderr.push('\n');
                    } else {
                        stdout.push_str(&line);
                        stdout.push('\n');
                    }
                    on_output(is_stderr, line);
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
    };

    for task in reader_tasks {
        let _ = task.await;
    }
    while let Ok((is_stderr, line)) = line_rx.try_recv() {
        if is_stderr {
            stderr.push_str(&line);
            stderr.push('\n');
        } else {
            stdout.push_str(&line);
            stdout.push('\n');
        }
        on_output(is_stderr, line);
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    Ok(ExecResult {
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, WorkspaceConfig, WorkspaceHostdo};

    #[test]
    fn resolve_env_handles_profiles() {
        let mut config = Config::default();
        config.env_profiles.insert(
            "test".to_string(),
            crate::config::EnvProfile {
                vars: [("KEY".to_string(), "VAL".to_string())]
                    .into_iter()
                    .collect(),
            },
        );

        let env = resolve_env(Some("test"), &config);
        assert_eq!(env.get("KEY"), Some(&"VAL".to_string()));

        let env_none = resolve_env(None, &config);
        assert!(env_none.is_empty());

        let env_missing = resolve_env(Some("missing"), &config);
        assert!(env_missing.is_empty());
    }

    #[test]
    fn check_denied_blocks_metacharacters() {
        let proj = WorkspaceConfig::default();
        let config = Config::default();

        assert!(
            check_denied(
                &["ls".into(), "file; cat /etc/shadow".into()],
                &proj,
                &config
            )
            .is_some()
        );
        assert!(check_denied(&["ls".into(), "file && rm -rf /".into()], &proj, &config).is_some());
        assert!(
            check_denied(&["ls".into(), "file | grep secret".into()], &proj, &config).is_some()
        );
        assert!(check_denied(&["ls".into(), "file \n /".into()], &proj, &config).is_some());

        // Clean
        assert!(check_denied(&["ls".into(), "clean-file".into()], &proj, &config).is_none());
    }

    #[test]
    fn check_denied_blocks_denied_executables() {
        let mut proj = WorkspaceConfig::default();
        let mut config = Config::default();
        config.defaults.hostdo.denied_executables = vec!["cat".to_string()];

        assert!(check_denied(&["cat".into(), "secret.txt".into()], &proj, &config).is_some());
        assert!(check_denied(&["ls".into(), "file.txt".into()], &proj, &config).is_none());

        // Per-project deny
        proj.hostdo = Some(WorkspaceHostdo {
            denied_executables: Some(vec!["ls".to_string()]),
            denied_argument_fragments: None,
            command_aliases: None,
        });
        assert!(check_denied(&["ls".into(), "file.txt".into()], &proj, &config).is_some());
    }

    #[test]
    fn parse_exec_target_defaults_to_host() {
        let parsed = parse_exec_target(&["cargo".into(), "test".into()]).unwrap();
        assert_eq!(parsed.target, ExecTarget::Host);
        assert_eq!(parsed.argv, vec!["cargo", "test"]);
    }

    #[test]
    fn parse_exec_target_extracts_image() {
        let parsed = parse_exec_target(&[
            "--image".into(),
            "node:20".into(),
            "npm".into(),
            "test".into(),
        ])
        .unwrap();
        assert_eq!(parsed.target, ExecTarget::DockerImage("node:20".into()));
        assert_eq!(parsed.argv, vec!["npm", "test"]);
    }

    #[test]
    fn parse_exec_target_accepts_protocol_image() {
        let parsed =
            parse_exec_target_with_image("node:20", &["npm".into(), "test".into()]).unwrap();
        assert_eq!(parsed.target, ExecTarget::DockerImage("node:20".into()));
        assert_eq!(parsed.argv, vec!["npm", "test"]);
    }

    #[test]
    fn parse_exec_target_rejects_missing_image_command() {
        let err = parse_exec_target(&["--image".into(), "node:20".into()]).unwrap_err();
        assert!(matches!(err, DenyReason::EmptyArgv));
    }

    #[test]
    fn parse_docker_pull_progress_line_reads_json_status() {
        let progress = parse_docker_pull_progress_line(
            r#"{"id":"abc123","status":"Downloading","progressDetail":{"current":1048576,"total":2097152}}"#,
        )
        .unwrap();

        assert_eq!(progress.id.as_deref(), Some("abc123"));
        assert_eq!(progress.status.as_deref(), Some("Downloading"));
        assert_eq!(progress.detail.as_deref(), Some("1.0 MB / 2.0 MB"));
        assert_eq!(progress.message, "Downloading abc123 1.0 MB / 2.0 MB");
    }

    #[test]
    fn parse_docker_pull_progress_line_accepts_plain_text() {
        let progress = parse_docker_pull_progress_line("Pulling image...").unwrap();
        assert_eq!(progress.message, "Pulling image...");
        assert_eq!(progress.id, None);
    }

    #[test]
    fn parse_docker_pull_progress_line_reads_json_progress_string() {
        let progress = parse_docker_pull_progress_line(
            r#"{"id":"layer1","status":"Extracting","progress":"[====>] 4.2MB/8.4MB"}"#,
        )
        .unwrap();

        assert_eq!(progress.id.as_deref(), Some("layer1"));
        assert_eq!(progress.status.as_deref(), Some("Extracting"));
        assert_eq!(progress.detail.as_deref(), Some("[====>] 4.2MB/8.4MB"));
        assert_eq!(progress.message, "Extracting layer1 [====>] 4.2MB/8.4MB");
    }

    #[test]
    fn parse_docker_pull_progress_line_ignores_empty_lines() {
        assert!(parse_docker_pull_progress_line("   ").is_none());
    }

    #[test]
    fn parse_docker_pull_progress_line_preserves_digest_lines() {
        let progress = parse_docker_pull_progress_line("Digest: sha256:0123456789abcdef").unwrap();

        assert_eq!(progress.message, "Digest: sha256:0123456789abcdef");
        assert_eq!(progress.status, None);
        assert_eq!(progress.detail, None);
    }

    #[test]
    fn should_emit_pull_progress_throttles_repetitive_updates() {
        let progress = DockerPullProgress {
            message: "Downloading layer1 [====>] 1.0MB/10.0MB".to_string(),
            id: Some("layer1".to_string()),
            status: Some("Downloading".to_string()),
            detail: Some("[====>] 1.0MB/10.0MB".to_string()),
        };
        let mut last_emit = Instant::now();

        assert!(!should_emit_pull_progress(&progress, &mut last_emit));
    }

    #[test]
    fn should_emit_pull_progress_keeps_completion_updates() {
        let progress = DockerPullProgress {
            message: "layer1: Pull complete".to_string(),
            id: Some("layer1".to_string()),
            status: Some("Pull complete".to_string()),
            detail: None,
        };
        let mut last_emit = Instant::now();

        assert!(should_emit_pull_progress(&progress, &mut last_emit));
    }

    #[tokio::test]
    async fn read_process_progress_splits_newlines_and_carriage_returns() {
        use tokio::io::AsyncWriteExt;

        let (mut writer, reader) = tokio::io::duplex(64);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(read_process_progress(reader, true, tx));

        writer
            .write_all(b"Downloading layer\rExtracting layer\nDone")
            .await
            .unwrap();
        drop(writer);
        task.await.unwrap();

        let mut lines = Vec::new();
        while let Ok((is_stderr, line)) = rx.try_recv() {
            lines.push((is_stderr, line));
        }

        assert_eq!(
            lines,
            vec![
                (true, "Downloading layer".to_string()),
                (true, "Extracting layer".to_string()),
                (true, "Done".to_string()),
            ]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_command_streaming_drains_stdout_and_stderr_after_exit() {
        let mut seen = Vec::new();
        let result = run_command_streaming(
            &[
                "sh".to_string(),
                "-c".to_string(),
                "printf 'out\\n'; printf 'err\\n' >&2".to_string(),
            ],
            Path::new("."),
            &HashMap::new(),
            5,
            Arc::new(AtomicBool::new(false)),
            |is_stderr, line| seen.push((is_stderr, line)),
        )
        .await
        .expect("stream command");

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "out\n");
        assert_eq!(result.stderr, "err\n");
        assert!(seen.contains(&(false, "out".to_string())));
        assert!(seen.contains(&(true, "err".to_string())));
    }
}
