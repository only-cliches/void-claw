use anyhow::{Context, Result};
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
    /// This is required at startup and is auto-populated by `void-claw --init`.
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
    /// Path to the global void-claw-rules.toml where auto-approved commands are persisted.
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

fn bool_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiDefaults {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,
}

fn default_sidebar_width() -> u16 {
    24
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
    fn expand_cwd(&mut self) -> Result<()> {
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
    "VOID_CLAW_TOKEN".to_string()
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
    ///   1) transparently redirect outbound HTTP/HTTPS through the void-claw proxy
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
    /// Use when specific endpoints must bypass the void-claw proxy.
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

fn default_mount_target() -> PathBuf {
    PathBuf::from("/workspace")
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    /// Optional OTLP export configuration. Absent = no OTel export.
    pub otlp: Option<OtlpConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_dir: default_log_dir(),
            otlp: None,
        }
    }
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("~/.local/share/void-claw")
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

// ── Rule loading ─────────────────────────────────────────────────────────────

/// Load and compose rules for a specific project (global + that project's
/// void-claw-rules.toml). Called at request time so edits take effect without
/// restart.
pub fn load_composed_rules_for_project(
    config: &Config,
    project_name: Option<&str>,
) -> Result<crate::rules::ComposedRules> {
    let mut errors = Vec::new();

    let global = match crate::rules::load(&config.manager.global_rules_file) {
        Ok(rules) => rules,
        Err(e) => {
            errors.push(format!(
                "global rules '{}': {e}",
                config.manager.global_rules_file.display()
            ));
            crate::rules::ProjectRules::default()
        }
    };

    let mut proj_rules = Vec::new();
    if let Some(project_name) = project_name {
        if let Some(project) = config.projects.iter().find(|p| p.name == project_name) {
            let path = project.canonical_path.join("void-claw-rules.toml");
            match crate::rules::load(&path) {
                Ok(rules) => proj_rules.push(rules),
                Err(e) => {
                    errors.push(format!(
                        "project '{}' rules '{}': {e}",
                        project.name,
                        path.display()
                    ));
                }
            }
        }
    }

    if !errors.is_empty() {
        anyhow::bail!(
            "failed to load one or more rule files:\n{}",
            errors.join("\n")
        );
    }

    Ok(crate::rules::ComposedRules::compose(&global, &proj_rules))
}

// ── Loading ──────────────────────────────────────────────────────────────────

pub fn load(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading config: {}", path.display()))?;
    let mut config: Config =
        toml::from_str(&raw).with_context(|| format!("parsing config: {}", path.display()))?;
    expand_config_paths(&mut config)?;
    validate_docker_dir(&config, path)?;
    resolve_container_profiles(&mut config)?;
    validate(&config)?;
    Ok(config)
}

/// Expand `~` in all path fields so downstream code always sees absolute paths.
fn expand_config_paths(config: &mut Config) -> Result<()> {
    config.manager.global_rules_file = expand_path(&config.manager.global_rules_file)?;
    config.logging.log_dir = expand_path(&config.logging.log_dir)?;
    config.workspace.root = expand_path(&config.workspace.root)?;
    if !config.docker_dir.as_os_str().is_empty() {
        config.docker_dir = expand_path(&config.docker_dir)?;
    }
    for proj in &mut config.projects {
        proj.canonical_path = expand_path(&proj.canonical_path)?;
        if let Some(p) = &proj.workspace_path {
            proj.workspace_path = Some(expand_path(p)?);
        }
        if let Some(he) = &mut proj.hostdo {
            if let Some(aliases) = &mut he.command_aliases {
                for alias in aliases.values_mut() {
                    alias.expand_cwd()?;
                }
            }
        }
    }
    for alias in config.defaults.hostdo.command_aliases.values_mut() {
        alias.expand_cwd()?;
    }
    for ctr in &mut config.containers {
        for mount in &mut ctr.mounts {
            mount.host = expand_path(&mount.host)?;
        }
    }
    if let Some(p) = &config.defaults.containers.mount_target {
        config.defaults.containers.mount_target = Some(expand_path(p)?);
    }
    for mount in &mut config.defaults.containers.mounts {
        mount.host = expand_path(&mount.host)?;
    }
    for profile in config.container_profiles.values_mut() {
        if let Some(p) = &profile.mount_target {
            profile.mount_target = Some(expand_path(p)?);
        }
        for mount in &mut profile.mounts {
            mount.host = expand_path(&mount.host)?;
        }
    }
    Ok(())
}

fn resolve_container_profiles(config: &mut Config) -> Result<()> {
    let defaults = config.defaults.containers.clone();
    let profiles = config.container_profiles.clone();

    for ctr in &mut config.containers {
        let profile_name = ctr
            .profile
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("container '{}': profile is required", ctr.name))?;
        let profile = profiles.get(profile_name).ok_or_else(|| {
            anyhow::anyhow!(
                "container '{}': unknown profile '{}'",
                ctr.name,
                profile_name
            )
        })?;

        // Breaking schema: these fields now come from profiles/defaults.
        anyhow::ensure!(
            ctr.image.trim().is_empty(),
            "container '{}': 'image' is no longer supported; set [container_profiles.{}].image",
            ctr.name,
            profile_name
        );
        anyhow::ensure!(
            ctr.mount_target == default_mount_target(),
            "container '{}': 'mount_target' is no longer supported; set [container_profiles.{}].mount_target",
            ctr.name,
            profile_name
        );
        anyhow::ensure!(
            ctr.agent == AgentKind::None,
            "container '{}': 'agent' is no longer supported; set [container_profiles.{}].agent",
            ctr.name,
            profile_name
        );

        ctr.image = profile.image.clone().ok_or_else(|| {
            anyhow::anyhow!("container profile '{}': image is required", profile_name)
        })?;
        ctr.mount_target = profile
            .mount_target
            .clone()
            .or_else(|| defaults.mount_target.clone())
            .unwrap_or_else(default_mount_target);
        ctr.agent = profile
            .agent
            .clone()
            .or_else(|| defaults.agent.clone())
            .unwrap_or_default();

        ctr.mounts = merge_mounts(&defaults.mounts, &profile.mounts, &ctr.mounts);
        ctr.env_passthrough = merge_unique_strings(
            &defaults.env_passthrough,
            &profile.env_passthrough,
            &ctr.env_passthrough,
        );
        ctr.bypass_proxy = merge_unique_strings(
            &defaults.bypass_proxy,
            &profile.bypass_proxy,
            &ctr.bypass_proxy,
        );
    }

    Ok(())
}

