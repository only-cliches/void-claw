#[cfg(test)]
mod tests {
    use crate::config::{
        Config, DefaultsConfig, combined_excludes, load, load_composed_rules_for_project,
    };
    use crate::rules::{ApprovalMode, NetworkPolicy};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agent-zero-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn build_config(
        global_rules_file: &Path,
        workspace_root: &Path,
        project_name: Option<&str>,
        project_path: Option<&Path>,
    ) -> Config {
        let mut raw = format!(
            r#"
docker_dir = "{}"

[manager]
global_rules_file = "{}"

[workspace]
root = "{}"
"#,
            workspace_root.display(),
            global_rules_file.display(),
            workspace_root.display(),
        );
        if let (Some(name), Some(path)) = (project_name, project_path) {
            raw.push_str(&format!(
                r#"
[[projects]]
name = "{name}"
canonical_path = "{}"
"#,
                path.display()
            ));
        }
        toml::from_str(&raw).expect("parse minimal config")
    }

    #[test]
    fn defaults_sidebar_width_defaults_to_32() {
        assert_eq!(DefaultsConfig::default().ui.sidebar_width, 32);
    }

    #[test]
    fn load_applies_custom_sidebar_width() {
        let root = unique_temp_dir("sidebar-width-override");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[manager]
global_rules_file = "{}"

[workspace]
root = "{}"

[defaults.ui]
sidebar_width = 28
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert_eq!(cfg.defaults.ui.sidebar_width, 28);
    }

