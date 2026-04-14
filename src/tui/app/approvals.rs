use super::*;

impl App {
    pub(crate) fn approve_exec(&mut self, idx: usize, remember: bool) {
        if idx >= self.pending_exec.len() {
            return;
        }
        if remember {
            let item = &self.pending_exec[idx];
            let argv = item.argv.clone();
            let project_name = item.project.clone();
            let cwd = self.portable_cwd(&item.rule_cwd, &project_name);
            if let Some(rules_path) = self.project_rules_path(&project_name) {
                match self.persist_exec_rule(
                    &rules_path,
                    &argv,
                    &cwd,
                    crate::rules::ApprovalMode::Auto,
                ) {
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
                    format!("Cannot remember: unknown workspace '{project_name}'"),
                    true,
                );
            }
        }
        if let Some(tx) = self.pending_exec[idx].response_tx.take() {
            let _ = tx.send(ApprovalDecision::Approve { remember });
        }
        self.pending_exec.remove(idx);
    }

    pub(crate) fn deny_exec(&mut self, idx: usize) {
        if idx >= self.pending_exec.len() {
            return;
        }
        if let Some(tx) = self.pending_exec[idx].response_tx.take() {
            let _ = tx.send(ApprovalDecision::Deny);
        }
        self.pending_exec.remove(idx);
    }

    pub(crate) fn deny_exec_forever(&mut self, idx: usize) {
        if idx >= self.pending_exec.len() {
            return;
        }
        let item = &self.pending_exec[idx];
        let argv = item.argv.clone();
        let project_name = item.project.clone();
        let cwd = self.portable_cwd(&item.rule_cwd, &project_name);
        if let Some(rules_path) = self.project_rules_path(&project_name) {
            match self.persist_exec_rule(&rules_path, &argv, &cwd, crate::rules::ApprovalMode::Deny)
            {
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
                format!("Cannot persist deny: unknown workspace '{project_name}'"),
                true,
            );
        }
        self.deny_exec(idx);
    }

    pub(crate) fn approve_net(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let tx = std::mem::replace(&mut self.pending_net[idx].response_tx, oneshot_dummy());
        let _ = tx.send(NetworkDecision::Allow);
        self.pending_net.remove(idx);
    }

    pub(crate) fn deny_net(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let tx = std::mem::replace(&mut self.pending_net[idx].response_tx, oneshot_dummy());
        let _ = tx.send(NetworkDecision::Deny);
        self.pending_net.remove(idx);
    }

    pub(crate) fn approve_net_forever(&mut self, idx: usize) {
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

    pub(crate) fn deny_net_forever(&mut self, idx: usize) {
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
                        format!(
                            "network host '{}' denied by default (no explicit rule needed)",
                            host
                        ),
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

    pub(crate) fn resolve_pending_network_project(&self, idx: usize) -> Option<String> {
        let item = self.pending_net.get(idx)?;
        if let Some(project) = item.source_project.clone() {
            return Some(project);
        }
        if let Some(container_name) = item.source_container.as_deref() {
            let mut workspaces = self
                .sessions
                .iter()
                .filter(|s| !s.is_exited() && s.container_name == container_name)
                .map(|s| s.project.clone())
                .collect::<Vec<_>>();
            workspaces.sort();
            workspaces.dedup();
            if workspaces.len() == 1 {
                return workspaces.into_iter().next();
            }
        }
        let cfg = self.config.get();
        self.selected_project_idx()
            .and_then(|pi| cfg.workspaces.get(pi))
            .map(|p| p.name.clone())
    }

    pub(crate) fn persist_exec_rule(
        &mut self,
        rules_path: &std::path::Path,
        argv: &[String],
        cwd: &str,
        approval_mode: crate::rules::ApprovalMode,
    ) -> Result<()> {
        let is_new = !rules_path.exists();
        let mut rules = crate::rules::load(rules_path)
            .with_context(|| format!("loading rules file '{}'", rules_path.display()))?;
        if rules.hostdo.commands.iter().any(|c| c.argv == argv) {
            return Ok(());
        }
        rules.hostdo.commands.push(crate::rules::RuleCommand {
            name: None,
            argv: argv.to_vec(),
            cwd: cwd.to_string(),
            env_profile: None,
            timeout_secs: 60,
            concurrency: crate::rules::ConcurrencyPolicy::Queue,
            approval_mode,
        });
        let expected_content = crate::rules::render_rules_file(&rules, is_new)
            .with_context(|| format!("rendering rules file '{}'", rules_path.display()))?;
        self.note_rules_internal_write(rules_path.to_path_buf(), expected_content);
        crate::rules::write_rules_file(rules_path, &rules, is_new)
            .with_context(|| format!("writing rules file '{}'", rules_path.display()))?;
        Ok(())
    }

    pub(crate) fn persist_network_rule(
        &mut self,
        host: &str,
        policy: NetworkPolicy,
        project_name: Option<&str>,
    ) -> Result<Option<std::path::PathBuf>> {
        let rules_path = match project_name {
            Some(name) => match self.project_rules_path(name) {
                Some(path) => path,
                None => anyhow::bail!(
                    "cannot persist network rule: workspace '{}' not found",
                    name
                ),
            },
            None => anyhow::bail!(
                "cannot persist network rule: unknown workspace (request lacked workspace attribution)"
            ),
        };

        let is_new = !rules_path.exists();
        let mut rules = crate::rules::load(&rules_path)
            .with_context(|| format!("loading rules file '{}'", rules_path.display()))?;

        if policy == NetworkPolicy::Deny {
            // Network is deny-by-default under the Coder-style allowlist engine.
            return Ok(None);
        }
        let entry = format!("domain={host}");
        let exists = rules
            .network
            .allowlist
            .iter()
            .any(|raw| raw.trim().eq_ignore_ascii_case(&entry));
        if exists {
            return Ok(None);
        }
        rules.network.allowlist.push(entry);

        let expected_content = crate::rules::render_rules_file(&rules, is_new)
            .with_context(|| format!("rendering rules file '{}'", rules_path.display()))?;
        self.note_rules_internal_write(rules_path.clone(), expected_content);
        crate::rules::write_rules_file(&rules_path, &rules, is_new)
            .with_context(|| format!("writing rules file '{}'", rules_path.display()))?;
        Ok(Some(rules_path))
    }

    pub(crate) fn log_missing_network_project_context(&mut self, idx: usize, action: &str) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        self.push_log(
            format!("cannot persist permanent {action} rule for '{}' because the network request had no source workspace metadata", host),
            true,
        );
    }

    pub(crate) fn portable_cwd(&self, cwd: &Path, project_name: &str) -> String {
        let cfg = self.config.get();
        let mount_target = cfg
            .workspaces
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

    pub(crate) fn project_rules_path(&self, project_name: &str) -> Option<std::path::PathBuf> {
        let cfg = self.config.get();
        cfg.workspaces
            .iter()
            .find(|p| p.name == project_name)
            .map(|p| p.canonical_path.join("void-rules.toml"))
    }

    pub(crate) fn sync_rules_to_workspace(&mut self, project_name: &str) {
        let _ = project_name;
    }
}
