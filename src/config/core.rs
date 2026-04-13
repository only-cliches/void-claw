use super::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Re-export rule enums so callers don't need to import both modules.
pub use crate::rules::ApprovalMode;

// ── Top-level ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub manager: ManagerConfig,
    pub workspace: WorkspaceSection,
    /// Directory containing the repository root used for Docker builds.
    /// This is required at startup and is auto-populated by `agent-zero --init`.
    #[serde(default)]
    pub docker_dir: PathBuf,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub env_profiles: HashMap<String, EnvProfile>,
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,
    #[serde(default)]
    pub container_profiles: HashMap<String, ContainerProfile>,
    #[serde(default)]
    pub containers: Vec<ContainerDef>,
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            manager: ManagerConfig::default(),
            workspace: WorkspaceSection::default(),
            docker_dir: PathBuf::new(),
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
            env_profiles: HashMap::new(),
            projects: Vec::new(),
            container_profiles: HashMap::new(),
            containers: Vec::new(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ManagerConfig {
    /// Path to the global zero-rules.toml where auto-approved commands are persisted.
    /// Created on first use if it does not exist.
    #[serde(alias = "rules_file")]
    pub global_rules_file: PathBuf,
}

/// The single managed workspace root.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct WorkspaceSection {
    /// All project workspace copies land at `root/<project.name>/`.
    pub root: PathBuf,
}