fn validate_docker_dir(config: &Config, config_path: &Path) -> Result<()> {
    anyhow::ensure!(
        !config.docker_dir.as_os_str().is_empty(),
        "config {}: docker_dir is required",
        config_path.display()
    );
    anyhow::ensure!(
        !config.docker_dir.exists() || config.docker_dir.is_dir(),
        "config {}: docker_dir exists but is not a directory: {}",
        config_path.display(),
        config.docker_dir.display()
    );
    Ok(())
}

fn merge_unique_strings(
    base: &[String],
    profile: &[String],
    override_items: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    for s in base.iter().chain(profile).chain(override_items) {
        if !out.iter().any(|v| v == s) {
            out.push(s.clone());
        }
    }
    out
}

fn merge_mounts(
    base: &[ContainerMount],
    profile: &[ContainerMount],
    override_items: &[ContainerMount],
) -> Vec<ContainerMount> {
    let mut out = Vec::new();
    for m in base.iter().chain(profile).chain(override_items) {
        let dup = out.iter().any(|x: &ContainerMount| {
            x.host == m.host && x.container == m.container && x.mode == m.mode
        });
        if !dup {
            out.push(m.clone());
        }
    }
    out
}

fn validate(config: &Config) -> Result<()> {
    for (alias, target) in &config.defaults.hostdo.command_aliases {
        anyhow::ensure!(
            !alias.trim().is_empty(),
            "defaults.hostdo.command_aliases contains an empty alias name"
        );
        anyhow::ensure!(
            !target.cmd().trim().is_empty(),
            "defaults.hostdo.command_aliases.{} has an empty command",
            alias
        );
    }

    let mut seen = std::collections::HashSet::new();
    for proj in &config.projects {
        anyhow::ensure!(
            seen.insert(&proj.name),
            "duplicate project name: {}",
            proj.name
        );
        anyhow::ensure!(
            !proj.canonical_path.as_os_str().is_empty(),
            "project '{}': canonical_path is required",
            proj.name
        );
        anyhow::ensure!(
            proj.canonical_path.exists(),
            "project '{}': canonical_path does not exist: {}",
            proj.name,
            proj.canonical_path.display()
        );
        anyhow::ensure!(
            proj.canonical_path.is_dir(),
            "project '{}': canonical_path is not a directory: {}",
            proj.name,
            proj.canonical_path.display()
        );

        let effective_mode = effective_sync_mode(proj, &config.defaults);
        if effective_mode == SyncMode::Direct {
            anyhow::ensure!(
                !proj.disposable,
                "project '{}': disposable=true is not allowed with projects.sync.mode='direct' (it would allow deleting the canonical directory)",
                proj.name
            );
            if let Some(p) = &proj.workspace_path {
                anyhow::ensure!(
                    p == &proj.canonical_path,
                    "project '{}': workspace_path must be omitted (or equal canonical_path) with projects.sync.mode='direct' (got workspace_path={})",
                    proj.name,
                    p.display()
                );
            }
        }
        if let Some(he) = &proj.hostdo {
            if let Some(aliases) = &he.command_aliases {
                for (alias, target) in aliases {
                    anyhow::ensure!(
                        !alias.trim().is_empty(),
                        "project '{}': hostdo.command_aliases contains an empty alias name",
                        proj.name
                    );
                    anyhow::ensure!(
                        !target.cmd().trim().is_empty(),
                        "project '{}': hostdo.command_aliases.{} has an empty command",
                        proj.name,
                        alias
                    );
                }
            }
        }
    }
    let mut seen_containers = std::collections::HashSet::new();
    for ctr in &config.containers {
        anyhow::ensure!(
            seen_containers.insert(&ctr.name),
            "duplicate container name: {}",
            ctr.name
        );
        for mount in &ctr.mounts {
            anyhow::ensure!(
                !mount.host.as_os_str().is_empty(),
                "container '{}': mount.host must not be empty",
                ctr.name
            );
            anyhow::ensure!(
                !mount.container.as_os_str().is_empty(),
                "container '{}': mount.container must not be empty",
                ctr.name
            );
            anyhow::ensure!(
                mount.container.is_absolute(),
                "container '{}': mount.container must be an absolute path: {}",
                ctr.name,
                mount.container.display()
            );
        }
        for name in &ctr.env_passthrough {
            anyhow::ensure!(
                !name.trim().is_empty(),
                "container '{}': env_passthrough contains an empty name",
                ctr.name
            );
            anyhow::ensure!(
                !name.contains('='),
                "container '{}': env_passthrough must be env var names only (no '='): {}",
                ctr.name,
                name
            );
        }
        for host in &ctr.bypass_proxy {
            anyhow::ensure!(
                !host.trim().is_empty(),
                "container '{}': bypass_proxy contains an empty host",
                ctr.name
            );
        }
    }
    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Expand `~` at the start of a path.
pub fn expand_path(path: &Path) -> Result<PathBuf> {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        Ok(home.join(rest))
    } else if s == "~" {
        dirs::home_dir().context("cannot determine home directory")
    } else {
        Ok(path.to_path_buf())
    }
}

