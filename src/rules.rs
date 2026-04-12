/// Parses `zero-rules.toml` files and composes global + per-project rules.
///
/// `zero-rules.toml` lives in the canonical project root (committed to git).
/// It controls what the AI agent is allowed to do: which host-side commands
/// can run, and which network destinations are reachable.
use anyhow::{Context, Result};
use globset::Glob;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::AliasValue;

// ── Enums (re-used across config and rules) ──────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    Auto,
    Prompt,
    Deny,
}

impl Default for ApprovalMode {
    fn default() -> Self {
        Self::Prompt
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConcurrencyPolicy {
    Queue,
    Reject,
    Parallel,
}

impl Default for ConcurrencyPolicy {
    fn default() -> Self {
        Self::Queue
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Auto,
    Prompt,
    Deny,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self::Prompt
    }
}

// ── zero-rules.toml schema ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ProjectRules {
    /// Optional instructions for a human or LLM agent. This field is preserved
    /// across automatic edits to this file (e.g. when agent-zero appends a new
    /// `hostdo` command rule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_instructions: Option<String>,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    #[serde(default)]
    pub hostdo: HostdoRules,
    #[serde(default)]
    pub network: NetworkRules,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HostdoRules {
    #[serde(default)]
    pub default_policy: ApprovalMode,
    #[serde(default)]
    pub commands: Vec<RuleCommand>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub command_aliases: HashMap<String, AliasValue>,
}

/// A single allowed host-side command.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RuleCommand {
    /// Optional human-readable label shown in the TUI and audit log.
    /// Defaults to the argv joined with spaces when absent.
    pub name: Option<String>,
    /// Exact argv that must match the request.
    pub argv: Vec<String>,
    /// Absolute path on the host. Use the canonical project path.
    pub cwd: String,
    pub env_profile: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub concurrency: ConcurrencyPolicy,
    pub approval_mode: ApprovalMode,
}

impl RuleCommand {
    /// Human-readable label for TUI display and audit log.
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.argv.join(" "))
    }
}

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkRules {
    #[serde(default)]
    pub default_policy: NetworkPolicy,
    #[serde(default)]
    pub rules: Vec<NetworkRule>,
}

impl Default for NetworkRules {
    fn default() -> Self {
        Self {
            default_policy: NetworkPolicy::Prompt,
            rules: vec![],
        }
    }
}

/// One network policy rule.  Rules are checked in declaration order.
/// Among rules that match the same request, the one with the longest
/// `path_prefix` wins.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkRule {
    /// HTTP methods this rule applies to.  `["*"]` matches any method.
    pub methods: Vec<String>,
    /// Hostname or glob pattern (e.g. `"api.github.com"` or `"*.npmjs.org"`).
    pub host: String,
    /// Path prefix. `"/"` matches everything; `"/api/v2/"` is more specific.
    pub path_prefix: String,
    pub policy: NetworkPolicy,
}

// ── ComposedRules ────────────────────────────────────────────────────────────

/// Effective rule set for a given request context.
/// Global rules and all project rules are unioned together.
#[derive(Debug, Clone, Default)]
pub struct ComposedRules {
    pub hostdo: HostdoRules,
    pub network_rules: Vec<NetworkRule>,
    pub network_default: NetworkPolicy,
}

impl ComposedRules {
    /// Compose global rules + one or more project rule sets.
    /// Global rules come first (higher priority in declaration order).
    pub fn compose(global: &ProjectRules, projects: &[ProjectRules]) -> Self {
        let mut commands = global.hostdo.commands.clone();
        let mut network_rules = global.network.rules.clone();

        // Union: add project rules that don't duplicate a global argv.
        for proj in projects {
            for cmd in &proj.hostdo.commands {
                if !commands.iter().any(|c| c.argv == cmd.argv) {
                    commands.push(cmd.clone());
                }
            }
            network_rules.extend(proj.network.rules.clone());
        }

        // Hostdo default policy: most restrictive wins (deny > prompt > auto).
        let hostdo_default = std::iter::once(&global.hostdo.default_policy)
            .chain(projects.iter().map(|p| &p.hostdo.default_policy))
            .fold(ApprovalMode::Auto, |acc, p| match (&acc, p) {
                (ApprovalMode::Deny, _) | (_, ApprovalMode::Deny) => ApprovalMode::Deny,
                (ApprovalMode::Prompt, _) | (_, ApprovalMode::Prompt) => ApprovalMode::Prompt,
                _ => ApprovalMode::Auto,
            });

        // Network default policy: most restrictive wins (deny > prompt > auto).
        let network_default = std::iter::once(&global.network.default_policy)
            .chain(projects.iter().map(|p| &p.network.default_policy))
            .fold(NetworkPolicy::Auto, |acc, p| match (&acc, p) {
                (NetworkPolicy::Deny, _) | (_, NetworkPolicy::Deny) => NetworkPolicy::Deny,
                (NetworkPolicy::Prompt, _) | (_, NetworkPolicy::Prompt) => NetworkPolicy::Prompt,
                _ => NetworkPolicy::Auto,
            });

        Self {
            hostdo: HostdoRules {
                default_policy: hostdo_default,
                commands,
                command_aliases: HashMap::new(),
            },
            network_rules,
            network_default,
        }
    }

