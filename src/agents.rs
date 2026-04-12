use anyhow::Context;
use std::path::Path;

use crate::config::AgentKind;
use crate::rules::{ApprovalMode, HostdoRules, NetworkPolicy, NetworkRule, NetworkRules, ProjectRules};
// ── zero-rules.toml starter ───────────────────────────────────────────────────

/// Generate a starter `zero-rules.toml` for the given agent kind.
///
/// Includes common-sense `auto`-approved rules for developer tools (GitHub,
/// npm, PyPI, crates.io) plus agent-specific API domains.  The default policy
/// for anything not listed is `prompt`, so the developer still sees and
/// approves unexpected destinations.
/// Build the initial `zero-rules.toml` template for a given agent runtime.
pub fn generate_starter_project_rules(agent: &AgentKind) -> ProjectRules {
    let mut rules = vec![
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "github.com".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "api.github.com".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "raw.githubusercontent.com".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "objects.githubusercontent.com".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "registry.npmjs.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "*.npmjs.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "pypi.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "files.pythonhosted.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "crates.io".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "static.crates.io".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "index.crates.io".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "rubygems.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "api.rubygems.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "pkg.go.dev".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "sum.golang.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
        NetworkRule {
            methods: vec!["*".to_string()],
            host: "proxy.golang.org".to_string(),
            path_prefix: "/".to_string(),
            policy: NetworkPolicy::Auto,
        },
    ];

    match agent {
        AgentKind::Claude => {
            rules.extend([
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "api.anthropic.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "statsig.anthropic.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "sentry.io".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
            ]);
        }
        AgentKind::Codex => {
            rules.push(NetworkRule {
                methods: vec!["*".to_string()],
                host: "api.openai.com".to_string(),
                path_prefix: "/".to_string(),
                policy: NetworkPolicy::Auto,
            });
        }
        AgentKind::Gemini => {
            rules.extend([
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "generativelanguage.googleapis.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "aistudio.google.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "accounts.google.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "oauth2.googleapis.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "www.googleapis.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
            ]);
        }
        AgentKind::Opencode => {
            rules.extend([
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "api.anthropic.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "api.openai.com".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "openrouter.ai".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
                NetworkRule {
                    methods: vec!["*".to_string()],
                    host: "api.openrouter.ai".to_string(),
                    path_prefix: "/".to_string(),
                    policy: NetworkPolicy::Auto,
                },
            ]);
        }
        AgentKind::None => {}
    }

    ProjectRules {
        llm_instructions: None,
        exclude_patterns: vec![],
        hostdo: HostdoRules {
            default_policy: ApprovalMode::Prompt,
            ..HostdoRules::default()
        },
        network: NetworkRules {
            default_policy: NetworkPolicy::Prompt,
            rules,
        },
    }
}

// ── inject_agent_config ───────────────────────────────────────────────────────

/// Inject agent configuration files into the workspace and, if no
/// `zero-rules.toml` exists in the canonical project directory, write a
/// starter one with sensible network allowlist rules.
///
/// Called just before spawning a container so the files are present on the
/// bind-mounted workspace when the agent starts.  Safe to call on every launch;
/// agent config files are always overwritten with fresh content.
///
/// Files written per agent:
/// - Claude:   `CLAUDE.md`, `.claude/settings.json`
/// - Codex:    `AGENTS.md`, `codex.json`
/// - Gemini:   `GEMINI.md`
/// - opencode: `AGENTS.md`
/// - All:      `<canonical>/zero-rules.toml` (only if it does not already exist)
/// - None:     nothing
/// Seed a workspace with `zero-rules.toml` guidance for the selected agent.
pub fn inject_agent_config(
    agent: &AgentKind,
    workspace_path: &Path,
    canonical_path: &Path,
    project_name: &str,
    direct_mount: bool,
    _mount_target: &Path,
    _exec_url: &str,
    _proxy_url: &str,
    extra_instructions: Option<&str>,
) -> anyhow::Result<bool> {
    // Returns true if a new zero-rules.toml was created.
    if *agent == AgentKind::None {
        return Ok(false);
    }

    // Ensure the workspace directory exists (it may not have been seeded yet).
    std::fs::create_dir_all(workspace_path).with_context(|| {
        format!(
            "creating workspace directory '{}'",
            workspace_path.display()
        )
    })?;

    // Write a starter zero-rules.toml to the canonical project dir if absent.
    // This is the file the server/proxy reads for policy enforcement.
    let rules_path = canonical_path.join("zero-rules.toml");
    let created_rules = if !rules_path.exists() {
        std::fs::create_dir_all(canonical_path).with_context(|| {
            format!(
                "creating canonical project directory '{}'",
                canonical_path.display()
            )
        })?;
        let mut starter = generate_starter_project_rules(agent);
        let extra = extra_instructions
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("\n\nAdditional instructions:\n{s}\n"))
            .unwrap_or_default();
        starter.llm_instructions = Some(format!(
            "Project: {project_name}\n\
\n\
Environment:\n\
- You are operating inside a Linux Docker container.\n\
- Workspace mount path (inside container): {}\n\
{}\n\
\n\
{}\n\
\n\
Rules of engagement:\n\
- Read and follow this file before taking actions.\n\
- Use `hostdo` only when the user explicitly asks for host activity.\n\
- Use `killme` only when the user explicitly asks to end this container.\n\
- Network access is filtered by agent-zero; allowed destinations are in `[network]`.\n",
            _mount_target.display(),
            if direct_mount {
                "- This project uses direct-mount sync; edits persist to the host."
            } else {
                "- This project uses a managed workspace; be careful about canonical vs workspace paths."
            },
            extra
        ));
        crate::rules::write_rules_file(&rules_path, &starter, true)
            .with_context(|| format!("writing starter rules file '{}'", rules_path.display()))?;
        true
    } else {
        false
    };

    Ok(created_rules)
}

/// Instructions shown to the developer after first CA generation.
/// Return the CA bootstrap instructions used inside generated agent guidance.
pub fn ca_setup_instructions(_ca_cert_pem: &str, ca_cert_path: &str) -> String {
    format!(
        r#"── agent-zero CA Certificate ────────────────────────────────────────
The proxy CA was generated.  Containers must trust it.

  Export path: {ca_cert_path}

  In your Dockerfile:
    COPY agent-zero-ca.crt /usr/local/share/ca-certificates/
    RUN update-ca-certificates          # Debian/Ubuntu
    # or: update-ca-trust               # RHEL/Fedora

  Runtime env vars (included in the docker run snippet):
    NODE_EXTRA_CA_CERTS, REQUESTS_CA_BUNDLE, SSL_CERT_FILE

  Set AGENT_ZERO_CA_CERT_PATH to the cert file location so the snippet works:
    export AGENT_ZERO_CA_CERT_PATH={ca_cert_path}
────────────────────────────────────────────────────────────────────────────────
"#,
        ca_cert_path = ca_cert_path,
    )
}

#[cfg(test)]
mod tests {
    use super::generate_starter_project_rules;
    use crate::config::AgentKind;

    #[test]
    fn gemini_starter_rules_include_google_hosts() {
        let rules = generate_starter_project_rules(&AgentKind::Gemini);
        let hosts: Vec<&str> = rules.network.rules.iter().map(|rule| rule.host.as_str()).collect();
        assert!(hosts.contains(&"generativelanguage.googleapis.com"));
        assert!(hosts.contains(&"accounts.google.com"));
        assert!(hosts.contains(&"oauth2.googleapis.com"));
    }
}