/// Effective workspace path for a project (managed workspace copy).
///
/// For direct-mount projects, use `effective_mount_source_path`.
pub fn effective_workspace_path(proj: &ProjectConfig, ws: &WorkspaceSection) -> PathBuf {
    proj.workspace_path
        .clone()
        .unwrap_or_else(|| ws.root.join(&proj.name))
}

/// Host-side directory that should be mounted into the container at `mount_target`.
pub fn effective_mount_source_path(
    proj: &ProjectConfig,
    ws: &WorkspaceSection,
    defaults: &DefaultsConfig,
) -> PathBuf {
    match effective_sync_mode(proj, defaults) {
        SyncMode::Direct => proj.canonical_path.clone(),
        _ => effective_workspace_path(proj, ws),
    }
}

/// Effective sync mode for a project.
pub fn effective_sync_mode(proj: &ProjectConfig, defaults: &DefaultsConfig) -> SyncMode {
    proj.sync
        .as_ref()
        .and_then(|s| s.mode.clone())
        .unwrap_or_else(|| defaults.sync.mode.clone())
}

/// Combined exclude patterns (global + per-project config + per-project rules).
pub fn combined_excludes(proj: &ProjectConfig, defaults: &DefaultsConfig) -> Result<Vec<String>> {
    let mut patterns = defaults.sync.global_exclude_patterns.clone();
    patterns.extend(proj.exclude_patterns.iter().cloned());
    let rules_path = proj.canonical_path.join("void-claw-rules.toml");
    let rules = crate::rules::load(&rules_path)
        .with_context(|| format!("loading project excludes from {}", rules_path.display()))?;
    patterns.extend(rules.exclude_patterns);
    Ok(patterns)
}

