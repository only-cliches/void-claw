pub mod ui;

use alacritty_terminal::grid::Dimensions;
use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{
        DisableBracketedPaste, DisableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyModifiers,
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
    Arc,
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

use crate::container::ContainerSession;
use crate::rules::{NetworkPolicy, NetworkRule};
use crate::server::{ApprovalDecision, ContainerStopDecision, ContainerStopItem, PendingItem};
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, StateManager};
use crate::sync::SyncReport;
use crate::server::SessionRegistry;
use crate::{
    config::SyncMode,
    proxy::{NetworkDecision, PendingNetworkItem, ProxyState},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsAction {
    Seed,
    Pushback,
    WatchToggle,
    ReloadRules,
    Clear,
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
    Project(usize),
    Session(usize),
    Settings(usize),
    Launch(usize),
    Build(usize),
    NewProject,
}

#[derive(Debug, Clone, PartialEq)]
/// The currently focused UI region.
pub enum Focus {
    Sidebar,
    Terminal,
    Settings,
    ContainerPicker,
    ImageBuild,
    NewProject,
}

#[derive(Debug, Clone)]
/// Transient state for the new-project wizard.
pub struct NewProjectState {
    pub cursor: usize,
    pub name: String,
    pub canonical_dir: String,
    pub sync_mode: SyncMode,
    pub project_type: crate::new_project::ProjectType,
    pub error: Option<String>,
}

/// Top-level TUI application state and event loop ownership.
pub struct App {
    pub config: SharedConfig,
    pub loaded_config_path: PathBuf,
    pub token: String,
    pub session_registry: SessionRegistry,
    pub ca_cert_path: String,
    proxy_state: ProxyState,

    pub projects: Vec<ProjectStatus>,
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
    pub new_project: Option<NewProjectState>,

    pub exec_pending_rx: mpsc::Receiver<PendingItem>,
    pub stop_pending_rx: mpsc::Receiver<ContainerStopItem>,
    pub net_pending_rx: mpsc::Receiver<PendingNetworkItem>,
    pub audit_rx: mpsc::Receiver<AuditEntry>,
    build_event_rx: mpsc::UnboundedReceiver<BuildEvent>,
    build_event_tx: mpsc::UnboundedSender<BuildEvent>,
    build_task: Option<BuildTaskState>,

    pub should_quit: bool,
    pub log_fullscreen: bool,
    pub terminal_fullscreen: bool,
    ctrl_c_times: Vec<std::time::Instant>,
    last_terminal_esc: Option<std::time::Instant>,
    pub scroll_mode: bool,
    pub terminal_scroll: usize,
    project_watch: HashMap<usize, ProjectWatchState>,
    last_watch_tick: std::time::Instant,
}

/// Cached project metadata and latest sync report for the sidebar.
pub struct ProjectStatus {
    pub name: String,
    pub last_report: Option<SyncReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSignature {
    size: u64,
    mtime_secs: u64,
    mtime_nanos: u32,
}

struct ProjectWatchState {
    enabled: bool,
    spinner_phase: usize,
    canonical_files: HashMap<PathBuf, FileSignature>,
    workspace_files: HashMap<PathBuf, FileSignature>,
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

impl App {
    fn sidebar_item_is_selectable(item: &SidebarItem) -> bool {
        !matches!(item, SidebarItem::Project(_))
    }

    fn first_selectable_sidebar_idx(items: &[SidebarItem]) -> usize {
        items
            .iter()
            .position(Self::sidebar_item_is_selectable)
            .unwrap_or(0)
    }

    pub fn new(
        config: SharedConfig,
        loaded_config_path: PathBuf,
        token: String,
        session_registry: SessionRegistry,
        exec_pending_rx: mpsc::Receiver<PendingItem>,
        stop_pending_rx: mpsc::Receiver<ContainerStopItem>,
        net_pending_rx: mpsc::Receiver<PendingNetworkItem>,
        audit_rx: mpsc::Receiver<AuditEntry>,
        state: StateManager,
        proxy_state: ProxyState,
        _proxy_addr: String,
        ca_cert_path: String,
    ) -> Result<Self> {
        let cfg = config.get();

        let projects = cfg
            .projects
            .iter()
            .map(|p| ProjectStatus {
                name: p.name.clone(),
                last_report: None,
            })
            .collect();

        let mut log = state
            .recent_audit(200)
            .unwrap_or_default()
            .into_iter()
            .map(LogEntry::Audit)
            .collect::<VecDeque<_>>();

        log.push_front(LogEntry::Msg {
            text: format!("loaded config from {}", loaded_config_path.display()),
            is_error: false,
            timestamp: chrono::Utc::now(),
        });

        let (build_event_tx, build_event_rx) = mpsc::unbounded_channel();

        let rules_path = &cfg.manager.global_rules_file;
        let (hostdo_rule_count, network_rule_count) = crate::rules::load(rules_path)
            .map(|r| (r.hostdo.commands.len(), r.network.rules.len()))
            .unwrap_or((0, 0));
        log.push_front(LogEntry::Msg {
            text: format!(
                "Loaded rules from {} (hostdo: {}, network: {})",
                rules_path.display(),
                hostdo_rule_count,
                network_rule_count
            ),
            is_error: false,
            timestamp: chrono::Utc::now(),
        });

        Ok(Self {
            config,
            loaded_config_path,
            token,
            session_registry,
            ca_cert_path,
            proxy_state,
            projects,
            pending_exec: vec![],
            pending_stop: vec![],
            pending_net: vec![],
            log,
            log_scroll: 0,
            focus: Focus::Sidebar,
            sidebar_idx: Self::first_selectable_sidebar_idx(
                &cfg.projects
                    .iter()
                    .enumerate()
                    .flat_map(|(pi, _)| {
                        [
                            SidebarItem::Project(pi),
                            SidebarItem::Launch(pi),
                            SidebarItem::Settings(pi),
                        ]
                    })
                    .chain(std::iter::once(SidebarItem::NewProject))
                    .collect::<Vec<_>>(),
            ),
            sidebar_offset: 0,
            active_session: None,
            preview_session: None,
            active_settings_project: None,
            settings_cursor: 0,
            container_picker: None,
            build_container_idx: None,
            build_project_idx: None,
            build_cursor: 0,
            build_output: VecDeque::new(),
            build_scroll: 0,
            sessions: vec![],
            new_project: None,
            exec_pending_rx,
            stop_pending_rx,
            net_pending_rx,
            audit_rx,
            build_event_rx,
            build_event_tx,
            build_task: None,
            should_quit: false,
            log_fullscreen: false,
            terminal_fullscreen: false,
            ctrl_c_times: Vec::new(),
            last_terminal_esc: None,
            scroll_mode: false,
            terminal_scroll: 0,
            project_watch: HashMap::new(),
            last_watch_tick: std::time::Instant::now(),
        })
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let cfg = self.config.get();
        let mut items = Vec::new();
        for (pi, proj) in cfg.projects.iter().enumerate() {
            items.push(SidebarItem::Project(pi));
            for (si, session) in self.sessions.iter().enumerate() {
                if session.project == proj.name {
                    items.push(SidebarItem::Session(si));
                }
            }
            if self.build_project_idx == Some(pi) && self.build_is_running() {
                items.push(SidebarItem::Build(pi));
            }
            items.push(SidebarItem::Launch(pi));
            items.push(SidebarItem::Settings(pi));
        }
        items.push(SidebarItem::NewProject);
        items
    }

    pub fn selected_project_idx(&self) -> Option<usize> {
        match self.sidebar_items().get(self.sidebar_idx) {
            Some(SidebarItem::Project(pi)) => Some(*pi),
            Some(SidebarItem::Session(si)) => {
                let cfg = self.config.get();
                let name = self.sessions.get(*si)?.project.as_str();
                cfg.projects.iter().position(|p| p.name == name)
            }
            Some(SidebarItem::Settings(pi)) => Some(*pi),
            Some(SidebarItem::Launch(pi)) => Some(*pi),
            Some(SidebarItem::Build(pi)) => Some(*pi),
            Some(SidebarItem::NewProject) => None,
            None => None,
        }
    }

    pub fn is_project_watching(&self, project_idx: usize) -> bool {
        self.project_watch
            .get(&project_idx)
            .map(|s| s.enabled)
            .unwrap_or(false)
    }

    pub fn project_watch_spinner(&self, project_idx: usize) -> Option<&'static str> {
        if !self.is_project_watching(project_idx) {
            return None;
        }
        const FRAMES: [&str; 2] = ["●", "○"];
        let phase = self
            .project_watch
            .get(&project_idx)
            .map(|s| s.spinner_phase)
            .unwrap_or(0);
        Some(FRAMES[phase % FRAMES.len()])
    }

    pub fn pending_for_session(&self, session_idx: usize) -> Vec<usize> {
        let project = match self.sessions.get(session_idx) {
            Some(s) => s.project.as_str(),
            None => return vec![],
        };
        self.pending_exec
            .iter()
            .enumerate()
            .filter(|(_, item)| item.project == project)
            .map(|(i, _)| i)
            .collect()
    }

    fn active_exec_modal_idx(&self) -> Option<usize> {
        let si = self.active_session?;
        self.pending_for_session(si).into_iter().next()
    }

    fn session_is_loading(&self, session_idx: usize) -> bool {
        let Some(session) = self.sessions.get(session_idx) else {
            return false;
        };
        if session.is_exited() {
            return false;
        }
        let term = session.term.lock();
        let mut content = term.renderable_content();
        !content
            .display_iter
            .any(|indexed| !indexed.cell.c.is_whitespace())
    }

    fn close_session(&mut self, idx: usize) {
        if idx >= self.sessions.len() {
            return;
        }
        if let Some(tok) = self.sessions.get(idx).map(|s| s.session_token.clone()) {
            self.session_registry.remove(&tok);
        }
        if let Some(session) = self.sessions.get(idx) {
            if !session.is_exited() {
                session.terminate();
            }
        }
        self.sessions.remove(idx);
        self.remap_session_indices_after_removal(idx);
        let items = self.sidebar_items();
        if self.sidebar_idx >= items.len() {
            self.sidebar_idx = items.len().saturating_sub(1);
        }
    }

    fn clear_terminal_fullscreen_for_removed_session(&mut self, removed_idx: usize) {
        if self.active_session == Some(removed_idx) {
            self.terminal_fullscreen = false;
            self.last_terminal_esc = None;
        }
    }

    fn remap_session_indices_after_removal(&mut self, removed_idx: usize) {
        self.clear_terminal_fullscreen_for_removed_session(removed_idx);
        match self.active_session {
            Some(si) if si == removed_idx => {
                self.active_session = None;
                self.focus = Focus::Sidebar;
            }
            Some(si) if si > removed_idx => {
                self.active_session = Some(si - 1);
            }
            _ => {}
        }
        match self.preview_session {
            Some(si) if si == removed_idx => {
                self.preview_session = None;
            }
            Some(si) if si > removed_idx => {
                self.preview_session = Some(si - 1);
            }
            _ => {}
        }
    }

    fn terminate_all_sessions(&mut self) {
        for session in &self.sessions {
            if !session.is_exited() {
                session.terminate();
            }
        }
    }

    fn handle_stop_request(
        &mut self,
        project: &str,
        container_id: &str,
    ) -> ContainerStopDecision {
        let normalized = container_id.trim();
        let Some(idx) = self.sessions.iter().position(|session| {
            session.project == project
                && (session.container_id == normalized
                    || session.container_id.starts_with(normalized)
                    || normalized.starts_with(&session.container_id))
        }) else {
            self.push_log(
                format!(
                    "killme request for project '{}' could not find container {}",
                    project, normalized
                ),
                true,
            );
            return ContainerStopDecision::NotFound;
        };

        let label = self.sessions[idx].tab_label();
        if self.sessions[idx].is_exited() {
            self.push_log(
                format!("killme request for '{}' ignored; container already exited", label),
                false,
            );
            return ContainerStopDecision::Stopped;
        }

        self.push_log(format!("killme requested for '{}'", label), false);
        self.sessions[idx].terminate();
        self.sessions[idx]
            .exited
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if self.active_session == Some(idx) {
            self.active_session = None;
            self.focus = Focus::Sidebar;
        }
        ContainerStopDecision::Stopped
    }

    fn push_log(&mut self, text: impl Into<String>, is_error: bool) {
        self.log.push_front(LogEntry::Msg {
            text: text.into(),
            is_error,
            timestamp: chrono::Utc::now(),
        });
        if self.log.len() > 500 {
            self.log.pop_back();
        }
    }

    fn log_project_rules_status(&mut self, project: &crate::config::ProjectConfig) {
        let rules_path = project.canonical_path.join("zero-rules.toml");
        if !rules_path.exists() {
            self.push_log(
                format!(
                    "Searched for rules at {} but zero-rules.toml was not found",
                    rules_path.display()
                ),
                false,
            );
            return;
        }

        match crate::rules::load(&rules_path) {
            Ok(r) => self.push_log(
                format!(
                    "Loaded rules from {} (hostdo: {}, network: {})",
                    rules_path.display(),
                    r.hostdo.commands.len(),
                    r.network.rules.len()
                ),
                false,
            ),
            Err(e) => self.push_log(
                format!("Failed loading rules from {}: {}", rules_path.display(), e),
                true,
            ),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.build_is_running() {
                self.cancel_build();
                return;
            }

            if self.focus == Focus::Terminal {
                if let Some(si) = self.active_session {
                    if self.session_is_loading(si) {
                        let label = self.sessions[si].tab_label();
                        self.push_log(format!("Cancelled container startup: {}", label), false);
                        self.close_session(si);
                        return;
                    }
                }
            }

            let running = self.sessions.iter().any(|s| !s.is_exited());
            if !running {
                self.should_quit = true;
                return;
            }

            if self.focus == Focus::Terminal {
                if let Some(si) = self.active_session {
                    if let Some(session) = self.sessions.get(si) {
                        session.send_input(vec![0x03]);
                    }
                }
            }

            let now = std::time::Instant::now();
            let window = std::time::Duration::from_secs(2);
            self.ctrl_c_times
                .retain(|t| now.duration_since(*t) < window);
            self.ctrl_c_times.push(now);
            if self.ctrl_c_times.len() >= 4 {
                self.should_quit = true;
            }
            return;
        }

        if let Some(idx) = self.active_exec_modal_idx() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => self.approve_exec(idx, false),
                KeyCode::Char('r') => self.approve_exec(idx, true),
                KeyCode::Char('n') | KeyCode::Esc => self.deny_exec(idx),
                KeyCode::Char('d') => self.deny_exec_forever(idx),
                _ => {}
            }
            return;
        }
        if !self.pending_net.is_empty() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => self.approve_net(0),
                KeyCode::Char('r') => self.approve_net_forever(0),
                KeyCode::Char('n') | KeyCode::Esc => self.deny_net(0),
                KeyCode::Char('d') => self.deny_net_forever(0),
                _ => {}
            }
            return;
        }

        if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.focus_sidebar_shortcut();
            return;
        }

        if self.log_fullscreen {
            match key.code {
                KeyCode::Char('o') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.log_fullscreen = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.log_scroll = self.log_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.log_scroll = self.log_scroll.saturating_add(1);
                }
                _ => {}
            }
            return;
        }

        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Terminal => self.handle_terminal_key(key),
            Focus::Settings => self.handle_settings_key(key),
            Focus::ContainerPicker => self.handle_picker_key(key),
            Focus::ImageBuild => self.handle_build_key(key),
            Focus::NewProject => self.handle_new_project_key(key),
        }
    }

    fn focus_sidebar_shortcut(&mut self) {
        self.last_terminal_esc = None;
        self.log_fullscreen = false;
        self.terminal_fullscreen = false;
        match self.focus {
            Focus::Sidebar => {}
            Focus::Terminal => {
                self.focus = Focus::Sidebar;
            }
            Focus::Settings => {
                self.active_settings_project = None;
                self.focus = Focus::Sidebar;
            }
            Focus::ContainerPicker => {
                self.container_picker = None;
                self.focus = Focus::Sidebar;
            }
            Focus::ImageBuild => {
                if self.build_is_running() {
                    self.focus = Focus::Sidebar;
                } else {
                    self.build_container_idx = None;
                    self.build_project_idx = None;
                    self.focus = Focus::Sidebar;
                }
            }
            Focus::NewProject => {
                self.new_project = None;
                self.focus = Focus::Sidebar;
            }
        }
        let items = self.sidebar_items();
        self.update_sidebar_preview(&items);
    }

    fn open_log_fullscreen(&mut self) {
        self.terminal_fullscreen = false;
        self.log_fullscreen = true;
    }

    fn open_terminal_fullscreen(&mut self) {
        self.log_fullscreen = false;
        self.terminal_fullscreen = true;
        self.last_terminal_esc = None;
    }

    fn close_terminal_fullscreen(&mut self) {
        self.terminal_fullscreen = false;
        self.last_terminal_esc = None;
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        let items = self.sidebar_items();
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.sidebar_move_up(&items);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.sidebar_move_down(&items);
            }
            KeyCode::Char('o') => self.open_log_fullscreen(),
            KeyCode::Enter | KeyCode::Char('l') => self.handle_sidebar_enter(&items),
            _ => {}
        }
    }

    fn sidebar_move_up(&mut self, items: &[SidebarItem]) {
        self.sidebar_move_to_next_selectable(items, -1);
        self.update_sidebar_preview(items);
        self.ensure_sidebar_visible(items, 10); // Default height
    }

    fn sidebar_move_down(&mut self, items: &[SidebarItem]) {
        self.sidebar_move_to_next_selectable(items, 1);
        self.update_sidebar_preview(items);
        self.ensure_sidebar_visible(items, 10); // Default height
    }

    fn sidebar_move_to_next_selectable(&mut self, items: &[SidebarItem], dir: i8) {
        if items.is_empty() {
            return;
        }

        let len = items.len();
        let mut idx = self.sidebar_idx.min(len.saturating_sub(1));

        // Move at least one step, then keep stepping until we find a selectable row.
        for _ in 0..len {
            idx = if dir < 0 {
                if idx == 0 { len - 1 } else { idx - 1 }
            } else if idx >= len - 1 {
                0
            } else {
                idx + 1
            };

            if Self::sidebar_item_is_selectable(&items[idx]) {
                self.sidebar_idx = idx;
                return;
            }
        }
        // Degenerate case: everything is non-selectable (shouldn't happen).
        self.sidebar_idx = 0;
    }

    fn ensure_sidebar_visible(&mut self, items: &[SidebarItem], visible_height: usize) {
        if items.is_empty() || visible_height == 0 {
            return;
        }
        if self.sidebar_idx < self.sidebar_offset {
            self.sidebar_offset = self.sidebar_idx;
        } else if self.sidebar_idx >= self.sidebar_offset + visible_height {
            self.sidebar_offset = self.sidebar_idx - visible_height + 1;
        }
    }

    fn update_sidebar_preview(&mut self, items: &[SidebarItem]) {
        self.preview_session = match items.get(self.sidebar_idx) {
            Some(SidebarItem::Session(si)) => Some(*si),
            _ => None,
        };
    }

    fn handle_sidebar_enter(&mut self, items: &[SidebarItem]) {
        match items.get(self.sidebar_idx).cloned() {
            Some(SidebarItem::Project(_)) => {
                // do nothing
            }
            Some(SidebarItem::Settings(pi)) => {
                self.active_settings_project = Some(pi);
                self.settings_cursor = 0;
                self.focus = Focus::Settings;
            }
            Some(SidebarItem::Launch(_)) => self.open_picker(),
            Some(SidebarItem::Build(_)) => {
                self.active_session = None;
                self.focus = Focus::ImageBuild;
                self.active_settings_project = None;
            }
            Some(SidebarItem::Session(si)) => {
                if let Some(session) = self.sessions.get(si) {
                    session.clear_bell();
                }
                self.active_session = Some(si);
                self.preview_session = Some(si);
                self.scroll_mode = false;
                self.terminal_scroll = 0;
                self.focus = Focus::Terminal;
                self.active_settings_project = None;
            }
            Some(SidebarItem::NewProject) => self.open_new_project(),
            None => {}
        }
    }

    const NEW_PROJECT_ROW_COUNT: usize = 6;

    fn open_new_project(&mut self) {
        let cfg = self.config.get();
        self.new_project = Some(NewProjectState {
            cursor: 0,
            name: String::new(),
            canonical_dir: String::new(),
            sync_mode: cfg.defaults.sync.mode.clone(),
            project_type: crate::new_project::ProjectType::None,
            error: None,
        });
        self.focus = Focus::NewProject;
        self.active_session = None;
        self.active_settings_project = None;
        self.container_picker = None;
    }

    fn handle_new_project_key(&mut self, key: KeyEvent) {
        let Some(state) = self.new_project.as_mut() else {
            self.focus = Focus::Sidebar;
            return;
        };

        if matches!(state.cursor, 0 | 1)
            && let KeyCode::Char(c) = key.code
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.append_new_project_text(&c.to_string());
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.new_project = None;
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.cursor = state.cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                state.cursor = (state.cursor + 1).min(Self::NEW_PROJECT_ROW_COUNT - 1);
            }
            KeyCode::Left => match state.cursor {
                2 => state.sync_mode = prev_sync_mode(&state.sync_mode),
                3 => state.project_type = state.project_type.prev(),
                _ => {}
            },
            KeyCode::Right => match state.cursor {
                2 => state.sync_mode = next_sync_mode(&state.sync_mode),
                3 => state.project_type = state.project_type.next(),
                _ => {}
            },
            KeyCode::Backspace => match state.cursor {
                0 => {
                    state.name.pop();
                }
                1 => {
                    state.canonical_dir.pop();
                }
                _ => {}
            },
            KeyCode::Enter => match state.cursor {
                2 => state.sync_mode = next_sync_mode(&state.sync_mode),
                3 => state.project_type = state.project_type.next(),
                4 => self.submit_new_project(),
                5 => {
                    self.new_project = None;
                    self.focus = Focus::Sidebar;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn append_new_project_text(&mut self, text: &str) {
        let Some(state) = self.new_project.as_mut() else {
            return;
        };
        let cleaned = text.replace(['\r', '\n'], "");
        if cleaned.is_empty() {
            return;
        }
        match state.cursor {
            0 => state.name.push_str(&cleaned),
            1 => state.canonical_dir.push_str(&cleaned),
            _ => {}
        }
    }

    fn submit_new_project(&mut self) {
        let Some((name, canonical_raw, sync_mode, project_type)) =
            self.new_project.as_mut().map(|state| {
                state.error = None;
                (
                    state.name.trim().to_string(),
                    state.canonical_dir.trim().to_string(),
                    state.sync_mode.clone(),
                    state.project_type,
                )
            })
        else {
            return;
        };

        if name.is_empty() {
            self.set_new_project_error("project name is required".to_string());
            return;
        }
        if canonical_raw.is_empty() {
            self.set_new_project_error("canonical dir is required".to_string());
            return;
        }

        let canonical_path = match crate::config::expand_path(std::path::Path::new(&canonical_raw))
        {
            Ok(p) => p,
            Err(e) => {
                self.set_new_project_error(format!("canonical dir is invalid: {e}"));
                return;
            }
        };
        if !canonical_path.exists() {
            self.set_new_project_error(format!(
                "canonical dir does not exist: {}",
                canonical_path.display()
            ));
            return;
        }
        if !canonical_path.is_dir() {
            self.set_new_project_error(format!(
                "canonical dir is not a directory: {}",
                canonical_path.display()
            ));
            return;
        }

        let cfg = self.config.get();
        if cfg.projects.iter().any(|p| p.name == name) {
            self.set_new_project_error(format!("project name already exists: '{name}'"));
            return;
        }

        match crate::new_project::write_rules_if_missing(&canonical_path, project_type) {
            Ok(false) => {}
            Ok(true) => self.push_log(
                format!(
                    "created {}",
                    canonical_path.join("zero-rules.toml").display()
                ),
                false,
            ),
            Err(e) => {
                self.set_new_project_error(format!("failed writing zero-rules.toml: {e}"));
                return;
            }
        };

        if let Err(e) = crate::new_project::append_project_block(
            &self.loaded_config_path,
            &name,
            &canonical_path,
            sync_mode,
        ) {
            self.set_new_project_error(format!("failed updating config: {e}"));
            return;
        }

        let new_config = match crate::config::load(&self.loaded_config_path) {
            Ok(c) => c,
            Err(e) => {
                self.set_new_project_error(format!("config reload failed: {e}"));
                return;
            }
        };
        let new_pi = new_config.projects.iter().position(|p| p.name == name);
        self.config.set(std::sync::Arc::new(new_config));
        self.refresh_projects_cache();

        self.push_log(format!("added project '{name}'"), false);
        self.new_project = None;
        self.focus = Focus::Sidebar;

        if let Some(pi) = new_pi {
            if let Some(pos) = self
                .sidebar_items()
                .iter()
                .position(|item| *item == SidebarItem::Launch(pi))
            {
                self.sidebar_idx = pos;
            }
        }
    }

    fn set_new_project_error(&mut self, msg: String) {
        if let Some(state) = self.new_project.as_mut() {
            state.error = Some(msg);
        }
    }

    fn refresh_projects_cache(&mut self) {
        let cfg = self.config.get();
        let mut last_reports: std::collections::HashMap<String, Option<SyncReport>> =
            std::collections::HashMap::new();
        for p in &self.projects {
            last_reports.insert(p.name.clone(), p.last_report.clone());
        }

        self.projects = cfg
            .projects
            .iter()
            .map(|p| ProjectStatus {
                name: p.name.clone(),
                last_report: last_reports.get(&p.name).cloned().unwrap_or(None),
            })
            .collect();
    }

    pub(super) fn settings_action_rows_for(
        mode: SyncMode,
        watching: bool,
    ) -> Vec<SettingsActionRow> {
        if mode == SyncMode::Direct {
            return vec![SettingsActionRow {
                key: 'r',
                label: "Reload rules".to_string(),
                desc: "Rescan and reload zero-rules.toml for this project.",
                action: SettingsAction::ReloadRules,
            }];
        }

        vec![
            SettingsActionRow {
                key: 's',
                label: "Seed workspace now".to_string(),
                desc: "Copy canonical files into workspace using sync rules.",
                action: SettingsAction::Seed,
            },
            SettingsActionRow {
                key: 'p',
                label: "Pushback workspace now".to_string(),
                desc: "Copy workspace edits back to canonical using sync rules.",
                action: SettingsAction::Pushback,
            },
            SettingsActionRow {
                key: if watching { 't' } else { 'w' },
                label: if watching {
                    "Stop file system watching".to_string()
                } else {
                    "Watch file system (runs Seed first)".to_string()
                },
                desc: if watching {
                    "Disable continuous sync for this project."
                } else {
                    "Continuously apply sync behavior based on sync mode."
                },
                action: SettingsAction::WatchToggle,
            },
            SettingsActionRow {
                key: 'r',
                label: "Reload rules".to_string(),
                desc: "Rescan and reload zero-rules.toml for this project.",
                action: SettingsAction::ReloadRules,
            },
            SettingsActionRow {
                key: 'x',
                label: "Clear workspace".to_string(),
                desc: "Delete the entire workspace directory for a clean re-seed.",
                action: SettingsAction::Clear,
            },
        ]
    }

    pub(super) fn settings_action_rows(&self, project_idx: usize) -> Vec<SettingsActionRow> {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(project_idx) else {
            return Vec::new();
        };
        let mode = crate::config::effective_sync_mode(proj, &cfg.defaults);
        let watching = self.is_project_watching(project_idx);
        Self::settings_action_rows_for(mode, watching)
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        let Some(pi) = self.active_settings_project else {
            self.focus = Focus::Sidebar;
            return;
        };

        let actions_len = self.settings_action_rows(pi).len();
        if actions_len == 0 {
            self.focus = Focus::Sidebar;
            self.active_settings_project = None;
            return;
        }
        if self.settings_cursor >= actions_len {
            self.settings_cursor = actions_len.saturating_sub(1);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => {
                self.focus = Focus::Sidebar;
                self.active_settings_project = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.settings_cursor > 0 {
                    self.settings_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.settings_cursor + 1 < actions_len {
                    self.settings_cursor += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => self.run_settings_action(pi),
            KeyCode::Char('r') | KeyCode::Char('R') => self.do_reload_rules(pi),
            KeyCode::Char('s')
            | KeyCode::Char('S')
            | KeyCode::Char('p')
            | KeyCode::Char('P')
            | KeyCode::Char('w')
            | KeyCode::Char('W')
            | KeyCode::Char('t')
            | KeyCode::Char('T')
            | KeyCode::Char('x')
            | KeyCode::Char('X') => {
                let cfg = self.config.get();
                if let Some(proj) = cfg.projects.get(pi) {
                    if crate::config::effective_sync_mode(proj, &cfg.defaults) != SyncMode::Direct {
                        match key.code {
                            KeyCode::Char('s') | KeyCode::Char('S') => self.do_seed_project(pi),
                            KeyCode::Char('p') | KeyCode::Char('P') => self.do_pushback_project(pi),
                            KeyCode::Char('w') | KeyCode::Char('W') => self.start_project_watch(pi),
                            KeyCode::Char('t') | KeyCode::Char('T') => self.stop_project_watch(pi),
                            KeyCode::Char('x') | KeyCode::Char('X') => self.do_clear_workspace(pi),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn run_settings_action(&mut self, pi: usize) {
        let actions = self.settings_action_rows(pi);
        let Some(row) = actions.get(self.settings_cursor) else {
            return;
        };
        match row.action {
            SettingsAction::Seed => self.do_seed_project(pi),
            SettingsAction::Pushback => self.do_pushback_project(pi),
            SettingsAction::WatchToggle => {
                if self.is_project_watching(pi) {
                    self.stop_project_watch(pi);
                } else {
                    self.start_project_watch(pi);
                }
            }
            SettingsAction::ReloadRules => self.do_reload_rules(pi),
            SettingsAction::Clear => self.do_clear_workspace(pi),
        }
    }

    fn do_reload_rules(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        let proj = proj.clone();
        self.log_project_rules_status(&proj);
    }

    fn do_clear_workspace(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        if crate::config::effective_sync_mode(proj, &cfg.defaults) == SyncMode::Direct {
            self.push_log(
                format!(
                    "clear '{}': disabled for projects.sync.mode='direct' (would affect canonical directory)",
                    proj.name
                ),
                true,
            );
            return;
        }
        let workspace_path = crate::config::effective_workspace_path(proj, &cfg.workspace);
        if !workspace_path.exists() {
            self.push_log(
                format!("clear '{}': workspace directory does not exist", proj.name),
                false,
            );
            return;
        }
        match std::fs::remove_dir_all(&workspace_path) {
            Ok(()) => self.push_log(
                format!(
                    "clear '{}': removed {}",
                    proj.name,
                    workspace_path.display()
                ),
                false,
            ),
            Err(e) => self.push_log(format!("clear '{}' failed: {}", proj.name, e), true),
        }
    }

    fn handle_terminal_key(&mut self, key: KeyEvent) {
        if self.build_is_running() && self.active_session.is_none() {
            self.handle_build_scroll_key(key);
            return;
        }

        if key.code == KeyCode::Char('g') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.terminal_fullscreen {
                self.close_terminal_fullscreen();
            } else {
                self.open_terminal_fullscreen();
            }
            return;
        }

        if self.scroll_mode {
            self.handle_scroll_mode_key(key);
            return;
        }

        if key.code == KeyCode::Esc {
            let now = std::time::Instant::now();
            let threshold = std::time::Duration::from_millis(400);
            if self
                .last_terminal_esc
                .map(|prev| now.duration_since(prev) <= threshold)
                .unwrap_or(false)
            {
                self.last_terminal_esc = None;
                if self.terminal_fullscreen {
                    self.close_terminal_fullscreen();
                } else {
                    self.should_quit = true;
                }
                return;
            }
            self.last_terminal_esc = Some(now);
            return;
        } else {
            self.last_terminal_esc = None;
        }

        if let Some(si) = self.active_session {
            if self.session_is_loading(si) {
                return;
            }
        }

        if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::ALT) {
            self.open_log_fullscreen();
            return;
        }

        if is_scroll_mode_toggle_key(key) {
            self.scroll_mode = true;
            return;
        }

        if let Some(si) = self.active_session {
            if let Some(bytes) = key_to_bytes(key) {
                if let Some(session) = self.sessions.get(si) {
                    session.send_input(bytes);
                }
            }
        }
    }

    fn handle_scroll_mode_key(&mut self, key: KeyEvent) {
        let half_page = self
            .active_session
            .and_then(|si| self.sessions.get(si))
            .map(|s| s.term.lock().screen_lines().max(2) / 2)
            .unwrap_or(15);

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.terminal_scroll = self.terminal_scroll.saturating_add(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.terminal_scroll = self.terminal_scroll.saturating_sub(1)
            }
            KeyCode::PageUp => {
                self.terminal_scroll = self.terminal_scroll.saturating_add(half_page)
            }
            KeyCode::PageDown => {
                self.terminal_scroll = self.terminal_scroll.saturating_sub(half_page)
            }
            KeyCode::Home | KeyCode::Char('g') => self.terminal_scroll = usize::MAX,
            KeyCode::End | KeyCode::Char('G') => self.terminal_scroll = 0,
            KeyCode::Esc | KeyCode::Char('q') => self.exit_scroll_mode(),
            _ => self.exit_scroll_mode(),
        }
    }

    fn exit_scroll_mode(&mut self) {
        self.scroll_mode = false;
        self.terminal_scroll = 0;
    }

    fn handle_build_scroll_key(&mut self, key: KeyEvent) {
        let max_scroll = self.build_output.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.build_scroll = self.build_scroll.saturating_add(1).min(max_scroll)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.build_scroll = self.build_scroll.saturating_sub(1)
            }
            KeyCode::PageUp => {
                self.build_scroll = self.build_scroll.saturating_add(15).min(max_scroll)
            }
            KeyCode::PageDown => self.build_scroll = self.build_scroll.saturating_sub(15),
            KeyCode::Home | KeyCode::Char('g') => self.build_scroll = max_scroll,
            KeyCode::End | KeyCode::Char('G') => self.build_scroll = 0,
            KeyCode::Esc => self.focus = Focus::Sidebar,
            _ => {}
        }
    }

    fn open_picker(&mut self) {
        let cfg = self.config.get();
        if cfg.containers.is_empty() {
            self.push_log("no containers defined in config", true);
            return;
        }
        self.container_picker = Some(0);
        self.focus = Focus::ContainerPicker;
    }

    fn handle_picker_key(&mut self, key: KeyEvent) {
        let cfg = self.config.get();
        let n = cfg.containers.len();
        let idx = self.container_picker.as_mut().unwrap();

        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => {
                self.container_picker = None;
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if *idx > 0 {
                    *idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *idx + 1 < n {
                    *idx += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                let ctr_idx = *idx;
                self.container_picker = None;
                self.focus = Focus::Sidebar;
                self.do_launch_container(ctr_idx);
            }
            _ => {}
        }
    }

    const BUILD_ACTION_COUNT: usize = 2;

    fn handle_build_key(&mut self, key: KeyEvent) {
        if self.build_is_running() {
            let max_scroll = self.build_output.len();
            match key.code {
                KeyCode::Esc | KeyCode::Char('h') => self.focus = Focus::Sidebar,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.build_scroll = self.build_scroll.saturating_add(1).min(max_scroll)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.build_scroll = self.build_scroll.saturating_sub(1)
                }
                KeyCode::PageUp => {
                    self.build_scroll = self.build_scroll.saturating_add(15).min(max_scroll)
                }
                KeyCode::PageDown => self.build_scroll = self.build_scroll.saturating_sub(15),
                KeyCode::Home | KeyCode::Char('g') => self.build_scroll = max_scroll,
                KeyCode::End | KeyCode::Char('G') => self.build_scroll = 0,
                _ => {}
            }
            return;
        }

        if matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R')) {
            self.build_cursor = 0;
            self.run_build_action();
            return;
        }
        if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')) {
            self.build_cursor = 1;
            self.run_build_action();
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => {
                self.build_container_idx = None;
                self.build_project_idx = None;
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.build_cursor > 0 {
                    self.build_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.build_cursor + 1 < Self::BUILD_ACTION_COUNT {
                    self.build_cursor += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => self.run_build_action(),
            _ => {}
        }
    }

    fn run_build_action(&mut self) {
        let cfg = self.config.get();
        let Some(ctr_idx) = self.build_container_idx else {
            return;
        };
        let Some(ctr) = cfg.containers.get(ctr_idx) else {
            return;
        };
        let (base_cmd, agent_cmd) = Self::build_commands_for(&cfg.docker_dir, &ctr.image);

        let requested = match self.build_cursor {
            0 => match agent_cmd.as_ref() {
                Some(agent_cmd) => Some((
                    "build + launch",
                    format!(
                        "{} && {}",
                        shell_command_for_docker_args(&base_cmd),
                        shell_command_for_docker_args(agent_cmd)
                    ),
                )),
                None => Some(("build + launch", shell_command_for_docker_args(&base_cmd))),
            },
            1 => {
                self.build_container_idx = None;
                self.build_project_idx = None;
                self.focus = Focus::Sidebar;
                return;
            }
            _ => None,
        };

        let Some((label, shell_command)) = requested else {
            return;
        };

        self.build_project_idx = self.selected_project_idx();
        let Some(launch_project_idx) = self.build_project_idx else {
            self.push_log("cannot start build: no project selected", true);
            return;
        };
        self.start_docker_build(label, shell_command, launch_project_idx, ctr_idx);
    }

    pub fn build_is_running(&self) -> bool {
        self.build_task.is_some()
    }

    pub fn active_build_command(&self) -> Option<&str> {
        self.build_task
            .as_ref()
            .map(|task| task.shell_command.as_str())
    }

    fn start_docker_build(
        &mut self,
        label: &str,
        shell_command: String,
        launch_project_idx: usize,
        launch_container_idx: usize,
    ) {
        if self.build_task.is_some() {
            self.push_log("a docker build is already running", true);
            return;
        }

        self.build_output.clear();
        self.build_scroll = 0;
        if self.build_project_idx.is_none() {
            self.build_project_idx = self.selected_project_idx();
        }
        self.active_session = None;
        self.focus = Focus::ImageBuild;
        self.push_log(format!("starting {label} in shell"), false);
        self.push_log(format!("$ {shell_command}"), false);

        if let Some(pi) = self.build_project_idx {
            let items = self.sidebar_items();
            if let Some(pos) = items
                .iter()
                .position(|item| *item == SidebarItem::Build(pi))
            {
                self.sidebar_idx = pos;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.build_task = Some(BuildTaskState {
            label: label.to_string(),
            shell_command: shell_command.clone(),
            cancel_flag: cancel_flag.clone(),
        });

        let tx = self.build_event_tx.clone();
        let label = label.to_string();
        tokio::spawn(async move {
            run_build_shell_command(
                label,
                shell_command,
                launch_project_idx,
                launch_container_idx,
                cancel_flag,
                tx,
            )
            .await;
        });
    }

    fn cancel_build(&mut self) {
        let Some(task) = self.build_task.as_ref() else {
            return;
        };
        task.cancel_flag.store(true, Ordering::SeqCst);
        self.push_log(format!("cancelling {}...", task.label), true);
    }

    fn push_build_output(&mut self, line: impl Into<String>, is_error: bool) {
        self.build_output.push_back((line.into(), is_error));
        if self.build_output.len() > 400 {
            self.build_output.pop_front();
        }
        if self.build_scroll > 0 {
            self.build_scroll = self.build_scroll.saturating_add(1);
        }
    }

    pub fn build_commands_for(
        docker_dir: &Path,
        image: &str,
    ) -> (Vec<String>, Option<Vec<String>>) {
        let parts: Vec<&str> = image.splitn(2, ':').collect();
        let name = parts[0].split('/').last().unwrap_or(parts[0]);
        let tag = parts.get(1).copied().unwrap_or("ubuntu-24.04");
        let dockerfile_root = docker_dir;
        let base_dockerfile = dockerfile_root.join(format!("{tag}.Dockerfile"));
        let mut base_cmd = vec![
            "build".to_string(),
            "-t".to_string(),
            image.to_string(),
            "-f".to_string(),
            base_dockerfile.display().to_string(),
            docker_dir.display().to_string(),
        ];

        let agent_cmd = name.strip_prefix("agent-zero-").map(|agent| {
            base_cmd[2] = format!("my-agent:{tag}");
            vec![
                "build".to_string(),
                "-t".to_string(),
                image.to_string(),
                "-f".to_string(),
                dockerfile_root
                    .join(agent)
                    .join(format!("{tag}.Dockerfile"))
                    .display()
                    .to_string(),
                docker_dir.display().to_string(),
            ]
        });

        (base_cmd, agent_cmd)
    }

    fn do_seed_project(&mut self, pi: usize) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        if crate::config::effective_sync_mode(&proj_cfg, &cfg.defaults) == SyncMode::Direct {
            return;
        }
        match crate::sync::seed(&proj_cfg, &cfg.workspace, &cfg.defaults) {
            Ok(report) => {
                let mut msg = format!(
                    "seed '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("seed failed: {e}"), true),
        }
    }

    fn do_pushback_project(&mut self, pi: usize) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        if crate::config::effective_sync_mode(&proj_cfg, &cfg.defaults) == SyncMode::Direct {
            self.push_log(
                format!(
                    "pushback '{}': disabled for projects.sync.mode='direct'",
                    proj_cfg.name
                ),
                false,
            );
            return;
        }
        match crate::sync::pushback(&proj_cfg, &cfg.workspace, &cfg.defaults) {
            Ok(report) => {
                let mut msg = format!(
                    "pushback '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("pushback failed: {e}"), true),
        }
    }

    fn do_pushback_files(&mut self, pi: usize, changed: &[PathBuf]) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        match crate::sync::pushback_files(&proj_cfg, &cfg.workspace, &cfg.defaults, changed) {
            Ok(report) => {
                let mut msg = format!(
                    "pushback '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("pushback failed: {e}"), true),
        }
    }

    fn do_seed_files(&mut self, pi: usize, changed: &[PathBuf]) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        match crate::sync::seed_files(&proj_cfg, &cfg.workspace, &cfg.defaults, changed) {
            Ok(report) => {
                let mut msg = format!(
                    "seed '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("seed failed: {e}"), true),
        }
    }

    fn start_project_watch(&mut self, pi: usize) {
        if self.is_project_watching(pi) {
            return;
        }
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        if crate::config::effective_sync_mode(proj, &cfg.defaults) == SyncMode::Direct {
            self.push_log(
                format!(
                    "watch start '{}': disabled for projects.sync.mode='direct'",
                    proj.name
                ),
                false,
            );
            return;
        }
        self.do_seed_project(pi);
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        let workspace = crate::config::effective_workspace_path(proj, &cfg.workspace);
        let exclude_matcher = match crate::sync::build_project_exclude_matcher(proj, &cfg.defaults)
        {
            Ok(matcher) => matcher,
            Err(e) => {
                self.push_log(format!("watch start failed for '{}': {e}", proj.name), true);
                return;
            }
        };
        let canonical_files = compute_tree_file_map(&proj.canonical_path, &exclude_matcher);
        let workspace_files = compute_tree_file_map(&workspace, &exclude_matcher);
        self.project_watch.insert(
            pi,
            ProjectWatchState {
                enabled: true,
                spinner_phase: 0,
                canonical_files,
                workspace_files,
            },
        );
        self.push_log(format!("watch enabled for '{}'", proj.name), false);
    }

    fn stop_project_watch(&mut self, pi: usize) {
        let Some(state) = self.project_watch.get_mut(&pi) else {
            return;
        };
        if !state.enabled {
            return;
        }
        state.enabled = false;
        let cfg = self.config.get();
        if let Some(proj) = cfg.projects.get(pi) {
            self.push_log(format!("watch stopped for '{}'", proj.name), false);
        }
    }

    fn do_launch_container(&mut self, ctr_idx: usize) {
        let pi = match self.selected_project_idx() {
            Some(i) => i,
            None => {
                self.push_log("no project selected", true);
                return;
            }
        };
        self.do_launch_container_on_project(pi, ctr_idx);
    }

    fn open_image_build_prompt(&mut self, pi: usize, ctr_idx: usize, image: &str) {
        self.build_project_idx = Some(pi);
        self.build_container_idx = Some(ctr_idx);
        self.build_cursor = 0;
        self.build_output.clear();
        self.build_scroll = 0;
        self.active_session = None;
        self.active_settings_project = None;
        self.container_picker = None;
        self.focus = Focus::ImageBuild;
        self.push_log(format!("docker image '{image}' not found locally; build required"), true);
    }

    fn preflight_image_or_prompt_build<F>(
        &mut self,
        pi: usize,
        ctr_idx: usize,
        image: &str,
        image_exists: F,
    ) -> bool
    where
        F: FnOnce(&str) -> std::io::Result<bool>,
    {
        match image_exists(image) {
            Ok(true) => true,
            Ok(false) => {
                self.open_image_build_prompt(pi, ctr_idx, image);
                false
            }
            Err(e) => {
                // If we can't check, preserve legacy behavior: attempt to run and
                // surface the real docker error in the session/logs.
                self.push_log(format!("warning: failed to check docker image '{image}': {e}"), true);
                true
            }
        }
    }

    fn do_launch_container_on_project(&mut self, pi: usize, ctr_idx: usize) {
        let cfg = self.config.get();
        let exec_host = cfg.defaults.hostdo.server_host.trim();
        if host_bind_is_loopback(exec_host) {
            self.push_log(
                format!("cannot launch container: defaults.hostdo.server_host='{}' is loopback; set it to '0.0.0.0'", exec_host),
                true,
            );
            return;
        }
        let ctr = match cfg.containers.get(ctr_idx) {
            Some(c) => c.clone(),
            None => return,
        };
        let extra_instructions = match ctr.agent {
            crate::config::AgentKind::Claude => cfg
                .agents
                .claude
                .as_ref()
                .and_then(|agent| agent.extra_instructions.as_deref()),
            crate::config::AgentKind::Codex => cfg
                .agents
                .codex
                .as_ref()
                .and_then(|agent| agent.extra_instructions.as_deref()),
            crate::config::AgentKind::Gemini => cfg
                .agents
                .gemini
                .as_ref()
                .and_then(|agent| agent.extra_instructions.as_deref()),
            crate::config::AgentKind::Opencode | crate::config::AgentKind::None => None,
        };

        if ctr.agent == crate::config::AgentKind::Claude {
            let has_claude_json = ctr
                .mounts
                .iter()
                .any(|m| m.container == PathBuf::from("/home/ubuntu/.claude.json"));
            let has_claude_dir = ctr
                .mounts
                .iter()
                .any(|m| m.container == PathBuf::from("/home/ubuntu/.claude"));
            if !has_claude_json || !has_claude_dir {
                self.push_log("hint: Claude containers usually need mounts for '~/.claude.json' and '~/.claude'".to_string(), false);
            }
        }
        if ctr.agent == crate::config::AgentKind::Gemini {
            let has_gemini_home = ctr.mounts.iter().any(|m| {
                m.container == PathBuf::from("/home/ubuntu/.gemini")
                    || m.container == PathBuf::from("/root/.gemini")
            });
            if !has_gemini_home {
                self.push_log(
                    "hint: Gemini containers usually need a mount for '~/.gemini' to persist sign-in/session state".to_string(),
                    false,
                );
            }
        }

        let proj = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };

        if !self.preflight_image_or_prompt_build(pi, ctr_idx, &ctr.image, docker_image_exists) {
            return;
        }

        let mount_source_path =
            crate::config::effective_mount_source_path(&proj, &cfg.workspace, &cfg.defaults);
        self.log_project_rules_status(&proj);

        let exec_port = cfg.defaults.hostdo.server_port;
        let exec_host = &cfg.defaults.hostdo.server_host;
        let exec_url = format!("http://{exec_host}:{exec_port}");
        let proxy_host = &cfg.defaults.proxy.proxy_host;
        let scoped_proxy = match crate::proxy::spawn_scoped_listener(
            &self.proxy_state,
            proxy_host,
            &proj.name,
            &ctr.name,
        ) {
            Ok(listener) => listener,
            Err(e) => {
                self.push_log(
                    format!("cannot launch '{}' on '{}': {e}", ctr.name, proj.name),
                    true,
                );
                return;
            }
        };
        let proxy_url = format!("http://{}", scoped_proxy.addr);
        self.push_log(
            format!("launching '{}' on '{}'", ctr.name, proj.name),
            false,
        );

        match crate::agents::inject_agent_config(
            &ctr.agent,
            &mount_source_path,
            &proj.canonical_path,
            &proj.name,
            crate::config::effective_sync_mode(&proj, &cfg.defaults) == SyncMode::Direct,
            &ctr.mount_target,
            &exec_url,
            &proxy_url,
            extra_instructions,
        ) {
            Ok(true) => self.push_log(
                format!(
                    "created starter zero-rules.toml in '{}'",
                    proj.canonical_path.display()
                ),
                false,
            ),
            Ok(false) => {}
            Err(e) => self.push_log(format!("agent config injection warning: {e}"), true),
        }

        let (term_cols, term_rows) = crossterm::terminal::size().unwrap_or((120, 40));
        let pty_cols = term_cols.saturating_sub(38).max(20);
        let pty_rows = term_rows.saturating_sub(10).max(6);

        let codex_home_dir = cfg
            .logging
            .log_dir
            .join("codex-home")
            .join(crate::container::sanitize_docker_name(&proj.name));
        let has_host_codex_state_mount = ctr.mounts.iter().any(|m| {
            let p = &m.container;
            if p.file_name().and_then(|s| s.to_str()) == Some(".codex") {
                return true;
            }
            if p.file_name().and_then(|s| s.to_str()) != Some("codex") {
                return false;
            }
            p.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|s| s.to_str())
                == Some(".config")
        });

        let codex_home_host_path: Option<&std::path::Path> = if ctr.agent
            == crate::config::AgentKind::Codex
            && !ctr.env_passthrough.iter().any(|v| v == "CODEX_HOME")
            && !has_host_codex_state_mount
        {
            Some(codex_home_dir.as_path())
        } else {
            None
        };

        let gemini_home_dir = cfg
            .logging
            .log_dir
            .join("gemini-home")
            .join(crate::container::sanitize_docker_name(&proj.name));
        let has_host_gemini_state_mount = ctr.mounts.iter().any(|m| {
            let p = &m.container;
            if p.file_name().and_then(|s| s.to_str()) == Some(".gemini") {
                return true;
            }
            if p.file_name().and_then(|s| s.to_str()) != Some("gemini") {
                return false;
            }
            p.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|s| s.to_str())
                == Some(".config")
        });

        let gemini_home_host_path: Option<&std::path::Path> = if ctr.agent
            == crate::config::AgentKind::Gemini
            && !has_host_gemini_state_mount
        {
            Some(gemini_home_dir.as_path())
        } else {
            None
        };

        #[cfg(target_os = "macos")]
        if cfg.defaults.proxy.strict_network {
            self.push_log(
                "strict_network on macOS requires Docker `--privileged`; agent-zero applies it automatically for this container launch",
                false,
            );
        }

        let session_token = uuid::Uuid::new_v4().simple().to_string();
        self.session_registry.insert(
            session_token.clone(),
            crate::server::SessionIdentity {
                project: proj.name.clone(),
                container_id: String::new(),
                mount_target: ctr.mount_target.display().to_string(),
            },
        );

        match crate::container::spawn(
            &ctr,
            &proj.name,
            &mount_source_path,
            codex_home_host_path,
            gemini_home_host_path,
            &session_token,
            &self.token,
            &exec_url,
            &proxy_url,
            &self.ca_cert_path,
            Some(scoped_proxy),
            cfg.defaults.proxy.strict_network,
            pty_rows,
            pty_cols,
        ) {
            Ok((session, launch_notes)) => {
                let new_si = self.sessions.len();
                self.sessions.push(session);
                if let Some(s) = self.sessions.get(new_si) {
                    self.session_registry.insert(
                        s.session_token.clone(),
                        crate::server::SessionIdentity {
                            project: s.project.clone(),
                            container_id: s.container_id.clone(),
                            mount_target: s.mount_target.clone(),
                        },
                    );
                }
                self.active_session = Some(new_si);
                self.scroll_mode = false;
                self.terminal_scroll = 0;
                self.focus = Focus::Terminal;
                for note in launch_notes {
                    self.push_log(note, false);
                }
                if let Some(pos) = self
                    .sidebar_items()
                    .iter()
                    .position(|item| *item == SidebarItem::Session(new_si))
                {
                    self.sidebar_idx = pos;
                }
            }
            Err(e) => {
                self.push_log(
                    format!("launch '{}' on '{}' failed: {e}", ctr.name, proj.name),
                    true,
                );
            }
        }
    }

    fn approve_exec(&mut self, idx: usize, remember: bool) {
        if idx >= self.pending_exec.len() {
            return;
        }
        if remember {
            let item = &self.pending_exec[idx];
            let argv = item.argv.clone();
            let project_name = item.project.clone();
            let cwd = self.portable_cwd(&item.rule_cwd, &project_name);
            if let Some(rules_path) = self.project_rules_path(&project_name) {
                match crate::rules::append_auto_approval(&rules_path, &argv, &cwd) {
                    Ok(()) => {
                        self.push_log(
                            format!("Saved rule to {}: {}", rules_path.display(), argv.join(" ")),
                            false,
                        );
                        self.sync_rules_to_workspace(&project_name);
                    }
                    Err(e) => self.push_log(format!("Failed to save rule: {e}"), true),
                }
            } else {
                self.push_log(
                    format!("Cannot remember: unknown project '{project_name}'"),
                    true,
                );
            }
        }
        if let Some(tx) = self.pending_exec[idx].response_tx.take() {
            let _ = tx.send(ApprovalDecision::Approve { remember });
        }
        self.pending_exec.remove(idx);
    }

    fn deny_exec(&mut self, idx: usize) {
        if idx >= self.pending_exec.len() {
            return;
        }
        if let Some(tx) = self.pending_exec[idx].response_tx.take() {
            let _ = tx.send(ApprovalDecision::Deny);
        }
        self.pending_exec.remove(idx);
    }

    fn deny_exec_forever(&mut self, idx: usize) {
        if idx >= self.pending_exec.len() {
            return;
        }
        let item = &self.pending_exec[idx];
        let argv = item.argv.clone();
        let project_name = item.project.clone();
        let cwd = self.portable_cwd(&item.rule_cwd, &project_name);
        if let Some(rules_path) = self.project_rules_path(&project_name) {
            match crate::rules::append_deny_rule(&rules_path, &argv, &cwd) {
                Ok(()) => {
                    self.push_log(
                        format!(
                            "Saved deny rule to {}: {}",
                            rules_path.display(),
                            argv.join(" ")
                        ),
                        false,
                    );
                    self.sync_rules_to_workspace(&project_name);
                }
                Err(e) => self.push_log(format!("Failed to save deny rule: {e}"), true),
            }
        } else {
            self.push_log(
                format!("Cannot persist deny: unknown project '{project_name}'"),
                true,
            );
        }
        self.deny_exec(idx);
    }

    fn approve_net(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let tx = std::mem::replace(&mut self.pending_net[idx].response_tx, oneshot_dummy());
        let _ = tx.send(NetworkDecision::Allow);
        self.pending_net.remove(idx);
    }

    fn deny_net(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let tx = std::mem::replace(&mut self.pending_net[idx].response_tx, oneshot_dummy());
        let _ = tx.send(NetworkDecision::Deny);
        self.pending_net.remove(idx);
    }

    fn approve_net_forever(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        let project_name = self.pending_net[idx].source_project.clone();
        if project_name.is_none() {
            self.log_missing_network_project_context(idx, "allow");
        }
        match self.persist_network_rule(&host, NetworkPolicy::Auto, project_name.as_deref()) {
            Ok(updated_path) => {
                if let Some(path) = &updated_path {
                    self.push_log(
                        format!(
                            "added permanent allow rule for '{}' in {}",
                            host,
                            path.display()
                        ),
                        false,
                    );
                    if let Some(name) = &project_name {
                        self.sync_rules_to_workspace(name);
                    }
                } else {
                    self.push_log(
                        format!("network host '{}' already permanently allowed", host),
                        false,
                    );
                }
            }
            Err(e) => {
                self.push_log(
                    format!(
                        "failed to persist permanent allow rule for '{}': {}",
                        host, e
                    ),
                    true,
                );
            }
        }
        self.approve_net(idx);
    }

    fn deny_net_forever(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        let project_name = self.resolve_pending_network_project(idx);
        match self.persist_network_rule(&host, NetworkPolicy::Deny, project_name.as_deref()) {
            Ok(updated_path) => {
                if let Some(path) = &updated_path {
                    self.push_log(
                        format!(
                            "added permanent deny rule for '{}' in {}",
                            host,
                            path.display()
                        ),
                        false,
                    );
                    if let Some(name) = &project_name {
                        self.sync_rules_to_workspace(name);
                    }
                } else {
                    self.push_log(
                        format!("network host '{}' already permanently denied", host),
                        false,
                    );
                }
            }
            Err(e) => {
                self.push_log(
                    format!(
                        "failed to persist permanent deny rule for '{}': {}",
                        host, e
                    ),
                    true,
                );
            }
        }
        self.deny_net(idx);
    }

    fn resolve_pending_network_project(&self, idx: usize) -> Option<String> {
        let item = self.pending_net.get(idx)?;
        if let Some(project) = item.source_project.clone() {
            return Some(project);
        }
        if let Some(container_name) = item.source_container.as_deref() {
            let mut projects = self
                .sessions
                .iter()
                .filter(|s| !s.is_exited() && s.container_name == container_name)
                .map(|s| s.project.clone())
                .collect::<Vec<_>>();
            projects.sort();
            projects.dedup();
            if projects.len() == 1 {
                return projects.into_iter().next();
            }
        }
        let cfg = self.config.get();
        self.selected_project_idx()
            .and_then(|pi| cfg.projects.get(pi))
            .map(|p| p.name.clone())
    }

    fn persist_network_rule(
        &self,
        host: &str,
        policy: NetworkPolicy,
        project_name: Option<&str>,
    ) -> Result<Option<std::path::PathBuf>> {
        let rules_path = match project_name {
            Some(name) => match self.project_rules_path(name) {
                Some(path) => path,
                None => anyhow::bail!("cannot persist network rule: project '{}' not found", name),
            },
            None => anyhow::bail!(
                "cannot persist network rule: unknown project (request lacked project attribution)"
            ),
        };

        let is_new = !rules_path.exists();
        let mut rules = crate::rules::load(&rules_path)
            .with_context(|| format!("loading rules file '{}'", rules_path.display()))?;

        let exists = rules.network.rules.iter().any(|r| {
            r.host.eq_ignore_ascii_case(host)
                && r.policy == policy
                && r.path_prefix == "/"
                && r.methods.len() == 1
                && r.methods[0] == "*"
        });
        if exists {
            return Ok(None);
        }

        rules.network.rules.push(NetworkRule {
            methods: vec!["*".to_string()],
            host: host.to_string(),
            path_prefix: "/".to_string(),
            policy,
        });

        crate::rules::write_rules_file(&rules_path, &rules, is_new)
            .with_context(|| format!("writing rules file '{}'", rules_path.display()))?;
        Ok(Some(rules_path))
    }

    fn log_missing_network_project_context(&mut self, idx: usize, action: &str) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        self.push_log(
            format!("cannot persist permanent {action} rule for '{}' because the network request had no source project metadata", host),
            true,
        );
    }

    fn portable_cwd(&self, cwd: &Path, project_name: &str) -> String {
        let cfg = self.config.get();
        let mount_target = cfg
            .projects
            .iter()
            .find(|p| p.name == project_name)
            .and_then(|_| Some("/workspace"))
            .unwrap_or("/workspace");
        let cwd_str = cwd.display().to_string();
        if cwd_str == mount_target {
            "$WORKSPACE".to_string()
        } else if let Some(rest) = cwd_str.strip_prefix(&format!("{}/", mount_target)) {
            format!("$WORKSPACE/{rest}")
        } else {
            cwd_str
        }
    }

    fn project_rules_path(&self, project_name: &str) -> Option<std::path::PathBuf> {
        let cfg = self.config.get();
        cfg.projects
            .iter()
            .find(|p| p.name == project_name)
            .map(|p| p.canonical_path.join("zero-rules.toml"))
    }

    fn sync_rules_to_workspace(&mut self, project_name: &str) {
        let cfg = self.config.get();
        if let Some(pi) = cfg.projects.iter().position(|p| p.name == project_name) {
            self.do_seed_project(pi);
        }
    }

    fn drain_channels(&mut self) {
        for _ in 0..32 {
            match self.exec_pending_rx.try_recv() {
                Ok(item) => self.pending_exec.push(item),
                Err(_) => break,
            }
        }
        for _ in 0..32 {
            match self.stop_pending_rx.try_recv() {
                Ok(item) => self.pending_stop.push(item),
                Err(_) => break,
            }
        }
        for _ in 0..32 {
            match self.net_pending_rx.try_recv() {
                Ok(item) => self.pending_net.push(item),
                Err(_) => break,
            }
        }
        for _ in 0..32 {
            match self.audit_rx.try_recv() {
                Ok(entry) => {
                    self.log.push_front(LogEntry::Audit(entry));
                    if self.log.len() > 500 {
                        self.log.pop_back();
                    }
                }
                Err(_) => break,
            }
        }
        for _ in 0..256 {
            match self.build_event_rx.try_recv() {
                Ok(BuildEvent::Output { line, is_error }) => {
                    self.push_build_output(line, is_error);
                }
                Ok(BuildEvent::Finished {
                    label,
                    launch_project_idx,
                    launch_container_idx,
                    success,
                    cancelled,
                    exit_code,
                    error,
                    diagnostic,
                }) => {
                    self.build_task = None;
                    if let Some(error) = error {
                        self.build_project_idx = None;
                        self.push_log(format!("{label} failed: {error}"), true);
                        if let Some(diagnostic) = diagnostic {
                            self.push_log(format!("  build detail: {diagnostic}"), true);
                        }
                        self.focus = Focus::ImageBuild;
                        continue;
                    }
                    if cancelled {
                        self.build_project_idx = None;
                        self.push_log(format!("{label} cancelled"), true);
                        self.focus = Focus::ImageBuild;
                        continue;
                    }
                    if success {
                        self.build_project_idx = None;
                        self.push_log(format!("{label} finished successfully"), false);
                        self.build_container_idx = None;
                        self.do_launch_container_on_project(
                            launch_project_idx,
                            launch_container_idx,
                        );
                    } else {
                        self.build_project_idx = None;
                        let suffix = exit_code
                            .map(|code| format!(" (exit code {code})"))
                            .unwrap_or_default();
                        self.push_log(format!("{label} failed{suffix}"), true);
                        if let Some(diagnostic) = diagnostic {
                            self.push_log(format!("  build detail: {diagnostic}"), true);
                        }
                        self.focus = Focus::ImageBuild;
                    }
                }
                Err(_) => break,
            }
        }

        for i in (0..self.sessions.len()).rev() {
            if !self.sessions[i].is_exited() {
                continue;
            }
            let exited_for = self.sessions[i].launched_at.elapsed();
            if !self.sessions[i].exit_reported {
                self.sessions[i].exit_reported = true;
                let label = self.sessions[i].tab_label();
                match crate::container::inspect_container_exit(&self.sessions[i].docker_name) {
                    Ok(Some((exit_code, error))) => {
                        let suffix = exit_code
                            .map(|code| format!(" (exit code {code})"))
                            .unwrap_or_default();
                        if error.is_empty() {
                            self.push_log(
                                format!("{label} exited immediately{suffix}"),
                                true,
                            );
                        } else {
                            self.push_log(
                                format!("{label} exited immediately{suffix}: {error}"),
                                true,
                            );
                        }
                    }
                    Ok(None) => {
                        self.push_log(format!("{label} exited immediately"), true);
                    }
                    Err(e) => {
                        self.push_log(
                            format!("{label} exited immediately; failed to inspect exit status: {e}"),
                            true,
                        );
                    }
                }
                continue;
            }
            if exited_for < std::time::Duration::from_secs(15) {
                continue;
            }
            let label = self.sessions[i].tab_label();
            self.push_log(format!("container '{}' exited", label), false);
            let tok = self.sessions[i].session_token.clone();
            self.sessions.remove(i);
            self.session_registry.remove(&tok);
            self.remap_session_indices_after_removal(i);
            if self.active_session.is_none() && self.focus != Focus::ImageBuild {
                self.focus = Focus::Sidebar;
            }
        }

        for idx in (0..self.pending_stop.len()).rev() {
            let Some((project, container_id)) = self
                .pending_stop
                .get(idx)
                .map(|item| (item.project.clone(), item.container_id.clone()))
            else {
                continue;
            };
            let decision = self.handle_stop_request(&project, &container_id);
            if let Some(tx) = self.pending_stop[idx].response_tx.take() {
                let _ = tx.send(decision);
            }
            self.pending_stop.remove(idx);
        }

        if self.focus == Focus::Terminal {
            if let Some(si) = self.active_session {
                if let Some(session) = self.sessions.get(si) {
                    session.clear_bell();
                }
            }
        }

        let max = self.sidebar_items().len().saturating_sub(1);
        if self.sidebar_idx > max {
            self.sidebar_idx = max;
        }
    }

    fn tick_watchers(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_watch_tick) < std::time::Duration::from_secs(1) {
            return;
        }
        self.last_watch_tick = now;

        let mut active_projects = Vec::new();
        for (pi, state) in &mut self.project_watch {
            if state.enabled {
                state.spinner_phase = state.spinner_phase.wrapping_add(1);
                active_projects.push(*pi);
            }
        }

        let cfg = self.config.get();
        for pi in active_projects {
            let Some(proj) = cfg.projects.get(pi).cloned() else {
                continue;
            };
            let mode = crate::config::effective_sync_mode(&proj, &cfg.defaults);
            let workspace = crate::config::effective_workspace_path(&proj, &cfg.workspace);
            let exclude_matcher =
                match crate::sync::build_project_exclude_matcher(&proj, &cfg.defaults) {
                    Ok(matcher) => matcher,
                    Err(e) => {
                        self.push_log(
                            format!(
                                "watch skipped for '{}': cannot load excludes: {e}",
                                proj.name
                            ),
                            true,
                        );
                        continue;
                    }
                };
            let canonical_files_now = compute_tree_file_map(&proj.canonical_path, &exclude_matcher);
            let workspace_files_now = compute_tree_file_map(&workspace, &exclude_matcher);

            let (canonical_changed, workspace_changed) = match self.project_watch.get(&pi) {
                Some(state) => (
                    diff_file_maps(&state.canonical_files, &canonical_files_now),
                    diff_file_maps(&state.workspace_files, &workspace_files_now),
                ),
                None => (vec![], vec![]),
            };

            match mode {
                SyncMode::WorkspaceOnly => {}
                SyncMode::Pushback => {
                    if !workspace_changed.is_empty() {
                        self.do_pushback_files(pi, &workspace_changed);
                    }
                }
                SyncMode::Bidirectional => {
                    if !canonical_changed.is_empty() {
                        self.do_seed_files(pi, &canonical_changed);
                    }
                    if !workspace_changed.is_empty() {
                        self.do_pushback_files(pi, &workspace_changed);
                    }
                }
                SyncMode::Pullthrough => {
                    if !canonical_changed.is_empty() {
                        self.do_seed_files(pi, &canonical_changed);
                    }
                }
                SyncMode::Direct => {}
            }

            if let Some(state) = self.project_watch.get_mut(&pi) {
                state.canonical_files =
                    compute_tree_file_map(&proj.canonical_path, &exclude_matcher);
                state.workspace_files = compute_tree_file_map(&workspace, &exclude_matcher);
            }
        }
    }
}

fn shell_command_for_docker_args(args: &[String]) -> String {
    format!("docker {}", shell_words::join(args))
}

fn build_line_looks_like_error(line: &str) -> bool {
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
                if is_error && let Some(tail) = stderr_tail.as_ref() && let Ok(mut lines) = tail.lock() {
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

async fn run_build_shell_command(
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
    let diagnostic = stderr_tail
        .lock()
        .ok()
        .and_then(|lines| (!lines.is_empty()).then(|| lines.iter().cloned().collect::<Vec<_>>().join(" | ")));
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

fn compute_tree_file_map(
    root: &std::path::Path,
    exclude_matcher: &crate::sync::ExcludeMatcher,
) -> HashMap<PathBuf, FileSignature> {
    let mut map = HashMap::new();
    if !root.exists() {
        return map;
    }
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let rel = match e.path().strip_prefix(root) {
                Ok(r) => r,
                Err(_) => return true,
            };
            if rel.as_os_str().is_empty() {
                return true;
            }
            !exclude_matcher.is_excluded(rel, e.file_type().is_dir())
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        if let Ok(md) = entry.metadata() {
            let (mtime_secs, mtime_nanos) = md
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| (d.as_secs(), d.subsec_nanos()))
                .unwrap_or((0, 0));
            map.insert(
                rel.to_path_buf(),
                FileSignature {
                    size: md.len(),
                    mtime_secs,
                    mtime_nanos,
                },
            );
        }
    }
    map
}

fn diff_file_maps(
    old: &HashMap<PathBuf, FileSignature>,
    new: &HashMap<PathBuf, FileSignature>,
) -> Vec<PathBuf> {
    let mut changed = Vec::new();
    for (path, new_sig) in new {
        match old.get(path) {
            Some(old_sig) if old_sig == new_sig => {}
            _ => changed.push(path.clone()),
        }
    }
    for path in old.keys() {
        if !new.contains_key(path) {
            changed.push(path.clone());
        }
    }
    changed
}

fn host_bind_is_loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

fn docker_image_exists(image: &str) -> std::io::Result<bool> {
    let status = std::process::Command::new("docker")
        .args(["image", "inspect", image])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    Ok(status.success())
}

fn is_scroll_mode_toggle_key(key: KeyEvent) -> bool {
    (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('\u{13}') && key.modifiers.is_empty())
}

fn next_sync_mode(mode: &SyncMode) -> SyncMode {
    match mode {
        SyncMode::WorkspaceOnly => SyncMode::Pullthrough,
        SyncMode::Pullthrough => SyncMode::Pushback,
        SyncMode::Pushback => SyncMode::Bidirectional,
        SyncMode::Bidirectional => SyncMode::Direct,
        SyncMode::Direct => SyncMode::WorkspaceOnly,
    }
}

fn prev_sync_mode(mode: &SyncMode) -> SyncMode {
    match mode {
        SyncMode::WorkspaceOnly => SyncMode::Direct,
        SyncMode::Direct => SyncMode::Bidirectional,
        SyncMode::Bidirectional => SyncMode::Pushback,
        SyncMode::Pushback => SyncMode::Pullthrough,
        SyncMode::Pullthrough => SyncMode::WorkspaceOnly,
    }
}

fn oneshot_dummy() -> tokio::sync::oneshot::Sender<NetworkDecision> {
    let (tx, _) = tokio::sync::oneshot::channel();
    tx
}

// ── Key → PTY bytes (Streamlined mapping) ────────────────────────────────────

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

    loop {
        app.drain_channels();
        app.tick_watchers();
        terminal.draw(|frame| ui::render(frame, app))?;

        if app.should_quit {
            app.terminate_all_sessions();
            break;
        }

        let timeout = tokio::time::sleep(tick);

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => app.handle_key(key),
                    Some(Ok(Event::Paste(text))) => {
                        if app.focus == Focus::NewProject {
                            app.append_new_project_text(&text);
                        } else if let Some(si) = app.active_session {
                            if let Some(session) = app.sessions.get(si) {
                                session.send_input(text.into_bytes());
                            }
                        }
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        let pty_cols = cols.saturating_sub(38).max(20);
                        let pty_rows = rows.saturating_sub(10).max(6);
                        for session in &mut app.sessions {
                            let _ = session.resize(pty_rows, pty_cols);
                        }
                    }
                    None => break,
                    _ => {}
                }
            }
            _ = timeout => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{App, Focus, SidebarItem, restore_terminal_output};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use crate::ca::CaStore;
    use crate::config::Config;
    use crate::proxy::ProxyState;
    use crate::shared_config::SharedConfig;
    use crate::state::StateManager;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn restore_terminal_output_emits_reset_sequences() {
        let mut buf = Vec::new();
        restore_terminal_output(&mut buf).expect("restore commands should serialize");
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("\u{1b}[?1049l"), "missing leave alt-screen");
        assert!(out.contains("\u{1b}[?25h"), "missing show cursor");
        assert!(out.contains("\u{1b}[?1000l"), "missing disable mouse");
        assert!(out.contains("\u{1b}[?2004l"), "missing disable bracketed paste");
        assert!(out.contains("\u{1b}[?7h"), "missing enable line wrap");
        assert!(out.contains("\u{1b}[0m"), "missing reset color");
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agent-zero-{prefix}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn build_test_app() -> App {
        let root = unique_temp_dir("tui-build-flow");
        let global_rules_file = root.join("global-rules.toml");
        let workspace_root = root.join("workspace");
        let docker_dir = root.join("docker-root");
        let project_path = root.join("project-a");
        std::fs::create_dir_all(&workspace_root).expect("create workspace");
        std::fs::create_dir_all(&docker_dir).expect("create docker dir");
        std::fs::create_dir_all(&project_path).expect("create project path");

        let raw = format!(
            r#"
[manager]
global_rules_file = "{}"

[workspace]
root = "{}"

docker_dir = "{}"

[[projects]]
name = "project-a"
canonical_path = "{}"

[[containers]]
name = "test"
image = "missing-image:latest"
"#,
            global_rules_file.display(),
            workspace_root.display(),
            docker_dir.display(),
            project_path.display()
        );
        let config: Config = toml::from_str(&raw).expect("parse minimal config");
        let shared = SharedConfig::new(Arc::new(config));

        let (_exec_tx, exec_rx) = mpsc::channel(8);
        let (_stop_tx, stop_rx) = mpsc::channel(8);
        let (net_tx, net_rx) = mpsc::channel(8);
        let (_audit_tx, audit_rx) = mpsc::channel(8);

        let ca = Arc::new(CaStore::load_or_create(&root.join("ca")).expect("create CA"));
        let proxy_state = ProxyState::new(ca, shared.clone(), net_tx).expect("proxy state");
        let state = StateManager::open(&root.join("state")).expect("state manager");

        App::new(
            shared,
            root.join("config.toml"),
            "token".to_string(),
            crate::server::SessionRegistry::default(),
            exec_rx,
            stop_rx,
            net_rx,
            audit_rx,
            state,
            proxy_state,
            "127.0.0.1:0".to_string(),
            root.join("ca/ca.crt").display().to_string(),
        )
        .expect("App::new")
    }

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

#[test]
    fn build_commands_use_configured_docker_root() {
        let docker_dir = std::path::Path::new("/tmp/agent-zero-docker-root");
        let (base_cmd, agent_cmd) =
            App::build_commands_for(docker_dir, "agent-zero-codex:ubuntu-24.04");

        assert_eq!(
            base_cmd,
            vec![
                "build".to_string(),
                "-t".to_string(),
                "my-agent:ubuntu-24.04".to_string(),
                "-f".to_string(),
                "/tmp/agent-zero-docker-root/ubuntu-24.04.Dockerfile".to_string(),
                "/tmp/agent-zero-docker-root".to_string(),
            ]
        );
        assert_eq!(
            agent_cmd,
            Some(vec![
                "build".to_string(),
                "-t".to_string(),
                "agent-zero-codex:ubuntu-24.04".to_string(),
                "-f".to_string(),
                "/tmp/agent-zero-docker-root/codex/ubuntu-24.04.Dockerfile".to_string(),
                "/tmp/agent-zero-docker-root".to_string(),
            ])
        );
    }

    #[test]
    fn preflight_missing_image_opens_image_build_pane() {
        let mut app = build_test_app();
        let proceed =
            app.preflight_image_or_prompt_build(0, 0, "missing-image:latest", |_| Ok(false));
        assert!(!proceed);
        assert_eq!(app.focus, Focus::ImageBuild);
        assert_eq!(app.build_project_idx, Some(0));
        assert_eq!(app.build_container_idx, Some(0));
        assert_eq!(app.build_cursor, 0);
    }

    #[test]
    fn sidebar_selection_tracks_session_preview() {
        let mut app = build_test_app();
        let items = vec![
            SidebarItem::Project(0),
            SidebarItem::Launch(0),
            SidebarItem::Session(2),
        ];

        app.sidebar_idx = 2;
        app.update_sidebar_preview(&items);
        assert_eq!(app.preview_session, Some(2));

        app.sidebar_idx = 1;
        app.update_sidebar_preview(&items);
        assert_eq!(app.preview_session, None);
    }

    #[test]
    fn ctrl_g_toggles_terminal_fullscreen() {
        let mut app = build_test_app();
        app.focus = Focus::Terminal;
        app.active_session = Some(0);

        app.handle_terminal_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
        assert!(app.terminal_fullscreen);
        assert!(!app.log_fullscreen);

        app.handle_terminal_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
        assert!(!app.terminal_fullscreen);
    }

    #[test]
    fn double_escape_exits_terminal_fullscreen() {
        let mut app = build_test_app();
        app.focus = Focus::Terminal;
        app.active_session = Some(0);
        app.terminal_fullscreen = true;

        app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.terminal_fullscreen);

        app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.terminal_fullscreen);
    }

    #[test]
    fn double_escape_quits_when_not_fullscreen() {
        let mut app = build_test_app();
        app.focus = Focus::Terminal;
        app.active_session = Some(0);

        app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.should_quit);

        app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.should_quit);
    }

    #[test]
    fn removing_active_session_clears_terminal_fullscreen() {
        let mut app = build_test_app();
        app.active_session = Some(0);
        app.terminal_fullscreen = true;
        app.last_terminal_esc = Some(std::time::Instant::now());

        app.clear_terminal_fullscreen_for_removed_session(0);

        assert!(!app.terminal_fullscreen);
        assert!(app.last_terminal_esc.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn termios_guard_only_restores_ixon() {
        use super::disable_xon_xoff_on_fd;

        fn get_termios(fd: i32) -> libc::termios {
            unsafe {
                let mut t = std::mem::MaybeUninit::<libc::termios>::uninit();
                assert_eq!(libc::tcgetattr(fd, t.as_mut_ptr()), 0);
                t.assume_init()
            }
        }

        fn set_termios(fd: i32, t: &libc::termios) {
            unsafe {
                assert_eq!(libc::tcsetattr(fd, libc::TCSANOW, t), 0);
            }
        }

        unsafe {
            let mut master: libc::c_int = 0;
            let mut slave: libc::c_int = 0;
            assert_eq!(
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut()
                ),
                0
            );

            // Ensure IXON is enabled so we can observe disable+restore.
            let mut t0 = get_termios(slave);
            t0.c_iflag |= libc::IXON;
            set_termios(slave, &t0);

            let echo_was_enabled = (t0.c_lflag & libc::ECHO) != 0;
            let expected_echo_enabled = !echo_was_enabled;

            {
                let _guard =
                    disable_xon_xoff_on_fd(slave).expect("guard should be created for PTY");
                let t_mid = get_termios(slave);
                assert_eq!((t_mid.c_iflag & libc::IXON) != 0, false);

                // Mutate an unrelated bit while guard is alive; the guard must not
                // overwrite it on drop.
                let mut t1 = t_mid;
                if echo_was_enabled {
                    t1.c_lflag &= !libc::ECHO;
                } else {
                    t1.c_lflag |= libc::ECHO;
                }
                set_termios(slave, &t1);
            }

            let t_after = get_termios(slave);
            assert_eq!((t_after.c_iflag & libc::IXON) != 0, true);
            assert_eq!(
                (t_after.c_lflag & libc::ECHO) != 0,
                expected_echo_enabled,
                "TermiosGuard must not restore unrelated flags like ECHO"
            );

            let _ = libc::close(master);
            let _ = libc::close(slave);
        }
    }

    #[test]
    fn sidebar_navigation_wraps_and_scrolls() {
        let mut app = build_test_app();
        // build_test_app only adds 1 project ("project-a")
        // sidebar_items() should return [Project(0), Launch(0), Settings(0), NewProject]
        
        // Project rows are section headers: they render, but can't be selected/highlighted.
        app.sidebar_idx = 0;
        
        // Down -> Launch(0)
        app.handle_sidebar_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.sidebar_idx, 1);
        
        // Up -> Wrap to NewProject (index 3), skipping Project(0)
        app.handle_sidebar_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.sidebar_idx, 3);
        
        // Up -> Settings(0)
        app.handle_sidebar_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.sidebar_idx, 2);
        
        // Down -> NewProject
        app.handle_sidebar_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.sidebar_idx, 3);
    }
}
