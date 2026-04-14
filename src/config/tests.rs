#[cfg(test)]
mod tests {
    use crate::config::{
        Config, ContainerMount, DefaultsConfig, MountMode, SyncMode, effective_mount_source_path,
        effective_sync_mode, effective_workspace_path, image_tag_for_stem, load,
        load_composed_rules_for_workspace, merge_mounts, merge_unique_strings,
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
        let dir = std::env::temp_dir().join(format!("void-claw-{prefix}-{nanos}"));
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

[workspace]

[manager]
global_rules_file = "{}"
"#,
            workspace_root.display(),
            global_rules_file.display(),
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
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"

[defaults.ui]
sidebar_width = 28
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert_eq!(cfg.defaults.ui.sidebar_width, 28);
    }

    #[test]
    fn load_persists_logging_instance_id() {
        let root = unique_temp_dir("instance-id-persist");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
        );
        fs::write(&cfg_path, raw).expect("write config");

        let cfg = load(&cfg_path).expect("config should load");
        let instance_id = cfg
            .logging
            .instance_id
            .as_deref()
            .expect("instance id should be generated");

        let contents = fs::read_to_string(&cfg_path).expect("read config");
        let parsed: toml::Value = toml::from_str(&contents).expect("parse config");
        assert_eq!(parsed["logging"]["instance_id"].as_str(), Some(instance_id));
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
allowlist = ["domain=github.com"]
"#,
        )
        .expect("write global rules");

        let config = build_config(&global, &workspace, Some("project-a"), Some(&project_path));

        let composed =
            load_composed_rules_for_workspace(&config, Some("project-a")).expect("compose rules");
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

        let composed = load_composed_rules_for_workspace(&config, None).expect("compose rules");
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
allowlist = ["domain=api.openai.com"]
"#,
        )
        .expect("write global rules");

        fs::write(
            project_path.join("void-rules.toml"),
            r#"
[hostdo]
default_policy = "prompt"

[network]
allowlist = ["domain=github.com"]
"#,
        )
        .expect("write project rules");

        let config = build_config(&global, &workspace, Some("project-b"), Some(&project_path));

        let composed =
            load_composed_rules_for_workspace(&config, Some("project-b")).expect("compose rules");
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Prompt);
        assert_eq!(composed.network_default, NetworkPolicy::Prompt);
    }

    #[test]
    fn load_fails_when_project_canonical_path_is_missing() {
        let root = unique_temp_dir("missing-canonical-path");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"

[[projects]]
name = "missing-proj"
canonical_path = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
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
    fn load_rejects_workspace_exclude_patterns_field() {
        let root = unique_temp_dir("reject-workspace-exclude-patterns");
        let cfg_path = root.join("void-claw.toml");
        let canonical_path = root.join("repo");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&canonical_path).expect("create repo");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"

[[workspaces]]
name = "a"
canonical_path = "{}"
exclude_patterns = ["node_modules/**"]
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            canonical_path.display(),
        );
        fs::write(&cfg_path, raw).expect("write config");
        load(&cfg_path).expect_err("config should reject workspace exclude_patterns");
    }

    #[test]
    fn load_fails_when_docker_dir_is_missing() {
        let root = unique_temp_dir("missing-docker-dir");
        let cfg_path = root.join("void-claw.toml");
        let raw = format!(
            r#"
[workspace]

[manager]
global_rules_file = "{}"
"#,
            root.join("global-rules.toml").display()
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
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert_eq!(cfg.docker_dir, docker_dir);
    }

    #[test]
    fn load_fails_when_docker_dir_is_a_file() {
        let root = unique_temp_dir("docker-dir-file");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::write(&docker_dir, "not a directory").expect("write docker file");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
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
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert!(cfg.workspaces.is_empty());
    }

    #[test]
    fn load_accepts_workspaces_alias() {
        let root = unique_temp_dir("workspaces-alias");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        let workspace_path = root.join("workspace-a");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        fs::create_dir_all(&workspace_path).expect("create workspace dir");

        let raw = format!(
            r#"
docker_dir = "{}"
[workspace]

[manager]
global_rules_file = "{}"
[[workspaces]]
name = "workspace-a"
canonical_path = "{}"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            workspace_path.display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        assert_eq!(cfg.workspaces.len(), 1);
        assert_eq!(cfg.workspaces[0].name, "workspace-a");
        assert_eq!(cfg.workspaces[0].canonical_path, workspace_path);
    }

    #[test]
    fn effective_mode_is_always_direct() {
        let root = unique_temp_dir("direct-disposable");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        let project_path = root.join("project-a");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        fs::create_dir_all(&project_path).expect("create project path");

        let raw = format!(
            r#"
docker_dir = "{}"
[workspace]

[manager]
global_rules_file = "{}"
[[projects]]
name = "project-a"
canonical_path = "{}"
disposable = true
[projects.sync]
mode = "direct"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display(),
            project_path.display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        let proj = cfg.workspaces.first().expect("project present");
        assert_eq!(effective_sync_mode(proj, &cfg.defaults), SyncMode::Direct);
    }

    #[test]
    fn workspace_and_mount_paths_always_resolve_to_canonical() {
        let root = unique_temp_dir("direct-workspace-path");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        let project_path = root.join("project-a");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        fs::create_dir_all(&project_path).expect("create project path");

        let raw = format!(
            r#"
docker_dir = "{}"
[workspace]

[manager]
global_rules_file = "{}"
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
            project_path.display(),
            root.join("some-other-place").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config should load");
        let proj = cfg.workspaces.first().expect("project present");
        assert_eq!(
            effective_workspace_path(proj, &cfg.workspace),
            proj.canonical_path
        );
        assert_eq!(
            effective_mount_source_path(proj, &cfg.workspace, &cfg.defaults),
            proj.canonical_path
        );
    }

    // New tests for merge_unique_strings
    #[test]
    fn merge_unique_strings_handles_empty_inputs() {
        let base: Vec<String> = vec![];
        let profile: Vec<String> = vec![];
        let override_items: Vec<String> = vec![];
        let result = merge_unique_strings(&base, &profile, &override_items);
        assert!(result.is_empty());
    }

    #[test]
    fn merge_unique_strings_merges_all_unique_items() {
        let base = vec!["a".to_string(), "b".to_string()];
        let profile = vec!["c".to_string(), "d".to_string()];
        let override_items = vec!["e".to_string(), "f".to_string()];
        let result = merge_unique_strings(&base, &profile, &override_items);
        assert_eq!(result.len(), 6);
        assert!(result.contains(&"a".to_string()));
        assert!(result.contains(&"f".to_string()));
    }

    #[test]
    fn merge_unique_strings_handles_duplicates() {
        let base = vec!["a".to_string(), "b".to_string()];
        let profile = vec!["b".to_string(), "c".to_string()];
        let override_items = vec!["c".to_string(), "a".to_string(), "d".to_string()];
        let result = merge_unique_strings(&base, &profile, &override_items);
        assert_eq!(result.len(), 4);
        assert!(result.contains(&"a".to_string()));
        assert!(result.contains(&"b".to_string()));
        assert!(result.contains(&"c".to_string()));
        assert!(result.contains(&"d".to_string()));
    }

    #[test]
    fn merge_unique_strings_preserves_order_of_first_appearance() {
        let base = vec!["a".to_string(), "b".to_string()];
        let profile = vec!["c".to_string(), "a".to_string()]; // 'a' appears again
        let override_items = vec!["d".to_string(), "b".to_string()]; // 'b' appears again
        let result = merge_unique_strings(&base, &profile, &override_items);
        assert_eq!(
            result,
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string()
            ]
        );
    }

    #[test]
    fn merge_mounts_handles_empty_inputs() {
        let base: Vec<ContainerMount> = vec![];
        let profile: Vec<ContainerMount> = vec![];
        let override_items: Vec<ContainerMount> = vec![];
        let result = merge_mounts(&base, &profile, &override_items);
        assert!(result.is_empty());
    }

    #[test]
    fn merge_mounts_merges_all_unique_items() {
        let m1 = ContainerMount {
            host: "h1".into(),
            container: "c1".into(),
            mode: MountMode::Rw,
        };
        let m2 = ContainerMount {
            host: "h2".into(),
            container: "c2".into(),
            mode: MountMode::Ro,
        };
        let m3 = ContainerMount {
            host: "h3".into(),
            container: "c3".into(),
            mode: MountMode::Rw,
        };

        let base = vec![m1.clone()];
        let profile = vec![m2.clone()];
        let override_items = vec![m3.clone()];
        let result = merge_mounts(&base, &profile, &override_items);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&m1));
        assert!(result.contains(&m2));
        assert!(result.contains(&m3));
    }

    #[test]
    fn merge_mounts_handles_duplicates() {
        let m1 = ContainerMount {
            host: "h1".into(),
            container: "c1".into(),
            mode: MountMode::Rw,
        };
        let m2 = ContainerMount {
            host: "h2".into(),
            container: "c2".into(),
            mode: MountMode::Ro,
        };
        let m3_diff_mode = ContainerMount {
            host: "h1".into(),
            container: "c1".into(),
            mode: MountMode::Ro,
        }; // Same paths, different mode

        let base = vec![m1.clone()];
        let profile = vec![m1.clone(), m2.clone()]; // m1 duplicated
        let override_items = vec![m2.clone(), m3_diff_mode.clone()]; // m2 duplicated, m3_diff_mode is new

        let result = merge_mounts(&base, &profile, &override_items);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&m1));
        assert!(result.contains(&m2));
        assert!(result.contains(&m3_diff_mode));
        assert_eq!(result, vec![m1, m2, m3_diff_mode]);
    }

    #[test]
    fn merge_mounts_with_different_paths_are_unique() {
        let m1 = ContainerMount {
            host: "h1".into(),
            container: "c1".into(),
            mode: MountMode::Rw,
        };
        let m2 = ContainerMount {
            host: "h1".into(),
            container: "c2".into(),
            mode: MountMode::Rw,
        }; // Same host, diff container
        let m3 = ContainerMount {
            host: "h2".into(),
            container: "c1".into(),
            mode: MountMode::Rw,
        }; // Diff host, same container

        let base = vec![m1.clone()];
        let profile = vec![m2.clone()];
        let override_items = vec![m3.clone()];

        let result = merge_mounts(&base, &profile, &override_items);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&m1));
        assert!(result.contains(&m2));
        assert!(result.contains(&m3));
    }

    #[test]
    fn load_rejects_legacy_containers_section() {
        let root = unique_temp_dir("reject-legacy-containers");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"

[[containers]]
name = "legacy"
profile = "codex"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let err = load(&cfg_path).expect_err("legacy containers must be rejected");
        assert!(
            err.to_string()
                .contains("legacy [[containers]] is no longer supported"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_synthesizes_runtime_containers_from_profiles() {
        let root = unique_temp_dir("profiles-synthesize-containers");
        let cfg_path = root.join("void-claw.toml");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&docker_dir).expect("create docker dir");
        let raw = format!(
            r#"
docker_dir = "{}"

[workspace]

[manager]
global_rules_file = "{}"

[container_profiles.codex]
image = "default"
agent = "codex"
"#,
            docker_dir.display(),
            root.join("global-rules.toml").display()
        );
        fs::write(&cfg_path, raw).expect("write config");
        let cfg = load(&cfg_path).expect("config load should work");
        assert_eq!(cfg.containers.len(), 1);
        assert_eq!(cfg.containers[0].name, "codex");
        assert_eq!(cfg.containers[0].image_stem, "default");
        assert_eq!(cfg.containers[0].image, "void-claw-default:local");
    }

    #[test]
    fn image_tag_for_stem_normalizes_non_alnum_chars() {
        assert_eq!(image_tag_for_stem("default"), "void-claw-default:local");
        assert_eq!(
            image_tag_for_stem("Rust.Tools"),
            "void-claw-rust.tools:local"
        );
        assert_eq!(image_tag_for_stem("a b"), "void-claw-a-b:local");
    }
}
