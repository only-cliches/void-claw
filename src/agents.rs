use anyhow::Context;
use std::path::Path;

use crate::config::AgentKind;
use crate::rules::{ApprovalMode, HostdoRules, NetworkRules, ProjectRules};
// ── void-rules.toml starter ───────────────────────────────────────────────────

/// Generate a starter `void-rules.toml` for the given agent kind.
///
/// Includes common-sense `auto`-approved rules for developer tools (GitHub,
/// npm, PyPI, crates.io) plus agent-specific API domains.  The default policy
/// for anything not listed is `prompt`, so the developer still sees and
/// approves unexpected destinations.
/// Build the initial `void-rules.toml` template for a given agent runtime.
pub fn generate_starter_project_rules(agent: &AgentKind) -> ProjectRules {
    let mut allowlist = vec![
        "domain=github.com".to_string(),
        "domain=api.github.com".to_string(),
        "domain=raw.githubusercontent.com".to_string(),
        "domain=objects.githubusercontent.com".to_string(),
        "domain=registry.npmjs.org".to_string(),
        "domain=*.npmjs.org".to_string(),
        "domain=pypi.org".to_string(),
        "domain=files.pythonhosted.org".to_string(),
        "domain=crates.io".to_string(),
        "domain=static.crates.io".to_string(),
        "domain=index.crates.io".to_string(),
        "domain=rubygems.org".to_string(),
        "domain=api.rubygems.org".to_string(),
        "domain=pkg.go.dev".to_string(),
        "domain=sum.golang.org".to_string(),
        "domain=proxy.golang.org".to_string(),
    ];

    match agent {
        AgentKind::Claude => {
            allowlist.extend([
                "domain=api.anthropic.com".to_string(),
                "domain=statsig.anthropic.com".to_string(),
                "domain=sentry.io".to_string(),
            ]);
        }
        AgentKind::Codex => {
            allowlist.push("domain=api.openai.com".to_string());
        }
        AgentKind::Gemini => {
            allowlist.extend([
                "domain=generativelanguage.googleapis.com".to_string(),
                "domain=aistudio.google.com".to_string(),
                "domain=accounts.google.com".to_string(),
                "domain=oauth2.googleapis.com".to_string(),
                "domain=www.googleapis.com".to_string(),
            ]);
        }
        AgentKind::Opencode => {
            allowlist.extend([
                "domain=api.anthropic.com".to_string(),
                "domain=api.openai.com".to_string(),
                "domain=openrouter.ai".to_string(),
                "domain=api.openrouter.ai".to_string(),
            ]);
        }
        AgentKind::None => {}
    }

    ProjectRules {
        llm_instructions: None,
        hostdo: HostdoRules {
            default_policy: ApprovalMode::Prompt,
            ..HostdoRules::default()
        },
        network: NetworkRules { allowlist },
    }
}

// ── inject_agent_config ───────────────────────────────────────────────────────

/// Inject agent configuration files into the workspace and, if no
/// `void-rules.toml` exists in the canonical project directory, write a
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
/// - All:      `<canonical>/void-rules.toml` (only if it does not already exist)
/// - None:     nothing
/// Seed a workspace with `void-rules.toml` guidance for the selected agent.
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
    // Returns true if a new void-rules.toml was created.
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

    // Write a starter void-rules.toml to the canonical project dir if absent.
    // This is the file the server/proxy reads for policy enforcement.
    let rules_path = canonical_path.join("void-rules.toml");
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
- Network access is filtered by void-claw; allowed destinations are in `[network]`.\n",
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
        r#"── void-claw CA Certificate ─────────────────────────────────────────
The proxy CA was generated.  Containers must trust it.

  Export path: {ca_cert_path}

  In your Dockerfile:
    COPY void-claw-ca.crt /usr/local/share/ca-certificates/
    RUN update-ca-certificates          # Debian/Ubuntu
    # or: update-ca-trust               # RHEL/Fedora

  Runtime env vars (included in the docker run snippet):
    NODE_EXTRA_CA_CERTS, REQUESTS_CA_BUNDLE, SSL_CERT_FILE

  Set VOID_CLAW_CA_CERT_PATH to the cert file location so the snippet works:
    export VOID_CLAW_CA_CERT_PATH={ca_cert_path}
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
        let allowlist = rules.network.allowlist;
        assert!(allowlist.iter().any(|r| r == "domain=generativelanguage.googleapis.com"));
        assert!(allowlist.iter().any(|r| r == "domain=accounts.google.com"));
        assert!(allowlist.iter().any(|r| r == "domain=oauth2.googleapis.com"));
    }
}