    /// Find the best matching network policy for a given request.
    pub fn match_network(&self, method: &str, host: &str, path: &str) -> NetworkPolicy {
        let candidates: Vec<&NetworkRule> = self
            .network_rules
            .iter()
            .filter(|r| method_matches(&r.methods, method) && host_matches(&r.host, host))
            .filter(|r| path.starts_with(r.path_prefix.as_str()))
            .collect();

        // Longest path prefix wins (most specific).
        candidates
            .into_iter()
            .max_by_key(|r| r.path_prefix.len())
            .map(|r| r.policy.clone())
            .unwrap_or_else(|| self.network_default.clone())
    }

    /// Expand `$CANONICAL` and `$WORKSPACE` prefixes in rule command cwds.
    pub fn expand_cwd_vars(&mut self, canonical_path: &str, mount_target: &str) {
        for cmd in &mut self.hostdo.commands {
            if cmd.cwd == "$CANONICAL" {
                cmd.cwd = canonical_path.to_string();
            } else if cmd.cwd == "$WORKSPACE" {
                cmd.cwd = mount_target.to_string();
            } else if let Some(rest) = cmd.cwd.strip_prefix("$CANONICAL/") {
                cmd.cwd = format!("{canonical_path}/{rest}");
            } else if let Some(rest) = cmd.cwd.strip_prefix("$WORKSPACE/") {
                cmd.cwd = format!("{mount_target}/{rest}");
            }
        }
    }

    /// Find an exact-match hostdo command by argv.
    ///
    /// `cwd` remains part of the rule because it still determines where the
    /// command runs and is persisted for developer review, but it is not part
    /// of the approval identity.
    pub fn find_hostdo_command<'a>(&'a self, argv: &[String]) -> Option<&'a RuleCommand> {
        self.hostdo.commands.iter().find(|c| c.argv == argv)
    }
}

fn method_matches(methods: &[String], method: &str) -> bool {
    methods
        .iter()
        .any(|m| m == "*" || m.eq_ignore_ascii_case(method))
}

fn host_matches(pattern: &str, host: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern.eq_ignore_ascii_case(host);
    }
    // Treat leading "*." as matching both subdomains and the apex domain.
    // Example: "*.example.com" matches "api.example.com" and "example.com".
    if let Some(apex) = pattern.strip_prefix("*.") {
        if host.eq_ignore_ascii_case(apex) {
            return true;
        }
    }
    // Glob match (e.g. "*.example.com")
    let pattern_lc = pattern.to_ascii_lowercase();
    let host_lc = host.to_ascii_lowercase();
    Glob::new(&pattern_lc)
        .ok()
        .map(|g| g.compile_matcher().is_match(&host_lc))
        .unwrap_or(false)
}

// ── Loading / saving ─────────────────────────────────────────────────────────

