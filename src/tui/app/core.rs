use super::*;

impl App {
    fn watched_rules_paths(cfg: &crate::config::Config) -> Vec<PathBuf> {
        let mut paths = Vec::with_capacity(cfg.workspaces.len());
        for workspace in &cfg.workspaces {
            paths.push(workspace.canonical_path.join("harness-rules.toml"));
        }
        paths.sort();
        paths.dedup();
        paths
    }

    fn content_hash_for_path(path: &Path) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match std::fs::read(path) {
            Ok(bytes) => bytes.hash(&mut hasher),
            Err(_) => 0u8.hash(&mut hasher),
        }
        hasher.finish()
    }

    pub(crate) fn watched_file_stamp(path: &Path) -> WatchedFileStamp {
        match std::fs::metadata(path) {
            Ok(md) => {
                let (mtime_secs, mtime_nanos) = md
                    .modified()
                    .ok()
                    .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| (d.as_secs(), d.subsec_nanos()))
                    .unwrap_or((0, 0));
                WatchedFileStamp {
                    exists: true,
                    size: md.len(),
                    mtime_secs,
                    mtime_nanos,
                    content_hash: Self::content_hash_for_path(path),
                }
            }
            Err(_) => WatchedFileStamp {
                exists: false,
                size: 0,
                mtime_secs: 0,
                mtime_nanos: 0,
                content_hash: 0,
            },
        }
    }

    pub(crate) fn sidebar_item_is_selectable(item: &SidebarItem) -> bool {
        !matches!(item, SidebarItem::Workspace(_))
    }

    pub(crate) fn first_selectable_sidebar_idx(items: &[SidebarItem]) -> usize {
        items
            .iter()
            .position(Self::sidebar_item_is_selectable)
            .unwrap_or(0)
    }

    pub(crate) fn selected_sidebar_item_from(&self, items: &[SidebarItem]) -> Option<SidebarItem> {
        items.get(self.sidebar_idx).cloned()
    }

    pub(crate) fn restore_sidebar_selection(
        &mut self,
        selected: Option<&SidebarItem>,
        items: &[SidebarItem],
    ) {
        if items.is_empty() {
            self.sidebar_idx = 0;
            self.sidebar_offset = 0;
            self.preview_session = None;
            return;
        }

        if let Some(selected) = selected
            && let Some(idx) = items.iter().position(|item| item == selected)
        {
            self.sidebar_idx = idx;
            self.update_sidebar_preview(items);
            return;
        }

        self.sidebar_idx = self.sidebar_idx.min(items.len().saturating_sub(1));
        if !Self::sidebar_item_is_selectable(&items[self.sidebar_idx]) {
            self.sidebar_idx = Self::first_selectable_sidebar_idx(items);
        }
        self.update_sidebar_preview(items);
    }

    pub fn new(
        config: SharedConfig,
        loaded_config_path: PathBuf,
        token: String,
        session_registry: SessionRegistry,
        exec_pending_rx: mpsc::Receiver<PendingItem>,
        stop_pending_rx: mpsc::Receiver<ContainerStopItem>,
        net_pending_rx: mpsc::Receiver<PendingNetworkItem>,
        activity_rx: mpsc::UnboundedReceiver<ActivityEvent>,
        audit_rx: mpsc::Receiver<AuditEntry>,
        state: StateManager,
        proxy_state: ProxyState,
        _proxy_addr: String,
        ca_cert_path: String,
    ) -> Result<Self> {
        let cfg = config.get();
        let watched_rules_stamps = Self::watched_rules_paths(&cfg)
            .into_iter()
            .map(|path| {
                let stamp = Self::watched_file_stamp(&path);
                (path, stamp)
            })
            .collect::<std::collections::HashMap<_, _>>();

        let workspaces = cfg
            .workspaces
            .iter()
            .map(|p| WorkspaceStatus {
                name: p.name.clone(),
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
            .map(|r| {
                (
                    r.hostdo.commands.len(),
                    r.network.allowlist.len() + r.network.denylist.len(),
                )
            })
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
            workspaces,
            pending_exec: vec![],
            pending_stop: vec![],
            pending_net: vec![],
            activities: vec![],
            log,
            log_scroll: 0,
            focus: Focus::Sidebar,
            sidebar_idx: Self::first_selectable_sidebar_idx(
                &cfg.workspaces
                    .iter()
                    .enumerate()
                    .flat_map(|(pi, _)| {
                        [
                            SidebarItem::Workspace(pi),
                            SidebarItem::Launch(pi),
                            SidebarItem::Settings(pi),
                        ]
                    })
                    .chain(std::iter::once(SidebarItem::NewWorkspace))
                    .collect::<Vec<_>>(),
            ),
            sidebar_offset: 0,
            active_session: None,
            active_activity: None,
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
            remove_workspace_confirm: None,
            base_rules_changed: None,
            exec_pending_rx,
            stop_pending_rx,
            net_pending_rx,
            activity_rx,
            audit_rx,
            build_event_rx,
            build_event_tx,
            build_task: None,
            should_quit: false,
            passthrough_mode: false,
            passthrough_exit_code_slot: None,
            log_fullscreen: false,
            terminal_fullscreen: false,
            ctrl_c_times: Vec::new(),
            last_terminal_esc: None,
            scroll_mode: false,
            scroll_mouse_passthrough: false,
            terminal_scroll: 0,
            last_base_rules_poll: std::time::Instant::now(),
            watched_rules_stamps,
            pending_base_rules_internal_write: std::collections::HashMap::new(),
        })
    }

    pub fn enable_passthrough_mode(&mut self, exit_code_slot: Arc<std::sync::atomic::AtomicI32>) {
        self.passthrough_mode = true;
        self.passthrough_exit_code_slot = Some(exit_code_slot);
    }

    pub(crate) fn tick_base_rules_file_watch(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_base_rules_poll) < std::time::Duration::from_millis(750) {
            return;
        }
        self.last_base_rules_poll = now;

        let cfg = self.config.get();
        let watched_paths = Self::watched_rules_paths(&cfg);
        let now = std::time::Instant::now();

        self.watched_rules_stamps
            .retain(|path, _| watched_paths.iter().any(|watched| watched == path));
        self.pending_base_rules_internal_write
            .retain(|path, pending| {
                watched_paths.iter().any(|watched| watched == path) && now <= pending.expires_at
            });
        for path in &watched_paths {
            self.watched_rules_stamps
                .entry(path.clone())
                .or_insert_with(|| Self::watched_file_stamp(path));
        }

        for path in watched_paths {
            let current_stamp = Self::watched_file_stamp(&path);
            let prev = self
                .watched_rules_stamps
                .entry(path.clone())
                .or_insert_with(|| current_stamp.clone());
            if current_stamp == *prev {
                continue;
            }
            *prev = current_stamp;

            if let Some(pending) = self.pending_base_rules_internal_write.get(&path).cloned() {
                if now <= pending.expires_at {
                    let current = std::fs::read_to_string(&path).unwrap_or_default();
                    if current == pending.expected_content {
                        self.pending_base_rules_internal_write.remove(&path);
                        continue;
                    }
                }
                self.pending_base_rules_internal_write.remove(&path);
            }

            if self.base_rules_changed.is_none() {
                self.base_rules_changed = Some(BaseRulesChangedState { path: path.clone() });
            }
            self.push_log(
                format!(
                    "SECURITY ALERT: rules file changed outside CLI: {}",
                    path.display()
                ),
                true,
            );
        }
    }

    pub(crate) fn note_rules_internal_write(&mut self, path: PathBuf, expected_content: String) {
        self.pending_base_rules_internal_write.insert(
            path,
            PendingBaseRulesInternalWrite {
                expected_content,
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(2),
            },
        );
    }

    pub(crate) fn record_completed_rules_internal_write(
        &mut self,
        path: PathBuf,
        expected_content: String,
    ) {
        match std::fs::read_to_string(&path) {
            Ok(current) if current == expected_content => {
                self.pending_base_rules_internal_write.remove(&path);
                self.watched_rules_stamps
                    .insert(path.clone(), Self::watched_file_stamp(&path));
            }
            _ => self.note_rules_internal_write(path, expected_content),
        }
    }

    #[cfg(test)]
    pub(crate) fn note_base_rules_internal_write(&mut self, expected_content: String) {
        let path = self.config.get().manager.global_rules_file.clone();
        self.note_rules_internal_write(path, expected_content);
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let cfg = self.config.get();
        let mut items = Vec::new();
        for (pi, proj) in cfg.workspaces.iter().enumerate() {
            items.push(SidebarItem::Workspace(pi));
            for (si, session) in self.sessions.iter().enumerate() {
                if session.project == proj.name {
                    items.push(SidebarItem::Session(si));
                    for activity in self.activities_for_session(si) {
                        items.push(SidebarItem::Activity(activity.id.clone()));
                    }
                }
            }
            if self.build_project_idx == Some(pi) && self.build_is_running() {
                items.push(SidebarItem::Build(pi));
            }
            items.push(SidebarItem::Launch(pi));
            items.push(SidebarItem::Settings(pi));
        }
        items.push(SidebarItem::NewWorkspace);
        items
    }

    pub fn selected_project_idx(&self) -> Option<usize> {
        match self.sidebar_items().get(self.sidebar_idx) {
            Some(SidebarItem::Workspace(pi)) => Some(*pi),
            Some(SidebarItem::Session(si)) => {
                let cfg = self.config.get();
                let name = self.sessions.get(*si)?.project.as_str();
                cfg.workspaces.iter().position(|p| p.name == name)
            }
            Some(SidebarItem::Activity(id)) => {
                let cfg = self.config.get();
                let project = self.activity_by_id(id)?.project.as_str();
                cfg.workspaces.iter().position(|p| p.name == project)
            }
            Some(SidebarItem::Settings(pi)) => Some(*pi),
            Some(SidebarItem::Launch(pi)) => Some(*pi),
            Some(SidebarItem::Build(pi)) => Some(*pi),
            Some(SidebarItem::NewWorkspace) => None,
            None => None,
        }
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

    pub(crate) fn activity_by_id(&self, id: &str) -> Option<&Activity> {
        self.activities.iter().find(|activity| activity.id == id)
    }

    pub(crate) fn activity_by_id_mut(&mut self, id: &str) -> Option<&mut Activity> {
        self.activities
            .iter_mut()
            .find(|activity| activity.id == id)
    }

    pub(crate) fn activities_for_session(&self, session_idx: usize) -> Vec<&Activity> {
        let Some(session) = self.sessions.get(session_idx) else {
            return vec![];
        };
        self.activities
            .iter()
            .filter(|activity| {
                activity.project == session.project
                    && activity.container.as_deref().is_some_and(|container| {
                        Self::container_matches_session(container, session)
                    })
            })
            .collect()
    }

    pub(crate) fn session_for_activity(&self, id: &str) -> Option<usize> {
        let activity = self.activity_by_id(id)?;
        self.sessions.iter().position(|session| {
            activity.project == session.project
                && activity
                    .container
                    .as_deref()
                    .is_some_and(|container| Self::container_matches_session(container, session))
        })
    }

    fn container_matches_session(container: &str, session: &ContainerSession) -> bool {
        Self::container_identity_matches(
            container,
            &session.container_id,
            &session.container_name,
            &session.docker_name,
        )
    }

    pub(crate) fn container_identity_matches(
        container: &str,
        container_id: &str,
        container_name: &str,
        docker_name: &str,
    ) -> bool {
        let normalized = container.trim();
        !normalized.is_empty()
            && ((!container_id.is_empty()
                && (container_id == normalized
                    || container_id.starts_with(normalized)
                    || normalized.starts_with(container_id)))
                || container_name == normalized
                || docker_name == normalized)
    }

    pub(crate) fn apply_activity_event(&mut self, event: ActivityEvent) {
        match event {
            ActivityEvent::Started(activity) => {
                if let Some(existing) = self.activity_by_id_mut(&activity.id) {
                    *existing = activity;
                } else {
                    self.activities.push(activity);
                }
            }
            ActivityEvent::State { id, state, status } => {
                if let Some(activity) = self.activity_by_id_mut(&id) {
                    activity.state = state;
                    activity.status = status;
                    activity.updated_at = std::time::Instant::now();
                    if activity.state == crate::activity::ActivityState::Running {
                        activity.mark_command_started(activity.updated_at);
                    } else if matches!(
                        activity.state,
                        crate::activity::ActivityState::PendingApproval
                            | crate::activity::ActivityState::PullingImage
                    ) {
                        activity.clear_command_timing();
                    }
                    if activity.state.is_terminal() {
                        activity.finished_at.get_or_insert(activity.updated_at);
                        activity.mark_command_finished(activity.updated_at);
                    } else {
                        activity.finished_at = None;
                        activity.terminal_unselected_at = None;
                    }
                }
            }
            ActivityEvent::Line { id, line } => {
                if let Some(activity) = self.activity_by_id_mut(&id) {
                    activity.push_line(line);
                }
            }
            ActivityEvent::Finished { id, state, status } => {
                if let Some(activity) = self.activity_by_id_mut(&id) {
                    activity.state = state;
                    activity.status = status;
                    activity.updated_at = std::time::Instant::now();
                    activity.finished_at = Some(activity.updated_at);
                    activity.mark_command_finished(activity.updated_at);
                    activity.terminal_unselected_at = None;
                }
            }
        }
    }

    pub(crate) fn cancel_activity(&mut self, id: &str) {
        if let Some(activity) = self.activity_by_id(id) {
            activity.request_cancel();
        }
        if let Some(idx) = self
            .pending_exec
            .iter()
            .position(|item| item.activity_id == id)
        {
            self.deny_exec(idx);
        }
        if let Some(idx) = self
            .pending_net
            .iter()
            .position(|item| item.activity_id == id)
        {
            self.deny_net(idx);
        }
        if let Some(activity) = self.activity_by_id_mut(id) {
            activity.state = crate::activity::ActivityState::Cancelled;
            activity.status = Some("cancel requested".to_string());
            activity.updated_at = std::time::Instant::now();
            activity.finished_at = Some(activity.updated_at);
            activity.mark_command_finished(activity.updated_at);
            activity.terminal_unselected_at = None;
        }
    }

    pub(crate) fn refresh_terminal_activity_selection(&mut self, items: &[SidebarItem]) {
        let now = std::time::Instant::now();
        let active_activity = self.active_activity.clone();
        let sidebar_activity = items.get(self.sidebar_idx).and_then(|item| match item {
            SidebarItem::Activity(id) => Some(id.clone()),
            _ => None,
        });
        for activity in &mut self.activities {
            if !activity.state.is_terminal() {
                activity.terminal_unselected_at = None;
                continue;
            }
            let is_selected = active_activity.as_deref() == Some(activity.id.as_str())
                || sidebar_activity.as_deref() == Some(activity.id.as_str());
            if is_selected {
                activity.terminal_unselected_at = None;
            } else if activity.terminal_unselected_at.is_none() {
                activity.terminal_unselected_at = Some(now);
            }
        }
    }

    pub(crate) fn prune_terminal_activities(&mut self) {
        let now = std::time::Instant::now();
        let ttl = std::time::Duration::from_secs(crate::activity::ACTIVITY_TERMINAL_TTL_SECS);
        let items = self.sidebar_items();
        self.refresh_terminal_activity_selection(&items);
        self.activities.retain(|activity| {
            !activity.state.is_terminal()
                || activity
                    .terminal_unselected_at
                    .map(|unselected_at| now.duration_since(unselected_at) < ttl)
                    .unwrap_or(true)
        });
    }

    pub(crate) fn active_exec_modal_idx(&self) -> Option<usize> {
        let si = self.active_session?;
        self.pending_for_session(si).into_iter().next()
    }

    pub(crate) fn session_is_loading(&self, session_idx: usize) -> bool {
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

    pub(crate) fn close_session(&mut self, idx: usize) {
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

    pub(crate) fn clear_terminal_fullscreen_for_removed_session(&mut self, removed_idx: usize) {
        if self.active_session == Some(removed_idx) {
            self.terminal_fullscreen = false;
            self.last_terminal_esc = None;
        }
    }

    pub(crate) fn remap_session_indices_after_removal(&mut self, removed_idx: usize) {
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

    pub(crate) fn terminate_all_sessions(&mut self) {
        for session in &self.sessions {
            if !session.is_exited() {
                session.terminate();
            }
        }
    }

    pub(crate) fn handle_stop_request(
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
                    "killme request for workspace '{}' could not find container {}",
                    project, normalized
                ),
                true,
            );
            return ContainerStopDecision::NotFound;
        };

        let label = self.sessions[idx].tab_label();
        if self.sessions[idx].is_exited() {
            self.push_log(
                format!(
                    "killme request for '{}' ignored; container already exited",
                    label
                ),
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

    pub(crate) fn push_log(&mut self, text: impl Into<String>, is_error: bool) {
        self.log.push_front(LogEntry::Msg {
            text: text.into(),
            is_error,
            timestamp: chrono::Utc::now(),
        });
        if self.log.len() > 500 {
            self.log.pop_back();
        }
    }

    pub(crate) fn log_project_rules_status(&mut self, project: &crate::config::WorkspaceConfig) {
        let rules_path = project.canonical_path.join("harness-rules.toml");
        if !rules_path.exists() {
            self.push_log(
                format!(
                    "Searched for rules at {} but harness-rules.toml was not found",
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
                    r.network.allowlist.len() + r.network.denylist.len()
                ),
                false,
            ),
            Err(e) => self.push_log(
                format!("Failed loading rules from {}: {}", rules_path.display(), e),
                true,
            ),
        }
    }
}
