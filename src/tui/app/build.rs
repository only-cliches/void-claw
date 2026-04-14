use super::*;

impl App {
    pub(crate) fn start_docker_build(
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

    pub(crate) fn cancel_build(&mut self) {
        let Some(task) = self.build_task.as_ref() else {
            return;
        };
        task.cancel_flag.store(true, Ordering::SeqCst);
        self.push_log(format!("cancelling {}...", task.label), true);
    }

    pub(crate) fn push_build_output(&mut self, line: impl Into<String>, is_error: bool) {
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

        let agent_cmd = name.strip_prefix("void-claw-").map(|agent| {
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

    pub(crate) fn do_launch_container(&mut self, ctr_idx: usize) {
        let pi = match self.selected_project_idx() {
            Some(i) => i,
            None => {
                self.push_log("no workspace selected", true);
                return;
            }
        };
        self.do_launch_container_on_project(pi, ctr_idx);
    }

    pub(crate) fn open_image_build_prompt(&mut self, pi: usize, ctr_idx: usize, image: &str) {
        self.build_project_idx = Some(pi);
        self.build_container_idx = Some(ctr_idx);
        self.build_cursor = 0;
        self.build_output.clear();
        self.build_scroll = 0;
        self.active_session = None;
        self.active_settings_project = None;
        self.container_picker = None;
        self.focus = Focus::ImageBuild;
        self.push_log(
            format!("docker image '{image}' not found locally; build required"),
            true,
        );
    }

    pub(crate) fn preflight_image_or_prompt_build<F>(
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
                self.push_log(
                    format!("warning: failed to check docker image '{image}': {e}"),
                    true,
                );
                true
            }
        }
    }
}
