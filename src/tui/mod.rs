mod app;
pub mod render;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;
use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableMouseCapture, Event, EventStream,
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    style::ResetColor,
    terminal::{
        EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicI32, Ordering},
};
use tokio::sync::mpsc;

use crate::container::ContainerSession;
use crate::proxy::{NetworkDecision, PendingNetworkItem, ProxyState};
use crate::rules::NetworkPolicy;
use crate::server::SessionRegistry;
use crate::server::{ApprovalDecision, ContainerStopDecision, ContainerStopItem, PendingItem};
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, StateManager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsAction {
    ReloadRules,
    RemoveWorkspace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SettingsActionRow {
    pub key: char,
    pub label: String,
    pub desc: &'static str,
    action: SettingsAction,
}

#[derive(Debug, Clone)]
/// A log line shown in the TUI log pane.
pub enum LogEntry {
    Audit(AuditEntry),
    Msg {
        text: String,
        is_error: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, PartialEq)]
/// Selectable entries in the left sidebar.
pub enum SidebarItem {
    Workspace(usize),
    Session(usize),
    Settings(usize),
    Launch(usize),
    Build(usize),
    NewWorkspace,
}

#[derive(Debug, Clone, PartialEq)]
/// The currently focused UI region.
pub enum Focus {
    Sidebar,
    Terminal,
    Settings,
    ContainerPicker,
    ImageBuild,
    NewWorkspace,
}

#[derive(Debug, Clone)]
/// Transient state for the new-workspace wizard.
pub struct NewWorkspaceState {
    pub cursor: usize,
    pub name: String,
    pub workspace_dir: String,
    pub project_type: crate::new_project::ProjectType,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RemoveWorkspaceConfirmState {
    pub workspace_name: String,
}

#[derive(Debug, Clone)]
pub struct BaseRulesChangedState {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchedFileStamp {
    pub exists: bool,
    pub size: u64,
    pub mtime_secs: u64,
    pub mtime_nanos: u32,
    pub content_hash: u64,
}

#[derive(Debug, Clone)]
pub struct PendingBaseRulesInternalWrite {
    pub expected_content: String,
    pub expires_at: std::time::Instant,
}

/// Top-level TUI application state and event loop ownership.
pub struct App {
    pub config: SharedConfig,
    pub loaded_config_path: PathBuf,
    pub token: String,
    pub session_registry: SessionRegistry,
    pub ca_cert_path: String,
    proxy_state: ProxyState,

    pub workspaces: Vec<WorkspaceStatus>,
    pub pending_exec: Vec<PendingItem>,
    pub pending_stop: Vec<ContainerStopItem>,
    pub pending_net: Vec<PendingNetworkItem>,
    pub log: VecDeque<LogEntry>,
    pub log_scroll: usize,

    pub focus: Focus,
    pub sidebar_idx: usize,
    pub sidebar_offset: usize,
    pub active_session: Option<usize>,
    pub preview_session: Option<usize>,
    pub active_settings_project: Option<usize>,
    pub settings_cursor: usize,

    pub container_picker: Option<usize>,
    pub build_container_idx: Option<usize>,
    pub build_project_idx: Option<usize>,
    pub build_cursor: usize,
    pub build_output: VecDeque<(String, bool)>,
    pub build_scroll: usize,
    pub sessions: Vec<ContainerSession>,
    pub new_project: Option<NewWorkspaceState>,
    pub remove_workspace_confirm: Option<RemoveWorkspaceConfirmState>,
    pub base_rules_changed: Option<BaseRulesChangedState>,

    pub exec_pending_rx: mpsc::Receiver<PendingItem>,
    pub stop_pending_rx: mpsc::Receiver<ContainerStopItem>,
    pub net_pending_rx: mpsc::Receiver<PendingNetworkItem>,
    pub audit_rx: mpsc::Receiver<AuditEntry>,
    build_event_rx: mpsc::UnboundedReceiver<BuildEvent>,
    build_event_tx: mpsc::UnboundedSender<BuildEvent>,
    build_task: Option<BuildTaskState>,

    pub should_quit: bool,
    pub passthrough_mode: bool,
    pub passthrough_exit_code_slot: Option<Arc<AtomicI32>>,
    pub log_fullscreen: bool,
    pub terminal_fullscreen: bool,
    ctrl_c_times: Vec<std::time::Instant>,
    last_terminal_esc: Option<std::time::Instant>,
    pub scroll_mode: bool,
    pub scroll_mouse_passthrough: bool,
    pub terminal_scroll: usize,
    last_base_rules_poll: std::time::Instant,
    watched_rules_stamps: HashMap<PathBuf, WatchedFileStamp>,
    pending_base_rules_internal_write: HashMap<PathBuf, PendingBaseRulesInternalWrite>,
}

/// Cached workspace metadata for the sidebar.
pub struct WorkspaceStatus {
    pub name: String,
}

#[derive(Debug)]
enum BuildEvent {
    Output {
        line: String,
        is_error: bool,
    },
    Finished {
        label: String,
        launch_project_idx: usize,
        launch_container_idx: usize,
        success: bool,
        cancelled: bool,
        exit_code: Option<i32>,
        error: Option<String>,
        diagnostic: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct BuildTaskState {
    label: String,
    shell_command: String,
    cancel_flag: Arc<AtomicBool>,
}

fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let b = c as u8;
                if b.is_ascii_alphabetic() {
                    Some(vec![b & 0x1f])
                } else {
                    Some(c.to_string().into_bytes())
                }
            } else {
                let mut buf = [0u8; 4];
                Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
            }
        }
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Esc => Some(b"\x1b".to_vec()),
        KeyCode::F(n) if (1..=12).contains(&n) => {
            let f_keys: [&[u8]; 12] = [
                b"\x1bOP",
                b"\x1bOQ",
                b"\x1bOR",
                b"\x1bOS",
                b"\x1b[15~",
                b"\x1b[17~",
                b"\x1b[18~",
                b"\x1b[19~",
                b"\x1b[20~",
                b"\x1b[21~",
                b"\x1b[23~",
                b"\x1b[24~",
            ];
            Some(f_keys[(n - 1) as usize].to_vec())
        }
        _ => None,
    }
}

// ── Event loop ────────────────────────────────────────────────────────────────

pub async fn run(mut app: App) -> Result<()> {
    // Must run *before* `enable_raw_mode()`: the guard restores full termios on
    // drop, so capturing termios while already in raw mode would "restore" the
    // raw settings after shutdown and permanently corrupt the user's shell.
    let _termios_guard = disable_xon_xoff();
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let mut restore_guard = TerminalRestoreGuard::new();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    restore_terminal_output(terminal.backend_mut())?;
    terminal.show_cursor()?;
    restore_guard.disarm();

    result
}

fn restore_terminal_output<W: std::io::Write>(writer: &mut W) -> std::io::Result<()> {
    execute!(
        writer,
        LeaveAlternateScreen,
        cursor::Show,
        DisableMouseCapture,
        DisableBracketedPaste,
        EnableLineWrap,
        ResetColor
    )
}

struct TerminalRestoreGuard {
    armed: bool,
}

impl TerminalRestoreGuard {
    fn new() -> Self {
        Self { armed: true }
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = restore_terminal_output(&mut stdout);
    }
}

#[cfg(unix)]
fn disable_xon_xoff() -> Option<TermiosGuard> {
    disable_xon_xoff_on_fd(libc::STDIN_FILENO)
}

#[cfg(unix)]
fn disable_xon_xoff_on_fd(fd: i32) -> Option<TermiosGuard> {
    use std::mem::MaybeUninit;
    unsafe {
        let mut orig = MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(fd, orig.as_mut_ptr()) != 0 {
            return None;
        }
        let orig = orig.assume_init();
        let ixon_was_enabled = (orig.c_iflag & libc::IXON) != 0;
        let mut t = orig;
        t.c_iflag &= !libc::IXON;
        if libc::tcsetattr(fd, libc::TCSANOW, &t) != 0 {
            return None;
        }
        Some(TermiosGuard {
            fd,
            ixon_was_enabled,
        })
    }
}

#[cfg(not(unix))]
fn disable_xon_xoff() -> Option<()> {
    None
}

#[cfg(unix)]
struct TermiosGuard {
    fd: i32,
    ixon_was_enabled: bool,
}

#[cfg(unix)]
impl Drop for TermiosGuard {
    fn drop(&mut self) {
        unsafe {
            let mut cur = std::mem::MaybeUninit::<libc::termios>::uninit();
            if libc::tcgetattr(self.fd, cur.as_mut_ptr()) != 0 {
                return;
            }
            let mut cur = cur.assume_init();
            if self.ixon_was_enabled {
                cur.c_iflag |= libc::IXON;
            } else {
                cur.c_iflag &= !libc::IXON;
            }
            let _ = libc::tcsetattr(self.fd, libc::TCSANOW, &cur);
        }
    }
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut events = EventStream::new();
    let tick = tokio::time::Duration::from_millis(50);
    let mut mouse_capture_enabled = false;

    loop {
        sync_mouse_capture(terminal.backend_mut(), app, &mut mouse_capture_enabled)?;
        app.drain_channels();
        app.tick_base_rules_file_watch();
        terminal.draw(|frame| render::render(frame, app))?;

        if app.should_quit {
            app.terminate_all_sessions();
            break;
        }

        let timeout = tokio::time::sleep(tick);

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => app.handle_key(key),
                    Some(Ok(Event::Mouse(mouse))) => app.handle_mouse(mouse),
                    Some(Ok(Event::Paste(text))) => {
                        if app.focus == Focus::NewWorkspace {
                            app.append_new_project_text(&text);
                        } else if let Some(si) = app.active_session {
                            if let Some(session) = app.sessions.get(si) {
                                session.send_input(text.into_bytes());
                            }
                        }
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        let (pty_cols, pty_rows) = if app.passthrough_mode {
                            (cols.max(20), rows.max(6))
                        } else {
                            (cols.saturating_sub(38).max(20), rows.saturating_sub(10).max(6))
                        };
                        for session in &mut app.sessions {
                            let _ = session.resize(pty_rows, pty_cols);
                        }
                    }
                    None => break,
                    _ => {}
                }
                sync_mouse_capture(terminal.backend_mut(), app, &mut mouse_capture_enabled)?;
            }
            _ = timeout => {}
        }
    }

    Ok(())
}

fn session_mode_requires_mouse_capture(mode: TermMode) -> bool {
    mode.intersects(TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
        && mode.contains(TermMode::SGR_MOUSE)
}

fn should_enable_mouse_capture(app: &App) -> bool {
    if app.focus != Focus::Terminal {
        return false;
    }
    // In explicit scroll mode, keep mouse capture disabled so terminal-native
    // text selection works while reviewing scrollback.
    if app.scroll_mode {
        return false;
    }
    let Some(si) = app.active_session else {
        return false;
    };
    let Some(session) = app.sessions.get(si) else {
        return false;
    };
    let mode = *session.term.lock().mode();
    session_mode_requires_mouse_capture(mode)
}

fn sync_mouse_capture<W: std::io::Write>(
    writer: &mut W,
    app: &App,
    mouse_capture_enabled: &mut bool,
) -> std::io::Result<()> {
    let should_enable = should_enable_mouse_capture(app);
    if should_enable == *mouse_capture_enabled {
        return Ok(());
    }
    if should_enable {
        execute!(writer, EnableMouseCapture)?;
    } else {
        execute!(writer, DisableMouseCapture)?;
    }
    *mouse_capture_enabled = should_enable;
    Ok(())
}

#[cfg(test)]
mod tests;