    #[test]
    fn composed_rules_use_global_when_project_file_is_missing() {
        let root = unique_temp_dir("composed-global-fallback");
        let global = root.join("global-rules.toml");
        let workspace = root.join("workspace");
        let project_path = root.join("project-a");
        fs::create_dir_all(&workspace).expect("create workspace");
        fs::create_dir_all(&project_path).expect("create project path");

        fs::write(
            &global,
            r#"
[hostdo]
default_policy = "deny"

[network]
default_policy = "auto"
"#,
        )
        .expect("write global rules");

        let config = build_config(&global, &workspace, Some("project-a"), Some(&project_path));

        let composed =
            load_composed_rules_for_project(&config, Some("project-a")).expect("compose rules");
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Deny);
        assert_eq!(composed.network_default, NetworkPolicy::Prompt);
    }

    #[test]
    fn composed_rules_default_to_prompt_when_no_rules_files_exist() {
        let root = unique_temp_dir("composed-default-prompt");
        let global = root.join("missing-global.toml");
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("create workspace");
        let config = build_config(&global, &workspace, None, None);

        let composed = load_composed_rules_for_project(&config, None).expect("compose rules");
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Prompt);
        assert_eq!(composed.network_default, NetworkPolicy::Prompt);
    }

    #[test]
    fn composed_rules_merge_project_over_global_defaults() {
        let root = unique_temp_dir("composed-project-overrides");
        let global = root.join("global-rules.toml");
        let workspace = root.join("workspace");
        let project_path = root.join("project-b");
        fs::create_dir_all(&workspace).expect("create workspace");
        fs::create_dir_all(&project_path).expect("create project path");

        fs::write(
            &global,
            r#"
[hostdo]
default_policy = "auto"

[network]
default_policy = "deny"
"#,
        )
        .expect("write global rules");

        fs::write(
            project_path.join("zero-rules.toml"),
            r#"
[hostdo]
default_policy = "prompt"

[network]
default_policy = "auto"
"#,
        )
        .expect("write project rules");

        let config = build_config(&global, &workspace, Some("project-b"), Some(&project_path));

        let composed =
            load_composed_rules_for_project(&config, Some("project-b")).expect("compose rules");
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Prompt);
        assert_eq!(composed.network_default, NetworkPolicy::Deny);
    }

    #[test]
    fn load_fails_when_project_canonical_path_is_missing() {
        let root = unique_temp_dir("missing-canonical-path");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[manager]
global_rules_file = "{}"

[workspace]
root = "{}"

[[projects]]
name = "missing-proj"
canonical_path = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display(),
            root.join("does-not-exist").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let err = load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string().contains("canonical_path does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_fails_when_docker_dir_is_missing() {
        let root = unique_temp_dir("missing-docker-dir");
        let cfg_path = root.join("agent-zero.toml");
        let raw = format!(
            r#"
[manager]
global_rules_file = "{}"

[workspace]
root = "{}"
"#,
            root.join("global-rules.toml").display(),
            root.join("workspace").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let err = load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string().contains("docker_dir is required"),
            "unexpected error: {err}"
        );
        assert!(
            err.to_string().contains(&cfg_path.display().to_string()),
            "missing config path in error: {err}"
        );
    }

    #[test]
    fn load_accepts_when_docker_dir_does_not_exist() {
        let root = unique_temp_dir("missing-docker-dir-path");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        let raw = format!(
            r#"
docker_dir = "{}"

[manager]
global_rules_file = "{}"

[workspace]
root = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert_eq!(cfg.docker_dir, docker_dir);
    }

    #[test]
    fn load_fails_when_docker_dir_is_a_file() {
        let root = unique_temp_dir("docker-dir-file");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        fs::write(&docker_dir, "not a directory").expect("write docker file");
        let raw = format!(
            r#"
docker_dir = "{}"

[manager]
global_rules_file = "{}"

[workspace]
root = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let err = load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string()
                .contains("docker_dir exists but is not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_accepts_config_with_no_projects() {
        let root = unique_temp_dir("no-projects");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[manager]
global_rules_file = "{}"

[workspace]
root = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert!(cfg.projects.is_empty());
    }

    #[test]
    fn load_fails_when_direct_mode_has_disposable_true() {
        let root = unique_temp_dir("direct-disposable");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        let project_path = root.join("project-a");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        fs::create_dir_all(&project_path).expect("create project path");

        let raw = format!(
            r#"
docker_dir = "{}"
[manager]
global_rules_file = "{}"
[workspace]
root = "{}"

[[projects]]
name = "project-a"
canonical_path = "{}"
disposable = true
[projects.sync]
mode = "direct"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display(),
            project_path.display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let err = load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string()
                .contains("disposable=true is not allowed with projects.sync.mode='direct'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_fails_when_direct_mode_has_explicit_workspace_path() {
        let root = unique_temp_dir("direct-workspace-path");
        let cfg_path = root.join("agent-zero.toml");
        let docker_dir = root.join("docker-root");
        let project_path = root.join("project-a");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        fs::create_dir_all(&project_path).expect("create project path");

        let raw = format!(
            r#"
docker_dir = "{}"
[manager]
global_rules_file = "{}"
[workspace]
root = "{}"

[[projects]]
name = "project-a"
canonical_path = "{}"
disposable = false
workspace_path = "{}"
[projects.sync]
mode = "direct"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            root.join("workspace").display(),
            project_path.display(),
            root.join("some-other-place").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let err = load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string().contains("workspace_path must be omitted"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn combined_excludes_include_project_rules_file_patterns() {
        let root = unique_temp_dir("combined-excludes-rules");
        let project_path = root.join("project-c");
        fs::create_dir_all(&project_path).expect("create project path");
        fs::write(
            project_path.join("zero-rules.toml"),
            r#"
exclude_patterns = ["node_modules", "dist/**"]
"#,
        )
        .expect("write project rules");

        let config = build_config(
            &root.join("global-rules.toml"),
            &root.join("workspace"),
            Some("project-c"),
            Some(&project_path),
        );
        let project = config
            .projects
            .iter()
            .find(|p| p.name == "project-c")
            .expect("project config");

        let excludes = combined_excludes(project, &config.defaults).expect("combined excludes");
        assert!(excludes.iter().any(|p| p == "node_modules"));
        assert!(excludes.iter().any(|p| p == "dist/**"));
        assert!(excludes.iter().any(|p| p == ".git"));
    }
}
