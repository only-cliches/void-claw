use super::*;

impl App {
    pub(crate) fn refresh_projects_cache(&mut self) {
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

    pub(crate) fn settings_action_rows_for(
        mode: SyncMode,
        watching: bool,
    ) -> Vec<SettingsActionRow> {
        if mode == SyncMode::Direct {
            return vec![SettingsActionRow {
                key: 'r',
                label: "Reload rules".to_string(),
                desc: "Rescan and reload void-rules.toml for this project.",
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
                desc: "Rescan and reload void-rules.toml for this project.",
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

    pub(crate) fn settings_action_rows(&self, project_idx: usize) -> Vec<SettingsActionRow> {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(project_idx) else {
            return Vec::new();
        };
        let mode = crate::config::effective_sync_mode(proj, &cfg.defaults);
        let watching = self.is_project_watching(project_idx);
        Self::settings_action_rows_for(mode, watching)
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

    pub(crate) fn run_settings_action(&mut self, pi: usize) {
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

    pub(crate) fn do_reload_rules(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        let proj = proj.clone();
        self.log_project_rules_status(&proj);
    }

    pub(crate) fn do_clear_workspace(&mut self, pi: usize) {
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
}
