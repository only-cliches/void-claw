use super::*;

pub(crate) fn maybe_encode_sgr_mouse_for_session(
    session: &crate::container::ContainerSession,
    mouse: MouseEvent,
) -> Option<Vec<u8>> {
    // Only forward mouse events when the terminal app has explicitly enabled mouse reporting.
    // Without this gating, shells and other apps would see raw escape sequences.
    let mode = *session.term.lock().mode();
    if !mode
        .intersects(TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
    {
        return None;
    }

    // Only emit SGR mouse sequences for now; this matches most modern TUIs (including OpenCode).
    if !mode.contains(TermMode::SGR_MOUSE) {
        return None;
    }

    encode_sgr_mouse(mouse)
}

pub(crate) fn encode_sgr_mouse(mouse: MouseEvent) -> Option<Vec<u8>> {
    let mut cb: u16 = 0;
    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
        cb |= 4;
    }
    if mouse.modifiers.contains(KeyModifiers::ALT) {
        cb |= 8;
    }
    if mouse.modifiers.contains(KeyModifiers::CONTROL) {
        cb |= 16;
    }

    let (button_code, suffix): (u16, u8) = match mouse.kind {
        MouseEventKind::Down(button) => (button_to_code(button)?, b'M'),
        MouseEventKind::Up(button) => (button_to_code(button)?, b'm'),
        MouseEventKind::Drag(button) => (button_to_code(button)? + 32, b'M'),
        MouseEventKind::ScrollUp => (64, b'M'),
        MouseEventKind::ScrollDown => (65, b'M'),
        MouseEventKind::ScrollLeft => (66, b'M'),
        MouseEventKind::ScrollRight => (67, b'M'),
        MouseEventKind::Moved => return None,
    };

    let cb = cb + button_code;
    let x = mouse.column.saturating_add(1);
    let y = mouse.row.saturating_add(1);

    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(b"\x1b[<");
    out.extend_from_slice(cb.to_string().as_bytes());
    out.push(b';');
    out.extend_from_slice(x.to_string().as_bytes());
    out.push(b';');
    out.extend_from_slice(y.to_string().as_bytes());
    out.push(suffix);
    Some(out)
}

pub(crate) fn button_to_code(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
    }
}

pub(crate) fn shell_command_for_docker_args(args: &[String]) -> String {
    format!("docker {}", shell_words::join(args))
}

pub(crate) fn build_line_looks_like_error(line: &str) -> bool {
    let text = line.to_ascii_lowercase();
    [
        " error",
        "failed",
        "denied",
        "no such file",
        "not found",
        "permission denied",
        "unauthorized",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

async fn forward_build_stream<R>(
    reader: R,
    prefix: &'static str,
    mark_stderr: bool,
    stderr_tail: Option<Arc<Mutex<VecDeque<String>>>>,
    tx: mpsc::UnboundedSender<BuildEvent>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncBufReadExt;
    let mut lines = tokio::io::BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let is_error = mark_stderr && build_line_looks_like_error(&line);
                if is_error
                    && let Some(tail) = stderr_tail.as_ref()
                    && let Ok(mut lines) = tail.lock()
                {
                    lines.push_back(line.clone());
                    if lines.len() > 6 {
                        lines.pop_front();
                    }
                }
                let _ = tx.send(BuildEvent::Output {
                    line: format!("{prefix}{line}"),
                    is_error,
                });
            }
            Ok(None) | Err(_) => break,
        }
    }
}

pub(crate) async fn run_build_shell_command(
    label: String,
    shell_command: String,
    launch_project_idx: usize,
    launch_container_idx: usize,
    cancel_flag: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<BuildEvent>,
) {
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-lc")
        .arg(&shell_command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            let rc = libc::setpgid(0, 0);
            if rc == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(BuildEvent::Finished {
                label,
                launch_project_idx,
                launch_container_idx,
                success: false,
                cancelled: false,
                exit_code: None,
                error: Some(e.to_string()),
                diagnostic: None,
            });
            return;
        }
    };

    let stderr_tail: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let stdout_task = child.stdout.take().map(|stdout| {
        let tx = tx.clone();
        tokio::spawn(async move {
            forward_build_stream(stdout, "build: ", false, None, tx).await;
        })
    });
    let stderr_task = child.stderr.take().map(|stderr| {
        let tx = tx.clone();
        let stderr_tail = stderr_tail.clone();
        tokio::spawn(async move {
            forward_build_stream(stderr, "build: ", true, Some(stderr_tail), tx).await;
        })
    });

    let mut cancelled = false;
    let status = loop {
        if cancel_flag.load(Ordering::SeqCst) {
            cancelled = true;
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                let pgid = format!("-{}", pid);
                let _ = tokio::process::Command::new("kill")
                    .args(["-TERM", &pgid])
                    .status()
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                let _ = tokio::process::Command::new("kill")
                    .args(["-KILL", &pgid])
                    .status()
                    .await;
            }
            let _ = child.start_kill();
            break child.wait().await.ok();
        }

        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
            Err(_) => break None,
        }
    };

    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }

    let success = !cancelled && status.map(|s| s.success()).unwrap_or(false);
    let exit_code = status.and_then(|s| s.code());
    let diagnostic = stderr_tail.lock().ok().and_then(|lines| {
        (!lines.is_empty()).then(|| lines.iter().cloned().collect::<Vec<_>>().join(" | "))
    });
    let _ = tx.send(BuildEvent::Finished {
        label,
        launch_project_idx,
        launch_container_idx,
        success,
        cancelled,
        exit_code,
        error: None,
        diagnostic,
    });
}

pub(crate) fn host_bind_is_loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

pub(crate) fn docker_image_exists(image: &str) -> std::io::Result<bool> {
    let status = std::process::Command::new("docker")
        .args(["image", "inspect", image])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    Ok(status.success())
}

pub(crate) fn is_scroll_mode_toggle_key(key: KeyEvent) -> bool {
    (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('\u{13}') && key.modifiers.is_empty())
}

pub(crate) fn oneshot_dummy() -> tokio::sync::oneshot::Sender<NetworkDecision> {
    let (tx, _) = tokio::sync::oneshot::channel();
    tx
}

// ── Key → PTY bytes (Streamlined mapping) ────────────────────────────────────
