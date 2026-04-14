use super::*;

impl App {
    pub(crate) fn refresh_projects_cache(&mut self) {
        let cfg = self.config.get();
        self.workspaces = cfg
            .workspaces
            .iter()
            .map(|p| WorkspaceStatus {
                name: p.name.clone(),
            })
            .collect();
    }

    pub(crate) fn settings_action_rows_for() -> Vec<SettingsActionRow> {
        vec![
            SettingsActionRow {
                key: 'r',
                label: "Reload rules".to_string(),
                desc: "Rescan and reload void-rules.toml for this workspace.",
                action: SettingsAction::ReloadRules,
            },
            SettingsActionRow {
                key: 'x',
                label: "Remove workspace".to_string(),
                desc: "Remove from config and stop any running containers in this workspace.",
                action: SettingsAction::RemoveWorkspace,
            },
        ]
    }

    pub(crate) fn settings_action_rows(&self, project_idx: usize) -> Vec<SettingsActionRow> {
        let cfg = self.config.get();
        let Some(proj) = cfg.workspaces.get(project_idx) else {
            return Vec::new();
        };
        let _ = proj;
        Self::settings_action_rows_for()
    }

    pub(crate) fn handle_settings_key(&mut self, key: KeyEvent) {
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
            KeyCode::Char('x') | KeyCode::Char('X') => self.prompt_remove_workspace(pi),
            _ => {}
        }
    }

    pub(crate) fn run_settings_action(&mut self, pi: usize) {
        let actions = self.settings_action_rows(pi);
        let Some(row) = actions.get(self.settings_cursor) else {
            return;
        };
        match row.action {
            SettingsAction::ReloadRules => self.do_reload_rules(pi),
            SettingsAction::RemoveWorkspace => self.prompt_remove_workspace(pi),
        }
    }

    pub(crate) fn do_reload_rules(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(proj) = cfg.workspaces.get(pi) else {
            return;
        };
        let proj = proj.clone();
        self.log_project_rules_status(&proj);
    }

    pub(crate) fn prompt_remove_workspace(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(workspace) = cfg.workspaces.get(pi) else {
            return;
        };
        self.remove_workspace_confirm = Some(RemoveWorkspaceConfirmState {
            workspace_name: workspace.name.clone(),
        });
    }

    pub(crate) fn finish_remove_workspace_confirm(&mut self, confirmed: bool) {
        let Some(state) = self.remove_workspace_confirm.take() else {
            return;
        };
        if !confirmed {
            self.push_log(
                format!("workspace removal cancelled: '{}'", state.workspace_name),
                false,
            );
            return;
        }

        for idx in (0..self.sessions.len()).rev() {
            if self.sessions[idx].project == state.workspace_name {
                self.close_session(idx);
            }
        }

        match crate::new_project::remove_workspace_block(
            &self.loaded_config_path,
            &state.workspace_name,
        ) {
            Ok(false) => {
                self.push_log(
                    format!(
                        "workspace '{}' was not found in config; nothing removed",
                        state.workspace_name
                    ),
                    true,
                );
            }
            Ok(true) => {
                let new_config = match crate::config::load(&self.loaded_config_path) {
                    Ok(c) => c,
                    Err(e) => {
                        self.push_log(
                            format!(
                                "workspace '{}' removed, but failed to reload config: {}",
                                state.workspace_name, e
                            ),
                            true,
                        );
                        return;
                    }
                };
                self.config.set(std::sync::Arc::new(new_config));
                self.refresh_projects_cache();
                self.pending_exec.retain(|item| item.project != state.workspace_name);
                self.pending_stop.retain(|item| item.project != state.workspace_name);
                self.pending_net
                    .retain(|item| item.source_project.as_deref() != Some(&state.workspace_name));
                self.active_settings_project = None;
                self.focus = Focus::Sidebar;
                self.settings_cursor = 0;
                let items = self.sidebar_items();
                self.sidebar_idx = Self::first_selectable_sidebar_idx(&items);
                self.update_sidebar_preview(&items);
                self.push_log(format!("removed workspace '{}'", state.workspace_name), false);
            }
            Err(e) => {
                self.push_log(
                    format!(
                        "failed removing workspace '{}' from config: {}",
                        state.workspace_name, e
                    ),
                    true,
                );
            }
        }
    }

    pub(crate) fn handle_terminal_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_scroll_mode_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn exit_scroll_mode(&mut self) {
        self.scroll_mode = false;
        self.terminal_scroll = 0;
    }

    pub(crate) fn handle_build_scroll_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn open_picker(&mut self) {
        let cfg = self.config.get();
        if cfg.containers.is_empty() {
            self.push_log("no containers defined in config", true);
            return;
        }
        self.container_picker = Some(0);
        self.focus = Focus::ContainerPicker;
    }

    pub(crate) fn handle_picker_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_build_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn run_build_action(&mut self) {
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
            self.push_log("cannot start build: no workspace selected", true);
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
}
