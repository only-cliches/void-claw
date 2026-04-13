use super::*;

impl App {
    pub(crate) fn do_launch_container_on_project(&mut self, pi: usize, ctr_idx: usize) {
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
                    "created starter void-rules.toml in '{}'",
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

        let gemini_home_host_path: Option<&std::path::Path> =
            if ctr.agent == crate::config::AgentKind::Gemini && !has_host_gemini_state_mount {
                Some(gemini_home_dir.as_path())
            } else {
                None
            };

        #[cfg(target_os = "macos")]
        if cfg.defaults.proxy.strict_network {
            self.push_log(
                "strict_network on macOS requires Docker `--privileged`; void-claw applies it automatically for this container launch",
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
}
