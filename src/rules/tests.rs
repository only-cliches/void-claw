#[cfg(test)]
mod tests {
    use crate::rules::{
        ApprovalMode, ComposedRules, ConcurrencyPolicy, HostdoRules, NetworkPolicy, NetworkRule,
        NetworkRules, ProjectRules, RuleCommand, append_auto_approval, host_matches, load,
        write_rules_file,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_current_schema() {
        let raw = r#"
exclude_patterns = ["node_modules", "dist/**"]

[hostdo]
default_policy = "prompt"

[[hostdo.commands]]
argv = ["cargo", "check"]
cwd = "$WORKSPACE"
approval_mode = "auto"

# Aliases: plain passthrough and with cwd override.
[hostdo.command_aliases]
lint = "cargo clippy"
tests = { cmd = "cargo test", cwd = "$CANONICAL" }

[network]
default_policy = "prompt"

[[network.rules]]
methods = ["*"]
host = "github.com"
path_prefix = "/"
policy = "auto"
"#;

        let parsed: Result<ProjectRules, toml::de::Error> = toml::from_str(raw);
        let rules = parsed.expect("expected current schema to parse");
        assert_eq!(rules.exclude_patterns, vec!["node_modules", "dist/**"]);
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
default_policy = "prompt"

[[network.rules]]
host = "github.com"
policy = "allow"
"#;

        let parsed: Result<ProjectRules, toml::de::Error> = toml::from_str(raw);
        assert!(
            parsed.is_err(),
            "legacy schema should be rejected to avoid silent misconfiguration"
        );
    }

    #[test]
    fn wildcard_host_matches_subdomain_and_apex() {
        assert!(host_matches("*.oaistatic.com", "cdn.oaistatic.com"));
        assert!(host_matches("*.oaistatic.com", "oaistatic.com"));
    }

    #[test]
    fn wildcard_host_match_is_case_insensitive() {
        assert!(host_matches("*.OpenAI.com", "AUTH.OPENAI.COM"));
        assert!(host_matches("*.OpenAI.com", "openai.com"));
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
            network_default: NetworkPolicy::Prompt,
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
        let dir = std::env::temp_dir().join(format!("agent-zero-rules-test-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("zero-rules.toml");
        let argv = vec!["cargo".to_string(), "test".to_string()];

        append_auto_approval(&path, &argv, "$WORKSPACE").expect("first append");
        append_auto_approval(&path, &argv, "$CANONICAL").expect("second append");

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
        let dir = std::env::temp_dir().join(format!("agent-zero-rules-header-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("zero-rules.toml");

        write_rules_file(&path, &ProjectRules::default(), false).expect("write");
        let s = std::fs::read_to_string(&path).expect("read");
        assert!(
            s.starts_with("# zero-rules.toml — policy"),
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
        let dir = std::env::temp_dir().join(format!("agent-zero-rules-header-append-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("zero-rules.toml");

        append_auto_approval(&path, &["echo".to_string(), "hi".to_string()], "/tmp")
            .expect("append");
        let s = std::fs::read_to_string(&path).expect("read");
        assert!(
            s.starts_with("# zero-rules.toml — policy"),
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
            network: NetworkRules {
                default_policy: NetworkPolicy::Auto,
                ..Default::default()
            },
            ..Default::default()
        };
        let proj1 = ProjectRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Auto,
                ..Default::default()
            },
            network: NetworkRules {
                default_policy: NetworkPolicy::Deny,
                ..Default::default()
            },
            ..Default::default()
        };
        let proj2 = ProjectRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Deny,
                ..Default::default()
            },
            network: NetworkRules {
                default_policy: NetworkPolicy::Prompt,
                ..Default::default()
            },
            ..Default::default()
        };

        let composed = ComposedRules::compose(&global, &[proj1, proj2]);

        // Deny > Prompt > Auto
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Deny);
        assert_eq!(composed.network_default, NetworkPolicy::Deny);
    }

    #[test]
    fn match_network_longest_path_prefix_wins() {
        let rules = ComposedRules {
            network_rules: vec![
                NetworkRule {
                    methods: vec!["*".into()],
                    host: "api.example.com".into(),
                    path_prefix: "/".into(),
                    policy: NetworkPolicy::Prompt,
                },
                NetworkRule {
                    methods: vec!["*".into()],
                    host: "api.example.com".into(),
                    path_prefix: "/api/v2".into(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".into()],
                    host: "api.example.com".into(),
                    path_prefix: "/api/v2/auth".into(),
                    policy: NetworkPolicy::Deny,
                },
            ],
            network_default: NetworkPolicy::Prompt,
            ..Default::default()
        };

        // Matches "/", "/api/v2", and "/api/v2/auth". Most specific (longest) is Deny.
        assert_eq!(
            rules.match_network("GET", "api.example.com", "/api/v2/auth/login"),
            NetworkPolicy::Deny
        );

        // Matches "/" and "/api/v2". Longest is Auto.
        assert_eq!(
            rules.match_network("GET", "api.example.com", "/api/v2/user"),
            NetworkPolicy::Auto
        );

        // Matches only "/". Policy is Prompt.
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
                        cwd: "$CANONICAL".into(),
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

        rules.expand_cwd_vars("/home/user/project", "/tmp/ws/project");

        assert_eq!(rules.hostdo.commands[0].cwd, "/home/user/project");
        assert_eq!(rules.hostdo.commands[1].cwd, "/tmp/ws/project/subdir");
        assert_eq!(rules.hostdo.commands[2].cwd, "/absolute/path");
    }
}