/// Effective denied executables.
pub fn effective_denied_executables(
    proj: &ProjectConfig,
    defaults: &DefaultsConfig,
) -> Vec<String> {
    proj.hostdo
        .as_ref()
        .and_then(|he| he.denied_executables.clone())
        .unwrap_or_else(|| defaults.hostdo.denied_executables.clone())
}

/// Effective denied argument fragments.
pub fn effective_denied_fragments(proj: &ProjectConfig, defaults: &DefaultsConfig) -> Vec<String> {
    proj.hostdo
        .as_ref()
        .and_then(|he| he.denied_argument_fragments.clone())
        .unwrap_or_else(|| defaults.hostdo.denied_argument_fragments.clone())
}

/// Effective hostdo command aliases for a project.
/// Merge order (later wins): global defaults → per-project config → per-project rules.
pub fn effective_command_aliases(
    proj: &ProjectConfig,
    defaults: &DefaultsConfig,
) -> HashMap<String, AliasValue> {
    let mut out = defaults.hostdo.command_aliases.clone();
    if let Some(project_aliases) = proj
        .hostdo
        .as_ref()
        .and_then(|he| he.command_aliases.clone())
    {
        out.extend(project_aliases);
    }
    // Layer on aliases from the project's void-claw-rules.toml (highest priority).
    let rules_path = proj.canonical_path.join("void-claw-rules.toml");
    if let Ok(rules) = crate::rules::load(&rules_path) {
        if !rules.hostdo.command_aliases.is_empty() {
            out.extend(rules.hostdo.command_aliases);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Config, DefaultsConfig, combined_excludes, load_composed_rules_for_project};
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
    fn defaults_sidebar_width_defaults_to_24() {
        assert_eq!(DefaultsConfig::default().ui.sidebar_width, 24);
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
        let cfg = super::load(&cfg_path).expect("config should load");
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
            project_path.join("void-claw-rules.toml"),
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
        let cfg_path = root.join("void-claw.toml");
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
        let err = super::load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string().contains("canonical_path does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_fails_when_docker_dir_is_missing() {
        let root = unique_temp_dir("missing-docker-dir");
        let cfg_path = root.join("void-claw.toml");
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
        let err = super::load(&cfg_path).expect_err("config load should fail");
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
        let cfg = super::load(&cfg_path).expect("config should load");
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
        let err = super::load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string().contains("docker_dir exists but is not a directory"),
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
        let cfg = super::load(&cfg_path).expect("config should load");
        assert!(cfg.projects.is_empty());
    }

    #[test]
    fn load_fails_when_direct_mode_has_disposable_true() {
        let root = unique_temp_dir("direct-disposable");
        let cfg_path = root.join("void-claw.toml");
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
        let err = super::load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string().contains("disposable=true is not allowed with projects.sync.mode='direct'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn load_fails_when_direct_mode_has_explicit_workspace_path() {
        let root = unique_temp_dir("direct-workspace-path");
        let cfg_path = root.join("void-claw.toml");
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
        let err = super::load(&cfg_path).expect_err("config load should fail");
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
            project_path.join("void-claw-rules.toml"),
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
