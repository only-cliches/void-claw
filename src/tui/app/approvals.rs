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

    pub(crate) fn resolve_pending_network_project(&self, idx: usize) -> Option<String> {
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

    pub(crate) fn persist_network_rule(
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

    pub(crate) fn log_missing_network_project_context(&mut self, idx: usize, action: &str) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        self.push_log(
            format!("cannot persist permanent {action} rule for '{}' because the network request had no source project metadata", host),
            true,
        );
    }

    pub(crate) fn portable_cwd(&self, cwd: &Path, project_name: &str) -> String {
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

    pub(crate) fn project_rules_path(&self, project_name: &str) -> Option<std::path::PathBuf> {
        let cfg = self.config.get();
        cfg.projects
            .iter()
            .find(|p| p.name == project_name)
            .map(|p| p.canonical_path.join("void-rules.toml"))
    }

    pub(crate) fn sync_rules_to_workspace(&mut self, project_name: &str) {
        let cfg = self.config.get();
        if let Some(pi) = cfg.projects.iter().position(|p| p.name == project_name) {
            self.do_seed_project(pi);
        }
    }
}