// ── Agents ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AgentsConfig {
    pub claude: Option<ClaudeAgentConfig>,
    pub codex: Option<CodexAgentConfig>,
    pub gemini: Option<GeminiAgentConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClaudeAgentConfig {
    /// Where to write the generated `settings.json`.
    pub settings_path: Option<PathBuf>,
    /// Additional instructions appended to `CLAUDE.md`.
    pub extra_instructions: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CodexAgentConfig {
    pub config_path: Option<PathBuf>,
    pub extra_instructions: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiAgentConfig {
    pub extra_instructions: Option<String>,
}

// ── Defaults ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DefaultsConfig {
    #[serde(default)]
    pub sync: SyncDefaults,
    #[serde(default)]
    pub workspace: WorkspaceDefaults,
    #[serde(default)]
    pub ui: UiDefaults,
    #[serde(default)]
    pub hostdo: HostdoDefaults,
    #[serde(default)]
    pub proxy: ProxyDefaults,
    #[serde(default)]
    pub containers: ContainerDefaults,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SyncDefaults {
    pub mode: SyncMode,
    pub delete_propagation: bool,
    pub rename_propagation: bool,
    pub symlink_policy: SymlinkPolicy,
    pub conflict_policy: ConflictPolicy,
    #[serde(default = "default_exclude_patterns")]
    pub global_exclude_patterns: Vec<String>,
}

impl Default for SyncDefaults {
    fn default() -> Self {
        Self {
            mode: SyncMode::default(),
            delete_propagation: false,
            rename_propagation: false,
            symlink_policy: SymlinkPolicy::default(),
            conflict_policy: ConflictPolicy::default(),
            global_exclude_patterns: default_exclude_patterns(),
        }
    }
}

fn default_exclude_patterns() -> Vec<String> {
    [
        ".*",
        ".git",
        ".git/**",
        ".env",
        ".env.*",
        "*.pem",
        "*.key",
        "*.pfx",
        "*.p12",
        "id_rsa",
        "id_ed25519",
        "id_ecdsa",
        ".ssh",
        ".ssh/**",
        ".gnupg",
        ".gnupg/**",
        ".aws",
        ".aws/**",
        ".claude",
        ".claude/**",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkspaceDefaults {
    #[serde(default = "bool_true")]
    pub disposable: bool,
    #[serde(default)]
    pub default_policy: ApprovalMode,
}

impl Default for WorkspaceDefaults {
    fn default() -> Self {
        Self {
            disposable: true,
            default_policy: ApprovalMode::default(),
        }
    }
}

pub(crate) fn bool_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiDefaults {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,
}

fn default_sidebar_width() -> u16 {
    32
}

impl Default for UiDefaults {
    fn default() -> Self {
        Self {
            sidebar_width: default_sidebar_width(),
        }
    }
}

/// A command alias: either a plain command string or a table with `cmd` and optional `cwd`.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum AliasValue {
    Simple(String),
    WithOptions {
        cmd: String,
        #[serde(default)]
        cwd: Option<PathBuf>,
    },
}

/// Magic cwd values resolved at request time with project context.
const ALIAS_CWD_CANONICAL: &str = "$CANONICAL";
const ALIAS_CWD_WORKSPACE: &str = "$WORKSPACE";

impl AliasValue {
    pub fn cmd(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::WithOptions { cmd, .. } => cmd,
        }
    }

    /// Resolve the alias cwd, substituting magic values:
    /// - `$CANONICAL` → project's canonical (host) path
    /// - `$WORKSPACE` → project's effective workspace path
    pub fn resolve_cwd(&self, canonical_path: &Path, workspace_path: &Path) -> Option<PathBuf> {
        match self {
            Self::Simple(_) => None,
            Self::WithOptions { cwd: None, .. } => None,
            Self::WithOptions { cwd: Some(p), .. } => {
                let s = p.as_os_str();
                if s == ALIAS_CWD_CANONICAL {
                    Some(canonical_path.to_path_buf())
                } else if s == ALIAS_CWD_WORKSPACE {
                    Some(workspace_path.to_path_buf())
                } else {
                    Some(p.clone())
                }
            }
        }
    }

    /// Expand `~` in the cwd path, if present.  Skips magic values like
    /// `$CANONICAL` / `$WORKSPACE` which are resolved later with project context.
    pub(crate) fn expand_cwd(&mut self) -> Result<()> {
        if let Self::WithOptions { cwd: Some(p), .. } = self {
            if !p.as_os_str().to_string_lossy().starts_with('$') {
                *p = expand_path(p)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HostdoDefaults {
    #[serde(default = "default_exec_port")]
    pub server_port: u16,
    #[serde(default = "default_host")]
    pub server_host: String,
    #[serde(default = "default_token_env")]
    pub token_env_var: String,
    #[serde(default = "default_denied_executables")]
    pub denied_executables: Vec<String>,
    #[serde(default)]
    pub denied_argument_fragments: Vec<String>,
    /// Optional command aliases expanded server-side for hostdo.
    /// Example: doMyHomeWork = "curl example.com"
    /// Example: tests = { cmd = "cargo test", cwd = "/home/user/project" }
    #[serde(default)]
    pub command_aliases: HashMap<String, AliasValue>,
}

fn default_exec_port() -> u16 {
    7878
}
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_token_env() -> String {
    "AGENT_ZERO_TOKEN".to_string()
}
fn default_denied_executables() -> Vec<String> {
    [
        "sh", "bash", "zsh", "fish", "csh", "ksh", "sudo", "su", "doas",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

impl Default for HostdoDefaults {
    fn default() -> Self {
        Self {
            server_port: default_exec_port(),
            server_host: default_host(),
            token_env_var: default_token_env(),
            denied_executables: default_denied_executables(),
            denied_argument_fragments: vec![],
            command_aliases: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProxyDefaults {
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_host")]
    pub proxy_host: String,
    /// When enabled, containers are launched with NET_ADMIN + root so they can:
    ///   1) transparently redirect outbound HTTP/HTTPS through the agent-zero proxy
    ///   2) install strict outbound egress rules (iptables) to block direct egress
    ///      outside the proxy and exec bridge.
    ///
    /// This is the recommended "near-impossible to bypass" mode.
    ///
    #[serde(default)]
    pub strict_network: bool,
}

fn default_proxy_port() -> u16 {
    8081
}

impl Default for ProxyDefaults {
    fn default() -> Self {
        Self {
            proxy_port: default_proxy_port(),
            proxy_host: default_host(),
            strict_network: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct EnvProfile {
    #[serde(default)]
    pub vars: HashMap<String, String>,
}
