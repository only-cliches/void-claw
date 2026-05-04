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
    /// This is required at startup and is auto-populated by
    /// `harness-hat-manager --init`.
    #[serde(default)]
    pub docker_dir: PathBuf,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub env_profiles: HashMap<String, EnvProfile>,
    #[serde(default, alias = "projects")]
    pub workspaces: Vec<WorkspaceConfig>,
    #[serde(default)]
    pub container_profiles: HashMap<String, ContainerProfile>,
    /// Internal resolved launch entries synthesized from `container_profiles`.
    /// Config parsing rejects legacy `[[containers]]` entries.
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
            // `docker_dir` is expected to be populated during
            // `harness-hat-manager --init`.
            // An empty PathBuf here signifies an uninitialized state.
            docker_dir: PathBuf::new(),
            defaults: DefaultsConfig::default(),
            agents: AgentsConfig::default(),
            env_profiles: HashMap::new(),
            workspaces: Vec::new(),
            container_profiles: HashMap::new(),
            containers: Vec::new(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ManagerConfig {
    /// Path to the global harness-rules.toml where auto-approved commands are persisted.
    /// Created on first use if it does not exist.
    #[serde(alias = "rules_file")]
    pub global_rules_file: PathBuf,
}

/// Reserved section for future workspace-scoped settings.
///
/// Breaking change: `workspace.root` has been removed.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSection {}

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
#[serde(deny_unknown_fields)]
pub struct SyncDefaults {
    pub mode: SyncMode,
    pub delete_propagation: bool,
    pub rename_propagation: bool,
    pub symlink_policy: SymlinkPolicy,
    pub conflict_policy: ConflictPolicy,
}

impl Default for SyncDefaults {
    fn default() -> Self {
        Self {
            mode: SyncMode::default(),
            delete_propagation: false,
            rename_propagation: false,
            symlink_policy: SymlinkPolicy::default(),
            conflict_policy: ConflictPolicy::default(),
        }
    }
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

/// Helper function for `serde(default = "bool_true")` to set a boolean field to true by default.
pub(crate) fn bool_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiDefaults {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,
}

/// Provides the default value for `UiDefaults.sidebar_width`.
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
const ALIAS_CWD_WORKSPACE: &str = "$WORKSPACE";

impl AliasValue {
    pub fn cmd(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::WithOptions { cmd, .. } => cmd,
        }
    }

    /// Resolve the alias cwd, substituting the `$WORKSPACE` placeholder.
    pub fn resolve_cwd(&self, workspace_path: &Path) -> Option<PathBuf> {
        match self {
            Self::Simple(_) => None,
            Self::WithOptions { cwd: None, .. } => None,
            Self::WithOptions { cwd: Some(p), .. } => {
                let raw = p.to_string_lossy();
                if raw == ALIAS_CWD_WORKSPACE {
                    Some(workspace_path.to_path_buf())
                } else if let Some(rest) = raw
                    .strip_prefix("$WORKSPACE/")
                    .or_else(|| raw.strip_prefix("$WORKSPACE\\"))
                {
                    Some(workspace_path.join(rest))
                } else {
                    Some(p.clone())
                }
            }
        }
    }

    /// Expand `~` in the cwd path, if present. Skips `$WORKSPACE`, which is
    /// resolved later with project context.
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
    /// List of exact executable names that are always denied, regardless of other rules.
    #[serde(default = "default_denied_executables")]
    pub denied_executables: Vec<String>,
    /// List of argument fragments (substrings) that, if present in any command's argv, will cause the command to be denied.
    #[serde(default)]
    pub denied_argument_fragments: Vec<String>,
    /// Optional command aliases expanded server-side for hostdo.
    /// Example: doMyHomeWork = "curl example.com"
    /// Example: tests = { cmd = "cargo test", cwd = "/home/user/project" }
    #[serde(default)]
    pub command_aliases: HashMap<String, AliasValue>,
}

/// Provides the default value for `HostdoDefaults.server_port`.
fn default_exec_port() -> u16 {
    7878
}
/// Provides the default value for `HostdoDefaults.server_host`.
fn default_host() -> String {
    "127.0.0.1".to_string()
}
/// Provides the default value for `HostdoDefaults.token_env_var`.
fn default_token_env() -> String {
    "HARNESS_HAT_TOKEN".to_string()
}
/// Provides the default value for `HostdoDefaults.denied_executables`.
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
    ///   1) transparently redirect outbound HTTP/HTTPS through the harness-hat proxy
    ///   2) install strict outbound egress rules (iptables) to block direct egress
    ///      outside the proxy and exec bridge.
    ///
    /// This is the recommended "near-impossible to bypass" mode.
    ///
    #[serde(default)]
    pub strict_network: bool,
}

/// Provides the default value for `ProxyDefaults.proxy_port`.
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
