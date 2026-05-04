#[cfg(test)]
mod tests {
    use crate::rules::{
        ApprovalMode, ComposedRules, ConcurrencyPolicy, HostdoRules, NetworkPolicy, NetworkRules,
        ProjectRules, RuleCommand, append_auto_approval, host_matches, load,
        parse_network_allowlist_rule, write_rules_file,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_current_schema() {
        let raw = r#"
[hostdo]
default_policy = "prompt"

[[hostdo.commands]]
argv = ["cargo", "check"]
cwd = "$WORKSPACE"
approval_mode = "auto"

# Aliases: plain passthrough and with cwd override.
[hostdo.command_aliases]
lint = "cargo clippy"
tests = { cmd = "cargo test", cwd = "$WORKSPACE" }

[network]
allowlist = ["domain=github.com"]
"#;

        let parsed: Result<ProjectRules, toml::de::Error> = toml::from_str(raw);
        let rules = parsed.expect("expected current schema to parse");
        assert_eq!(rules.hostdo.command_aliases.len(), 2);
        assert_eq!(rules.hostdo.command_aliases["lint"].cmd(), "cargo clippy");
        assert_eq!(rules.hostdo.command_aliases["tests"].cmd(), "cargo test");
    }

    #[test]
    fn rejects_legacy_readme_schema() {
        let raw = r#"
[[commands]]
argv = ["cargo", "check"]
cwd = "$WORKSPACE"
approval_mode = "auto"

[network]
allowlist = ["domain=github.com policy=allow"]
"#;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("harness-hat-rules-invalid-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("harness-rules.toml");
        std::fs::write(&path, raw).expect("write rules");

        let parsed = load(&path);
        assert!(parsed.is_err(), "legacy schema should be rejected");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn rejects_exclude_patterns_field() {
        let raw = r#"
exclude_patterns = ["node_modules/**"]

[network]
allowlist = ["domain=github.com"]
"#;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("harness-hat-rules-invalid-excludes-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("harness-rules.toml");
        std::fs::write(&path, raw).expect("write rules");
        let parsed = load(&path);
        assert!(parsed.is_err(), "exclude_patterns should be rejected");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn wildcard_host_matches_subdomain_only() {
        assert!(host_matches("*.oaistatic.com", "cdn.oaistatic.com"));
        assert!(!host_matches("*.oaistatic.com", "oaistatic.com"));
    }

    #[test]
    fn wildcard_host_match_is_case_insensitive() {
        assert!(host_matches("*.OpenAI.com", "AUTH.OPENAI.COM"));
        assert!(!host_matches("*.OpenAI.com", "openai.com"));
    }

    #[test]
    fn hostdo_command_match_ignores_cwd() {
        let rules = ComposedRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![RuleCommand {
                    name: None,
                    argv: vec!["cargo".into(), "test".into()],
                    cwd: "/some/path".into(),
                    env_profile: None,
                    timeout_secs: 60,
                    concurrency: ConcurrencyPolicy::Queue,
                    approval_mode: ApprovalMode::Auto,
                }],
                command_aliases: Default::default(),
            },
            network_rules: vec![],
            network_default: NetworkPolicy::Deny,
        };

        let matched = rules.find_hostdo_command(&["cargo".into(), "test".into()]);
        assert!(matched.is_some(), "argv match should not depend on cwd");
    }

    #[test]
    fn append_auto_approval_dedupes_by_argv() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("harness-hat-rules-test-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("harness-rules.toml");
        let argv = vec!["cargo".to_string(), "test".to_string()];

        append_auto_approval(&path, &argv, "$WORKSPACE").expect("first append");
        append_auto_approval(&path, &argv, "$WORKSPACE").expect("second append");

        let rules = load(&path).expect("load rules");
        assert_eq!(rules.hostdo.commands.len(), 1);
        assert_eq!(rules.hostdo.commands[0].argv, argv);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn write_rules_file_always_includes_header() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("harness-hat-rules-header-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("harness-rules.toml");

        write_rules_file(&path, &ProjectRules::default(), false).expect("write");
        let s = std::fs::read_to_string(&path).expect("read");
        assert!(
            s.starts_with("# harness-rules.toml — policy"),
            "missing header prefix"
        );
        assert!(
            s.contains("Preferred place for *human/LLM instructions*"),
            "missing instruction hint"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn append_auto_approval_preserves_header() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("harness-hat-rules-header-append-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("harness-rules.toml");

        append_auto_approval(&path, &["echo".to_string(), "hi".to_string()], "/tmp")
            .expect("append");
        let s = std::fs::read_to_string(&path).expect("read");
        assert!(
            s.starts_with("# harness-rules.toml — policy"),
            "missing header after append"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn composed_rules_pick_most_restrictive_default_policy() {
        let global = ProjectRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                ..Default::default()
            },
            network: NetworkRules::default(),
            ..Default::default()
        };
        let proj1 = ProjectRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Auto,
                ..Default::default()
            },
            network: NetworkRules::default(),
            ..Default::default()
        };
        let proj2 = ProjectRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Deny,
                ..Default::default()
            },
            network: NetworkRules::default(),
            ..Default::default()
        };

        let composed = ComposedRules::compose(&global, &[proj1, proj2]);

        // Deny > Prompt > Auto
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Deny);
        assert_eq!(composed.network_default, NetworkPolicy::Prompt);
    }

    #[test]
    fn match_network_allowlist_works() {
        let rules = ComposedRules {
            network_rules: vec![
                parse_network_allowlist_rule("domain=api.example.com path=/api/v2/*")
                    .expect("parse rule"),
                parse_network_allowlist_rule(
                    "method=POST domain=api.example.com path=/api/v2/auth/*",
                )
                .expect("parse rule"),
            ],
            network_default: NetworkPolicy::Prompt,
            ..Default::default()
        };

        // Method-specific match.
        assert_eq!(
            rules.match_network("POST", "api.example.com", "/api/v2/auth/login"),
            NetworkPolicy::Auto
        );

        // Path wildcard match.
        assert_eq!(
            rules.match_network("GET", "api.example.com", "/api/v2/user"),
            NetworkPolicy::Auto
        );

        // Unmatched path is prompted by default.
        assert_eq!(
            rules.match_network("GET", "api.example.com", "/other"),
            NetworkPolicy::Prompt
        );
    }

    #[test]
    fn expand_cwd_vars_replaces_placeholders() {
        let mut rules = ComposedRules {
            hostdo: HostdoRules {
                commands: vec![
                    RuleCommand {
                        argv: vec!["ls".into()],
                        cwd: "$WORKSPACE".into(),
                        ..Default::default()
                    },
                    RuleCommand {
                        argv: vec!["ls".into(), "-a".into()],
                        cwd: "$WORKSPACE/subdir".into(),
                        ..Default::default()
                    },
                    RuleCommand {
                        argv: vec!["pwd".into()],
                        cwd: "/absolute/path".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        rules.expand_cwd_vars("/home/user/project");

        assert_eq!(rules.hostdo.commands[0].cwd, "/home/user/project");
        assert_eq!(rules.hostdo.commands[1].cwd, "/home/user/project/subdir");
        assert_eq!(rules.hostdo.commands[2].cwd, "/absolute/path");
    }

    #[test]
    fn find_hostdo_command_exact_match() {
        let rules = ComposedRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![
                    RuleCommand {
                        argv: vec!["cargo".into(), "test".into()],
                        cwd: "/tmp".into(), // Cwd irrelevant for matching
                        approval_mode: ApprovalMode::Auto,
                        ..Default::default()
                    },
                    RuleCommand {
                        argv: vec!["npm".into(), "install".into()],
                        cwd: "/app".into(),
                        approval_mode: ApprovalMode::Prompt,
                        ..Default::default()
                    },
                ],
                command_aliases: Default::default(),
            },
            ..Default::default()
        };

        let matched = rules.find_hostdo_command(&["cargo".into(), "test".into()]);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().approval_mode, ApprovalMode::Auto);

        let matched_npm = rules.find_hostdo_command(&["npm".into(), "install".into()]);
        assert!(matched_npm.is_some());
        assert_eq!(matched_npm.unwrap().approval_mode, ApprovalMode::Prompt);
    }

    #[test]
    fn find_hostdo_command_no_partial_match() {
        let rules = ComposedRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![RuleCommand {
                    argv: vec!["cargo".into(), "test".into()],
                    cwd: "/tmp".into(),
                    approval_mode: ApprovalMode::Auto,
                    ..Default::default()
                }],
                command_aliases: Default::default(),
            },
            ..Default::default()
        };

        // Partial match (subset)
        let matched = rules.find_hostdo_command(&["cargo".into()]);
        assert!(matched.is_none());

        // Partial match (superset)
        let matched =
            rules.find_hostdo_command(&["cargo".into(), "test".into(), "--verbose".into()]);
        assert!(matched.is_none());
    }

    #[test]
    fn find_hostdo_command_respects_argument_order() {
        let rules = ComposedRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![RuleCommand {
                    argv: vec!["arg1".into(), "arg2".into()],
                    cwd: "/tmp".into(),
                    approval_mode: ApprovalMode::Auto,
                    ..Default::default()
                }],
                command_aliases: Default::default(),
            },
            ..Default::default()
        };

        let matched = rules.find_hostdo_command(&["arg2".into(), "arg1".into()]); // Different order
        assert!(matched.is_none());

        let matched = rules.find_hostdo_command(&["arg1".into(), "arg2".into()]); // Correct order
        assert!(matched.is_some());
    }

    #[test]
    fn find_hostdo_command_empty_argv() {
        let rules = ComposedRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![
                    RuleCommand {
                        argv: vec![], // Empty argv rule
                        cwd: "/tmp".into(),
                        approval_mode: ApprovalMode::Deny,
                        ..Default::default()
                    },
                    RuleCommand {
                        argv: vec!["ls".into()],
                        cwd: "/".into(),
                        approval_mode: ApprovalMode::Auto,
                        ..Default::default()
                    },
                ],
                command_aliases: Default::default(),
            },
            ..Default::default()
        };

        let matched = rules.find_hostdo_command(&[]);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().approval_mode, ApprovalMode::Deny);

        let matched = rules.find_hostdo_command(&["ls".into()]);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().approval_mode, ApprovalMode::Auto);
    }
}
