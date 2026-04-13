use super::*;

impl App {
    pub(crate) fn drain_channels(&mut self) {
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
                            self.push_log(format!("{label} exited immediately{suffix}"), true);
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
                            format!(
                                "{label} exited immediately; failed to inspect exit status: {e}"
                            ),
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

    pub(crate) fn tick_watchers(&mut self) {
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

    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.focus != Focus::Terminal {
            return;
        }
        if self.active_exec_modal_idx().is_some() || !self.pending_net.is_empty() {
            return;
        }
        let Some(si) = self.active_session else {
            return;
        };
        let Some(session) = self.sessions.get(si) else {
            return;
        };

        match mouse.kind {
            MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => {
                // If the inner terminal app requested SGR mouse reporting, prefer forwarding
                // scroll events so internal scrollbars (e.g. OpenCode) work.
                if !self.scroll_mode {
                    if let Some(bytes) = maybe_encode_sgr_mouse_for_session(session, mouse) {
                        session.send_input(bytes);
                        return;
                    }
                }

                // Otherwise, treat the scroll wheel as a viewport scroll gesture, without
                // requiring explicit scroll-mode activation.
                let max_scrollback = session.term.lock().history_size();
                let lines_per_tick = 3usize;
                match mouse.kind {
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollLeft => {
                        self.terminal_scroll = self.terminal_scroll.saturating_add(lines_per_tick);
                    }
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollRight => {
                        self.terminal_scroll = self.terminal_scroll.saturating_sub(lines_per_tick);
                    }
                    _ => {}
                }
                self.terminal_scroll = self.terminal_scroll.min(max_scrollback);
                self.scroll_mode = self.terminal_scroll > 0;
            }
            _ => {
                // When the user is scrolling the outer viewport, don't forward clicks/drags into
                // the PTY (it would be surprising and could trigger actions in the inner app).
                if self.scroll_mode {
                    return;
                }
                if let Some(bytes) = maybe_encode_sgr_mouse_for_session(session, mouse) {
                    session.send_input(bytes);
                }
            }
        }
    }
}
