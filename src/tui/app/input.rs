use super::*;

impl App {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.focus == Focus::Activity
                && let Some(id) = self.active_activity.clone()
            {
                self.cancel_activity(&id);
                return;
            }

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

        if self.base_rules_changed.is_some() {
            match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('y') | KeyCode::Char('n') => {
                    self.base_rules_changed = None;
                }
                _ => {}
            }
            return;
        }

        if self.remove_workspace_confirm.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => self.finish_remove_workspace_confirm(true),
                KeyCode::Char('n') | KeyCode::Esc => self.finish_remove_workspace_confirm(false),
                _ => {}
            }
            return;
        }

        if self.focus == Focus::Activity
            && (key.code == KeyCode::Esc
                || (key.code == KeyCode::Char('b')
                    && key.modifiers.contains(KeyModifiers::CONTROL)))
        {
            self.focus_sidebar_shortcut();
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
            Focus::Activity => self.handle_activity_key(key),
            Focus::Settings => self.handle_settings_key(key),
            Focus::ContainerPicker => self.handle_picker_key(key),
            Focus::ImageBuild => self.handle_build_key(key),
            Focus::NewWorkspace => self.handle_new_project_key(key),
        }
    }

    pub(crate) fn focus_sidebar_shortcut(&mut self) {
        self.last_terminal_esc = None;
        self.log_fullscreen = false;
        self.terminal_fullscreen = false;
        match self.focus {
            Focus::Sidebar => {}
            Focus::Terminal => {
                self.focus = Focus::Sidebar;
            }
            Focus::Activity => {
                self.active_activity = None;
                self.focus = Focus::Sidebar;
            }
            Focus::Settings => {
                self.remove_workspace_confirm = None;
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
            Focus::NewWorkspace => {
                self.new_project = None;
                self.focus = Focus::Sidebar;
            }
        }
        let items = self.sidebar_items();
        self.update_sidebar_preview(&items);
    }

    pub(crate) fn open_log_fullscreen(&mut self) {
        self.terminal_fullscreen = false;
        self.log_fullscreen = true;
    }

    pub(crate) fn open_terminal_fullscreen(&mut self) {
        self.log_fullscreen = false;
        self.terminal_fullscreen = true;
        self.last_terminal_esc = None;
    }

    pub(crate) fn close_terminal_fullscreen(&mut self) {
        self.terminal_fullscreen = false;
        self.last_terminal_esc = None;
    }

    pub(crate) fn handle_sidebar_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn sidebar_move_up(&mut self, items: &[SidebarItem]) {
        self.sidebar_move_to_next_selectable(items, -1);
        self.update_sidebar_preview(items);
        self.ensure_sidebar_visible(items, 10); // Default height
    }

    pub(crate) fn sidebar_move_down(&mut self, items: &[SidebarItem]) {
        self.sidebar_move_to_next_selectable(items, 1);
        self.update_sidebar_preview(items);
        self.ensure_sidebar_visible(items, 10); // Default height
    }

    pub(crate) fn sidebar_move_to_next_selectable(&mut self, items: &[SidebarItem], dir: i8) {
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

    pub(crate) fn ensure_sidebar_visible(&mut self, items: &[SidebarItem], visible_height: usize) {
        if items.is_empty() || visible_height == 0 {
            return;
        }
        if self.sidebar_idx < self.sidebar_offset {
            self.sidebar_offset = self.sidebar_idx;
        } else if self.sidebar_idx >= self.sidebar_offset + visible_height {
            self.sidebar_offset = self.sidebar_idx - visible_height + 1;
        }
    }

    pub(crate) fn update_sidebar_preview(&mut self, items: &[SidebarItem]) {
        self.preview_session = match items.get(self.sidebar_idx) {
            Some(SidebarItem::Session(si)) => Some(*si),
            Some(SidebarItem::Activity(id)) => self.session_for_activity(id),
            _ => None,
        };
    }

    pub(crate) fn handle_sidebar_enter(&mut self, items: &[SidebarItem]) {
        match items.get(self.sidebar_idx).cloned() {
            Some(SidebarItem::Workspace(_)) => {
                // do nothing
            }
            Some(SidebarItem::Settings(pi)) => {
                self.active_settings_project = Some(pi);
                self.active_activity = None;
                self.settings_cursor = 0;
                self.focus = Focus::Settings;
            }
            Some(SidebarItem::Launch(_)) => {
                self.active_activity = None;
                self.open_picker();
            }
            Some(SidebarItem::Build(_)) => {
                self.active_session = None;
                self.active_activity = None;
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
                self.scroll_mouse_passthrough = false;
                self.terminal_scroll = 0;
                self.focus = Focus::Terminal;
                self.active_activity = None;
                self.active_settings_project = None;
            }
            Some(SidebarItem::Activity(id)) => {
                self.active_session = self.session_for_activity(&id);
                self.preview_session = self.active_session;
                self.active_activity = Some(id);
                self.focus = Focus::Activity;
                self.active_settings_project = None;
            }
            Some(SidebarItem::NewWorkspace) => self.open_new_project(),
            None => {}
        }
    }

    pub(crate) fn handle_activity_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.focus_sidebar_shortcut(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(id) = self.active_activity.clone() {
                    self.cancel_activity(&id);
                }
            }
            _ => {}
        }
    }

    const NEW_PROJECT_ROW_COUNT: usize = 5;

    pub(crate) fn open_new_project(&mut self) {
        self.new_project = Some(NewWorkspaceState {
            cursor: 0,
            name: String::new(),
            workspace_dir: String::new(),
            project_type: crate::new_project::ProjectType::None,
            error: None,
        });
        self.focus = Focus::NewWorkspace;
        self.active_session = None;
        self.active_activity = None;
        self.active_settings_project = None;
        self.container_picker = None;
    }

    pub(crate) fn handle_new_project_key(&mut self, key: KeyEvent) {
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
            KeyCode::Left => {
                if state.cursor == 2 {
                    state.project_type = state.project_type.prev();
                }
            }
            KeyCode::Right => {
                if state.cursor == 2 {
                    state.project_type = state.project_type.next();
                }
            }
            KeyCode::Backspace => match state.cursor {
                0 => {
                    state.name.pop();
                }
                1 => {
                    state.workspace_dir.pop();
                }
                _ => {}
            },
            KeyCode::Enter => match state.cursor {
                2 => state.project_type = state.project_type.next(),
                3 => self.submit_new_project(),
                4 => {
                    self.new_project = None;
                    self.focus = Focus::Sidebar;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub(crate) fn append_new_project_text(&mut self, text: &str) {
        let Some(state) = self.new_project.as_mut() else {
            return;
        };
        let cleaned = text.replace(['\r', '\n'], "");
        if cleaned.is_empty() {
            return;
        }
        match state.cursor {
            0 => state.name.push_str(&cleaned),
            1 => state.workspace_dir.push_str(&cleaned),
            _ => {}
        }
    }

    pub(crate) fn submit_new_project(&mut self) {
        let Some((name, workspace_raw, project_type)) = self.new_project.as_mut().map(|state| {
            state.error = None;
            (
                state.name.trim().to_string(),
                state.workspace_dir.trim().to_string(),
                state.project_type,
            )
        }) else {
            return;
        };

        if name.is_empty() {
            self.set_new_project_error("workspace name is required".to_string());
            return;
        }
        if workspace_raw.is_empty() {
            self.set_new_project_error("workspace dir is required".to_string());
            return;
        }

        let workspace_path = match crate::config::expand_path(std::path::Path::new(&workspace_raw))
        {
            Ok(p) => p,
            Err(e) => {
                self.set_new_project_error(format!("workspace dir is invalid: {e}"));
                return;
            }
        };
        if !workspace_path.exists() {
            self.set_new_project_error(format!(
                "workspace dir does not exist: {}",
                workspace_path.display()
            ));
            return;
        }
        if !workspace_path.is_dir() {
            self.set_new_project_error(format!(
                "workspace dir is not a directory: {}",
                workspace_path.display()
            ));
            return;
        }

        let cfg = self.config.get();
        if cfg.workspaces.iter().any(|p| p.name == name) {
            self.set_new_project_error(format!("workspace name already exists: '{name}'"));
            return;
        }

        match crate::new_project::write_rules_if_missing(&workspace_path, project_type) {
            Ok(false) => {}
            Ok(true) => self.push_log(
                format!(
                    "created {}",
                    workspace_path.join("harness-rules.toml").display()
                ),
                false,
            ),
            Err(e) => {
                self.set_new_project_error(format!("failed writing harness-rules.toml: {e}"));
                return;
            }
        };

        if let Err(e) = crate::new_project::append_project_block(
            &self.loaded_config_path,
            &name,
            &workspace_path,
            crate::config::SyncMode::Direct,
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
        let new_pi = new_config.workspaces.iter().position(|p| p.name == name);
        self.config.set(std::sync::Arc::new(new_config));
        self.refresh_projects_cache();

        self.push_log(format!("added workspace '{name}'"), false);
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

    pub(crate) fn set_new_project_error(&mut self, msg: String) {
        if let Some(state) = self.new_project.as_mut() {
            state.error = Some(msg);
        }
    }
}
