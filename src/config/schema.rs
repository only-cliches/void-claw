use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{AliasValue, ApprovalMode, bool_true};

// ── Projects ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub canonical_path: PathBuf,
    /// Defaults to `workspace.root/<name>` when absent.
    pub workspace_path: Option<PathBuf>,
    #[serde(default = "bool_true")]
    pub disposable: bool,
    #[serde(default)]
    pub default_policy: ApprovalMode,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    pub sync: Option<SyncOverride>,
    pub hostdo: Option<ProjectHostdo>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            canonical_path: PathBuf::new(),
            workspace_path: None,
            disposable: true,
            default_policy: ApprovalMode::default(),
            exclude_patterns: Vec::new(),
            sync: None,
            hostdo: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SyncOverride {
    pub mode: Option<SyncMode>,
    pub delete_propagation: Option<bool>,
    pub rename_propagation: Option<bool>,
    pub symlink_policy: Option<SymlinkPolicy>,
    pub conflict_policy: Option<ConflictPolicy>,
}

// ── Containers ───────────────────────────────────────────────────────────────

/// Which AI agent CLI is installed in a container image.
/// Used to generate the right config files (CLAUDE.md, settings.json, etc.)
/// into the workspace before the container starts.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// No agent — skip config file injection.
    #[default]
    None,
    /// Claude Code CLI (`@anthropic-ai/claude-code`).
    Claude,
    /// OpenAI Codex CLI (`@openai/codex`).
    Codex,
    /// Google Gemini CLI (`@google/gemini-cli`).
    Gemini,
    /// opencode (`opencode-ai`).
    Opencode,
}

/// A named container definition.  Lives in `[[containers]]` at the top level
/// of the config file.  Containers are environment definitions — which project
/// workspace to mount is chosen at launch time in the TUI.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContainerDef {
    /// Human-readable identifier shown in the TUI tab bar.
    pub name: String,
    /// Optional profile key from `[container_profiles.<name>]`.
    #[serde(default)]
    pub profile: Option<String>,
    /// Docker image to run.
    #[serde(default)]
    pub image: String,
    /// Path inside the container where the project workspace is mounted.
    /// Defaults to `/workspace`.
    #[serde(default = "default_mount_target")]
    pub mount_target: PathBuf,
    /// Which agent CLI is installed in this image.  Controls which config
    /// files (CLAUDE.md, settings.json, AGENTS.md, etc.) are written into
    /// the workspace at launch time.
    #[serde(default)]
    pub agent: AgentKind,
    /// Extra host paths to mount into the container (for auth/session reuse).
    #[serde(default)]
    pub mounts: Vec<ContainerMount>,
    /// Host env var names to pass through with `docker run -e NAME`.
    #[serde(default)]
    pub env_passthrough: Vec<String>,
    /// Hostnames/domains to add to NO_PROXY for this container.
    /// Use when specific endpoints must bypass the agent-zero proxy.
    #[serde(default)]
    pub bypass_proxy: Vec<String>,
}

/// Named container profile used to reduce duplication in `[[containers]]`.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ContainerProfile {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub mount_target: Option<PathBuf>,
    #[serde(default)]
    pub agent: Option<AgentKind>,
    #[serde(default)]
    pub mounts: Vec<ContainerMount>,
    #[serde(default)]
    pub env_passthrough: Vec<String>,
    #[serde(default)]
    pub bypass_proxy: Vec<String>,
}

/// Shared defaults merged into every container definition.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ContainerDefaults {
    #[serde(default)]
    pub mount_target: Option<PathBuf>,
    #[serde(default)]
    pub agent: Option<AgentKind>,
    #[serde(default)]
    pub mounts: Vec<ContainerMount>,
    #[serde(default)]
    pub env_passthrough: Vec<String>,
    #[serde(default)]
    pub bypass_proxy: Vec<String>,
}

pub(crate) fn default_mount_target() -> PathBuf {
    PathBuf::from("/workspace")
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ContainerMount {
    /// Host-side source path (supports `~` expansion).
    pub host: PathBuf,
    /// Container target path.
    pub container: PathBuf,
    /// Mount mode: `ro` or `rw` (default).
    #[serde(default)]
    pub mode: MountMode,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MountMode {
    Ro,
    #[default]
    Rw,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectHostdo {
    pub denied_executables: Option<Vec<String>>,
    pub denied_argument_fragments: Option<Vec<String>>,
    pub command_aliases: Option<HashMap<String, AliasValue>>,
}

// ── Enums ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    WorkspaceOnly,
    Pushback,
    Bidirectional,
    Pullthrough,
    Direct,
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Pushback
    }
}

impl std::fmt::Display for SyncMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceOnly => write!(f, "workspace-only"),
            Self::Pushback => write!(f, "pushback"),
            Self::Bidirectional => write!(f, "bidirectional"),
            Self::Pullthrough => write!(f, "pullthrough"),
            Self::Direct => write!(f, "direct"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SymlinkPolicy {
    Reject,
    Copy,
    Follow,
}

impl Default for SymlinkPolicy {
    fn default() -> Self {
        Self::Reject
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    PreserveCanonical,
    PreserveWorkspace,
}

impl Default for ConflictPolicy {
    fn default() -> Self {
        Self::PreserveCanonical
    }
}

// ── Logging ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    /// Directory for runtime logs and local runtime state files.
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
    /// Stable instance identifier persisted into `agent-zero.toml`.
    /// Used as `service.instance.id` in OpenTelemetry exports.
    #[serde(default)]
    pub instance_id: Option<String>,
    /// Optional OTLP export configuration. Absent = no OTel export.
    pub otlp: Option<OtlpConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_dir: default_log_dir(),
            instance_id: None,
            otlp: None,
        }
    }
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("~/.local/share/agent-zero")
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OtlpConfig {
    /// Collector endpoint, e.g. `http://localhost:4317` (gRPC) or
    /// `http://localhost:4318/v1/traces` (HTTP/proto).
    pub endpoint: String,
    #[serde(default)]
    pub protocol: OtlpProtocol,
    #[serde(default)]
    pub level: AuditExportLevel,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OtlpProtocol {
    #[default]
    Grpc,
    Http,
}

/// Which events to emit as OTel spans.
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditExportLevel {
    /// Every hostdo / HTTP event (including auto-approved).
    All,
    /// Only events that required a manual developer approval prompt.
    #[default]
    Approvals,
    /// No OTel spans emitted.
    None,
}