/// Load a `zero-rules.toml` file.  Returns a default (empty) rule set if the
/// file does not exist, rather than an error.
pub fn load(path: &Path) -> Result<ProjectRules> {
    if !path.exists() {
        return Ok(ProjectRules::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading zero-rules.toml: {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing zero-rules.toml: {}", path.display()))
}

/// Append an auto-approved command to the rules file at `path`.
///
/// If the argv already exists in the file, the file is left unchanged. The
/// parent directory is created if needed.
pub fn append_auto_approval(path: &Path, argv: &[String], cwd: &str) -> Result<()> {
    append_command_rule(path, argv, cwd, ApprovalMode::Auto)
}

/// Append a permanently denied command to the rules file at `path`.
///
/// If the argv already exists in the file, the file is left unchanged. The
/// parent directory is created if needed.
pub fn append_deny_rule(path: &Path, argv: &[String], cwd: &str) -> Result<()> {
    append_command_rule(path, argv, cwd, ApprovalMode::Deny)
}

fn append_command_rule(path: &Path, argv: &[String], cwd: &str, mode: ApprovalMode) -> Result<()> {
    let is_new = !path.exists();
    let mut rules = load(path)?;
    if rules.hostdo.commands.iter().any(|c| c.argv == argv) {
        return Ok(());
    }
    rules.hostdo.commands.push(RuleCommand {
        name: None,
        argv: argv.to_vec(),
        cwd: cwd.to_string(),
        env_profile: None,
        timeout_secs: default_timeout(),
        concurrency: ConcurrencyPolicy::default(),
        approval_mode: mode,
    });
    write_rules_file(path, &rules, is_new)
}

/// Write rules to a file, adding a comment header if the file is new.
pub fn write_rules_file(path: &Path, rules: &ProjectRules, is_new: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory for {}", path.display()))?;
    }
    let toml_str = toml::to_string_pretty(rules).context("serializing rules to TOML")?;
    let _ = is_new; // retained for API compatibility
    let content = format!("{RULES_FILE_HEADER}{toml_str}");
    std::fs::write(path, &content).with_context(|| format!("writing {}", path.display()))
}

const RULES_FILE_HEADER: &str = "\
# zero-rules.toml — policy for what the AI agent can do in this project.
# Commit this file to your repository. agent-zero reads it but never pushes
# changes back during workspace sync.
#
# Preferred place for *human/LLM instructions*:
# llm_instructions = \"\"\"\n\
# \"\"\"
#
# Optional workspace seed exclusions:
# exclude_patterns = [\"node_modules\", \"dist/**\"]

# ── Hostdo (host-side command execution) ─────────────────────────────────────
#
# default_policy: what happens when a command doesn't match any rule below.
#   auto   — run without prompting (use with caution)
#   prompt — ask the developer in the TUI (default)
#   deny   — reject silently
#
# Passthrough command (exact argv match, auto-approved):
#   [[hostdo.commands]]
#   argv = [\"cargo\", \"test\"] # run inside container with `hostdo cargo test`
#   cwd = \"$WORKSPACE\"         # execution cwd only, not part of approval matching
#   approval_mode = \"auto\"
#
# Command alias (agent sends `hostdo tests`, expands server-side):
#   [hostdo.command_aliases]
#   tests = \"cargo test\" # run inside container with `hostdo test`
#   build = { cmd = \"cargo build --release\", cwd = \"$CANONICAL\" }
#
# $WORKSPACE = container mount target, $CANONICAL = host project path.

# ── Network (HTTP/HTTPS proxy policy) ────────────────────────────────────────
#
# default_policy: what happens when a request doesn't match any rule below.
#   auto   — allow without prompting
#   prompt — ask the developer in the TUI (default)
#   deny   — block silently

";

#[cfg(test)]
mod tests {
    use super::{
        ApprovalMode, ComposedRules, NetworkPolicy, NetworkRule, RuleCommand, ProjectRules,
        append_auto_approval, host_matches, write_rules_file,
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
            hostdo: super::HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![RuleCommand {
                    name: None,
                    argv: vec!["cargo".into(), "test".into()],
                    cwd: "/some/path".into(),
                    env_profile: None,
                    timeout_secs: 60,
                    concurrency: super::ConcurrencyPolicy::Queue,
                    approval_mode: ApprovalMode::Auto,
                }],
                command_aliases: Default::default(),
            },
            network_rules: vec![],
            network_default: super::NetworkPolicy::Prompt,
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

        let rules = super::load(&path).expect("load rules");
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
        assert!(s.starts_with("# zero-rules.toml — policy"), "missing header prefix");
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
        assert!(s.starts_with("# zero-rules.toml — policy"), "missing header after append");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn composed_rules_pick_most_restrictive_default_policy() {
        let global = ProjectRules {
            hostdo: super::HostdoRules {
                default_policy: ApprovalMode::Prompt,
                ..Default::default()
            },
            network: super::NetworkRules {
                default_policy: super::NetworkPolicy::Auto,
                ..Default::default()
            },
            ..Default::default()
        };
        let proj1 = ProjectRules {
            hostdo: super::HostdoRules {
                default_policy: ApprovalMode::Auto,
                ..Default::default()
            },
            network: super::NetworkRules {
                default_policy: super::NetworkPolicy::Deny,
                ..Default::default()
            },
            ..Default::default()
        };
        let proj2 = ProjectRules {
            hostdo: super::HostdoRules {
                default_policy: ApprovalMode::Deny,
                ..Default::default()
            },
            network: super::NetworkRules {
                default_policy: super::NetworkPolicy::Prompt,
                ..Default::default()
            },
            ..Default::default()
        };

        let composed = ComposedRules::compose(&global, &[proj1, proj2]);
        
        // Deny > Prompt > Auto
        assert_eq!(composed.hostdo.default_policy, ApprovalMode::Deny);
        assert_eq!(composed.network_default, super::NetworkPolicy::Deny);
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
        assert_eq!(rules.match_network("GET", "api.example.com", "/api/v2/auth/login"), NetworkPolicy::Deny);
        
        // Matches "/" and "/api/v2". Longest is Auto.
        assert_eq!(rules.match_network("GET", "api.example.com", "/api/v2/user"), NetworkPolicy::Auto);
        
        // Matches only "/". Policy is Prompt.
        assert_eq!(rules.match_network("GET", "api.example.com", "/other"), NetworkPolicy::Prompt);
    }

    #[test]
    fn expand_cwd_vars_replaces_placeholders() {
        let mut rules = ComposedRules {
            hostdo: super::HostdoRules {
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
