# Void Claw Source Code
## src/agents.rs

```rs
use anyhow::Context;
use std::path::Path;

use crate::config::AgentKind;
use crate::rules::{
    ApprovalMode, HostdoRules, NetworkPolicy, NetworkRule, NetworkRules, ProjectRules,
};
// ── void-rules.toml starter ───────────────────────────────────────────────────

/// Generate a starter `void-rules.toml` for the given agent kind.
///
/// Includes common-sense `auto`-approved rules for developer tools (GitHub,
/// npm, PyPI, crates.io) plus agent-specific API domains.  The default policy
/// for anything not listed is `prompt`, so the developer still sees and
/// approves unexpected destinations.
/// Build the initial `void-rules.toml` template for a given agent runtime.
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
        let hosts: Vec<&str> = rules
            .network
            .rules
            .iter()
            .map(|rule| rule.host.as_str())
            .collect();
        assert!(hosts.contains(&"generativelanguage.googleapis.com"));
        assert!(hosts.contains(&"accounts.google.com"));
        assert!(hosts.contains(&"oauth2.googleapis.com"));
    }
}

```

## src/ca.rs

```rs
/// Certificate Authority management for the void-claw MITM proxy.
///
/// Generates a self-signed CA on first run and persists it to disk.
/// Derives per-domain leaf certificates on demand (cached in memory).
/// The CA cert PEM is exposed so it can be injected into containers.
use anyhow::{Context, Result};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct CaStore {
    /// CA cert PEM — inject this into containers so they trust the proxy.
    pub cert_pem: String,
    ca_key: KeyPair,
    /// Reconstructed CA cert for signing leaf certs (may differ in validity
    /// period from the on-disk cert, but uses the same key and DN).
    ca_cert_for_signing: rcgen::Certificate,
    /// Original CA cert DER — included in leaf cert chains so TLS clients
    /// can verify the chain against what they imported.
    ca_cert_der: Vec<u8>,
    cert_cache: Mutex<HashMap<String, Arc<ServerConfig>>>,
}

impl CaStore {
    /// Load the CA from `dir`, or generate and persist a new one.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let cert_path = dir.join("ca.crt");
        let key_path = dir.join("ca.key");

        if cert_path.exists() && key_path.exists() {
            return Self::load(&cert_path, &key_path);
        }
        Self::generate_and_save(&cert_path, &key_path)
    }

    fn load(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let cert_pem = std::fs::read_to_string(cert_path)
            .with_context(|| format!("reading {}", cert_path.display()))?;
        let key_pem = std::fs::read_to_string(key_path)
            .with_context(|| format!("reading {}", key_path.display()))?;

        let ca_key = KeyPair::from_pem(&key_pem).context("parsing CA private key")?;

        // Reconstruct a signable Certificate from the same DN (fixed values).
        let ca_cert_for_signing = Self::build_ca_cert(&ca_key)?;

        // Extract the original DER bytes from the PEM for chain inclusion.
        let ca_cert_der = Self::pem_to_der(&cert_pem)?;

        Ok(Self {
            cert_pem,
            ca_key,
            ca_cert_for_signing,
            ca_cert_der,
            cert_cache: Mutex::new(HashMap::new()),
        })
    }

    fn generate_and_save(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let ca_key = KeyPair::generate().context("generating CA key pair")?;
        let ca_cert = Self::build_ca_cert(&ca_key)?;

        let cert_pem = ca_cert.pem();
        let key_pem = ca_key.serialize_pem();
        let ca_cert_der = ca_cert.der().to_vec();

        std::fs::write(cert_path, &cert_pem)
            .with_context(|| format!("writing {}", cert_path.display()))?;
        std::fs::write(key_path, &key_pem)
            .with_context(|| format!("writing {}", key_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("setting permissions on {}", key_path.display()))?;
        }

        Ok(Self {
            cert_pem,
            ca_key,
            ca_cert_for_signing: ca_cert,
            ca_cert_der,
            cert_cache: Mutex::new(HashMap::new()),
        })
    }

    fn build_ca_cert(key: &KeyPair) -> Result<rcgen::Certificate> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "Void Claw Proxy CA");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "void-claw");
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2124, 1, 1);
        params.self_signed(key).context("generating CA certificate")
    }

    fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
        let mut buf = pem.as_bytes();
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut buf)
            .collect::<std::result::Result<_, _>>()
            .context("parsing CA cert PEM")?;
        certs
            .into_iter()
            .next()
            .map(|c: CertificateDer<'static>| c.to_vec())
            .context("no certificate found in CA PEM")
    }

    /// Return (or generate and cache) a rustls `ServerConfig` presenting a
    /// leaf certificate for `domain`, signed by this CA.
    pub fn leaf_server_config(&self, domain: &str) -> Result<Arc<ServerConfig>> {
        {
            let cache = self.cert_cache.lock().unwrap();
            if let Some(cfg) = cache.get(domain) {
                return Ok(Arc::clone(cfg));
            }
        }

        let leaf_key = KeyPair::generate().context("generating leaf key")?;
        let mut params =
            CertificateParams::new(vec![domain.to_string()]).context("building leaf params")?;
        params.is_ca = IsCa::NoCa;
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 1, 1);

        let leaf_cert = params
            .signed_by(&leaf_key, &self.ca_cert_for_signing, &self.ca_key)
            .context("signing leaf certificate")?;

        // Chain: leaf + original CA cert (what the container's trust store knows).
        let cert_chain: Vec<CertificateDer<'static>> = vec![
            CertificateDer::from(leaf_cert.der().to_vec()),
            CertificateDer::from(self.ca_cert_der.clone()),
        ];

        // Private key for the leaf cert.
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));

        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key_der)
            .context("building leaf ServerConfig")?;

        let config = Arc::new(server_config);
        self.cert_cache
            .lock()
            .unwrap()
            .insert(domain.to_string(), config.clone());
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_or_create_is_idempotent() {
        let dir = tempdir().expect("create temp dir");
        let path = dir.path();

        // 1. Create for the first time
        let store1 = CaStore::load_or_create(path).expect("first create");
        let cert1 = store1.cert_pem.clone();
        assert!(path.join("ca.crt").exists());
        assert!(path.join("ca.key").exists());

        // 2. Load again from the same dir
        let store2 = CaStore::load_or_create(path).expect("second load");
        assert_eq!(
            store2.cert_pem, cert1,
            "CA certificate should be persistent"
        );
    }

    #[test]
    fn leaf_server_config_caches_results() {
        let dir = tempdir().expect("create temp dir");
        let store = CaStore::load_or_create(dir.path()).expect("create store");

        let config1 = store.leaf_server_config("example.com").expect("first leaf");
        let config2 = store
            .leaf_server_config("example.com")
            .expect("second leaf");

        assert!(
            Arc::ptr_eq(&config1, &config2),
            "server configs should be cached"
        );

        let config3 = store
            .leaf_server_config("other.com")
            .expect("different domain");
        assert!(
            !Arc::ptr_eq(&config1, &config3),
            "different domains should have different configs"
        );
    }
}

```

## src/cli.rs

```rs
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "void-claw",
    version,
    about = "LLM agent workspace manager — safely exposes filtered project workspaces to AI coding agents"
)]
pub struct Cli {
    /// Path to config file. Starts the interactive workspace manager.
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Generate a sample config file. Defaults to ./void-claw.toml if no path is given.
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        default_missing_value = "void-claw.toml"
    )]
    pub init: Option<PathBuf>,
}

```

## src/config/core.rs

```rs
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
            // `docker_dir` is expected to be populated during the `void-claw --init` process.
            // An empty PathBuf here signifies an uninitialized state.
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
    /// Path to the global void-rules.toml where auto-approved commands are persisted.
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

/// Defines common files/directories to exclude from workspace synchronization.
/// These are typically sensitive files or build artifacts.
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
    "VOID_CLAW_TOKEN".to_string()
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
    ///   1) transparently redirect outbound HTTP/HTTPS through the void-claw proxy
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

```

## src/config/load.rs

```rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, value};
use tracing::instrument;

use crate::config::{
    AgentKind, AliasValue, Config, ContainerMount, DefaultsConfig, ProjectConfig, SyncMode,
    WorkspaceSection, default_mount_target,
};

// ── Rule loading ─────────────────────────────────────────────────────────────

/// Load and compose rules for a specific project (global + that project's
/// void-rules.toml). Called at request time so edits take effect without
/// restart.
#[instrument(skip(config))]
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
            let path = project.canonical_path.join("void-rules.toml");
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

#[instrument(skip(path))]
pub fn load(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading config: {}", path.display()))?;
    let mut config: Config =
        toml::from_str(&raw).with_context(|| format!("parsing config: {}", path.display()))?;
    expand_config_paths(&mut config)?;
    validate_docker_dir(&config, path)?;
    resolve_container_profiles(&mut config)?;
    validate(&config)?;
    ensure_logging_instance_id(path, &raw, &mut config)?;
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

#[instrument(skip(config, config_path))]
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

pub(crate) fn merge_unique_strings(
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

pub(crate) fn merge_mounts(
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

fn ensure_logging_instance_id(path: &Path, raw: &str, config: &mut Config) -> Result<()> {
    let current = config
        .logging
        .instance_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Some(instance_id) = current {
        config.logging.instance_id = Some(instance_id);
        return Ok(());
    }

    let instance_id = uuid::Uuid::new_v4().to_string();
    let mut doc: DocumentMut = raw
        .parse()
        .with_context(|| format!("parsing config document: {}", path.display()))?;
    doc["logging"]["instance_id"] = value(instance_id.clone());
    std::fs::write(path, doc.to_string())
        .with_context(|| format!("writing config: {}", path.display()))?;
    config.logging.instance_id = Some(instance_id);
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
#[instrument(skip(proj, ws))]
pub fn effective_workspace_path(proj: &ProjectConfig, ws: &WorkspaceSection) -> PathBuf {
    proj.workspace_path
        .clone()
        .unwrap_or_else(|| ws.root.join(&proj.name))
}

/// Host-side directory that should be mounted into the container at `mount_target`.
#[instrument(skip(proj, ws, defaults))]
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
#[instrument(skip(proj, defaults))]
pub fn effective_sync_mode(proj: &ProjectConfig, defaults: &DefaultsConfig) -> SyncMode {
    proj.sync
        .as_ref()
        .and_then(|s| s.mode.clone())
        .unwrap_or_else(|| defaults.sync.mode.clone())
}

/// Combined exclude patterns (global + per-project config + per-project rules).
#[instrument(skip(proj, defaults))]
pub fn combined_excludes(proj: &ProjectConfig, defaults: &DefaultsConfig) -> Result<Vec<String>> {
    let mut patterns = defaults.sync.global_exclude_patterns.clone();
    patterns.extend(proj.exclude_patterns.iter().cloned());
    let rules_path = proj.canonical_path.join("void-rules.toml");
    let rules = crate::rules::load(&rules_path)
        .with_context(|| format!("loading project excludes from {}", rules_path.display()))?;
    patterns.extend(rules.exclude_patterns);
    Ok(patterns)
}

/// Effective denied executables.
#[instrument(skip(proj, defaults))]
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
#[instrument(skip(proj, defaults))]
pub fn effective_denied_fragments(proj: &ProjectConfig, defaults: &DefaultsConfig) -> Vec<String> {
    proj.hostdo
        .as_ref()
        .and_then(|he| he.denied_argument_fragments.clone())
        .unwrap_or_else(|| defaults.hostdo.denied_argument_fragments.clone())
}

/// Effective hostdo command aliases for a project.
/// Merge order (later wins): global defaults → per-project config → per-project rules.
#[instrument(skip(proj, defaults))]
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
    // Layer on aliases from the project's void-rules.toml (highest priority).
    let rules_path = proj.canonical_path.join("void-rules.toml");
    if let Ok(rules) = crate::rules::load(&rules_path) {
        if !rules.hostdo.command_aliases.is_empty() {
            out.extend(rules.hostdo.command_aliases);
        }
    }
    out
}

```

## src/config/mod.rs

```rs
mod core;
mod load;
mod schema;

pub use core::*;
pub use load::*;
pub use schema::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests_file;

```

## src/config/schema.rs

```rs
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
    /// Stable instance identifier persisted into `void-claw.toml`.
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

```

## src/config/tests.rs

```rs
#[cfg(test)]
mod tests {
    use crate::config::{
        Config, ContainerMount, DefaultsConfig, MountMode, combined_excludes, load,
        load_composed_rules_for_project, merge_mounts, merge_unique_strings,
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
            project_path.join("void-rules.toml"),
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
        let err = load(&cfg_path).expect_err("config load should fail");
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
        let cfg = load(&cfg_path).expect("config should load");
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
        let err = load(&cfg_path).expect_err("config load should fail");
        assert!(
            err.to_string()
                .contains("disposable=true is not allowed with projects.sync.mode='direct'"),
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
        let err = load(&cfg_path).expect_err("config load should fail");
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
            project_path.join("void-rules.toml"),
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
}

```

## src/container/core.rs

```rs
use alacritty_terminal::event::{Event, EventListener, Notify, OnResize, WindowSize};
use alacritty_terminal::event_loop::Msg;
use alacritty_terminal::event_loop::Notifier;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
/// Container session management.
///
/// Each running container gets a `ContainerSession` that owns a PTY process
/// (`docker run -it …`) and a `vt100::Parser` screen buffer updated in
/// real-time by a background reader thread.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tempfile::NamedTempFile;

use crate::config::{ContainerMount, MountMode};
use crate::container::helpers::{blend_toward_bg, luma_u8, xterm_256_index_to_rgb};
use tracing::instrument;

/// Live container session state and PTY plumbing for a launched container.
pub struct ContainerSession {
    pub container_name: String,
    pub container_id: String,
    pub docker_name: String,
    pub project: String,
    pub session_token: String,
    pub mount_target: String,
    pub launched_at: Instant,
    pub term: Arc<FairMutex<Term<SessionEventProxy>>>,
    pub(crate) notifier: Notifier,
    pub(crate) window_size: Arc<Mutex<WindowSize>>,
    pub exited: Arc<AtomicBool>,
    pub has_bell: Arc<AtomicBool>,
    pub exit_reported: bool,
    pub(crate) _scoped_proxy: Option<crate::proxy::ScopedProxyListener>,
    pub(crate) _cred_tempfile: Option<NamedTempFile>,
    pub(crate) _env_tempfile: Option<NamedTempFile>,
}

/// Event sink that keeps the Alacritty-backed PTY state synchronized with the
/// event loop and the UI.
#[derive(Clone)]
pub struct SessionEventProxy {
    pub(crate) sender: Arc<Mutex<Option<alacritty_terminal::event_loop::EventLoopSender>>>,
    pub(crate) window_size: Arc<Mutex<WindowSize>>,
    pub(crate) exited: Arc<AtomicBool>,
    pub(crate) has_bell: Arc<AtomicBool>,
    pub(crate) default_fg: alacritty_terminal::vte::ansi::Rgb,
    pub(crate) default_bg: alacritty_terminal::vte::ansi::Rgb,
    pub(crate) grayscale_palette: bool,
}

impl EventListener for SessionEventProxy {
    fn send_event(&self, event: Event) {
        let sender = self.sender.lock().ok().and_then(|s| s.clone());
        match event {
            Event::Bell => {
                self.has_bell.store(true, Ordering::Relaxed);
            }
            Event::Exit | Event::ChildExit(_) => {
                self.exited.store(true, Ordering::Relaxed);
            }
            Event::PtyWrite(s) => {
                if let Some(tx) = sender {
                    let _ = tx.send(Msg::Input(s.into_bytes().into()));
                }
            }
            Event::TextAreaSizeRequest(formatter) => {
                if let Some(tx) = sender {
                    let size = self.window_size.lock().map(|s| *s).unwrap_or(WindowSize {
                        num_lines: 24,
                        num_cols: 80,
                        cell_width: 0,
                        cell_height: 0,
                    });
                    let _ = tx.send(Msg::Input(formatter(size).into_bytes().into()));
                }
            }
            Event::ColorRequest(index, formatter) => {
                if let Some(tx) = sender {
                    let rgb = if index == 10 {
                        self.default_fg
                    } else if index == 11 {
                        self.default_bg
                    } else {
                        let (r, g, b) = xterm_256_index_to_rgb(index as u8);
                        let (r, g, b) = if self.grayscale_palette {
                            let y = luma_u8((r, g, b));
                            (y, y, y)
                        } else {
                            (r, g, b)
                        };
                        let blend_weight = if self.grayscale_palette { 0.45 } else { 0.35 };
                        let (r, g, b) = blend_toward_bg(
                            (r, g, b),
                            (self.default_bg.r, self.default_bg.g, self.default_bg.b),
                            blend_weight,
                        );
                        alacritty_terminal::vte::ansi::Rgb { r, g, b }
                    };
                    let _ = tx.send(Msg::Input(formatter(rgb).into_bytes().into()));
                }
            }
            Event::ClipboardLoad(_ty, formatter) => {
                if let Some(tx) = sender {
                    let _ = tx.send(Msg::Input(formatter("").into_bytes().into()));
                }
            }
            _ => {}
        }
    }
}

/// Helper struct to implement `alacritty_terminal::grid::Dimensions` for resizing the terminal view.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TermSize {
    pub(crate) cols: usize,
    pub(crate) lines: usize,
}

impl Dimensions for TermSize {
    fn columns(&self) -> usize {
        self.cols
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn last_column(&self) -> alacritty_terminal::index::Column {
        alacritty_terminal::index::Column(self.cols.saturating_sub(1))
    }
    fn topmost_line(&self) -> alacritty_terminal::index::Line {
        alacritty_terminal::index::Line(0)
    }
    fn bottommost_line(&self) -> alacritty_terminal::index::Line {
        alacritty_terminal::index::Line(self.lines.saturating_sub(1) as i32)
    }
    fn history_size(&self) -> usize {
        0
    }
}

impl ContainerSession {
    /// Checks if the container session has exited.
    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }
    /// Checks if the terminal has received a bell event.
    pub fn has_bell(&self) -> bool {
        self.has_bell.load(Ordering::Relaxed)
    }
    /// Resets the bell status for the terminal.
    pub fn clear_bell(&self) {
        self.has_bell.store(false, Ordering::Relaxed);
    }
    /// Sends input bytes to the container's PTY, mimicking user input.
    pub fn send_input(&self, bytes: Vec<u8>) {
        self.notifier.notify(bytes);
    }

    /// Forcibly terminates the Docker container associated with this session.
    ///
    /// Uses `docker rm -f` to stop and remove the container. The command's output
    /// is discarded, and any errors during `docker rm` are suppressed.
    pub fn terminate(&self) {
        let target = if !self.container_id.is_empty() {
            &self.container_id
        } else {
            &self.docker_name
        };
        if target.is_empty() || target == "unknown" {
            return;
        }
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", target])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    /// Resizes the terminal window within the container.
    ///
    /// This updates the internal window size and notifies the PTY of the change,
    /// which helps the running process inside the container adjust its output.
    pub fn resize(&mut self, rows: u16, cols: u16) -> anyhow::Result<()> {
        if let Ok(size) = self.window_size.lock() {
            if size.num_lines == rows && size.num_cols == cols {
                return Ok(());
            }
        }
        let ws = WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 0,
            cell_height: 0,
        };
        if let Ok(mut s) = self.window_size.lock() {
            *s = ws;
        }
        self.notifier.on_resize(ws);
        let mut term = self.term.lock();
        term.resize(TermSize {
            cols: cols as usize,
            lines: rows as usize,
        });
        Ok(())
    }

    /// Generates a human-readable label for the TUI tab, combining container and project names.
    pub fn tab_label(&self) -> String {
        format!("{} @ {}", self.container_name, self.project)
    }
}

/// Rewrites loopback addresses (127.0.0.1, localhost, 0.0.0.0) to `host.docker.internal`
/// for reliable container-to-host communication.
#[instrument(level = "trace", skip(url))]
pub(crate) fn loopback_to_host_docker(url: &str) -> String {
    url.replace("127.0.0.1", "host.docker.internal")
        .replace("localhost", "host.docker.internal")
        .replace("0.0.0.0", "host.docker.internal")
}

/// Convert an arbitrary project or container name into a Docker-safe name.
#[instrument(level = "trace", skip(input))]
pub fn sanitize_docker_name(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    if out.is_empty() {
        "container".to_string()
    } else {
        out
    }
}

pub(crate) fn mount_mode_arg(mode: &MountMode) -> &'static str {
    match mode {
        MountMode::Ro => "ro",
        MountMode::Rw => "rw",
    }
}

pub(crate) fn find_codex_home_container_path(mounts: &[ContainerMount]) -> Option<&Path> {
    mounts.iter().find_map(|mount| {
        (mount.container == PathBuf::from("/home/ubuntu/.codex")
            || mount.container == PathBuf::from("/root/.codex"))
        .then_some(mount.container.as_path())
    })
}

pub(crate) fn mounts_include_codex_session_state(mounts: &[ContainerMount]) -> bool {
    mounts.iter().any(|mount| {
        let container = mount.container.to_string_lossy();
        container.contains(".codex")
            || container.contains(".config/codex")
            || container.contains("codex")
    })
}

pub(crate) fn append_codex_home_args(
    docker_args: &mut Vec<String>,
    host_path: &Path,
) -> Result<()> {
    let codex_home = host_path.join(".codex");
    std::fs::create_dir_all(&codex_home).with_context(|| {
        format!(
            "failed to create codex home directory at {}",
            codex_home.display()
        )
    })?;
    let container_path = "/home/ubuntu/.codex";
    docker_args.push("-e".to_string());
    docker_args.push(format!("CODEX_HOME={container_path}"));
    docker_args.push("-v".to_string());
    docker_args.push(format!("{}:{container_path}:rw", codex_home.display()));
    Ok(())
}

pub(crate) fn find_gemini_home_container_path(mounts: &[ContainerMount]) -> Option<&Path> {
    mounts.iter().find_map(|mount| {
        (mount.container == PathBuf::from("/home/ubuntu/.gemini")
            || mount.container == PathBuf::from("/root/.gemini"))
        .then_some(mount.container.as_path())
    })
}

pub(crate) fn mounts_include_gemini_session_state(mounts: &[ContainerMount]) -> bool {
    mounts.iter().any(|mount| {
        let container = mount.container.to_string_lossy();
        container.contains(".gemini") || container.contains(".config/gemini")
    })
}

pub(crate) fn append_gemini_home_args(
    docker_args: &mut Vec<String>,
    host_path: &Path,
) -> Result<()> {
    let gemini_home = host_path.join(".gemini");
    std::fs::create_dir_all(&gemini_home).with_context(|| {
        format!(
            "failed to create gemini home directory at {}",
            gemini_home.display()
        )
    })?;
    for container_path in ["/home/ubuntu/.gemini", "/root/.gemini"] {
        docker_args.push("-v".to_string());
        docker_args.push(format!("{}:{container_path}:rw", gemini_home.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::append_gemini_home_args;

    #[test]
    fn gemini_home_args_mounts_both_possible_homes() {
        let root =
            std::env::temp_dir()
                .join(format!("void-claw-gemini-home-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let mut args = Vec::new();
        append_gemini_home_args(&mut args, &root).expect("append gemini args");

        let mounts: Vec<String> = args
            .chunks_exact(2)
            .filter_map(|chunk| {
                if chunk[0] == "-v" {
                    Some(chunk[1].clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            mounts
                .iter()
                .any(|m| m.ends_with(":/home/ubuntu/.gemini:rw"))
        );
        assert!(mounts.iter().any(|m| m.ends_with(":/root/.gemini:rw")));
    }

    #[test]
    fn compose_no_proxy_handles_empty_and_duplicates() {
        use crate::container::helpers::compose_no_proxy;
        let bypass = vec![
            "google.com".to_string(),
            "  ".to_string(),
            "localhost".to_string(),
        ];
        let out = compose_no_proxy(&bypass);
        // Default: localhost,127.0.0.1,host.docker.internal
        assert!(out.contains("localhost"));
        assert!(out.contains("127.0.0.1"));
        assert!(out.contains("host.docker.internal"));
        assert!(out.contains("google.com"));
        // "localhost" was already there, "  " should be ignored
        assert_eq!(out.split(',').filter(|&s| s == "localhost").count(), 1);
        assert!(!out.contains("  "));
    }

    #[test]
    fn sanitize_docker_name_works() {
        use super::sanitize_docker_name;
        assert_eq!(sanitize_docker_name("my project"), "my-project");
        assert_eq!(sanitize_docker_name("my@proj!ect"), "my-proj-ect");
        assert_eq!(sanitize_docker_name(""), "container");
    }

    use uuid; // Add this use statement

    #[test]
    fn codex_home_args_mounts_correct_paths() {
        let root = std::env::temp_dir()
            .join(format!("void-claw-codex-home-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let mut args = Vec::new();
        super::append_codex_home_args(&mut args, &root).expect("append codex args");

        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"CODEX_HOME=/home/ubuntu/.codex".to_string()));
        assert!(args.contains(&"-v".to_string()));
        assert!(args.contains(&format!("{}/.codex:/home/ubuntu/.codex:rw", root.display())));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClaudeSessionSource {
    SetupTokenFile,
    #[cfg(target_os = "macos")]
    SetupTokenKeychain,
}

#[cfg(target_os = "macos")]
fn read_keychain_value(service: &str) -> Option<String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", service, "-w"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let val = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if val.is_empty() { None } else { Some(val) }
}

#[cfg(target_os = "macos")]
pub(crate) fn extract_claude_keychain_credential() -> Option<String> {
    read_keychain_value("Claude Code-credentials")
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn extract_claude_keychain_credential() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
pub(crate) fn read_claude_setup_token() -> Option<(String, ClaudeSessionSource)> {
    if let Some(token) = read_keychain_value("void-claw-claude-setup-token") {
        return Some((token, ClaudeSessionSource::SetupTokenKeychain));
    }
    read_setup_token_file().map(|token| (token, ClaudeSessionSource::SetupTokenFile))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn read_claude_setup_token() -> Option<(String, ClaudeSessionSource)> {
    read_setup_token_file().map(|token| (token, ClaudeSessionSource::SetupTokenFile))
}

fn read_setup_token_file() -> Option<String> {
    let path = dirs::config_dir()?
        .join("void-claw")
        .join("claude-setup-token");
    let contents = std::fs::read_to_string(path).ok()?;
    let token = contents.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

```

## src/container/helpers.rs

```rs
use anyhow::{Context, Result};
use std::env;
use std::path::Path;

pub(crate) fn read_container_id(cidfile: &Path, docker_name: &str) -> Result<String> {
    for _ in 0..400 {
        if let Ok(contents) = std::fs::read_to_string(cidfile) {
            let id = contents.trim().to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
        if let Some(id) = inspect_container_id(docker_name)? {
            return Ok(id);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    anyhow::bail!(
        "failed to read docker container id from {} or inspect container {}",
        cidfile.display(),
        docker_name
    );
}

fn inspect_container_id(docker_name: &str) -> Result<Option<String>> {
    let output = std::process::Command::new("docker")
        .args(["inspect", "--format", "{{.Id}}", docker_name])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("running docker inspect")?;

    if !output.status.success() {
        return Ok(None);
    }

    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(id))
    }
}

pub fn inspect_container_exit(docker_name: &str) -> Result<Option<(Option<i32>, String)>> {
    let output = std::process::Command::new("docker")
        .args([
            "inspect",
            "--format",
            "{{.State.ExitCode}}|{{.State.Error}}",
            docker_name,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("running docker inspect")?;

    if !output.status.success() {
        return Ok(None);
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut parts = raw.trim().splitn(2, '|');
    let exit_code = parts.next().and_then(|s| s.trim().parse::<i32>().ok());
    let error = parts.next().unwrap_or("").trim().to_string();
    Ok(Some((exit_code, error)))
}

pub(crate) fn compose_no_proxy(bypass_proxy: &[String]) -> String {
    let mut entries = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "host.docker.internal".to_string(),
    ];
    for host in bypass_proxy {
        let host = host.trim();
        if host.is_empty() {
            continue;
        }
        if !entries.iter().any(|e| e == host) {
            entries.push(host.to_string());
        }
    }
    entries.join(",")
}

pub(crate) fn detect_default_colors() -> ((u8, u8, u8), (u8, u8, u8)) {
    parse_colorfgbg(env::var("COLORFGBG").ok().as_deref())
}

fn parse_colorfgbg(colorfgbg: Option<&str>) -> ((u8, u8, u8), (u8, u8, u8)) {
    let fallback = (ansi_index_to_rgb(15), ansi_index_to_rgb(0));
    let Some(val) = colorfgbg else {
        return fallback;
    };
    let parts: Vec<u8> = val
        .split(';')
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .collect();
    if parts.len() < 2 {
        return fallback;
    }
    let fg_idx = parts[parts.len().saturating_sub(2)];
    let bg_idx = parts[parts.len().saturating_sub(1)];
    if fg_idx == bg_idx {
        return fallback;
    }
    let fg = ansi_index_to_rgb(fg_idx);
    let bg = ansi_index_to_rgb(bg_idx);
    if fg == bg {
        return fallback;
    }
    (fg, bg)
}

fn ansi_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0x00, 0x00, 0x00),
        1 => (0xcd, 0x00, 0x00),
        2 => (0x00, 0xcd, 0x00),
        3 => (0xcd, 0xcd, 0x00),
        4 => (0x00, 0x00, 0xee),
        5 => (0xcd, 0x00, 0xcd),
        6 => (0x00, 0xcd, 0xcd),
        7 => (0xe5, 0xe5, 0xe5),
        8 => (0x7f, 0x7f, 0x7f),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x5c, 0x5c, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        _ => (0xff, 0xff, 0xff),
    }
}

pub(crate) fn xterm_256_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ansi_index_to_rgb(idx),
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i / 6) % 6;
            let b = i % 6;
            (xterm_cube(r), xterm_cube(g), xterm_cube(b))
        }
        232..=255 => {
            let shade = 8 + (idx - 232) * 10;
            (shade, shade, shade)
        }
    }
}

fn xterm_cube(v: u8) -> u8 {
    match v {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

pub(crate) fn blend_toward_bg(fg: (u8, u8, u8), bg: (u8, u8, u8), fg_weight: f32) -> (u8, u8, u8) {
    let fg_weight = fg_weight.clamp(0.0, 1.0);
    let bg_weight = 1.0 - fg_weight;
    let blend = |f: u8, b: u8| -> u8 {
        ((f as f32) * fg_weight + (b as f32) * bg_weight)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (blend(fg.0, bg.0), blend(fg.1, bg.1), blend(fg.2, bg.2))
}

pub(crate) fn luma_u8((r, g, b): (u8, u8, u8)) -> u8 {
    let y = 0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32);
    y.round().clamp(0.0, 255.0) as u8
}

```

## src/container/mod.rs

```rs
mod core;
mod helpers;
mod spawn;

pub use core::*;
pub use helpers::inspect_container_exit;
pub(crate) use helpers::{compose_no_proxy, read_container_id};
pub use spawn::*;

```

## src/container/spawn.rs

```rs
use alacritty_terminal::event::WindowSize;
use alacritty_terminal::event_loop::{EventLoop, Notifier};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Config as TermConfig;
use alacritty_terminal::term::Term;
use alacritty_terminal::tty;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::time::Instant;
use tracing::info;
use tracing::instrument;

use crate::config::{AgentKind, ContainerDef};
use crate::container::core::{
    TermSize, append_codex_home_args, append_gemini_home_args, extract_claude_keychain_credential,
    find_codex_home_container_path, find_gemini_home_container_path, loopback_to_host_docker,
    mount_mode_arg, mounts_include_codex_session_state, mounts_include_gemini_session_state,
    read_claude_setup_token, sanitize_docker_name,
};
use crate::container::helpers::detect_default_colors;
use crate::container::{ContainerSession, SessionEventProxy, compose_no_proxy, read_container_id};

/// Launch `docker run` for a container definition and wire it to a PTY-backed
/// terminal session.
#[instrument(skip(
    ctr,
    workspace_path,
    codex_home_host_path,
    gemini_home_host_path,
    scoped_proxy
))]
pub fn spawn(
    ctr: &ContainerDef,
    project_name: &str,
    workspace_path: &Path,
    codex_home_host_path: Option<&Path>,
    gemini_home_host_path: Option<&Path>,
    session_token: &str,
    token: &str,
    exec_url: &str,
    proxy_url: &str,
    ca_cert_host_path: &str,
    scoped_proxy: Option<crate::proxy::ScopedProxyListener>,
    strict_network: bool,
    rows: u16,
    cols: u16,
) -> Result<(ContainerSession, Vec<String>)> {
    let ca_env_path = "/usr/local/share/ca-certificates/void-claw-ca.crt";
    let no_proxy = if strict_network {
        compose_no_proxy(&[])
    } else {
        compose_no_proxy(&ctr.bypass_proxy)
    };
    let mount_str = ctr.mount_target.display().to_string();

    let cidfile =
        std::env::temp_dir().join(format!("void-claw-cid-{}.txt", uuid::Uuid::new_v4()));
    let docker_run_name = format!(
        "void-claw-{}-{}",
        sanitize_docker_name(&ctr.name),
        uuid::Uuid::new_v4().simple()
    );

    let container_exec_url = loopback_to_host_docker(exec_url);
    let container_proxy_url = loopback_to_host_docker(proxy_url);
    let container_proxy_addr = container_proxy_url
        .strip_prefix("http://")
        .or_else(|| container_proxy_url.strip_prefix("https://"))
        .unwrap_or(&container_proxy_url)
        .to_string();
    let mut launch_notes = Vec::new();

    let mut docker_args: Vec<String> = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-it".to_string(),
        "--name".to_string(),
        docker_run_name.clone(),
        "--cidfile".to_string(),
        cidfile.display().to_string(),
    ];

    #[cfg(target_os = "linux")]
    docker_args.push("--add-host=host.docker.internal:host-gateway".to_string());

    #[cfg(target_os = "linux")]
    if !strict_network {
        docker_args.extend_from_slice(&["--user".to_string(), "1000:1000".to_string()]);
    }

    if strict_network {
        docker_args.extend_from_slice(&["--user".to_string(), "0:0".to_string()]);
        #[cfg(target_os = "macos")]
        {
            // Docker Desktop exposes `/dev/net/tun` for strict mode only when the
            // container is privileged.
            docker_args.push("--privileged".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            docker_args.extend_from_slice(&["--cap-add".to_string(), "NET_ADMIN".to_string()]);
            if Path::new("/dev/net/tun").exists() {
                docker_args.extend_from_slice(&[
                    "--device".to_string(),
                    "/dev/net/tun:/dev/net/tun".to_string(),
                ]);
            } else {
                anyhow::bail!(
                    "Strict network mode requires /dev/net/tun on the host. Cannot safely fallback to --privileged."
                );
            }
        }
    }

    docker_args.extend_from_slice(&[
        "-v".to_string(),
        format!("{}:{}:rw", workspace_path.display(), mount_str),
        "-v".to_string(),
        format!("{ca_cert_host_path}:{ca_env_path}:ro"),
        "-w".to_string(),
        mount_str.clone(),
    ]);

    // Prepare secure env file to prevent token leakage via `ps`
    let mut env_file = tempfile::Builder::new()
        .prefix("void-claw-env-")
        .tempfile()
        .context("failed to create temp env file")?;

    let ca_env_vars = [
        "SSL_CERT_FILE",
        "CURL_CA_BUNDLE",
        "NODE_EXTRA_CA_CERTS",
        "DENO_CERT",
        "REQUESTS_CA_BUNDLE",
        "AWS_CA_BUNDLE",
        "GIT_SSL_CAINFO",
        "GRPC_DEFAULT_SSL_ROOTS_FILE_PATH",
    ];
    for var in ca_env_vars {
        writeln!(env_file, "{var}={ca_env_path}")?;
    }

    writeln!(env_file, "VOID_CLAW_TOKEN={token}")?;
    writeln!(env_file, "VOID_CLAW_SESSION_TOKEN={session_token}")?;
    writeln!(env_file, "VOID_CLAW_PROJECT={project_name}")?;
    writeln!(env_file, "VOID_CLAW_MOUNT_TARGET={mount_str}")?;
    writeln!(env_file, "VOID_CLAW_URL={container_exec_url}")?;
    writeln!(
        env_file,
        "VOID_CLAW_STRICT_NETWORK={}",
        if strict_network { "1" } else { "0" }
    )?;
    writeln!(
        env_file,
        "VOID_CLAW_SCOPED_PROXY_ADDR={container_proxy_addr}"
    )?;

    if !strict_network {
        writeln!(env_file, "HTTP_PROXY={container_proxy_url}")?;
        writeln!(env_file, "HTTPS_PROXY={container_proxy_url}")?;
        writeln!(env_file, "ALL_PROXY={container_proxy_url}")?;
        writeln!(env_file, "NO_PROXY={no_proxy}")?;
        writeln!(env_file, "http_proxy={container_proxy_url}")?;
        writeln!(env_file, "https_proxy={container_proxy_url}")?;
        writeln!(env_file, "all_proxy={container_proxy_url}")?;
        writeln!(env_file, "no_proxy={no_proxy}")?;
    }

    docker_args.push("--env-file".to_string());
    docker_args.push(env_file.path().display().to_string());

    if ctr.agent == AgentKind::Codex && !ctr.env_passthrough.iter().any(|v| v == "CODEX_HOME") {
        // Prefer a real host-mounted Codex home when the container already has
        // one; otherwise create a project-scoped cache directory so sessions
        // survive container restarts without leaking across projects.
        if let Some(container_codex_home) = find_codex_home_container_path(&ctr.mounts) {
            let note = format!(
                "Codex session data imported from mounted CODEX_HOME at {}",
                container_codex_home.display()
            );
            info!("{note}");
            launch_notes.push(note);
            docker_args.push("-e".to_string());
            docker_args.push(format!("CODEX_HOME={}", container_codex_home.display()));
        } else if mounts_include_codex_session_state(&ctr.mounts) {
            let note =
                "Codex session data is already mounted in the container; leaving existing Codex state paths untouched"
                    .to_string();
            info!("{note}");
            launch_notes.push(note);
        } else if let Some(host_path) = codex_home_host_path {
            // No existing host-state mounts — use per-project persistence.
            let note = format!(
                "Codex session data imported from host cache at {}",
                host_path.join(".codex").display()
            );
            info!("{note}");
            launch_notes.push(note);
            append_codex_home_args(&mut docker_args, host_path)?;
        }
    }

    if ctr.agent == AgentKind::Gemini {
        if let Some(container_gemini_home) = find_gemini_home_container_path(&ctr.mounts) {
            let note = format!(
                "Gemini session data imported from mounted .gemini at {}",
                container_gemini_home.display()
            );
            info!("{note}");
            launch_notes.push(note);
        } else if mounts_include_gemini_session_state(&ctr.mounts) {
            let note =
                "Gemini session data is already mounted in the container; leaving existing Gemini state paths untouched"
                    .to_string();
            info!("{note}");
            launch_notes.push(note);
        } else if let Some(host_path) = gemini_home_host_path {
            let note = format!(
                "Gemini session data imported from host cache at {}",
                host_path.join(".gemini").display()
            );
            info!("{note}");
            launch_notes.push(note);
            append_gemini_home_args(&mut docker_args, host_path)?;
        }
    }

    for mount in &ctr.mounts {
        if ctr.agent == crate::config::AgentKind::Claude {
            if mount.container == PathBuf::from("/home/ubuntu/.claude.json") {
                if let Ok(meta) = std::fs::metadata(&mount.host) {
                    if meta.is_dir() {
                        anyhow::bail!(
                            "invalid Claude mount: host path '{}' is a directory, but '{}' must be a file; fix by replacing ~/.claude.json with the credential file",
                            mount.host.display(),
                            mount.container.display()
                        );
                    } else {
                        let note = format!(
                            "Claude session data imported via mount {} -> {}",
                            mount.host.display(),
                            mount.container.display()
                        );
                        info!("{note}");
                        launch_notes.push(note);
                    }
                }
            }
            if mount.container == PathBuf::from("/home/ubuntu/.claude") {
                if let Ok(meta) = std::fs::metadata(&mount.host) {
                    if meta.is_file() {
                        anyhow::bail!(
                            "invalid Claude mount: host path '{}' is a file, but '{}' must be a directory",
                            mount.host.display(),
                            mount.container.display()
                        );
                    } else {
                        let note = format!(
                            "Claude session data imported via mount {} -> {}",
                            mount.host.display(),
                            mount.container.display()
                        );
                        info!("{note}");
                        launch_notes.push(note);
                    }
                }
            }
        }
        docker_args.push("-v".to_string());
        docker_args.push(format!(
            "{}:{}:{}",
            mount.host.display(),
            mount.container.display(),
            mount_mode_arg(&mount.mode),
        ));
    }

    for name in &ctr.env_passthrough {
        docker_args.push("-e".to_string());
        docker_args.push(name.to_string());
    }

    let mut _cred_tempfile = None;
    if ctr.agent == crate::config::AgentKind::Claude {
        if let Some((setup_token, source)) = read_claude_setup_token() {
            let note = format!(
                "Claude session data imported from {:?} and exported as CLAUDE_CODE_OAUTH_TOKEN",
                source
            );
            info!("{note}");
            launch_notes.push(note);
            docker_args.push("-e".to_string());
            docker_args.push(format!("CLAUDE_CODE_OAUTH_TOKEN={setup_token}"));
        } else {
            _cred_tempfile = extract_claude_keychain_credential().and_then(|cred_json| {
                let access_token: Option<String> =
                    serde_json::from_str::<serde_json::Value>(&cred_json)
                        .ok()
                        .and_then(|v| {
                            v.get("claudeAiOauth")?
                                .get("accessToken")?
                                .as_str()
                                .map(String::from)
                        });

                if let Some(ref tok) = access_token {
                    let note = "Claude session data imported from the macOS keychain credential and exported as CLAUDE_CODE_OAUTH_TOKEN".to_string();
                    info!("{note}");
                    launch_notes.push(note);
                    docker_args.push("-e".to_string());
                    docker_args.push(format!("CLAUDE_CODE_OAUTH_TOKEN={tok}"));
                }

                let staging_path = "/tmp/.zc-claude-credentials.json";
                tempfile::Builder::new()
                    .prefix("void-claw-claude-cred-")
                    .suffix(".json")
                    .tempfile()
                    .ok()
                    .and_then(|mut tf| {
                        tf.write_all(cred_json.as_bytes()).ok()?;
                        let host_path = tf.path().display().to_string();
                        docker_args.push("-v".to_string());
                        docker_args.push(format!("{host_path}:{staging_path}:ro"));
                        Some(tf)
                    })
            });
        }
    }

    docker_args.push(ctr.image.clone());

    info!(
        "launching container: docker {}",
        docker_args
            .iter()
            .map(|a| if a.contains(' ') || a.contains('=') {
                format!("'{a}'")
            } else {
                a.clone()
            })
            .collect::<Vec<_>>()
            .join(" ")
    );

    let (fg, bg) = detect_default_colors();
    let default_fg = alacritty_terminal::vte::ansi::Rgb {
        r: fg.0,
        g: fg.1,
        b: fg.2,
    };
    let default_bg = alacritty_terminal::vte::ansi::Rgb {
        r: bg.0,
        g: bg.1,
        b: bg.2,
    };

    let window_size = WindowSize {
        num_lines: rows,
        num_cols: cols,
        cell_width: 0,
        cell_height: 0,
    };
    let window_size_arc = Arc::new(Mutex::new(window_size));

    let exited = Arc::new(AtomicBool::new(false));
    let has_bell = Arc::new(AtomicBool::new(false));

    let proxy = SessionEventProxy {
        sender: Arc::new(Mutex::new(None)),
        window_size: Arc::clone(&window_size_arc),
        exited: Arc::clone(&exited),
        has_bell: Arc::clone(&has_bell),
        default_fg,
        default_bg,
        grayscale_palette: ctr.agent == crate::config::AgentKind::Codex,
    };

    let mut term_cfg = TermConfig::default();
    term_cfg.scrolling_history = 50_000;
    let term_size = TermSize {
        cols: cols as usize,
        lines: rows as usize,
    };
    let term = Arc::new(FairMutex::new(Term::new(
        term_cfg,
        &term_size,
        proxy.clone(),
    )));

    let mut options = tty::Options::default();
    options.shell = Some(tty::Shell::new("docker".to_string(), docker_args));
    options.working_directory = None;
    options.drain_on_exit = false;
    options.env = HashMap::new();

    let pty = tty::new(&options, window_size, 0).context("open PTY")?;
    let event_loop = EventLoop::new(Arc::clone(&term), proxy.clone(), pty, false, false)
        .context("event loop")?;
    let sender = event_loop.channel();
    let notifier = Notifier(sender.clone());
    if let Ok(mut s) = proxy.sender.lock() {
        *s = Some(sender);
    }
    let _handle = event_loop.spawn();

    let container_id =
        read_container_id(&cidfile, &docker_run_name).context("reading docker container id")?;
    let docker_name = docker_run_name.clone();
    let _ = std::fs::remove_file(&cidfile);

    Ok((
        ContainerSession {
            container_name: ctr.name.clone(),
            container_id,
            docker_name,
            project: project_name.to_owned(),
            session_token: session_token.to_string(),
            mount_target: mount_str,
            launched_at: Instant::now(),
            term,
            notifier,
            window_size: window_size_arc,
            exited,
            has_bell,
            exit_reported: false,
            _scoped_proxy: scoped_proxy,
            _cred_tempfile,
            _env_tempfile: Some(env_file),
        },
        launch_notes,
    ))
}

```

## src/exec.rs

```rs
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::config::{self, Config, ProjectConfig};
use crate::rules::{ComposedRules, RuleCommand};

#[derive(Debug)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub enum CommandMatch<'a> {
    /// Matched an explicit rule command.
    Explicit(&'a RuleCommand),
    /// Not in the allowlist — falls back to composed rules default_policy.
    Unlisted,
}

#[derive(Debug)]
pub enum DenyReason {
    DeniedExecutable(String),
    DeniedArgumentFragment(String),
    EmptyArgv,
}

impl std::fmt::Display for DenyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeniedExecutable(exe) => write!(f, "executable '{exe}' is on the deny list"),
            Self::DeniedArgumentFragment(frag) => {
                write!(f, "argument contains denied fragment '{frag}'")
            }
            Self::EmptyArgv => write!(f, "argv must not be empty"),
        }
    }
}

/// Check whether the request should be hard-denied before any approval flow.
/// Checks executable denylist, argument fragment denylist, and blocks shell metacharacters.
pub fn check_denied(argv: &[String], proj: &ProjectConfig, config: &Config) -> Option<DenyReason> {
    if argv.is_empty() {
        return Some(DenyReason::EmptyArgv);
    }

    let denied_exes = config::effective_denied_executables(proj, &config.defaults);
    let exe = argv[0].as_str();
    let exe_base = Path::new(exe)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(exe);

    if denied_exes.iter().any(|d| d == exe || d == exe_base) {
        return Some(DenyReason::DeniedExecutable(exe.to_string()));
    }

    // Hard-block shell metacharacters to prevent injections (e.g., `cargo test; cat /etc/shadow`)
    let shell_chars = ['|', '&', ';', '$', '>', '<', '`', '\n'];
    for arg in argv {
        if arg.contains(&shell_chars[..]) {
            return Some(DenyReason::DeniedArgumentFragment(
                "shell metacharacter".into(),
            ));
        }
    }

    let denied_frags = config::effective_denied_fragments(proj, &config.defaults);
    for arg in argv {
        for frag in &denied_frags {
            if arg.contains(frag.as_str()) {
                return Some(DenyReason::DeniedArgumentFragment(frag.clone()));
            }
        }
    }

    None
}

/// Find the first rule command that exactly matches argv.
pub fn find_matching_command<'a>(argv: &[String], rules: &'a ComposedRules) -> CommandMatch<'a> {
    match rules.find_hostdo_command(argv) {
        Some(cmd) => CommandMatch::Explicit(cmd),
        None => CommandMatch::Unlisted,
    }
}

/// Resolve env vars for a named profile (empty map if profile not found).
pub fn resolve_env(profile_name: Option<&str>, config: &Config) -> HashMap<String, String> {
    profile_name
        .and_then(|name| config.env_profiles.get(name))
        .map(|p| p.vars.clone())
        .unwrap_or_default()
}

/// Execute a command and return its output. Runs the real host-side process.
pub async fn run_command(
    argv: &[String],
    cwd: &Path,
    env_vars: &HashMap<String, String>,
    timeout_secs: u64,
) -> Result<ExecResult> {
    anyhow::ensure!(!argv.is_empty(), "argv must not be empty");

    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.current_dir(cwd);
    cmd.envs(env_vars);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();

    let output = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| anyhow::anyhow!("command timed out after {timeout_secs}s"))??;

    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(ExecResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ProjectConfig, ProjectHostdo};

    #[test]
    fn resolve_env_handles_profiles() {
        let mut config = Config::default();
        config.env_profiles.insert(
            "test".to_string(),
            crate::config::EnvProfile {
                vars: [("KEY".to_string(), "VAL".to_string())]
                    .into_iter()
                    .collect(),
            },
        );

        let env = resolve_env(Some("test"), &config);
        assert_eq!(env.get("KEY"), Some(&"VAL".to_string()));

        let env_none = resolve_env(None, &config);
        assert!(env_none.is_empty());

        let env_missing = resolve_env(Some("missing"), &config);
        assert!(env_missing.is_empty());
    }

    #[test]
    fn check_denied_blocks_metacharacters() {
        let proj = ProjectConfig::default();
        let config = Config::default();

        assert!(
            check_denied(
                &["ls".into(), "file; cat /etc/shadow".into()],
                &proj,
                &config
            )
            .is_some()
        );
        assert!(check_denied(&["ls".into(), "file && rm -rf /".into()], &proj, &config).is_some());
        assert!(
            check_denied(&["ls".into(), "file | grep secret".into()], &proj, &config).is_some()
        );
        assert!(check_denied(&["ls".into(), "file \n /".into()], &proj, &config).is_some());

        // Clean
        assert!(check_denied(&["ls".into(), "clean-file".into()], &proj, &config).is_none());
    }

    #[test]
    fn check_denied_blocks_denied_executables() {
        let mut proj = ProjectConfig::default();
        let mut config = Config::default();
        config.defaults.hostdo.denied_executables = vec!["cat".to_string()];

        assert!(check_denied(&["cat".into(), "secret.txt".into()], &proj, &config).is_some());
        assert!(check_denied(&["ls".into(), "file.txt".into()], &proj, &config).is_none());

        // Per-project deny
        proj.hostdo = Some(ProjectHostdo {
            denied_executables: Some(vec!["ls".to_string()]),
            denied_argument_fragments: None,
            command_aliases: None,
        });
        assert!(check_denied(&["ls".into(), "file.txt".into()], &proj, &config).is_some());
    }
}

```

## src/init.rs

```rs
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::instrument;

const SAMPLE_CONFIG: &str = include_str!("../void-claw.example.toml");
const DOCKER_DIR_PLACEHOLDER: &str = "__VOID_CLAW_DOCKER_DIR__";
const GITHUB_DOCKER_BASE_URL: &str =
    "https://raw.githubusercontent.com/only-cliches/void-claw/refs/heads/main/docker";
const BUILTIN_DOCKERFILES: &[&str] = &[
    "ubuntu-24.04.Dockerfile",
    "claude/ubuntu-24.04.Dockerfile",
    "codex/ubuntu-24.04.Dockerfile",
    "gemini/ubuntu-24.04.Dockerfile",
    "opencode/ubuntu-24.04.Dockerfile",
];

const HOSTDO_SCRIPT: &str = include_str!("../docker/scripts/hostdo.py");
const KILLME_SCRIPT: &str = include_str!("../docker/scripts/killme.py");

#[instrument(skip(output))]
pub fn write_sample_config(output: &Path) -> Result<()> {
    if output.exists() {
        bail!(
            "file already exists: {}  (delete it first or choose a different path)",
            output.display()
        );
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let cwd = std::env::current_dir()?;
    let home_config_root = dirs::home_dir()
        .context("could not determine home directory")?
        .join(".config/void-claw");
    let docker_dir = resolve_init_docker_dir(&cwd, &home_config_root);
    fs::create_dir_all(&docker_dir)?;
    let docker_dir_literal = toml::Value::String(docker_dir.display().to_string()).to_string();
    let sample = SAMPLE_CONFIG.replace(DOCKER_DIR_PLACEHOLDER, &docker_dir_literal);
    std::fs::write(output, sample)?;
    Ok(())
}

fn resolve_init_docker_dir(cwd: &Path, home_config_root: &Path) -> PathBuf {
    let local_docker_dir = cwd.join("docker");
    if local_docker_dir.is_dir() {
        local_docker_dir
    } else {
        home_config_root.join("docker")
    }
}

#[instrument(skip(docker_dir))]
pub fn ensure_docker_assets(docker_dir: &Path) -> Result<()> {
    let missing_dockerfiles = missing_builtin_dockerfiles(docker_dir);
    let missing_helper_scripts = missing_helper_scripts(docker_dir);

    if missing_dockerfiles.is_empty() && missing_helper_scripts.is_empty() {
        return Ok(());
    }

    println!(
        "void-claw: the docker assets in {} are incomplete",
        docker_dir.display()
    );
    if !missing_dockerfiles.is_empty() {
        println!("  Missing Dockerfiles:");
        for file in &missing_dockerfiles {
            println!("    - {}", file.display());
        }
        println!("  These can be fetched from GitHub automatically.");
    }
    if !missing_helper_scripts.is_empty() {
        println!("  Missing helper scripts:");
        for file in &missing_helper_scripts {
            println!("    - {}", file.display());
        }
        println!("  These will be written from the installed binary.");
    }

    if !prompt_yes_no("Create the missing docker assets now? [y/N]: ")? {
        return Ok(());
    }

    fs::create_dir_all(docker_dir)?;
    write_helper_scripts(docker_dir)?;
    download_missing_dockerfiles(docker_dir, &missing_dockerfiles)?;
    Ok(())
}

#[cfg(test)]
pub fn builtin_dockerfile_paths() -> &'static [&'static str] {
    BUILTIN_DOCKERFILES
}

fn missing_builtin_dockerfiles(docker_dir: &Path) -> Vec<PathBuf> {
    BUILTIN_DOCKERFILES
        .iter()
        .map(|rel| docker_dir.join(rel))
        .filter(|path| !path.exists())
        .collect()
}

fn missing_helper_scripts(docker_dir: &Path) -> Vec<PathBuf> {
    helper_script_paths(docker_dir)
        .into_iter()
        .filter(|path| !path.exists())
        .collect()
}

fn helper_script_paths(docker_dir: &Path) -> Vec<PathBuf> {
    vec![
        docker_dir.join("scripts/hostdo.py"),
        docker_dir.join("scripts/killme.py"),
    ]
}

fn prompt_yes_no(prompt: &str) -> Result<bool> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn write_helper_scripts(docker_dir: &Path) -> Result<()> {
    let scripts_dir = docker_dir.join("scripts");
    fs::create_dir_all(&scripts_dir)?;
    write_text_file(&scripts_dir.join("hostdo.py"), HOSTDO_SCRIPT)?;
    write_text_file(&scripts_dir.join("killme.py"), KILLME_SCRIPT)?;
    Ok(())
}

fn download_missing_dockerfiles(docker_dir: &Path, missing: &[PathBuf]) -> Result<()> {
    if missing.is_empty() {
        return Ok(());
    }

    let client = Client::builder()
        .build()
        .context("creating HTTP client for docker asset download")?;

    for path in missing {
        let rel = path
            .strip_prefix(docker_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let url = format!("{GITHUB_DOCKER_BASE_URL}/{rel}");
        let response = client
            .get(&url)
            .send()
            .and_then(|resp| resp.error_for_status())
            .with_context(|| format!("downloading {rel} from GitHub"))?;
        let text = response
            .text()
            .with_context(|| format!("reading {rel} from GitHub"))?;
        write_text_file(path, &text)?;
    }

    Ok(())
}

fn write_text_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        builtin_dockerfile_paths, ensure_docker_assets, resolve_init_docker_dir,
        write_sample_config,
    };
    use crate::config::Config;

    #[test]
    fn sample_config_writes_parseable_docker_dir() {
        let root = std::env::temp_dir()
            .join(format!("void-claw-init-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let output = root.join("void-claw.toml");
        let cwd = std::env::current_dir().expect("current dir");
        let sample = write_sample_config(&output);
        sample.expect("write sample config");

        let contents = std::fs::read_to_string(&output).expect("read sample config");
        let parsed: Config = toml::from_str(&contents).expect("parse sample config");
        assert_eq!(parsed.docker_dir, cwd.join("docker"));
    }

    #[test]
    fn resolve_init_docker_dir_prefers_local_docker_folder() {
        let root =
            std::env::temp_dir().join(format!("void-claw-init-local-{}", uuid::Uuid::new_v4()));
        let cwd = root.join("cwd");
        let home = root.join("home/.config/void-claw");
        std::fs::create_dir_all(cwd.join("docker")).expect("create local docker dir");
        let selected = resolve_init_docker_dir(&cwd, &home);
        assert_eq!(selected, cwd.join("docker"));
    }

    #[test]
    fn resolve_init_docker_dir_falls_back_to_home_config_root() {
        let root =
            std::env::temp_dir().join(format!("void-claw-init-home-{}", uuid::Uuid::new_v4()));
        let cwd = root.join("cwd");
        let home = root.join("home/.config/void-claw");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let selected = resolve_init_docker_dir(&cwd, &home);
        assert_eq!(selected, home.join("docker"));
    }

    #[test]
    fn builtin_dockerfile_paths_include_expected_templates() {
        let paths = builtin_dockerfile_paths();
        assert!(paths.contains(&"ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"codex/ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"claude/ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"gemini/ubuntu-24.04.Dockerfile"));
        assert!(paths.contains(&"opencode/ubuntu-24.04.Dockerfile"));
    }

    #[test]
    fn ensure_docker_assets_is_a_noop_when_complete() {
        let root =
            std::env::temp_dir().join(format!("void-claw-docker-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
        for path in builtin_dockerfile_paths() {
            let file = root.join(path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).expect("create dockerfile dir");
            }
            std::fs::write(&file, "FROM scratch").expect("write template");
        }
        std::fs::write(root.join("scripts/hostdo.py"), "hostdo").expect("write hostdo");
        std::fs::write(root.join("scripts/killme.py"), "killme").expect("write killme");

        ensure_docker_assets(&root).expect("ensure assets");
    }
}

```

## src/main.rs

```rs
#![allow(
    clippy::bind_instead_of_map,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::double_ended_iterator_last,
    clippy::doc_lazy_continuation,
    clippy::field_reassign_with_default,
    clippy::match_like_matches_macro,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::while_let_loop
)]

mod agents;
mod ca;
mod cli;
mod config;
mod container;
mod exec;
mod init;
mod new_project;
mod proxy;
mod rules;
mod server;
mod shared_config;
mod state;
mod sync;
mod telemetry;
mod tui;

use anyhow::Result;
use clap::Parser;
use crossterm::style::Stylize;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use cli::Cli;

// current_thread keeps all async tasks on one thread, which allows
// ContainerSession (containing Box<dyn MasterPty>, which is !Send) to be
// held in App across await points in the TUI event loop.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(init_path) = cli.init {
        init::write_sample_config(&init_path)?;
        info!("config written to: {}", init_path.display());
        info!(
            "edit it, then run: void-claw --config {}",
            init_path.display()
        );
        return Ok(());
    }

    let config_path = match cli.config {
        Some(path) => path,
        None => match discover_default_config_path() {
            Some(path) => path,
            None => match create_config_from_prompt()? {
                Some(path) => path,
                None => return Ok(()),
            },
        },
    };

    let config = config::load(&config_path)?;
    init::ensure_docker_assets(&config.docker_dir)?;
    ensure_workspace_root(&config.workspace.root)?;

    // Bail early if docker is not available.
    if which::which("docker").is_err() {
        anyhow::bail!(
            "docker not found in PATH — void-claw requires Docker to run containers"
        );
    }

    // Initialise tracing (+ optional OTel export) before anything else logs.
    let telemetry_handle = telemetry::init(&config)?;
    info!("loaded config from {}", config_path.display());

    let config = Arc::new(config);

    let shared_config = shared_config::SharedConfig::new(config.clone());

    // Initialize file-backed runtime state.
    let state = state::StateManager::open(&config.logging.log_dir)?;
    let token = state.get_or_create_token()?;

    // Initialize (or load) the proxy CA certificate.
    let ca_dir = config.logging.log_dir.join("ca");
    let ca = Arc::new(ca::CaStore::load_or_create(&ca_dir)?);

    // Print CA setup instructions on first run (when ca.crt didn't exist before).
    let ca_cert_path = ca_dir.join("ca.crt");
    info!(
        "{}",
        agents::ca_setup_instructions(&ca.cert_pem, &ca_cert_path.display().to_string())
    );

    // Communication channels.
    let (exec_pending_tx, exec_pending_rx) = mpsc::channel::<server::PendingItem>(64);
    let (stop_pending_tx, stop_pending_rx) = mpsc::channel::<server::ContainerStopItem>(64);
    let (net_pending_tx, net_pending_rx) = mpsc::channel::<proxy::PendingNetworkItem>(64);
    let (audit_tx, audit_rx) = mpsc::channel(256);

    let session_registry = server::SessionRegistry::default();

    // Start the hostdo HTTP server.
    let exec_port = config.defaults.hostdo.server_port;
    let exec_host = config.defaults.hostdo.server_host.clone();
    let exec_addr = format!("{exec_host}:{exec_port}");
    let server_state = server::ServerState {
        config: shared_config.clone(),
        state: state.clone(),
        pending_tx: exec_pending_tx,
        stop_tx: stop_pending_tx,
        audit_tx,
        token: token.clone(),
        sessions: session_registry.clone(),
    };
    let exec_listener = tokio::net::TcpListener::bind(&exec_addr)
        .await
        .map_err(|e| anyhow::anyhow!("binding exec bridge to {exec_addr}: {e}"))?;
    info!("exec bridge listening on {}", exec_addr);
    tokio::spawn(async move {
        if let Err(e) = server::run_with_listener(server_state, exec_listener).await {
            error!("exec server error: {e}");
        }
    });

    // Start the MITM proxy.
    let proxy_port = config.defaults.proxy.proxy_port;
    let proxy_host = config.defaults.proxy.proxy_host.clone();
    let proxy_addr = format!("{proxy_host}:{proxy_port}");
    let proxy_state = proxy::ProxyState::new(ca.clone(), shared_config.clone(), net_pending_tx)?;
    let proxy_addr_display = proxy_addr.clone();
    let proxy_state_for_server = proxy_state.clone();
    tokio::spawn(async move {
        if let Err(e) = proxy::run(proxy_state_for_server, proxy_addr).await {
            error!("proxy error: {e}");
        }
    });

    // Build and run the TUI.
    let ca_cert_path_str = ca_cert_path.display().to_string();
    let app = tui::App::new(
        shared_config,
        config_path.clone(),
        token,
        session_registry,
        exec_pending_rx,
        stop_pending_rx,
        net_pending_rx,
        audit_rx,
        state,
        proxy_state,
        proxy_addr_display,
        ca_cert_path_str,
    )?;
    tui::run(app).await?;

    // Flush any buffered OTel spans before exit.
    telemetry_handle.shutdown()?;

    Ok(())
}

fn discover_default_config_path() -> Option<PathBuf> {
    let cwd_candidate = PathBuf::from("void-claw.toml");
    if cwd_candidate.exists() {
        return Some(cwd_candidate);
    }
    let home_candidate = default_home_config_path().ok()?;
    if home_candidate.exists() {
        return Some(home_candidate);
    }
    None
}

fn default_home_config_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".config/void-claw/void-claw.toml"))
}

enum ConfigCreationChoice {
    CreateCwd,
    CreateHome,
    Cancel,
}

fn create_config_from_prompt() -> Result<Option<PathBuf>> {
    match prompt_config_creation_choice()? {
        ConfigCreationChoice::CreateHome => {
            let path = default_home_config_path()?;
            init::write_sample_config(&path)?;
            println!("created config: {}", path.display());
            Ok(Some(path))
        }
        ConfigCreationChoice::CreateCwd => {
            let path = PathBuf::from("void-claw.toml");
            init::write_sample_config(&path)?;
            println!("created config: {}", path.display());
            Ok(Some(path))
        }
        ConfigCreationChoice::Cancel => {
            println!("cancelled");
            Ok(None)
        }
    }
}

fn prompt_config_creation_choice() -> Result<ConfigCreationChoice> {
    let cwd = std::env::current_dir()?;
    println!("No config file found.");
    println!(
        "1. Create default config at ~/.config/void-claw/void-claw.toml {}",
        "(Recommended)".dark_grey()
    );
    println!(
        "2. Create default config at {}/void-claw.toml",
        cwd.display()
    );
    println!("3. Cancel and close");
    print!("Select an option [1-3]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = match input.trim() {
        "1" => ConfigCreationChoice::CreateHome,
        "2" => ConfigCreationChoice::CreateCwd,
        _ => ConfigCreationChoice::Cancel,
    };
    Ok(choice)
}

fn ensure_workspace_root(root: &PathBuf) -> Result<()> {
    if root.exists() {
        if !root.is_dir() {
            anyhow::bail!(
                "workspace.root exists but is not a directory: {}",
                root.display()
            );
        }
        return Ok(());
    }

    println!("workspace.root does not exist: {}", root.display());
    print!("Create it now? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let yes = matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes");
    if !yes {
        anyhow::bail!("workspace.root is missing; exiting");
    }

    std::fs::create_dir_all(root).map_err(|e| {
        anyhow::anyhow!("failed to create workspace.root '{}': {e}", root.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ConfigCreationChoice, create_config_from_prompt};
    use std::path::PathBuf;

    #[test]
    fn config_creation_choice_variants_are_stable() {
        assert!(matches!(
            ConfigCreationChoice::CreateCwd,
            ConfigCreationChoice::CreateCwd
        ));
        assert!(matches!(
            ConfigCreationChoice::CreateHome,
            ConfigCreationChoice::CreateHome
        ));
        assert!(matches!(
            ConfigCreationChoice::Cancel,
            ConfigCreationChoice::Cancel
        ));
    }

    #[test]
    fn prompt_creation_helper_not_used_directly_in_tests() {
        let _ = create_config_from_prompt as fn() -> anyhow::Result<Option<PathBuf>>;
    }
}

```

## src/new_project.rs

```rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::{AliasValue, SyncMode};
use crate::rules::{HostdoRules, ProjectRules};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Built-in project template families used when seeding a new workspace.
pub enum ProjectType {
    None,
    Go,
    Rust,
    Node,
    Python,
}

impl ProjectType {
    pub fn all() -> [Self; 5] {
        [Self::None, Self::Go, Self::Rust, Self::Node, Self::Python]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Go => "go",
            Self::Rust => "rust",
            Self::Node => "node",
            Self::Python => "python",
        }
    }

    pub fn next(self) -> Self {
        let all = Self::all();
        let pos = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(pos + 1) % all.len()]
    }

    pub fn prev(self) -> Self {
        let all = Self::all();
        let pos = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(pos + all.len() - 1) % all.len()]
    }
}

/// Create `void-rules.toml` in a canonical directory if it does not exist.
pub fn write_rules_if_missing(canonical_dir: &Path, project_type: ProjectType) -> Result<bool> {
    if matches!(project_type, ProjectType::None) {
        return Ok(false);
    }
    let rules_path = canonical_dir.join("void-rules.toml");
    if rules_path.exists() {
        return Ok(false);
    }
    let rules = default_rules(project_type);
    crate::rules::write_rules_file(&rules_path, &rules, true)
        .with_context(|| format!("writing {}", rules_path.display()))?;
    Ok(true)
}

/// Return the starter rule set for a new project type.
pub fn default_rules(project_type: ProjectType) -> ProjectRules {
    let (exclude_patterns, command_aliases) = match project_type {
        ProjectType::None => (vec![], HashMap::new()),
        ProjectType::Rust => (
            vec!["target/**".to_string()],
            aliases([
                ("build", "cargo build"),
                ("check", "cargo check"),
                ("test", "cargo test"),
                ("fmt", "cargo fmt"),
                ("lint", "cargo clippy"),
            ]),
        ),
        ProjectType::Node => (
            vec![
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".next/**".to_string(),
                ".cache/**".to_string(),
                ".turbo/**".to_string(),
            ],
            aliases([
                ("install", "npm install"),
                ("test", "npm run test"),
                ("lint", "npm run lint"),
                ("build", "npm run build"),
                ("dev", "npm run dev"),
                ("pnpm_test", "pnpm test"),
                ("yarn_test", "yarn test"),
                ("bun_test", "bun test"),
            ]),
        ),
        ProjectType::Python => (
            vec![
                "__pycache__/**".to_string(),
                ".pytest_cache/**".to_string(),
                ".mypy_cache/**".to_string(),
                ".ruff_cache/**".to_string(),
                ".tox/**".to_string(),
                ".venv/**".to_string(),
                "venv/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                "*.egg-info/**".to_string(),
            ],
            aliases([
                ("test", "pytest"),
                ("pytest", "pytest"),
                ("unittest", "python -m unittest"),
                ("ruff", "ruff check ."),
                ("black", "black ."),
                ("flake8", "flake8"),
                ("mypy", "mypy ."),
                ("pip_install", "pip install -r requirements.txt"),
                ("poetry_install", "poetry install"),
            ]),
        ),
        ProjectType::Go => (
            vec![
                "bin/**".to_string(),
                "dist/**".to_string(),
                "coverage/**".to_string(),
            ],
            aliases([
                ("build", "go build ./..."),
                ("test", "go test ./..."),
                ("fmt", "gofmt -w ."),
                ("vet", "go vet ./..."),
                ("tidy", "go mod tidy"),
            ]),
        ),
    };

    ProjectRules {
        exclude_patterns,
        hostdo: HostdoRules {
            command_aliases,
            ..HostdoRules::default()
        },
        ..ProjectRules::default()
    }
}

fn aliases<const N: usize>(
    items: [(&'static str, &'static str); N],
) -> HashMap<String, AliasValue> {
    items
        .into_iter()
        .map(|(name, cmd)| {
            (
                name.to_string(),
                AliasValue::WithOptions {
                    cmd: cmd.to_string(),
                    cwd: Some(PathBuf::from("$CANONICAL")),
                },
            )
        })
        .collect()
}

/// Append a project block to `void-rules.toml` using the built-in template.
pub fn append_project_block(
    config_path: &Path,
    project_name: &str,
    canonical_path: &Path,
    sync_mode: SyncMode,
) -> Result<()> {
    anyhow::ensure!(
        !project_name.trim().is_empty(),
        "project name must not be empty"
    );

    let name = toml_basic_string(project_name)?;
    let canonical = toml_basic_string(&canonical_path.display().to_string())?;
    let mode = sync_mode_toml_value(&sync_mode);
    let disposable = if matches!(sync_mode, SyncMode::Direct) {
        "disposable = false\n"
    } else {
        ""
    };

    let block = format!(
        r#"

[[projects]]
name = {name}
canonical_path = {canonical}
{disposable}

[projects.sync]
mode = "{mode}"
"#,
        disposable = disposable
    );

    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(config_path)
        .with_context(|| format!("opening config for append: {}", config_path.display()))?;
    f.write_all(block.as_bytes())
        .with_context(|| format!("appending to {}", config_path.display()))?;
    Ok(())
}

fn sync_mode_toml_value(mode: &SyncMode) -> &'static str {
    match mode {
        SyncMode::WorkspaceOnly => "workspace_only",
        SyncMode::Pushback => "pushback",
        SyncMode::Bidirectional => "bidirectional",
        SyncMode::Pullthrough => "pullthrough",
        SyncMode::Direct => "direct",
    }
}

fn toml_basic_string(s: &str) -> Result<String> {
    anyhow::ensure!(
        !s.contains('\n') && !s.contains('\r'),
        "value must not contain newlines"
    );
    let escaped = s.replace('\\', "\\\\").replace('\"', "\\\"");
    Ok(format!("\"{escaped}\""))
}

#[cfg(test)]
mod tests {
    use super::{ProjectType, append_project_block, default_rules, write_rules_if_missing};
    use crate::config::{Config, SyncMode};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("void-claw-new-project-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn templates_include_expected_aliases_and_excludes() {
        let rules = default_rules(ProjectType::Rust);
        assert!(rules.exclude_patterns.iter().any(|p| p == "target/**"));
        assert!(rules.hostdo.command_aliases.contains_key("build"));
        let build = rules
            .hostdo
            .command_aliases
            .get("build")
            .expect("build alias");
        match build {
            crate::config::AliasValue::WithOptions { cmd, cwd } => {
                assert_eq!(cmd, "cargo build");
                assert_eq!(cwd.as_ref().unwrap().as_os_str(), "$CANONICAL");
            }
            _ => panic!("expected WithOptions alias"),
        }
    }

    #[test]
    fn none_project_type_has_no_starter_rules() {
        let rules = default_rules(ProjectType::None);
        assert!(rules.exclude_patterns.is_empty());
        assert!(rules.hostdo.command_aliases.is_empty());
    }

    #[test]
    fn rules_write_is_idempotent_when_file_exists() {
        let root = unique_temp_dir("rules-idempotent");
        fs::create_dir_all(root.join("canon")).expect("create canon");
        let rules_path = root.join("canon").join("void-rules.toml");
        fs::write(&rules_path, "sentinel").expect("write sentinel");

        let wrote = write_rules_if_missing(&root.join("canon"), ProjectType::Node).expect("write");
        assert!(!wrote);
        assert_eq!(fs::read_to_string(&rules_path).expect("read"), "sentinel");
    }

    #[test]
    fn none_project_type_skips_rules_creation() {
        let root = unique_temp_dir("rules-none");
        fs::create_dir_all(root.join("canon")).expect("create canon");
        let wrote = write_rules_if_missing(&root.join("canon"), ProjectType::None).expect("write");
        assert!(!wrote);
        assert!(!root.join("canon").join("void-rules.toml").exists());
    }

    #[test]
    fn config_append_block_parses_and_sets_sync_mode() {
        let root = unique_temp_dir("append-config");
        let config_path = root.join("void-claw.toml");
        let canon = root.join("canon");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&canon).expect("create canon");
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
            root.join("ws").display(),
        );
        fs::write(&config_path, raw).expect("write base config");

        append_project_block(&config_path, "proj", &canon, SyncMode::Bidirectional)
            .expect("append");
        let cfg: Config = crate::config::load(&config_path).expect("load");

        let proj = cfg.projects.first().expect("project");
        assert_eq!(proj.name, "proj");
        assert_eq!(proj.canonical_path, canon);
        assert_eq!(
            proj.sync.as_ref().and_then(|s| s.mode.clone()),
            Some(SyncMode::Bidirectional)
        );
    }

    #[test]
    fn config_append_block_marks_direct_projects_non_disposable() {
        let root = unique_temp_dir("append-config-direct");
        let config_path = root.join("void-claw.toml");
        let canon = root.join("canon");
        let docker_dir = root.join("docker-root");
        fs::create_dir_all(&canon).expect("create canon");
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
            root.join("ws").display(),
        );
        fs::write(&config_path, raw).expect("write base config");

        append_project_block(&config_path, "proj", &canon, SyncMode::Direct).expect("append");
        let cfg: Config = crate::config::load(&config_path).expect("load");

        let proj = cfg.projects.first().expect("project");
        assert_eq!(proj.name, "proj");
        assert_eq!(proj.canonical_path, canon);
        assert_eq!(
            proj.sync.as_ref().and_then(|s| s.mode.clone()),
            Some(SyncMode::Direct)
        );
        assert!(!proj.disposable);
    }
}

```

## src/proxy/connect.rs

```rs
use anyhow::Result;
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

use crate::config;
use crate::proxy::helpers::{
    container_tls_passthrough_matches, write_error_any, write_response_any,
};
use crate::proxy::http::{
    connect_head_has_proxy_authorization, forward_request, parse_connect_target,
    parse_request_line_and_headers, parse_source_from_connect_head, prompt_network, read_body_any,
    read_request_head_any,
};
use crate::proxy::{ProxyState, SourceIdentityStatus};
use crate::rules::NetworkPolicy;

pub(crate) fn parse_sni_from_tls_client_hello(record: &[u8]) -> Option<String> {
    if record.len() < 5 + 4 {
        return None;
    }
    if record[0] != 0x16 {
        return None;
    }
    let rec_len = u16::from_be_bytes([record[3], record[4]]) as usize;
    if record.len() < 5 + rec_len {
        return None;
    }
    let mut i = 5;
    if record.get(i)? != &0x01 {
        return None;
    }
    i += 1;
    let hs_len = ((record.get(i)? as &u8).to_owned() as usize) << 16
        | (((record.get(i + 1)? as &u8).to_owned() as usize) << 8)
        | (record.get(i + 2)? as &u8).to_owned() as usize;
    i += 3;
    if record.len() < i + hs_len {
        return None;
    }
    i += 2 + 32;
    let sid_len = *record.get(i)? as usize;
    i += 1 + sid_len;
    let cs_len = u16::from_be_bytes([*record.get(i)?, *record.get(i + 1)?]) as usize;
    i += 2 + cs_len;
    let comp_len = *record.get(i)? as usize;
    i += 1 + comp_len;
    let ext_len = u16::from_be_bytes([*record.get(i)?, *record.get(i + 1)?]) as usize;
    i += 2;
    let ext_end = i + ext_len;
    if record.len() < ext_end {
        return None;
    }
    while i + 4 <= ext_end {
        let et = u16::from_be_bytes([record[i], record[i + 1]]);
        let el = u16::from_be_bytes([record[i + 2], record[i + 3]]) as usize;
        i += 4;
        if i + el > ext_end {
            return None;
        }
        if et == 0x0000 && el >= 2 {
            let list_len = u16::from_be_bytes([record[i], record[i + 1]]) as usize;
            let mut j = i + 2;
            let list_end = j + list_len;
            if list_end > i + el {
                return None;
            }
            while j + 3 <= list_end {
                let name_type = record[j];
                let name_len = u16::from_be_bytes([record[j + 1], record[j + 2]]) as usize;
                j += 3;
                if j + name_len > list_end {
                    return None;
                }
                if name_type == 0 {
                    let sni = String::from_utf8_lossy(&record[j..j + name_len]).to_string();
                    if !sni.is_empty() {
                        return Some(sni);
                    }
                }
                j += name_len;
            }
        }
        i += el;
    }
    None
}

// ── HTTPS CONNECT tunnel ──────────────────────────────────────────────────────

pub(crate) async fn handle_connect(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (head, connect_remainder) = read_request_head_any(&mut stream).await?;
    let head_str = std::str::from_utf8(&head).unwrap_or("");

    let (host, port) = parse_connect_target(head_str)
        .ok_or_else(|| anyhow::anyhow!("malformed CONNECT request"))?;
    let (source_project, source_container, source_status, connect_has_proxy_authorization): (
        Option<String>,
        Option<String>,
        SourceIdentityStatus,
        bool,
    ) = if let Some(fixed) = &state.fixed_source {
        (
            Some(fixed.project.clone()),
            Some(fixed.container.clone()),
            SourceIdentityStatus::ListenerBoundSource,
            false,
        )
    } else {
        let (project, container, status) = parse_source_from_connect_head(head_str);
        let has_auth = connect_head_has_proxy_authorization(head_str);
        (project, container, status, has_auth)
    };

    let cfg = state.config.get();

    if container_tls_passthrough_matches(&cfg, source_container.as_deref(), &host) {
        info!(
            host = %host,
            source_project = ?source_project,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            connect_has_proxy_authorization,
            "proxy CONNECT passthrough"
        );
        stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;
        let mut upstream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(|e| {
                anyhow::anyhow!("CONNECT passthrough connect to {host}:{port} failed: {e}")
            })?;
        if !connect_remainder.is_empty() {
            upstream.write_all(&connect_remainder).await?;
        }
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
        return Ok(());
    }

    let rules = match config::load_composed_rules_for_project(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            write_error_any(&mut stream, 500, "Invalid void-rules.toml configuration").await?;
            return Ok(());
        }
    };
    let preflight_policy = rules.match_network("CONNECT", &host, "/");
    let preflight_allowed = match preflight_policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                "CONNECT",
                &host,
                "/",
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                connect_has_proxy_authorization,
            )
            .await
        }
    };
    if !preflight_allowed {
        write_error_any(&mut stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }

    if port != 443 {
        info!(
            host = %host,
            port,
            source_project = ?source_project,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            connect_has_proxy_authorization,
            "proxy CONNECT raw tunnel path"
        );
        stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;
        let mut upstream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(|e| {
                anyhow::anyhow!("CONNECT raw tunnel connect to {host}:{port} failed: {e}")
            })?;
        if !connect_remainder.is_empty() {
            upstream.write_all(&connect_remainder).await?;
        }
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
        return Ok(());
    }

    info!(
        host = %host,
        source_project = ?source_project,
        source_container = ?source_container,
        source_status = source_status.as_str(),
        connect_has_proxy_authorization,
        "proxy CONNECT MITM path"
    );

    stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    let server_config = state.ca.leaf_server_config(&host)?;
    let acceptor = TlsAcceptor::from(server_config);
    let mut tls_stream = acceptor
        .accept(stream)
        .await
        .map_err(|e| anyhow::anyhow!("TLS accept for {host}: {e}"))?;

    debug!("proxy TLS established for host={host}");

    let (inner_head, inner_remainder) = read_request_head_any(&mut tls_stream).await?;
    let inner_str = match std::str::from_utf8(&inner_head) {
        Ok(s) => s,
        Err(_) => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };

    let (method, path, headers) = match parse_request_line_and_headers(inner_str) {
        Some(r) => r,
        None => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };

    let body = read_body_any(&mut tls_stream, &headers, inner_remainder).await?;

    if source_project.is_none() {
        warn!(
            host = %host,
            method = %method,
            path = %path,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            connect_has_proxy_authorization,
            "proxy request missing source project metadata; permanent network rule persistence will not know which project to update"
        );
    }

    let policy = rules.match_network(&method, &host, &path);

    let allowed = match policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                connect_has_proxy_authorization,
            )
            .await
        }
    };

    if !allowed {
        write_error_any(&mut tls_stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }

    let url = format!("https://{host}{path}");
    let response = forward_request(&state.client, &method, &url, &headers, body).await?;
    write_response_any(&mut tls_stream, response).await
}

```

## src/proxy/core.rs

```rs
/// MITM HTTP/HTTPS proxy enforcing network policies from void-rules.toml.
///
/// Containers route all traffic through this proxy. Plain HTTP requests are
/// intercepted and parsed directly. HTTPS traffic is intercepted via CONNECT
/// tunnels: the proxy terminates TLS with a per-domain leaf cert signed by
/// the void-claw CA (which containers are configured to trust), inspects the
/// inner HTTP request, then forwards to the real server.
///
/// Network policy (auto/prompt/deny) is determined by matching the composed
/// rules against method + host + path of each request.
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use crate::ca::CaStore;
use crate::config;
use crate::proxy::connect::{handle_connect, parse_sni_from_tls_client_hello};
use crate::proxy::helpers::{
    container_tls_passthrough_matches, is_expected_disconnect, write_error_any, write_response_any,
};
use crate::proxy::http::{
    forward_request, handle_plain_http, parse_request_line_and_headers, prompt_network,
    read_body_any, read_request_head_any,
};
use crate::rules::NetworkPolicy;
use crate::shared_config::SharedConfig;
use tracing::instrument;

/// A network request waiting on the TUI for an allow/deny decision.
pub struct PendingNetworkItem {
    pub source_project: Option<String>,
    pub source_container: Option<String>,
    pub source_status: String,
    pub has_proxy_authorization: bool,
    pub method: String,
    pub host: String,
    pub path: String,
    pub response_tx: oneshot::Sender<NetworkDecision>,
}

/// The result returned by the TUI for a pending network request.
#[derive(Debug)]
pub enum NetworkDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub(crate) struct FixedSourceIdentity {
    pub(crate) project: String,
    pub(crate) container: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceIdentityStatus {
    Ok,
    ListenerBoundSource,
    MissingProxyAuthorization,
    MalformedAuthHeader,
    UnsupportedAuthScheme,
    InvalidBase64,
    InvalidUtf8,
    MissingUsernamePasswordDelimiter,
    UnexpectedUsername,
    MissingProjectContainerDelimiter,
    InvalidProjectEncoding,
    InvalidContainerEncoding,
}

impl SourceIdentityStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ListenerBoundSource => "listener_bound_source",
            Self::MissingProxyAuthorization => "missing_proxy_authorization",
            Self::MalformedAuthHeader => "malformed_auth_header",
            Self::UnsupportedAuthScheme => "unsupported_auth_scheme",
            Self::InvalidBase64 => "invalid_base64",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::MissingUsernamePasswordDelimiter => "missing_username_password_delimiter",
            Self::UnexpectedUsername => "unexpected_username",
            Self::MissingProjectContainerDelimiter => "missing_project_container_delimiter",
            Self::InvalidProjectEncoding => "invalid_project_encoding",
            Self::InvalidContainerEncoding => "invalid_container_encoding",
        }
    }
}

// ── Proxy state ───────────────────────────────────────────────────────────────

#[derive(Clone)]
/// Shared proxy state used by all listener tasks.
pub struct ProxyState {
    pub ca: Arc<CaStore>,
    pub config: SharedConfig,
    pub pending_tx: mpsc::Sender<PendingNetworkItem>,
    pub(crate) client: reqwest::Client,
    pub(crate) fixed_source: Option<FixedSourceIdentity>,
}

impl ProxyState {
    pub fn new(
        ca: Arc<CaStore>,
        config: SharedConfig,
        pending_tx: mpsc::Sender<PendingNetworkItem>,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            ca,
            config,
            pending_tx,
            client,
            fixed_source: None,
        })
    }

    fn with_fixed_source(&self, project: &str, container: &str) -> Self {
        let mut cloned = self.clone();
        cloned.fixed_source = Some(FixedSourceIdentity {
            project: project.to_string(),
            container: container.to_string(),
        });
        cloned
    }
}

/// A scoped listener task that is aborted when dropped.
pub struct ScopedProxyListener {
    pub addr: String,
    abort_handle: tokio::task::AbortHandle,
}

impl Drop for ScopedProxyListener {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[instrument(skip(state))]
pub async fn run(state: ProxyState, addr: String) -> Result<()> {
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("proxy bind {addr}: {e}"))?;
    run_with_listener(state, listener).await
}

#[instrument(skip(state, listener))]
async fn run_with_listener(state: ProxyState, listener: TcpListener) -> Result<()> {
    loop {
        let (stream, _peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, state).await {
                if is_expected_disconnect(&e) {
                    debug!("proxy: {e}");
                } else {
                    error!("proxy: {e}");
                }
            }
        });
    }
}

/// Start a per-container proxy listener bound to the supplied host/port.
#[instrument(skip(state))]
pub fn spawn_scoped_listener(
    state: &ProxyState,
    bind_host: &str,
    project: &str,
    container: &str,
) -> Result<ScopedProxyListener> {
    let bind_addr = format!("{bind_host}:0");
    let std_listener = std::net::TcpListener::bind(&bind_addr)
        .map_err(|e| anyhow::anyhow!("proxy bind {bind_addr}: {e}"))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| anyhow::anyhow!("proxy set_nonblocking {bind_addr}: {e}"))?;
    let local_addr = std_listener.local_addr()?;
    let listener = TcpListener::from_std(std_listener)?;
    let addr = format!("{}:{}", bind_host, local_addr.port());
    let fixed_state = state.with_fixed_source(project, container);
    let task = tokio::spawn(async move {
        if let Err(e) = run_with_listener(fixed_state, listener).await {
            error!("scoped proxy server error: {e}");
        }
    });
    Ok(ScopedProxyListener {
        addr,
        abort_handle: task.abort_handle(),
    })
}

// ── Connection dispatch ───────────────────────────────────────────────────────

async fn handle_connection(stream: TcpStream, state: ProxyState) -> Result<()> {
    let mut peek = [0u8; 8];
    let n = stream.peek(&mut peek).await?;

    // Prefer explicit CONNECT first, then fall back to sniffing for raw TLS.
    // This lets the same listener handle both proxy-aware clients and clients
    // that try to talk TLS directly to the gateway.
    if n >= 7 && &peek[..7] == b"CONNECT" {
        handle_connect(stream, state).await
    } else if looks_like_tls_client_hello(&peek[..n]) {
        handle_transparent_tls(stream, state).await
    } else {
        handle_plain_http(stream, state).await
    }
}

fn looks_like_tls_client_hello(buf: &[u8]) -> bool {
    buf.len() >= 3 && buf[0] == 0x16 && buf[1] == 0x03 && (0x01..=0x04).contains(&buf[2])
}

// ── Transparent TLS (no CONNECT) ─────────────────────────────────────────────

struct PrefixedTcpStream {
    prefix: std::io::Cursor<Vec<u8>>,
    inner: TcpStream,
}

impl AsyncRead for PrefixedTcpStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if (self.prefix.position() as usize) < self.prefix.get_ref().len() {
            let before = buf.filled().len();
            let pos = self.prefix.position();
            let rem = &self.prefix.get_ref()[pos as usize..];
            let to_copy = rem.len().min(buf.remaining());
            buf.put_slice(&rem[..to_copy]);
            self.prefix.set_position(pos + to_copy as u64);
            let after = buf.filled().len();
            debug_assert!(after > before);
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for PrefixedTcpStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        data: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(cx, data)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

async fn handle_transparent_tls(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (source_project, source_container, source_status, has_proxy_authorization) =
        if let Some(fixed) = &state.fixed_source {
            (
                Some(fixed.project.clone()),
                Some(fixed.container.clone()),
                SourceIdentityStatus::ListenerBoundSource,
                false,
            )
        } else {
            (
                None,
                None,
                SourceIdentityStatus::MissingProxyAuthorization,
                false,
            )
        };

    let cfg = state.config.get();

    let prefix = read_tls_client_hello_prefix(&mut stream).await?;
    let Some(host) = parse_sni_from_tls_client_hello(&prefix) else {
        warn!("transparent TLS connection missing SNI; dropping");
        return Ok(());
    };

    if container_tls_passthrough_matches(&cfg, source_container.as_deref(), &host) {
        info!(
            host = %host,
            source_project = ?source_project,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            "proxy transparent TLS passthrough"
        );
        let mut upstream = TcpStream::connect(format!("{host}:443"))
            .await
            .map_err(|e| {
                anyhow::anyhow!("transparent passthrough connect to {host}:443 failed: {e}")
            })?;
        upstream.write_all(&prefix).await?;
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
        return Ok(());
    }

    let rules = match config::load_composed_rules_for_project(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            return Ok(());
        }
    };
    let preflight_policy = rules.match_network("CONNECT", &host, "/");
    let preflight_allowed = match preflight_policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                "CONNECT",
                &host,
                "/",
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
            )
            .await
        }
    };
    if !preflight_allowed {
        return Ok(());
    }

    let prefixed = PrefixedTcpStream {
        prefix: std::io::Cursor::new(prefix),
        inner: stream,
    };

    let server_config = state.ca.leaf_server_config(&host)?;
    let acceptor = TlsAcceptor::from(server_config);
    let mut tls_stream = acceptor
        .accept(prefixed)
        .await
        .map_err(|e| anyhow::anyhow!("TLS accept for {host}: {e}"))?;

    debug!("proxy TLS established for host={host} (transparent)");

    let (inner_head, inner_remainder) = read_request_head_any(&mut tls_stream).await?;
    let inner_str = match std::str::from_utf8(&inner_head) {
        Ok(s) => s,
        Err(_) => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };
    let (method, path, headers) = match parse_request_line_and_headers(inner_str) {
        Some(r) => r,
        None => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };
    let body = read_body_any(&mut tls_stream, &headers, inner_remainder).await?;

    if source_project.is_none() {
        warn!(
            host = %host,
            method = %method,
            path = %path,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            has_proxy_authorization,
            "proxy request missing source project metadata; permanent network rule persistence will not know which project to update"
        );
    }

    let policy = rules.match_network(&method, &host, &path);
    let allowed = match policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
            )
            .await
        }
    };
    if !allowed {
        write_error_any(&mut tls_stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }
    let url = format!("https://{host}{path}");
    let response = forward_request(&state.client, &method, &url, &headers, body).await?;
    write_response_any(&mut tls_stream, response).await
}

async fn read_tls_client_hello_prefix(stream: &mut TcpStream) -> Result<Vec<u8>> {
    // We only need enough of the ClientHello to recover SNI and route policy;
    // the rest of the handshake is forwarded untouched.
    let mut hdr = [0u8; 5];
    stream.read_exact(&mut hdr).await?;
    if hdr[0] != 0x16 {
        anyhow::bail!("not a TLS handshake record");
    }
    let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
    if len > 64 * 1024 {
        anyhow::bail!("TLS record too large");
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    let mut out = Vec::with_capacity(5 + len);
    out.extend_from_slice(&hdr);
    out.extend_from_slice(&body);
    Ok(out)
}

```

## src/proxy/helpers.rs

```rs
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use futures::StreamExt;
use globset::Glob;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::proxy::SourceIdentityStatus;
use crate::proxy::http::is_hop_by_hop;

pub(crate) fn parse_source_from_headers(
    headers: &[(String, String)],
) -> (Option<String>, Option<String>, SourceIdentityStatus) {
    let auth = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("proxy-authorization"))
        .map(|(_, v)| v.as_str());
    let Some(auth) = auth else {
        return (None, None, SourceIdentityStatus::MissingProxyAuthorization);
    };
    decode_source_from_proxy_authorization(auth)
}

pub(crate) fn decode_source_from_proxy_authorization(
    value: &str,
) -> (Option<String>, Option<String>, SourceIdentityStatus) {
    let Some((scheme, payload)) = value.split_once(' ') else {
        return (None, None, SourceIdentityStatus::MalformedAuthHeader);
    };
    if !scheme.eq_ignore_ascii_case("basic") {
        return (None, None, SourceIdentityStatus::UnsupportedAuthScheme);
    }
    let decoded = match STANDARD.decode(payload.trim()) {
        Ok(bytes) => bytes,
        Err(_) => return (None, None, SourceIdentityStatus::InvalidBase64),
    };
    let creds = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return (None, None, SourceIdentityStatus::InvalidUtf8),
    };
    let Some((username, password)) = creds.split_once(':') else {
        return (
            None,
            None,
            SourceIdentityStatus::MissingUsernamePasswordDelimiter,
        );
    };
    if username != "zcsrc" {
        return (None, None, SourceIdentityStatus::UnexpectedUsername);
    }
    let Some((project_enc, container_enc)) = password.split_once('.') else {
        return (
            None,
            None,
            SourceIdentityStatus::MissingProjectContainerDelimiter,
        );
    };
    let project = match URL_SAFE_NO_PAD.decode(project_enc.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(value) => value,
            Err(_) => return (None, None, SourceIdentityStatus::InvalidProjectEncoding),
        },
        Err(_) => return (None, None, SourceIdentityStatus::InvalidProjectEncoding),
    };
    let container = match URL_SAFE_NO_PAD.decode(container_enc.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(value) => value,
            Err(_) => return (None, None, SourceIdentityStatus::InvalidContainerEncoding),
        },
        Err(_) => return (None, None, SourceIdentityStatus::InvalidContainerEncoding),
    };
    (Some(project), Some(container), SourceIdentityStatus::Ok)
}

pub(crate) fn container_tls_passthrough_matches(
    config: &Config,
    source_container: Option<&str>,
    host: &str,
) -> bool {
    let Some(source_container) = source_container else {
        return false;
    };
    let Some(container) = config
        .containers
        .iter()
        .find(|c| c.name == source_container)
    else {
        return false;
    };
    container
        .bypass_proxy
        .iter()
        .any(|pattern| bypass_host_matches(pattern, host))
}

pub(crate) fn bypass_host_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" {
        return true;
    }

    let host_lc = host.to_ascii_lowercase();
    let pattern_lc = pattern.to_ascii_lowercase();

    if let Some(apex) = pattern_lc.strip_prefix('.') {
        return host_lc == apex || host_lc.ends_with(&format!(".{apex}"));
    }

    if let Some(apex) = pattern_lc.strip_prefix("*.") {
        return host_lc == apex || host_lc.ends_with(&format!(".{apex}"));
    }

    if !pattern_lc.contains('*') {
        return host_lc == pattern_lc;
    }

    Glob::new(&pattern_lc)
        .ok()
        .map(|g| g.compile_matcher().is_match(&host_lc))
        .unwrap_or(false)
}

pub(crate) fn extract_host(headers: &[(String, String)], path: &str) -> Option<String> {
    if let Some((_, v)) = headers.iter().find(|(n, _)| n.eq_ignore_ascii_case("host")) {
        return Some(strip_port(v.trim()));
    }
    if path.starts_with("http://") || path.starts_with("https://") {
        if let Ok(url) = path.parse::<url::Url>() {
            return url.host_str().map(|h| h.to_string());
        }
    }
    None
}

pub(crate) fn strip_port(host: &str) -> String {
    if host.starts_with('[') {
        if let Some(end) = host.find(']') {
            return host[1..end].to_string();
        }
        return host.to_string();
    }
    host.split(':').next().unwrap_or(host).to_string()
}

pub(crate) fn strip_scheme_and_host(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        if let Ok(url) = path.parse::<url::Url>() {
            let mut result = url.path().to_string();
            if let Some(q) = url.query() {
                result.push('?');
                result.push_str(q);
            }
            return result;
        }
    }
    path.to_string()
}

pub(crate) async fn write_response_any<W>(sink: &mut W, response: reqwest::Response) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let status = response.status().as_u16();
    let reason = response.status().canonical_reason().unwrap_or("Unknown");

    let resp_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter(|(name, _)| !is_hop_by_hop(name.as_str()))
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.to_string(), v.to_string()))
        })
        .collect();

    let content_length: Option<u64> = resp_headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok());

    let use_chunked = content_length.is_none();

    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in &resp_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if use_chunked {
        head.push_str("Transfer-Encoding: chunked\r\n");
    }
    head.push_str("Connection: close\r\n");
    head.push_str("\r\n");
    sink.write_all(head.as_bytes()).await?;

    let mut body_stream = response.bytes_stream();
    while let Some(chunk) = body_stream.next().await {
        let chunk = chunk?;
        if chunk.is_empty() {
            continue;
        }
        if use_chunked {
            sink.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await?;
            sink.write_all(&chunk).await?;
            sink.write_all(b"\r\n").await?;
        } else {
            sink.write_all(&chunk).await?;
        }
    }
    if use_chunked {
        sink.write_all(b"0\r\n\r\n").await?;
    }
    Ok(())
}

pub(crate) async fn write_error_any<W>(sink: &mut W, code: u16, msg: &str) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let body = msg.as_bytes();
    let reason = StatusCode::from_u16(code)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        .canonical_reason()
        .unwrap_or("Unknown");

    let out = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut raw = out.into_bytes();
    raw.extend_from_slice(body);
    sink.write_all(&raw).await?;
    Ok(())
}

pub(crate) fn is_expected_disconnect(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("close_notify")
        || msg.contains("unexpected eof")
        || msg.contains("connection reset by peer")
        || msg.contains("broken pipe")
}

```

## src/proxy/http.rs

```rs
use anyhow::Result;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tracing::warn;

use crate::config;
use crate::proxy::helpers::{
    extract_host, parse_source_from_headers, strip_scheme_and_host, write_error_any,
    write_response_any,
};
use crate::proxy::{NetworkDecision, PendingNetworkItem, ProxyState, SourceIdentityStatus};
use crate::rules::NetworkPolicy;

// ── Plain HTTP ────────────────────────────────────────────────────────────────

pub(crate) async fn handle_plain_http(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (head, body_remainder) = read_request_head_any(&mut stream).await?;
    let head_str = match std::str::from_utf8(&head) {
        Ok(s) => s,
        Err(_) => {
            write_error_any(&mut stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };

    let cfg = state.config.get();

    let (method, path, headers) = match parse_request_line_and_headers(head_str) {
        Some(r) => r,
        None => {
            write_error_any(&mut stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };
    let (source_project, source_container, source_status, has_proxy_authorization): (
        Option<String>,
        Option<String>,
        SourceIdentityStatus,
        bool,
    ) = if let Some(fixed) = &state.fixed_source {
        (
            Some(fixed.project.clone()),
            Some(fixed.container.clone()),
            SourceIdentityStatus::ListenerBoundSource,
            false,
        )
    } else {
        let (project, container, status) = parse_source_from_headers(&headers);

        let has_auth = headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case("proxy-authorization"));
        (project, container, status, has_auth)
    };

    let host = extract_host(&headers, &path).unwrap_or_default();
    let path = strip_scheme_and_host(&path);

    let body = read_body_any(&mut stream, &headers, body_remainder).await?;

    if source_project.is_none() {
        warn!(
            host = %host,
            method = %method,
            path = %path,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            has_proxy_authorization,
            "proxy request missing source project metadata; permanent network rule persistence will not know which project to update"
        );
    }

    let rules = match config::load_composed_rules_for_project(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            write_error_any(&mut stream, 500, "Invalid void-rules.toml configuration").await?;
            return Ok(());
        }
    };
    let policy = rules.match_network(&method, &host, &path);

    let allowed = match policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
            )
            .await
        }
    };

    if !allowed {
        write_error_any(&mut stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }

    let url = format!("http://{host}{path}");
    let response = forward_request(&state.client, &method, &url, &headers, body).await?;
    write_response_any(&mut stream, response).await
}

pub(crate) async fn prompt_network(
    state: &ProxyState,
    method: &str,
    host: &str,
    path: &str,
    source_project: Option<String>,
    source_container: Option<String>,
    source_status: &str,
    has_proxy_authorization: bool,
) -> bool {
    let (tx, rx) = oneshot::channel();
    let item = PendingNetworkItem {
        source_project,
        source_container,
        source_status: source_status.to_string(),
        has_proxy_authorization,
        method: method.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        response_tx: tx,
    };
    if state.pending_tx.send(item).await.is_err() {
        return false;
    }
    match tokio::time::timeout(Duration::from_secs(300), rx).await {
        Ok(Ok(NetworkDecision::Allow)) => true,
        _ => false,
    }
}

pub(crate) async fn forward_request(
    client: &reqwest::Client,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Vec<u8>,
) -> Result<reqwest::Response> {
    let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);

    let mut req = client.request(method, url);
    for (name, value) in headers {
        if !is_hop_by_hop(name) {
            req = req.header(name.as_str(), value.as_str());
        }
    }
    if !body.is_empty() {
        req = req.body(body);
    }
    let response = req.send().await?;
    Ok(response)
}

pub(crate) fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

pub(crate) async fn read_request_head_any<R>(stream: &mut R) -> Result<(Vec<u8>, Vec<u8>)>
where
    R: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if contains_double_crlf(&buf) {
            break;
        }
        if buf.len() > 64 * 1024 {
            anyhow::bail!("request head too large");
        }
    }
    split_head_and_remainder(buf)
}

pub(crate) fn contains_double_crlf(buf: &[u8]) -> bool {
    buf.windows(4).any(|w| w == b"\r\n\r\n")
}

pub(crate) fn split_head_and_remainder(buf: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>)> {
    if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        let end = end + 4;
        Ok((buf[..end].to_vec(), buf[end..].to_vec()))
    } else {
        anyhow::bail!("incomplete request head")
    }
}

pub(crate) async fn read_body_any<R>(
    stream: &mut R,
    headers: &[(String, String)],
    initial: Vec<u8>,
) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let content_length = content_length_from_headers(headers);
    if content_length == 0 {
        return Ok(vec![]);
    }
    let mut body = Vec::with_capacity(content_length);
    body.extend_from_slice(&initial[..initial.len().min(content_length)]);
    if body.len() < content_length {
        let mut rest = vec![0u8; content_length - body.len()];
        stream.read_exact(&mut rest).await?;
        body.extend_from_slice(&rest);
    }
    Ok(body)
}

pub(crate) fn content_length_from_headers(headers: &[(String, String)]) -> usize {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

pub(crate) fn parse_connect_target(head: &str) -> Option<(String, u16)> {
    let first_line = head.lines().next()?;
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let authority = parts[1];
    if authority.starts_with('[') {
        let end = authority.find(']')?;
        let host = authority[1..end].to_string();
        let port = authority[end + 1..].strip_prefix(':')?.parse().ok()?;
        return Some((host, port));
    }
    let (host, port) = authority.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

pub(crate) fn parse_request_line_and_headers(
    head: &str,
) -> Option<(String, String, Vec<(String, String)>)> {
    let mut lines = head.lines();
    let first = lines.next()?;
    let parts: Vec<&str> = first.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    Some((method, path, headers))
}

pub(crate) fn parse_source_from_connect_head(
    head: &str,
) -> (Option<String>, Option<String>, SourceIdentityStatus) {
    let Some((_, _, headers)) = parse_request_line_and_headers(head) else {
        return (None, None, SourceIdentityStatus::MalformedAuthHeader);
    };
    parse_source_from_headers(&headers)
}

pub(crate) fn connect_head_has_proxy_authorization(head: &str) -> bool {
    let Some((_, _, headers)) = parse_request_line_and_headers(head) else {
        return false;
    };
    headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("proxy-authorization"))
}

```

## src/proxy/mod.rs

```rs
mod connect;
mod core;
mod helpers;
mod http;

pub use core::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests_file;

```

## src/proxy/tests.rs

```rs
#[cfg(test)]
mod tests {
    use crate::ca::CaStore;
    use crate::proxy::core::{NetworkDecision, ProxyState, SourceIdentityStatus};
    use crate::proxy::helpers::{bypass_host_matches, decode_source_from_proxy_authorization};
    use crate::proxy::http::prompt_network;
    use crate::shared_config::SharedConfig;
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn decode_source_from_proxy_authorization_works() {
        let auth_payload = format!(
            "zcsrc:{}.{}",
            URL_SAFE_NO_PAD.encode("myproj"),
            URL_SAFE_NO_PAD.encode("mycont")
        );
        let header_value = format!("Basic {}", STANDARD.encode(auth_payload));
        let (project, container, status) = decode_source_from_proxy_authorization(&header_value);
        assert_eq!(status, SourceIdentityStatus::Ok);
        assert_eq!(project, Some("myproj".to_string()));
        assert_eq!(container, Some("mycont".to_string()));
    }

    #[test]
    fn bypass_host_matches_wildcards() {
        assert!(bypass_host_matches("*.google.com", "api.google.com"));
        assert!(bypass_host_matches("*.google.com", "google.com"));
        assert!(bypass_host_matches(".google.com", "api.google.com"));
        assert!(bypass_host_matches("google.com", "google.com"));
        assert!(!bypass_host_matches("google.com", "notgoogle.com"));
    }

    #[tokio::test]
    async fn prompt_network_sends_to_pending_tx() {
        let (_ca_tx, _ca_rx) = mpsc::channel::<()>(1); // dummy
        let (pending_tx, mut pending_rx) = mpsc::channel(1);
        let ca =
            Arc::new(CaStore::load_or_create(&std::env::temp_dir().join("proxy-test-ca")).unwrap());
        // Wait, I can just use build_test_app logic if I want but let's just make a dummy config.
        let raw = r#"
docker_dir = "/tmp"
[manager]
global_rules_file = "/tmp/global.toml"
[workspace]
root = "/tmp/ws"
"#;
        let cfg: crate::config::Config = toml::from_str(raw).unwrap();
        let state = ProxyState::new(ca, SharedConfig::new(Arc::new(cfg)), pending_tx).unwrap();

        let prompt_task = tokio::spawn(async move {
            prompt_network(
                &state,
                "GET",
                "example.com",
                "/test",
                Some("p".into()),
                Some("c".into()),
                "ok",
                true,
            )
            .await
        });

        // TUI side: receive the item
        let item = pending_rx
            .recv()
            .await
            .expect("should receive pending item");
        assert_eq!(item.host, "example.com");

        // TUI side: allow it
        item.response_tx.send(NetworkDecision::Allow).unwrap();

        let result = prompt_task.await.unwrap();
        assert!(result, "prompt_network should return true for Allow");
    }
}

```

## src/rules/core.rs

```rs
/// Parses `void-rules.toml` files and composes global + per-project rules.
///
/// `void-rules.toml` lives in the canonical project root (committed to git).
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

// ── void-rules.toml schema ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ProjectRules {
    /// Optional instructions for a human or LLM agent. This field is preserved
    /// across automatic edits to this file (e.g. when void-claw appends a new
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

pub fn host_matches(pattern: &str, host: &str) -> bool {
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

/// Load a `void-rules.toml` file.  Returns a default (empty) rule set if the
/// file does not exist, rather than an error.
pub fn load(path: &Path) -> Result<ProjectRules> {
    if !path.exists() {
        return Ok(ProjectRules::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading void-rules.toml: {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing void-rules.toml: {}", path.display()))
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
# void-rules.toml — policy for what the AI agent can do in this project.
# Commit this file to your repository. void-claw reads it but never pushes
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

```

## src/rules/mod.rs

```rs
mod core;

pub use core::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests_file;

```

## src/rules/tests.rs

```rs
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
        let dir = std::env::temp_dir().join(format!("void-claw-rules-test-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("void-rules.toml");
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
        let dir = std::env::temp_dir().join(format!("void-claw-rules-header-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("void-rules.toml");

        write_rules_file(&path, &ProjectRules::default(), false).expect("write");
        let s = std::fs::read_to_string(&path).expect("read");
        assert!(
            s.starts_with("# void-rules.toml — policy"),
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
        let dir = std::env::temp_dir().join(format!("void-claw-rules-header-append-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("void-rules.toml");

        append_auto_approval(&path, &["echo".to_string(), "hi".to_string()], "/tmp")
            .expect("append");
        let s = std::fs::read_to_string(&path).expect("read");
        assert!(
            s.starts_with("# void-rules.toml — policy"),
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
                commands: vec![
                    RuleCommand {
                        argv: vec!["cargo".into(), "test".into()],
                        cwd: "/tmp".into(),
                        approval_mode: ApprovalMode::Auto,
                        ..Default::default()
                    },
                ],
                command_aliases: Default::default(),
            },
            ..Default::default()
        };

        // Partial match (subset)
        let matched = rules.find_hostdo_command(&["cargo".into()]);
        assert!(matched.is_none());

        // Partial match (superset)
        let matched = rules.find_hostdo_command(&["cargo".into(), "test".into(), "--verbose".into()]);
        assert!(matched.is_none());
    }

    #[test]
    fn find_hostdo_command_respects_argument_order() {
        let rules = ComposedRules {
            hostdo: HostdoRules {
                default_policy: ApprovalMode::Prompt,
                commands: vec![
                    RuleCommand {
                        argv: vec!["arg1".into(), "arg2".into()],
                        cwd: "/tmp".into(),
                        approval_mode: ApprovalMode::Auto,
                        ..Default::default()
                    },
                ],
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

        let matched = rules.find_hostdo_command(&vec![]);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().approval_mode, ApprovalMode::Deny);

        let matched = rules.find_hostdo_command(&vec!["ls".into()]);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().approval_mode, ApprovalMode::Auto);
    }
}

```

## src/server/core.rs

```rs
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tracing::Instrument;

use crate::config::{self, ApprovalMode, AuditExportLevel};
use crate::exec::{self, CommandMatch, DenyReason};
use crate::server::{
    ApprovalDecision, ErrorResponse, ExecRequest, ExecResponse, PendingItem, ServerState,
    deny as server_deny, record_audit, require_session_identity, resolve_exec_argv_aliases,
    resolve_host_cwd,
};
use crate::state::{AuditEntry, DecisionKind};

// ── Handler ──────────────────────────────────────────────────────────────────

pub(super) async fn exec_handler(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(req): Json<ExecRequest>,
) -> Response {
    // Extract optional context headers injected by hostdo.
    let caller_pid: Option<u32> = headers
        .get("x-hostdo-pid")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());

    let identity = match require_session_identity(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let identity_project = identity.project.clone();
    let identity_container_id = identity.container_id.clone();
    let identity_mount_target = identity.mount_target.clone();

    let cfg = state.config.get();

    // Find project config.
    let proj = match cfg.projects.iter().find(|p| p.name == identity_project) {
        Some(p) => p,
        None => {
            return server_deny(format!("unknown project '{}'", identity_project));
        }
    };
    // Extract path strings before alias resolution (aliases may reference them).
    let canonical_path = proj.canonical_path.display().to_string();
    let workspace_path_buf =
        config::effective_mount_source_path(proj, &cfg.workspace, &cfg.defaults);

    let resolved = match resolve_exec_argv_aliases(
        &req.argv,
        &config::effective_command_aliases(proj, &cfg.defaults),
        &proj.canonical_path,
        &workspace_path_buf,
    ) {
        Ok(v) => v,
        Err(reason) => return server_deny(reason),
    };
    let exec_argv = resolved.argv;
    let workspace_path = workspace_path_buf.display().to_string();

    let request_cwd = PathBuf::from(&req.cwd);
    let has_cwd_override = resolved.cwd_override.is_some();
    let host_cwd = if let Some(cwd_override) = resolved.cwd_override {
        cwd_override
    } else {
        resolve_host_cwd(
            &request_cwd,
            Some(identity_mount_target.as_str()),
            &workspace_path_buf,
        )
    };

    // Hard-deny check (executable denylist, fragment denylist).
    if let Some(reason) = exec::check_denied(&exec_argv, proj, &cfg) {
        let kind = match &reason {
            DenyReason::EmptyArgv
            | DenyReason::DeniedExecutable(_)
            | DenyReason::DeniedArgumentFragment(_) => DecisionKind::DeniedByPolicy,
        };
        record_audit(
            &state,
            AuditEntry {
                project: identity_project.clone(),
                argv: exec_argv.clone(),
                cwd: req.cwd.clone(),
                decision: kind,
                exit_code: None,
                duration_ms: None,
                timestamp: Utc::now(),
            },
        )
        .await;
        return server_deny(reason.to_string());
    }

    // Load composed rules from void-rules.toml files (global + all projects).
    let mut rules =
        match config::load_composed_rules_for_project(&cfg, Some(identity_project.as_str())) {
            Ok(rules) => rules,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "invalid_rules".into(),
                        reason: e.to_string(),
                    }),
                )
                    .into_response();
            }
        };

    // Expand $CANONICAL / $WORKSPACE in rule cwds so matching works.
    let effective_mount_target = identity_mount_target.as_str();
    rules.expand_cwd_vars(&canonical_path, effective_mount_target);

    // Command matching against the composed rules.
    let cmd_match = exec::find_matching_command(&exec_argv, &rules);

    // For unlisted commands (which require approval), default the CWD to the
    // canonical project directory rather than the workspace copy.
    let host_cwd = match &cmd_match {
        CommandMatch::Unlisted if !has_cwd_override => resolve_host_cwd(
            &request_cwd,
            Some(identity_mount_target.as_str()),
            &proj.canonical_path,
        ),
        _ => host_cwd,
    };

    // Determine approval mode.
    let approval_mode = match &cmd_match {
        CommandMatch::Explicit(cmd) => cmd.approval_mode.clone(),
        CommandMatch::Unlisted => rules.hostdo.default_policy.clone(),
    };

    if approval_mode == ApprovalMode::Deny {
        let reason = match &cmd_match {
            CommandMatch::Explicit(_) => "command denied by rule".to_string(),
            CommandMatch::Unlisted => {
                "command not in allowlist and default_policy is deny".to_string()
            }
        };
        record_audit(
            &state,
            AuditEntry {
                project: identity_project.clone(),
                argv: exec_argv.clone(),
                cwd: req.cwd.clone(),
                decision: DecisionKind::DeniedByPolicy,
                exit_code: None,
                duration_ms: None,
                timestamp: Utc::now(),
            },
        )
        .await;
        return server_deny(reason);
    }

    let (env_profile, timeout_secs) = match &cmd_match {
        CommandMatch::Explicit(cmd) => (cmd.env_profile.clone(), cmd.timeout_secs),
        CommandMatch::Unlisted => (None, 60u64),
    };
    let matched_command_name = match &cmd_match {
        CommandMatch::Explicit(cmd) => Some(cmd.display_name()),
        CommandMatch::Unlisted => None,
    };

    // Export level from logging config.
    let export_level = cfg
        .logging
        .otlp
        .as_ref()
        .map(|o| o.level.clone())
        .unwrap_or(AuditExportLevel::None);

    // Build a pre-populated span for this request.
    let make_span = || -> tracing::Span {
        let span = tracing::info_span!(
            "hostdo",
            project = identity_project.as_str(),
            "caller.pid" = tracing::field::Empty,
            "container.id" = tracing::field::Empty,
            "project.canonical_path" = tracing::field::Empty,
            "project.workspace_path" = tracing::field::Empty,
            decision = tracing::field::Empty,
            exit_code = tracing::field::Empty,
            duration_ms = tracing::field::Empty,
            approval_wait_ms = tracing::field::Empty,
        );
        if let Some(pid) = caller_pid {
            span.record("caller.pid", pid);
        }
        span.record("container.id", identity_container_id.as_str());
        span.record("project.canonical_path", canonical_path.as_str());
        span.record("project.workspace_path", workspace_path.as_str());
        span
    };

    // ── Auto-run path ────────────────────────────────────────────────────────

    if approval_mode == ApprovalMode::Auto {
        let span = if export_level == AuditExportLevel::All {
            make_span()
        } else {
            tracing::Span::none()
        };
        let span_rec = span.clone();

        let env_vars = exec::resolve_env(env_profile.as_deref(), &cfg);
        let argv = exec_argv.clone();
        let cwd2 = host_cwd.clone();
        return async move {
            match exec::run_command(&argv, &cwd2, &env_vars, timeout_secs).await {
                Ok(result) => {
                    tracing::Span::current().record("decision", "auto");
                    tracing::Span::current().record("exit_code", result.exit_code);
                    tracing::Span::current().record("duration_ms", result.duration_ms as i64);
                    record_audit(
                        &state,
                        AuditEntry {
                            project: identity_project.clone(),
                            argv: exec_argv.clone(),
                            cwd: req.cwd.clone(),
                            decision: DecisionKind::Auto,
                            exit_code: Some(result.exit_code),
                            duration_ms: Some(result.duration_ms),
                            timestamp: Utc::now(),
                        },
                    )
                    .await;
                    Json(ExecResponse {
                        exit_code: result.exit_code,
                        stdout: result.stdout,
                        stderr: result.stderr,
                    })
                    .into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "execution_failed".into(),
                        reason: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
        .instrument(span_rec)
        .await;
    }

    // ── Prompt path — send to TUI and wait for approval ──────────────────────

    // For `approvals` or `all` level, create a span that covers the wait + exec.
    let span = if export_level != AuditExportLevel::None {
        make_span()
    } else {
        tracing::Span::none()
    };
    let span_rec = span.clone();

    let id = uuid::Uuid::new_v4().to_string();
    let (response_tx, response_rx) = oneshot::channel::<ApprovalDecision>();

    let pending = PendingItem {
        id: id.clone(),
        project: identity_project.clone(),
        container_id: Some(identity_container_id.clone()),
        argv: exec_argv.clone(),
        cwd: host_cwd.clone(),
        rule_cwd: request_cwd.clone(),
        matched_command: matched_command_name,
        response_tx: Some(response_tx),
    };

    if state.pending_tx.send(pending).await.is_err() {
        return server_deny("manager is shutting down".to_string());
    }

    // Await the developer decision (and execution) under the span.
    let env_vars = exec::resolve_env(env_profile.as_deref(), &cfg);
    async move {
        let approval_start = Instant::now();

        // Wait for developer decision (5 minute timeout).
        let decision = match tokio::time::timeout(Duration::from_secs(300), response_rx).await {
            Ok(Ok(d)) => d,
            Ok(Err(_)) | Err(_) => {
                let wait_ms = approval_start.elapsed().as_millis() as i64;
                tracing::Span::current().record("decision", "timed_out");
                tracing::Span::current().record("approval_wait_ms", wait_ms);
                record_audit(
                    &state,
                    AuditEntry {
                        project: identity_project.clone(),
                        argv: exec_argv.clone(),
                        cwd: req.cwd.clone(),
                        decision: DecisionKind::TimedOut,
                        exit_code: None,
                        duration_ms: None,
                        timestamp: Utc::now(),
                    },
                )
                .await;
                return server_deny("approval timed out (5 minutes)".to_string());
            }
        };

        let approval_wait_ms = approval_start.elapsed().as_millis() as i64;
        tracing::Span::current().record("approval_wait_ms", approval_wait_ms);

        match decision {
            ApprovalDecision::Deny => {
                tracing::Span::current().record("decision", "denied");
                record_audit(
                    &state,
                    AuditEntry {
                        project: identity_project.clone(),
                        argv: exec_argv.clone(),
                        cwd: req.cwd.clone(),
                        decision: DecisionKind::Denied,
                        exit_code: None,
                        duration_ms: None,
                        timestamp: Utc::now(),
                    },
                )
                .await;
                server_deny("denied by developer".to_string())
            }
            ApprovalDecision::Approve { remember } => {
                match exec::run_command(&exec_argv, &host_cwd, &env_vars, timeout_secs).await {
                    Ok(result) => {
                        let decision_label = if remember { "remembered" } else { "approved" };
                        tracing::Span::current().record("decision", decision_label);
                        tracing::Span::current().record("exit_code", result.exit_code);
                        tracing::Span::current().record("duration_ms", result.duration_ms as i64);
                        record_audit(
                            &state,
                            AuditEntry {
                                project: identity_project.clone(),
                                argv: exec_argv.clone(),
                                cwd: req.cwd.clone(),
                                decision: if remember {
                                    DecisionKind::Remembered
                                } else {
                                    DecisionKind::Approved
                                },
                                exit_code: Some(result.exit_code),
                                duration_ms: Some(result.duration_ms),
                                timestamp: Utc::now(),
                            },
                        )
                        .await;
                        Json(ExecResponse {
                            exit_code: result.exit_code,
                            stdout: result.stdout,
                            stderr: result.stderr,
                        })
                        .into_response()
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "execution_failed".into(),
                            reason: e.to_string(),
                        }),
                    )
                        .into_response(),
                }
            }
        }
    }
    .instrument(span_rec)
    .await
}

```

## src/server/handlers.rs

```rs
use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::instrument;

use crate::config::AliasValue;
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, StateManager};

/// A command request waiting for developer approval in the TUI.
pub struct PendingItem {
    /// Unique identifier for this pending item, used for TUI interaction and tracking.
    pub id: String,
    pub project: String,
    pub container_id: Option<String>,
    pub argv: Vec<String>,
    /// Host-side cwd used to actually execute the command.
    pub cwd: PathBuf,
    /// Container/request cwd used for rule matching and persistence.
    pub rule_cwd: PathBuf,
    pub matched_command: Option<String>,
    /// Sender for the `ApprovalDecision` once the TUI processes this item.
    pub response_tx: Option<oneshot::Sender<ApprovalDecision>>,
}

/// The decision returned by the TUI for a pending command request.
pub enum ApprovalDecision {
    /// Approve the command. `remember: true` means the approval will be persisted
    /// for future identical commands.
    Approve { remember: bool },
    /// Deny the command.
    Deny,
}

// ── HTTP types ───────────────────────────────────────────────────────────────

/// Request payload accepted by the hostdo HTTP endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ExecRequest {
    pub project: Option<String>,
    pub argv: Vec<String>,
    pub cwd: String,
}

/// Response payload returned by the hostdo HTTP endpoint.
#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Error payload returned by the hostdo HTTP endpoint.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub reason: String,
}

/// Request payload accepted by the container stop endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StopRequest {
    pub project: Option<String>,
    pub container_id: Option<String>,
}

/// Response payload returned by the container stop endpoint.
#[derive(Debug, Serialize)]
pub struct StopResponse {
    pub ok: bool,
}

// ── Server state ─────────────────────────────────────────────────────────────

/// Represents the identity of a running container session.
#[derive(Debug, Clone)]
pub struct SessionIdentity {
    pub project: String,
    pub container_id: String,
    pub mount_target: String,
}

/// A registry for active container sessions, mapping session tokens to their identities.
/// Provides thread-safe access to session information.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<String, SessionIdentity>>>,
}

impl SessionRegistry {
    /// Inserts a new session identity into the registry.
    /// Acquires a lock to safely modify the internal map.
    pub fn insert(&self, session_token: String, identity: SessionIdentity) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(session_token, identity);
        }
    }

    /// Removes a session identity from the registry.
    /// Acquires a lock to safely modify the internal map.
    pub fn remove(&self, session_token: &str) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(session_token);
        }
    }

    /// Retrieves a session identity from the registry.
    /// Acquires a lock to safely read from the internal map.
    pub fn get(&self, session_token: &str) -> Option<SessionIdentity> {
        self.inner
            .lock()
            .ok()
            .and_then(|map| map.get(session_token).cloned())
    }
}

/// Shared server state for hostdo requests and other manager operations.
/// This state is shared across all HTTP handlers.
#[derive(Clone)]
pub struct ServerState {
    pub config: SharedConfig,
    pub state: StateManager,
    /// Channel to send `PendingItem`s to the TUI for developer approval.
    pub pending_tx: mpsc::Sender<PendingItem>,
    /// Channel to send `ContainerStopItem`s to the TUI to handle container termination.
    pub stop_tx: mpsc::Sender<ContainerStopItem>,
    /// Channel to send `AuditEntry` events for logging and display in the TUI.
    pub audit_tx: mpsc::Sender<AuditEntry>,
    /// The secret token used for authenticating requests from containers.
    pub token: String,
    /// Registry of currently active container sessions.
    pub sessions: SessionRegistry,
}

/// A container stop request waiting to be handled by the TUI.
pub struct ContainerStopItem {
    pub project: String,
    pub container_id: String,
    pub response_tx: Option<oneshot::Sender<ContainerStopDecision>>,
}

/// The decision returned by the TUI for a stop request.
pub enum ContainerStopDecision {
    Stopped,
    NotFound,
}

// NOTE: hostdo commands execute on the developer machine.
// They must not inherit the managed network proxy environment.

/// Initializes and runs the Axum HTTP server to listen for incoming requests.
/// This server handles `/exec` commands from containers (via `hostdo`) and `/container/stop` requests (via `killme`).
#[instrument(skip(server_state, listener))]
pub async fn run_with_listener(
    server_state: ServerState,
    listener: tokio::net::TcpListener,
) -> Result<()> {
    // The server state is wrapped in Arc so it can be shared immutably across multiple handler instances.
    let router = Router::new()
        .route("/exec", post(super::core::exec_handler))
        .route("/container/stop", post(stop_handler))
        .with_state(Arc::new(server_state));

    axum::serve(listener, router).await?;
    Ok(())
}

/// Handles incoming requests to stop a container.
///
/// This endpoint is typically called by the `killme` script within a container.
/// It verifies the session identity and then sends a `ContainerStopItem` to the TUI
/// for processing. A timeout is applied for awaiting the TUI's decision.
pub(super) async fn stop_handler(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(_req): Json<StopRequest>,
) -> Response {
    let identity = match require_session_identity(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let (response_tx, response_rx) = oneshot::channel::<ContainerStopDecision>();
    let item = ContainerStopItem {
        project: identity.project.clone(),
        container_id: identity.container_id.clone(),
        response_tx: Some(response_tx),
    };
    if state.stop_tx.send(item).await.is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "manager_shutting_down".into(),
                reason: "manager is shutting down".into(),
            }),
        )
            .into_response();
    }

    // Wait for the TUI to process the stop request, with a 10-second timeout.
    // This timeout duration is currently fixed but could be made configurable.
    match tokio::time::timeout(Duration::from_secs(10), response_rx).await {
        Ok(Ok(ContainerStopDecision::Stopped)) => Json(StopResponse { ok: true }).into_response(),
        Ok(Ok(ContainerStopDecision::NotFound)) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".into(),
                reason: "no running container matched the request".into(),
            }),
        )
            .into_response(),
        Ok(Err(_)) | Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            Json(ErrorResponse {
                error: "timeout".into(),
                reason: "timed out waiting for the container stop request".into(),
            }),
        )
            .into_response(),
    }
}

/// Creates a standard HTTP 403 Forbidden response with a JSON error payload.
pub(super) fn deny(reason: String) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "denied".into(),
            reason,
        }),
    )
        .into_response()
}

/// Validates the session identity from incoming request headers.
///
/// This function checks for:
/// 1. A valid `Authorization` header with a `Bearer` token matching the server's secret token.
/// 2. A non-empty `x-void-claw-session-token` header.
/// 3. That the session token corresponds to an active session in the `SessionRegistry`.
///
/// Returns `Ok(SessionIdentity)` on success, or an `Err(Response)` with an appropriate
/// HTTP status code and error message on failure.
pub(super) fn require_session_identity(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<SessionIdentity, Response> {
    // Extract and validate the Authorization header.
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(""); // If header is missing or invalid, it defaults to an empty string.
    let expected = format!("Bearer {}", state.token);
    if auth != expected {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".into(),
                reason: "invalid or missing token".into(),
            }),
        )
            .into_response());
    }

    // Extract and validate the session token.
    let session_token = headers
        .get("x-void-claw-session-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .trim();
    if session_token.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".into(),
                reason: "missing session token".into(),
            }),
        )
            .into_response());
    }

    // Look up the session in the registry.
    state.sessions.get(session_token).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".into(),
                reason: "unknown session token".into(),
            }),
        )
            .into_response()
    })
}

/// Records an audit entry.
///
/// The entry is sent over a channel to the TUI for display and logged to persistent storage
/// on a blocking thread to avoid impacting the main event loop.
pub(super) async fn record_audit(state: &ServerState, entry: AuditEntry) {
    let _ = state.audit_tx.send(entry.clone()).await;
    let state_clone = state.state.clone();
    tokio::task::spawn_blocking(move || {
        let _ = state_clone.log_audit(&entry);
    });
}

/// Resolves the effective host-side current working directory (CWD) for a command.
///
/// This function translates a container's CWD into the corresponding host CWD,
/// taking into account explicit mount targets and a fallback to the historical
/// `/workspace` mapping.
pub(super) fn resolve_host_cwd(
    request_cwd: &Path,
    mount_target: Option<&str>,
    workspace_path: &Path,
) -> PathBuf {
    fn map_if_under(
        request_cwd: &Path,
        mount_target: &Path,
        workspace_path: &Path,
    ) -> Option<PathBuf> {
        if request_cwd == mount_target || request_cwd.starts_with(mount_target) {
            let rel = request_cwd.strip_prefix(mount_target).ok()?;
            return Some(workspace_path.join(rel));
        }
        None
    }

    if let Some(mt) = mount_target {
        let mt_path = Path::new(mt);
        if let Some(mapped) = map_if_under(request_cwd, mt_path, workspace_path) {
            return mapped;
        }
    }

    // Fall back to the historical `/workspace` mapping for older containers
    // and clients that still only report the workspace path.
    if let Some(mapped) = map_if_under(request_cwd, Path::new("/workspace"), workspace_path) {
        return mapped;
    }

    request_cwd.to_path_buf()
}

/// Represents a resolved command alias, including the expanded argv and an optional CWD override.
pub(super) struct ResolvedAlias {
    pub(super) argv: Vec<String>,
    pub(super) cwd_override: Option<PathBuf>,
}

/// Resolves command aliases for hostdo requests.
///
/// If the first argument of `argv` matches an alias, it expands the alias
/// command and appends any remaining arguments. It also resolves magic CWD
/// placeholders (`$CANONICAL`, `$WORKSPACE`) in alias definitions.
/// The `shell_words::split` crate is used to correctly parse shell-like alias commands.
pub(super) fn resolve_exec_argv_aliases(
    argv: &[String],
    aliases: &HashMap<String, AliasValue>,
    canonical_path: &Path,
    workspace_path: &Path,
) -> std::result::Result<ResolvedAlias, String> {
    if argv.is_empty() {
        return Ok(ResolvedAlias {
            argv: Vec::new(),
            cwd_override: None,
        });
    }
    let Some(alias) = aliases.get(&argv[0]) else {
        // No alias found, return original argv.
        return Ok(ResolvedAlias {
            argv: argv.to_vec(),
            cwd_override: None,
        });
    };
    let mut expanded = shell_words::split(alias.cmd())
        .map_err(|e| format!("invalid hostdo alias '{}': {}", argv[0], e))?;
    if expanded.is_empty() {
        return Err(format!(
            "invalid hostdo alias '{}': mapped command is empty",
            argv[0]
        ));
    }
    // Append any arguments that followed the alias.
    if argv.len() > 1 {
        expanded.extend_from_slice(&argv[1..]);
    }
    Ok(ResolvedAlias {
        argv: expanded,
        cwd_override: alias.resolve_cwd(canonical_path, workspace_path),
    })
}

#[cfg(test)]
mod tests {
    use super::{resolve_exec_argv_aliases, resolve_host_cwd};
    use crate::config::AliasValue;
    use crate::server::SessionRegistry;
    use crate::shared_config::SharedConfig;
    use crate::state::StateManager;
    use axum::{
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
    };
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn resolve_host_cwd_maps_using_mount_target_header() {
        let request = Path::new("/custom-mount/src/subdir");
        let workspace = Path::new("/tmp/workspaces/project-a");
        let mapped = resolve_host_cwd(request, Some("/custom-mount/src"), workspace);
        assert_eq!(mapped, PathBuf::from("/tmp/workspaces/project-a/subdir"));
    }

    #[test]
    fn resolve_host_cwd_uses_workspace_fallback_when_header_missing() {
        let request = Path::new("/workspace/api");
        let workspace = Path::new("/tmp/workspaces/project-b");
        let mapped = resolve_host_cwd(request, None, workspace);
        assert_eq!(mapped, PathBuf::from("/tmp/workspaces/project-b/api"));
    }

    #[test]
    fn alias_resolution_expands_and_appends_runtime_args() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "tests".to_string(),
            AliasValue::Simple("cargo test --all".to_string()),
        );
        let argv = vec!["tests".to_string(), "--release".to_string()];
        let out = resolve_exec_argv_aliases(
            &argv,
            &aliases,
            Path::new("/canonical"),
            Path::new("/workspace"),
        )
        .expect("alias should resolve");
        assert_eq!(out.argv, vec!["cargo", "test", "--all", "--release"]);
        assert_eq!(out.cwd_override, None);
    }

    #[test]
    fn alias_resolution_supports_magic_workspace_cwd() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "lint".to_string(),
            AliasValue::WithOptions {
                cmd: "cargo clippy".to_string(),
                cwd: Some(PathBuf::from("$WORKSPACE")),
            },
        );
        let argv = vec!["lint".to_string()];
        let out = resolve_exec_argv_aliases(
            &argv,
            &aliases,
            Path::new("/canonical"),
            Path::new("/workspace/path"),
        )
        .expect("alias should resolve");
        assert_eq!(out.argv, vec!["cargo", "clippy"]);
        assert_eq!(out.cwd_override, Some(PathBuf::from("/workspace/path")));
    }

    #[test]
    fn alias_resolution_rejects_empty_mapped_command() {
        let mut aliases = HashMap::new();
        aliases.insert("bad".to_string(), AliasValue::Simple("   ".to_string()));
        let argv = vec!["bad".to_string()];
        let err = match resolve_exec_argv_aliases(
            &argv,
            &aliases,
            Path::new("/canonical"),
            Path::new("/workspace"),
        ) {
            Ok(_) => panic!("empty alias should fail"),
            Err(e) => e,
        };
        assert!(err.contains("mapped command is empty"));
    }

    #[test]
    fn alias_resolution_does_not_recurse() {
        let mut aliases = HashMap::new();
        // alias a -> "hostdo b"
        // alias b -> "cargo test"
        // If it doesn't recurse, "hostdo a" becomes ["hostdo", "b"].
        aliases.insert("a".to_string(), AliasValue::Simple("hostdo b".to_string()));
        aliases.insert(
            "b".to_string(),
            AliasValue::Simple("cargo test".to_string()),
        );

        let argv = vec!["a".to_string()];
        let out = resolve_exec_argv_aliases(
            &argv,
            &aliases,
            Path::new("/canonical"),
            Path::new("/workspace"),
        )
        .expect("alias should resolve");

        assert_eq!(out.argv, vec!["hostdo", "b"]);
    }

    #[tokio::test]
    async fn require_session_identity_missing_auth_header() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(), // Use a real path for StateManager
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions: SessionRegistry::default(),
        };
        let headers = HeaderMap::new();

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_invalid_auth_token() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "valid_token".to_string(),
            sessions: SessionRegistry::default(),
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer invalid_token".parse().unwrap());
        headers.insert(
            "x-void-claw-session-token",
            "some_session_token".parse().unwrap(),
        );

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_missing_session_token() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions: SessionRegistry::default(),
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token".parse().unwrap());

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_unknown_session_token() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions: SessionRegistry::default(),
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token".parse().unwrap());
        headers.insert(
            "x-void-claw-session-token",
            "unknown_session".parse().unwrap(),
        );

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_valid_session() {
        let sessions = SessionRegistry::default();
        sessions.insert(
            "valid_session".to_string(),
            super::SessionIdentity {
                project: "test_project".to_string(),
                container_id: "test_container".to_string(),
                mount_target: "/workspace".to_string(),
            },
        );
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions,
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token".parse().unwrap());
        headers.insert(
            "x-void-claw-session-token",
            "valid_session".parse().unwrap(),
        );

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_ok());
        let identity = result.unwrap();
        assert_eq!(identity.project, "test_project");
        assert_eq!(identity.container_id, "test_container");
        assert_eq!(identity.mount_target, "/workspace");
    }
}

```

## src/server/mod.rs

```rs
mod core;
mod handlers;

pub use handlers::*;

```

## src/shared_config.rs

```rs
use crate::config::Config;
use std::sync::{Arc, RwLock};

/// Thread-safe hot-reloadable handle to the current config.
///
/// This is used so the TUI can update the config at runtime and the hostdo
/// server + proxy can see the new project list without restart.
#[derive(Clone)]
pub struct SharedConfig {
    inner: Arc<RwLock<Arc<Config>>>,
}

impl SharedConfig {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(config)),
        }
    }

    pub fn get(&self) -> Arc<Config> {
        self.inner.read().expect("config lock poisoned").clone()
    }

    pub fn set(&self, config: Arc<Config>) {
        *self.inner.write().expect("config lock poisoned") = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::Arc;

    #[test]
    fn shared_config_hot_reloads() {
        let config1 = Arc::new(Config::default());
        let shared = SharedConfig::new(config1);

        let config2 = Arc::new(Config {
            docker_dir: std::path::PathBuf::from("/new/docker"),
            ..Config::default()
        });

        shared.set(config2);

        let current = shared.get();
        assert_eq!(current.docker_dir, std::path::PathBuf::from("/new/docker"));
    }

    #[test]
    fn shared_config_clones_independent_reference() {
        let config1 = Arc::new(Config::default());
        let shared1 = SharedConfig::new(config1);
        let shared2 = shared1.clone();

        let config2 = Arc::new(Config {
            docker_dir: std::path::PathBuf::from("/shared/docker"),
            ..Config::default()
        });

        shared1.set(config2);

        // Both clones should see the update
        assert_eq!(
            shared2.get().docker_dir,
            std::path::PathBuf::from("/shared/docker")
        );
    }
}

```

## src/state.rs

```rs
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub project: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub decision: DecisionKind,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Auto,
    Approved,
    Remembered,
    Denied,
    DeniedByPolicy,
    TimedOut,
}

impl DecisionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Approved => "APPR",
            Self::Remembered => "REMB",
            Self::Denied => "DENY",
            Self::DeniedByPolicy => "DENY*",
            Self::TimedOut => "TOUT",
        }
    }
}

#[derive(Clone)]
pub struct StateManager {
    log_dir: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl StateManager {
    pub fn open(log_dir: &Path) -> Result<Self> {
        fs::create_dir_all(log_dir)
            .with_context(|| format!("creating log dir: {}", log_dir.display()))?;
        Ok(Self {
            log_dir: log_dir.to_path_buf(),
            lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn get_or_create_token(&self) -> Result<String> {
        let _guard = self.lock.lock().unwrap();
        let path = self.token_path();
        if path.exists() {
            let token = fs::read_to_string(&path)
                .with_context(|| format!("reading token file: {}", path.display()))?;
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }

        let token = uuid::Uuid::new_v4().to_string().replace('-', "");
        fs::write(&path, format!("{token}\n"))
            .with_context(|| format!("writing token file: {}", path.display()))?;
        Ok(token)
    }

    /// Append one audit event to the current UTC day file as JSONL.
    pub fn log_audit(&self, entry: &AuditEntry) -> Result<()> {
        let _guard = self.lock.lock().unwrap();
        let path = self.audit_path_for(entry.timestamp.date_naive());
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening audit log: {}", path.display()))?;
        let line = serde_json::to_string(entry).context("serializing audit entry")?;
        f.write_all(line.as_bytes())
            .with_context(|| format!("writing audit log: {}", path.display()))?;
        f.write_all(b"\n")
            .with_context(|| format!("writing audit newline: {}", path.display()))?;
        Ok(())
    }

    /// Load the most recent audit events (newest first) from daily JSONL files.
    pub fn recent_audit(&self, limit: usize) -> Result<Vec<AuditEntry>> {
        let mut files: Vec<PathBuf> = fs::read_dir(&self.log_dir)
            .with_context(|| format!("reading log dir: {}", self.log_dir.display()))?
            .filter_map(|ent| ent.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("audit-") && n.ends_with(".log"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        files.reverse();

        let mut out = Vec::new();
        for path in files {
            if out.len() >= limit {
                break;
            }
            let f = match fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let reader = BufReader::new(f);
            let mut day_entries = Vec::new();
            for line in reader.lines() {
                let Ok(line) = line else {
                    continue;
                };
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                    day_entries.push(entry);
                }
            }
            day_entries.sort_by_key(|e| e.timestamp);
            day_entries.reverse();
            for entry in day_entries {
                out.push(entry);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    fn token_path(&self) -> PathBuf {
        self.log_dir.join("token")
    }

    fn audit_path_for(&self, day: chrono::NaiveDate) -> PathBuf {
        self.log_dir
            .join(format!("audit-{}.log", day.format("%Y-%m-%d")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn get_or_create_token_is_persistent() {
        let dir = tempdir().expect("create temp dir");
        let state1 = StateManager::open(dir.path()).expect("open state1");
        let token1 = state1.get_or_create_token().expect("get token1");

        // Re-open same dir
        let state2 = StateManager::open(dir.path()).expect("open state2");
        let token2 = state2.get_or_create_token().expect("get token2");

        assert_eq!(token1, token2);
    }

    #[test]
    fn log_audit_and_recent_audit_works() {
        let dir = tempdir().expect("create temp dir");
        let state = StateManager::open(dir.path()).expect("open state");

        let now = Utc::now();
        let entry1 = AuditEntry {
            project: "p1".to_string(),
            argv: vec!["ls".into()],
            cwd: "/".into(),
            decision: DecisionKind::Auto,
            exit_code: Some(0),
            duration_ms: Some(10),
            timestamp: now - chrono::Duration::seconds(10),
        };
        let entry2 = AuditEntry {
            project: "p1".to_string(),
            argv: vec!["pwd".into()],
            cwd: "/".into(),
            decision: DecisionKind::Approved,
            exit_code: Some(0),
            duration_ms: Some(5),
            timestamp: now,
        };

        state.log_audit(&entry1).expect("log 1");
        state.log_audit(&entry2).expect("log 2");

        let recent = state.recent_audit(10).expect("recent");
        assert_eq!(recent.len(), 2);
        // Should be newest first
        assert_eq!(recent[0].argv[0], "pwd");
        assert_eq!(recent[1].argv[0], "ls");
    }

    #[test]
    fn recent_audit_handles_malformed_lines() {
        let dir = tempdir().expect("create temp dir");
        let state = StateManager::open(dir.path()).expect("open state");
        let path = state.audit_path_for(Utc::now().date_naive());

        fs::write(&path, "not json\n{\"project\":\"p\"}\n").expect("write malformed");

        // Only valid JSON lines should be returned (though my simple test entry is incomplete,
        // AuditEntry requires more fields, so it might skip both if not valid).
        // Let's write one valid entry and one invalid.
        let entry = AuditEntry {
            project: "valid".to_string(),
            argv: vec![],
            cwd: "".into(),
            decision: DecisionKind::Auto,
            exit_code: None,
            duration_ms: None,
            timestamp: Utc::now(),
        };
        state.log_audit(&entry).expect("log valid");

        let recent = state.recent_audit(10).expect("recent");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].project, "valid");
    }

    #[test]
    fn recent_audit_spans_multiple_days() {
        let dir = tempdir().expect("create temp dir");
        let state = StateManager::open(dir.path()).expect("open state");

        let day1 = Utc::now() - chrono::Duration::days(1);
        let day2 = Utc::now();

        let entry1 = AuditEntry {
            project: "p1".to_string(),
            argv: vec!["day1".into()],
            cwd: "".into(),
            decision: DecisionKind::Auto,
            exit_code: None,
            duration_ms: None,
            timestamp: day1,
        };
        let entry2 = AuditEntry {
            project: "p1".to_string(),
            argv: vec!["day2".into()],
            cwd: "".into(),
            decision: DecisionKind::Auto,
            exit_code: None,
            duration_ms: None,
            timestamp: day2,
        };

        state.log_audit(&entry1).expect("log 1");
        state.log_audit(&entry2).expect("log 2");

        let recent = state.recent_audit(10).expect("recent");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].argv[0], "day2");
        assert_eq!(recent[1].argv[0], "day1");
    }
}

```

## src/sync/core.rs

```rs
use super::helpers::{build_project_exclude_matcher, copy_symlink};
use anyhow::Result;
use chrono::{DateTime, Utc};
use globset::GlobSet;
use ignore::gitignore::Gitignore;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{
    self, ConflictPolicy, DefaultsConfig, ProjectConfig, SymlinkPolicy, SyncMode, WorkspaceSection,
};

/// void-rules.toml is always overwritten from canonical on seed and never
/// copied back to canonical on pushback.
const PROTECTED_RULE_FILE: &str = "void-rules.toml";

fn ensure_managed_workspace(proj: &ProjectConfig, defaults: &DefaultsConfig) -> Result<()> {
    let mode = config::effective_sync_mode(proj, defaults);
    anyhow::ensure!(
        mode != SyncMode::Direct,
        "sync is disabled for sync.mode='direct' (the container mounts canonical_path directly)"
    );
    Ok(())
}

/// Summary of a sync run, including copied/skipped counts and any errors.
#[derive(Debug, Clone)]
pub struct SyncReport {
    pub project: String,
    pub files_copied: usize,
    pub files_skipped: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<SyncError>,
    pub timestamp: DateTime<Utc>,
}

/// A file-level error captured during a sync run.
#[derive(Debug, Clone)]
pub struct SyncError {
    pub path: PathBuf,
    pub message: String,
}

pub(crate) struct ExcludeMatcher {
    pub(crate) exclude_set: GlobSet,
    pub(crate) gitignore: Gitignore,
}

// ── Shared Core Sync Logic ───────────────────────────────────────────────────

fn process_seed_file(
    src: &Path,
    dest: &Path,
    symlink_policy: &SymlinkPolicy,
    is_dir: bool,
    is_symlink: bool,
    report: &mut SyncReport,
) {
    if is_symlink {
        match symlink_policy {
            SymlinkPolicy::Reject => {
                report.files_skipped += 1;
                return;
            }
            SymlinkPolicy::Copy => {
                if let Err(e) = copy_symlink(src, dest) {
                    report.errors.push(SyncError {
                        path: src.to_path_buf(),
                        message: e.to_string(),
                    });
                } else {
                    report.files_copied += 1;
                }
                return;
            }
            SymlinkPolicy::Follow => {} // Fall through to standard copy
        }
    }

    if is_dir {
        if let Err(e) = std::fs::create_dir_all(dest) {
            report.errors.push(SyncError {
                path: dest.to_path_buf(),
                message: e.to_string(),
            });
        }
    } else {
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                report.errors.push(SyncError {
                    path: parent.to_path_buf(),
                    message: e.to_string(),
                });
                return;
            }
        }
        match std::fs::copy(src, dest) {
            Ok(_) => report.files_copied += 1,
            Err(e) => report.errors.push(SyncError {
                path: src.to_path_buf(),
                message: e.to_string(),
            }),
        }
    }
}

fn process_pushback_file(
    rel: &Path,
    src: &Path,
    canonical_dest: &Path,
    conflict_policy: &ConflictPolicy,
    canonical_rules_path: &Path,
    report: &mut SyncReport,
) {
    // Never push void-rules.toml back to canonical; warn if it was modified.
    if rel == Path::new(PROTECTED_RULE_FILE) {
        if src.exists() && canonical_rules_path.exists() {
            let ws_bytes = std::fs::read(src).unwrap_or_default();
            let canon_bytes = std::fs::read(canonical_rules_path).unwrap_or_default();
            if ws_bytes != canon_bytes {
                report.warnings.push(
                    "void-rules.toml was modified in workspace — changes discarded (edit the canonical copy instead)".to_string()
                );
            }
        }
        report.files_skipped += 1;
        return;
    }

    if !src.exists() || src.is_dir() {
        return; // Skip directories and deleted files
    }

    if src
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        report.files_skipped += 1;
        return;
    }

    // Conflict check
    if canonical_dest.exists() {
        if let (Ok(ws_meta), Ok(canon_meta)) =
            (std::fs::metadata(src), std::fs::metadata(canonical_dest))
        {
            if let (Ok(ws_mtime), Ok(canon_mtime)) = (ws_meta.modified(), canon_meta.modified()) {
                if canon_mtime > ws_mtime {
                    match conflict_policy {
                        ConflictPolicy::PreserveCanonical => {
                            report.files_skipped += 1;
                            return;
                        }
                        ConflictPolicy::PreserveWorkspace => {}
                    }
                }
            }
        }
    }

    if let Some(parent) = canonical_dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            report.errors.push(SyncError {
                path: parent.to_path_buf(),
                message: e.to_string(),
            });
            return;
        }
    }

    match std::fs::copy(src, canonical_dest) {
        Ok(_) => report.files_copied += 1,
        Err(e) => report.errors.push(SyncError {
            path: src.to_path_buf(),
            message: e.to_string(),
        }),
    }
}

// ── Public Sync API ──────────────────────────────────────────────────────────

/// Seed a workspace from canonical project files, honoring the project
/// exclude set and symlink policy.
pub fn seed(
    proj: &ProjectConfig,
    ws: &WorkspaceSection,
    defaults: &DefaultsConfig,
) -> Result<SyncReport> {
    ensure_managed_workspace(proj, defaults)?;
    let workspace_path = config::effective_workspace_path(proj, ws);
    let matcher = build_project_exclude_matcher(proj, defaults)?;
    let symlink_policy = proj
        .sync
        .as_ref()
        .and_then(|s| s.symlink_policy.clone())
        .unwrap_or_else(|| defaults.sync.symlink_policy.clone());

    std::fs::create_dir_all(&workspace_path)?;

    let mut report = SyncReport {
        project: proj.name.clone(),
        files_copied: 0,
        files_skipped: 0,
        warnings: vec![],
        errors: vec![],
        timestamp: Utc::now(),
    };

    for entry in WalkDir::new(&proj.canonical_path)
        .into_iter()
        .filter_entry(|e| {
            let rel = match e.path().strip_prefix(&proj.canonical_path) {
                Ok(r) => r,
                Err(_) => return true,
            };
            if rel == Path::new(PROTECTED_RULE_FILE) {
                return true;
            }
            !matcher.is_excluded(rel, e.file_type().is_dir())
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                report.errors.push(SyncError {
                    path: err.path().map(Path::to_path_buf).unwrap_or_default(),
                    message: err.to_string(),
                });
                continue;
            }
        };

        let rel = match entry.path().strip_prefix(&proj.canonical_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if rel.as_os_str().is_empty() {
            continue;
        }

        process_seed_file(
            entry.path(),
            &workspace_path.join(rel),
            &symlink_policy,
            entry.file_type().is_dir(),
            entry.path_is_symlink(),
            &mut report,
        );
    }

    Ok(report)
}

/// Seed only the supplied relative file list from canonical into workspace.
pub fn seed_files(
    proj: &ProjectConfig,
    ws: &WorkspaceSection,
    defaults: &DefaultsConfig,
    changed_paths: &[PathBuf],
) -> Result<SyncReport> {
    ensure_managed_workspace(proj, defaults)?;
    let workspace_path = config::effective_workspace_path(proj, ws);
    let symlink_policy = proj
        .sync
        .as_ref()
        .and_then(|s| s.symlink_policy.clone())
        .unwrap_or_else(|| defaults.sync.symlink_policy.clone());

    std::fs::create_dir_all(&workspace_path)?;

    let mut report = SyncReport {
        project: proj.name.clone(),
        files_copied: 0,
        files_skipped: 0,
        warnings: vec![],
        errors: vec![],
        timestamp: Utc::now(),
    };

    for rel in changed_paths {
        let src = proj.canonical_path.join(rel);
        if !src.exists() {
            continue;
        }

        let is_symlink = src
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        process_seed_file(
            &src,
            &workspace_path.join(rel),
            &symlink_policy,
            src.is_dir(),
            is_symlink,
            &mut report,
        );
    }

    Ok(report)
}

/// Push workspace changes back into canonical storage.
pub fn pushback(
    proj: &ProjectConfig,
    ws: &WorkspaceSection,
    defaults: &DefaultsConfig,
) -> Result<SyncReport> {
    ensure_managed_workspace(proj, defaults)?;
    let workspace_path = config::effective_workspace_path(proj, ws);

    anyhow::ensure!(workspace_path.exists(), "workspace path does not exist");
    anyhow::ensure!(
        proj.canonical_path.exists(),
        "canonical path does not exist"
    );

    let conflict_policy = proj
        .sync
        .as_ref()
        .and_then(|s| s.conflict_policy.clone())
        .unwrap_or_else(|| defaults.sync.conflict_policy.clone());

    let mut report = SyncReport {
        project: proj.name.clone(),
        files_copied: 0,
        files_skipped: 0,
        warnings: vec![],
        errors: vec![],
        timestamp: Utc::now(),
    };

    let matcher = build_project_exclude_matcher(proj, defaults)?;
    let canonical_rules_path = proj.canonical_path.join(PROTECTED_RULE_FILE);

    for entry in WalkDir::new(&workspace_path).into_iter().filter_entry(|e| {
        let rel = match e.path().strip_prefix(&workspace_path) {
            Ok(r) => r,
            Err(_) => return true,
        };
        if rel == Path::new(PROTECTED_RULE_FILE) {
            return true;
        }
        !matcher.is_excluded(rel, e.file_type().is_dir())
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                report.errors.push(SyncError {
                    path: err.path().map(Path::to_path_buf).unwrap_or_default(),
                    message: err.to_string(),
                });
                continue;
            }
        };

        let rel = match entry.path().strip_prefix(&workspace_path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        if rel.as_os_str().is_empty() {
            continue;
        }

        process_pushback_file(
            rel,
            entry.path(),
            &proj.canonical_path.join(rel),
            &conflict_policy,
            &canonical_rules_path,
            &mut report,
        );
    }

    Ok(report)
}

/// Push only the supplied relative file list from workspace into canonical.
pub fn pushback_files(
    proj: &ProjectConfig,
    ws: &WorkspaceSection,
    defaults: &DefaultsConfig,
    changed_paths: &[PathBuf],
) -> Result<SyncReport> {
    ensure_managed_workspace(proj, defaults)?;
    let workspace_path = config::effective_workspace_path(proj, ws);

    anyhow::ensure!(workspace_path.exists(), "workspace path does not exist");
    anyhow::ensure!(
        proj.canonical_path.exists(),
        "canonical path does not exist"
    );

    let conflict_policy = proj
        .sync
        .as_ref()
        .and_then(|s| s.conflict_policy.clone())
        .unwrap_or_else(|| defaults.sync.conflict_policy.clone());

    let mut report = SyncReport {
        project: proj.name.clone(),
        files_copied: 0,
        files_skipped: 0,
        warnings: vec![],
        errors: vec![],
        timestamp: Utc::now(),
    };

    let canonical_rules_path = proj.canonical_path.join(PROTECTED_RULE_FILE);

    for rel in changed_paths {
        process_pushback_file(
            rel,
            &workspace_path.join(rel),
            &proj.canonical_path.join(rel),
            &conflict_policy,
            &canonical_rules_path,
            &mut report,
        );
    }

    Ok(report)
}

```

## src/sync/helpers.rs

```rs
use crate::config::{self, DefaultsConfig, ProjectConfig};
use crate::sync::ExcludeMatcher;
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};

// ── Exclusion and Ignore Logic ───────────────────────────────────────────────

pub(crate) fn build_project_exclude_matcher(
    proj: &ProjectConfig,
    defaults: &DefaultsConfig,
) -> Result<ExcludeMatcher> {
    let patterns = config::combined_excludes(proj, defaults)?;
    Ok(ExcludeMatcher {
        exclude_set: build_exclude_set(&patterns)?,
        gitignore: build_gitignore_matcher(&proj.canonical_path)?,
    })
}

pub(crate) fn build_exclude_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
        if let Some(dir_pattern) = pattern.strip_suffix("/**") {
            builder.add(Glob::new(dir_pattern)?);
        }
    }
    Ok(builder.build()?)
}

fn build_gitignore_matcher(root: &Path) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    for path in discover_gitignore_files(root) {
        if let Some(err) = builder.add(&path) {
            return Err(err.into());
        }
    }
    Ok(builder.build()?)
}

fn discover_gitignore_files(root: &Path) -> Vec<PathBuf> {
    fn visit_dir(dir: &Path, out: &mut Vec<PathBuf>) {
        let gitignore = dir.join(".gitignore");
        if gitignore.is_file() {
            out.push(gitignore);
        }

        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        let mut child_dirs: Vec<(std::ffi::OsString, PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if !ft.is_dir() || ft.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            child_dirs.push((name, entry.path()));
        }

        child_dirs.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, child) in child_dirs {
            visit_dir(&child, out);
        }
    }

    let mut out = Vec::new();
    visit_dir(root, &mut out);
    out
}

pub(crate) fn is_excluded(rel: &Path, exclude_set: &GlobSet) -> bool {
    if exclude_set.is_match(rel) {
        return true;
    }
    for component in rel.components() {
        if let std::path::Component::Normal(name) = component {
            if name.to_str().map(|s| s.starts_with('.')).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

impl ExcludeMatcher {
    pub(crate) fn is_excluded(&self, rel: &Path, is_dir: bool) -> bool {
        if is_excluded(rel, &self.exclude_set) {
            return true;
        }
        self.gitignore
            .matched_path_or_any_parents(rel, is_dir)
            .is_ignore()
    }
}

#[cfg(unix)]
pub(crate) fn copy_symlink(src: &Path, dest: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    let target = std::fs::read_link(src)?;
    if dest.exists() || dest.is_symlink() {
        std::fs::remove_file(dest)?;
    }
    symlink(target, dest)?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn copy_symlink(_src: &Path, _dest: &Path) -> Result<()> {
    anyhow::bail!("symlink copy is not supported on this platform")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ApprovalMode, ConflictPolicy, DefaultsConfig, ProjectConfig, WorkspaceSection,
    };
    use crate::sync::{pushback, seed, seed_files};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("void-claw-sync-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn test_project(name: &str, canonical_path: &Path) -> ProjectConfig {
        ProjectConfig {
            name: name.to_string(),
            canonical_path: canonical_path.to_path_buf(),
            workspace_path: None,
            disposable: false,
            default_policy: ApprovalMode::default(),
            exclude_patterns: vec![],
            sync: None,
            hostdo: None,
        }
    }

    #[test]
    fn seed_copies_files_and_honors_excludes() {
        let root = unique_temp_dir("seed-basic");
        let canon = root.join("canon");
        let ws_root = root.join("ws");
        fs::create_dir_all(&canon).expect("create canon");
        fs::write(canon.join("file1.txt"), "hello").expect("write file1");
        fs::write(canon.join("secret.key"), "secret").expect("write secret");

        let mut proj = test_project("test-proj", &canon);
        proj.exclude_patterns = vec!["*.key".to_string()];
        let ws = WorkspaceSection {
            root: ws_root.clone(),
        };
        let defaults = DefaultsConfig::default();

        let _report = seed(&proj, &ws, &defaults).expect("seed");
        assert!(_report.files_copied == 1);

        let ws_path = ws_root.join("test-proj");
        assert!(ws_path.join("file1.txt").exists());
        assert!(!ws_path.join("secret.key").exists());
    }

    #[test]
    fn seed_files_only_copies_requested_paths() {
        let root = unique_temp_dir("seed-partial");
        let canon = root.join("canon");
        let ws_root = root.join("ws");
        fs::create_dir_all(&canon).expect("create canon");
        fs::write(canon.join("a.txt"), "a").expect("write a");
        fs::write(canon.join("b.txt"), "b").expect("write b");

        let proj = test_project("test-proj", &canon);
        let ws = WorkspaceSection {
            root: ws_root.clone(),
        };
        let defaults = DefaultsConfig::default();

        let report =
            seed_files(&proj, &ws, &defaults, &[PathBuf::from("a.txt")]).expect("seed partial");
        assert_eq!(report.files_copied, 1);

        let ws_path = ws_root.join("test-proj");
        assert!(ws_path.join("a.txt").exists());
        assert!(!ws_path.join("b.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn seed_rejects_symlinks_by_default() {
        let root = unique_temp_dir("seed-symlink-reject");
        let canon = root.join("canon");
        let ws_root = root.join("ws");
        fs::create_dir_all(&canon).expect("create canon");

        let target = canon.join("target.txt");
        let link = canon.join("link.txt");
        fs::write(&target, "target").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let proj = test_project("test-proj", &canon);
        let ws = WorkspaceSection {
            root: ws_root.clone(),
        };
        let defaults = DefaultsConfig::default();

        let report = seed(&proj, &ws, &defaults).expect("seed");
        // Target is copied, link is rejected by default
        assert!(report.files_copied >= 1);
        assert!(report.files_skipped >= 1);

        let ws_path = ws_root.join("test-proj");
        assert!(ws_path.join("target.txt").exists());
        assert!(!ws_path.join("link.txt").exists());
    }

    #[test]
    fn seed_respects_gitignore() {
        let root = unique_temp_dir("seed-gitignore");
        let canon = root.join("canon");
        let ws_root = root.join("ws");
        fs::create_dir_all(&canon).expect("create canon");
        fs::write(canon.join("file1.txt"), "hello").expect("write file1");
        fs::write(canon.join("ignored.txt"), "ignore me").expect("write ignored");
        fs::write(canon.join(".gitignore"), "ignored.txt").expect("write gitignore");

        let proj = test_project("test-proj", &canon);
        let ws = WorkspaceSection {
            root: ws_root.clone(),
        };
        let defaults = DefaultsConfig::default();

        let _report = seed(&proj, &ws, &defaults).expect("seed");
        let ws_path = ws_root.join("test-proj");
        assert!(ws_path.join("file1.txt").exists());
        assert!(!ws_path.join("ignored.txt").exists());
    }

    #[test]
    fn pushback_preserves_canonical_by_default() {
        let root = unique_temp_dir("pushback-conflict");
        let canon = root.join("canon");
        let ws_root = root.join("ws");
        let ws_path = ws_root.join("test-proj");
        fs::create_dir_all(&canon).expect("create canon");
        fs::create_dir_all(&ws_path).expect("create ws");

        let file_path = "conflict.txt";
        let canon_file = canon.join(file_path);
        let ws_file = ws_path.join(file_path);

        fs::write(&ws_file, "workspace version").expect("write ws");
        // Ensure canon is newer
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&canon_file, "canonical version").expect("write canon");

        let proj = test_project("test-proj", &canon);
        let ws = WorkspaceSection {
            root: ws_root.clone(),
        };
        let mut defaults = DefaultsConfig::default();
        defaults.sync.conflict_policy = ConflictPolicy::PreserveCanonical;

        let report = pushback(&proj, &ws, &defaults).expect("pushback");
        assert_eq!(report.files_copied, 0);
        assert_eq!(
            fs::read_to_string(&canon_file).unwrap(),
            "canonical version"
        );
    }

    #[test]
    fn pushback_overwrites_when_preserve_workspace() {
        let root = unique_temp_dir("pushback-overwrite");
        let canon = root.join("canon");
        let ws_root = root.join("ws");
        let ws_path = ws_root.join("test-proj");
        fs::create_dir_all(&canon).expect("create canon");
        fs::create_dir_all(&ws_path).expect("create ws");

        let file_path = "conflict.txt";
        let canon_file = canon.join(file_path);
        let ws_file = ws_path.join(file_path);

        fs::write(&ws_file, "workspace version").expect("write ws");
        fs::write(&canon_file, "canonical version").expect("write canon");

        let proj = test_project("test-proj", &canon);
        let ws = WorkspaceSection {
            root: ws_root.clone(),
        };
        let mut defaults = DefaultsConfig::default();
        defaults.sync.conflict_policy = ConflictPolicy::PreserveWorkspace;

        let report = pushback(&proj, &ws, &defaults).expect("pushback");
        assert_eq!(report.files_copied, 1);
        assert_eq!(
            fs::read_to_string(&canon_file).unwrap(),
            "workspace version"
        );
    }
}

```

## src/sync/mod.rs

```rs
mod core;
mod helpers;

pub use core::*;
pub(crate) use helpers::build_project_exclude_matcher;

```

## src/telemetry.rs

```rs
/// OpenTelemetry + tracing-subscriber initialisation.
///
/// Call [`init`] once at startup.  The returned [`TelemetryHandle`] must be
/// kept alive until shutdown; call [`TelemetryHandle::shutdown`] after the
/// main event loop exits to flush any in-flight spans.
use anyhow::Result;
use opentelemetry::{KeyValue, trace::TracerProvider as _};
use opentelemetry_sdk::{Resource, runtime, trace::TracerProvider};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{Config, OtlpProtocol};

// ── Public handle ─────────────────────────────────────────────────────────────

pub struct TelemetryHandle {
    provider: Option<TracerProvider>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl TelemetryHandle {
    /// Flush buffered spans and shut down the exporter.
    pub fn shutdown(self) -> Result<()> {
        if let Some(provider) = self.provider {
            provider.shutdown()?;
        }
        Ok(())
    }
}

// ── Init ─────────────────────────────────────────────────────────────────────

/// Initialise the global tracing subscriber (and optionally OTel export).
///
/// Must be called before any `tracing::*` macros are used.
pub fn init(config: &Config) -> Result<TelemetryHandle> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let (log_writer, log_guard) = build_log_writer(config)?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(log_writer)
        .with_ansi(false);
    let instance_id = config.logging.instance_id.as_deref().unwrap_or("unknown");

    if let Some(otlp_cfg) = &config.logging.otlp {
        let exporter = build_exporter(otlp_cfg)?;

        let hostname = machine_hostname();
        let resource = Resource::new(vec![
            KeyValue::new("service.name", "void-claw"),
            KeyValue::new("service.instance.id", instance_id.to_string()),
            KeyValue::new("host.name", hostname),
        ]);

        let provider = TracerProvider::builder()
            .with_batch_exporter(exporter, runtime::Tokio)
            .with_resource(resource)
            .build();

        // Get the tracer *before* boxing the provider, to satisfy the
        // `PreSampledTracer` bound required by `tracing_opentelemetry::layer`.
        let tracer = provider.tracer("void-claw");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        opentelemetry::global::set_tracer_provider(provider.clone());

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();

        info!(
            log_dir = %config.logging.log_dir.display(),
            instance_id = %instance_id,
            otlp_enabled = true,
            endpoint = %otlp_cfg.endpoint,
            protocol = ?otlp_cfg.protocol,
            level = ?otlp_cfg.level,
            "initialized tracing"
        );

        Ok(TelemetryHandle {
            provider: Some(provider),
            _log_guard: log_guard,
        })
    } else {
        info!("OpenTelemetry export disabled");
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        info!(
            log_dir = %config.logging.log_dir.display(),
            instance_id = %instance_id,
            otlp_enabled = false,
            "initialized tracing"
        );

        Ok(TelemetryHandle {
            provider: None,
            _log_guard: log_guard,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_exporter(otlp: &crate::config::OtlpConfig) -> Result<opentelemetry_otlp::SpanExporter> {
    use opentelemetry_otlp::{SpanExporter, WithExportConfig};
    match otlp.protocol {
        OtlpProtocol::Grpc => Ok(SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&otlp.endpoint)
            .build()?),
        OtlpProtocol::Http => Ok(SpanExporter::builder()
            .with_http()
            .with_endpoint(&otlp.endpoint)
            .build()?),
    }
}

/// Best-effort hostname for the OTel `host.name` resource attribute.
/// Reads `$HOSTNAME` first (always set inside containers), then
/// `$COMPUTERNAME` (Windows), falls back to `/etc/hostname` (Linux/macOS),
/// then "unknown".
pub fn machine_hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty()))
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn build_log_writer(
    config: &Config,
) -> Result<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let log_dir = &config.logging.log_dir;
    std::fs::create_dir_all(log_dir)?;
    let appender = tracing_appender::rolling::daily(log_dir, "void-claw.log");
    Ok(tracing_appender::non_blocking(appender))
}

```

## src/tui/app.rs

```rs
use super::*;

mod approvals;
mod build;
mod core;
mod helpers;
mod input;
mod launch;
mod runtime;
mod settings;

#[allow(unused_imports)]
pub(crate) use helpers::{
    compute_tree_file_map, diff_file_maps, docker_image_exists, encode_sgr_mouse,
    host_bind_is_loopback, is_scroll_mode_toggle_key, maybe_encode_sgr_mouse_for_session,
    next_sync_mode, oneshot_dummy, prev_sync_mode, run_build_shell_command,
    shell_command_for_docker_args,
};

```

## src/tui/app/approvals.rs

```rs
use super::*;

impl App {
    pub(crate) fn approve_exec(&mut self, idx: usize, remember: bool) {
        if idx >= self.pending_exec.len() {
            return;
        }
        if remember {
            let item = &self.pending_exec[idx];
            let argv = item.argv.clone();
            let project_name = item.project.clone();
            let cwd = self.portable_cwd(&item.rule_cwd, &project_name);
            if let Some(rules_path) = self.project_rules_path(&project_name) {
                match crate::rules::append_auto_approval(&rules_path, &argv, &cwd) {
                    Ok(()) => {
                        self.push_log(
                            format!("Saved rule to {}: {}", rules_path.display(), argv.join(" ")),
                            false,
                        );
                        self.sync_rules_to_workspace(&project_name);
                    }
                    Err(e) => self.push_log(format!("Failed to save rule: {e}"), true),
                }
            } else {
                self.push_log(
                    format!("Cannot remember: unknown project '{project_name}'"),
                    true,
                );
            }
        }
        if let Some(tx) = self.pending_exec[idx].response_tx.take() {
            let _ = tx.send(ApprovalDecision::Approve { remember });
        }
        self.pending_exec.remove(idx);
    }

    pub(crate) fn deny_exec(&mut self, idx: usize) {
        if idx >= self.pending_exec.len() {
            return;
        }
        if let Some(tx) = self.pending_exec[idx].response_tx.take() {
            let _ = tx.send(ApprovalDecision::Deny);
        }
        self.pending_exec.remove(idx);
    }

    pub(crate) fn deny_exec_forever(&mut self, idx: usize) {
        if idx >= self.pending_exec.len() {
            return;
        }
        let item = &self.pending_exec[idx];
        let argv = item.argv.clone();
        let project_name = item.project.clone();
        let cwd = self.portable_cwd(&item.rule_cwd, &project_name);
        if let Some(rules_path) = self.project_rules_path(&project_name) {
            match crate::rules::append_deny_rule(&rules_path, &argv, &cwd) {
                Ok(()) => {
                    self.push_log(
                        format!(
                            "Saved deny rule to {}: {}",
                            rules_path.display(),
                            argv.join(" ")
                        ),
                        false,
                    );
                    self.sync_rules_to_workspace(&project_name);
                }
                Err(e) => self.push_log(format!("Failed to save deny rule: {e}"), true),
            }
        } else {
            self.push_log(
                format!("Cannot persist deny: unknown project '{project_name}'"),
                true,
            );
        }
        self.deny_exec(idx);
    }

    pub(crate) fn approve_net(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let tx = std::mem::replace(&mut self.pending_net[idx].response_tx, oneshot_dummy());
        let _ = tx.send(NetworkDecision::Allow);
        self.pending_net.remove(idx);
    }

    pub(crate) fn deny_net(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let tx = std::mem::replace(&mut self.pending_net[idx].response_tx, oneshot_dummy());
        let _ = tx.send(NetworkDecision::Deny);
        self.pending_net.remove(idx);
    }

    pub(crate) fn approve_net_forever(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        let project_name = self.pending_net[idx].source_project.clone();
        if project_name.is_none() {
            self.log_missing_network_project_context(idx, "allow");
        }
        match self.persist_network_rule(&host, NetworkPolicy::Auto, project_name.as_deref()) {
            Ok(updated_path) => {
                if let Some(path) = &updated_path {
                    self.push_log(
                        format!(
                            "added permanent allow rule for '{}' in {}",
                            host,
                            path.display()
                        ),
                        false,
                    );
                    if let Some(name) = &project_name {
                        self.sync_rules_to_workspace(name);
                    }
                } else {
                    self.push_log(
                        format!("network host '{}' already permanently allowed", host),
                        false,
                    );
                }
            }
            Err(e) => {
                self.push_log(
                    format!(
                        "failed to persist permanent allow rule for '{}': {}",
                        host, e
                    ),
                    true,
                );
            }
        }
        self.approve_net(idx);
    }

    pub(crate) fn deny_net_forever(&mut self, idx: usize) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        let project_name = self.resolve_pending_network_project(idx);
        match self.persist_network_rule(&host, NetworkPolicy::Deny, project_name.as_deref()) {
            Ok(updated_path) => {
                if let Some(path) = &updated_path {
                    self.push_log(
                        format!(
                            "added permanent deny rule for '{}' in {}",
                            host,
                            path.display()
                        ),
                        false,
                    );
                    if let Some(name) = &project_name {
                        self.sync_rules_to_workspace(name);
                    }
                } else {
                    self.push_log(
                        format!("network host '{}' already permanently denied", host),
                        false,
                    );
                }
            }
            Err(e) => {
                self.push_log(
                    format!(
                        "failed to persist permanent deny rule for '{}': {}",
                        host, e
                    ),
                    true,
                );
            }
        }
        self.deny_net(idx);
    }

    pub(crate) fn resolve_pending_network_project(&self, idx: usize) -> Option<String> {
        let item = self.pending_net.get(idx)?;
        if let Some(project) = item.source_project.clone() {
            return Some(project);
        }
        if let Some(container_name) = item.source_container.as_deref() {
            let mut projects = self
                .sessions
                .iter()
                .filter(|s| !s.is_exited() && s.container_name == container_name)
                .map(|s| s.project.clone())
                .collect::<Vec<_>>();
            projects.sort();
            projects.dedup();
            if projects.len() == 1 {
                return projects.into_iter().next();
            }
        }
        let cfg = self.config.get();
        self.selected_project_idx()
            .and_then(|pi| cfg.projects.get(pi))
            .map(|p| p.name.clone())
    }

    pub(crate) fn persist_network_rule(
        &self,
        host: &str,
        policy: NetworkPolicy,
        project_name: Option<&str>,
    ) -> Result<Option<std::path::PathBuf>> {
        let rules_path = match project_name {
            Some(name) => match self.project_rules_path(name) {
                Some(path) => path,
                None => anyhow::bail!("cannot persist network rule: project '{}' not found", name),
            },
            None => anyhow::bail!(
                "cannot persist network rule: unknown project (request lacked project attribution)"
            ),
        };

        let is_new = !rules_path.exists();
        let mut rules = crate::rules::load(&rules_path)
            .with_context(|| format!("loading rules file '{}'", rules_path.display()))?;

        let exists = rules.network.rules.iter().any(|r| {
            r.host.eq_ignore_ascii_case(host)
                && r.policy == policy
                && r.path_prefix == "/"
                && r.methods.len() == 1
                && r.methods[0] == "*"
        });
        if exists {
            return Ok(None);
        }

        rules.network.rules.push(NetworkRule {
            methods: vec!["*".to_string()],
            host: host.to_string(),
            path_prefix: "/".to_string(),
            policy,
        });

        crate::rules::write_rules_file(&rules_path, &rules, is_new)
            .with_context(|| format!("writing rules file '{}'", rules_path.display()))?;
        Ok(Some(rules_path))
    }

    pub(crate) fn log_missing_network_project_context(&mut self, idx: usize, action: &str) {
        if idx >= self.pending_net.len() {
            return;
        }
        let host = self.pending_net[idx].host.clone();
        self.push_log(
            format!("cannot persist permanent {action} rule for '{}' because the network request had no source project metadata", host),
            true,
        );
    }

    pub(crate) fn portable_cwd(&self, cwd: &Path, project_name: &str) -> String {
        let cfg = self.config.get();
        let mount_target = cfg
            .projects
            .iter()
            .find(|p| p.name == project_name)
            .and_then(|_| Some("/workspace"))
            .unwrap_or("/workspace");
        let cwd_str = cwd.display().to_string();
        if cwd_str == mount_target {
            "$WORKSPACE".to_string()
        } else if let Some(rest) = cwd_str.strip_prefix(&format!("{}/", mount_target)) {
            format!("$WORKSPACE/{rest}")
        } else {
            cwd_str
        }
    }

    pub(crate) fn project_rules_path(&self, project_name: &str) -> Option<std::path::PathBuf> {
        let cfg = self.config.get();
        cfg.projects
            .iter()
            .find(|p| p.name == project_name)
            .map(|p| p.canonical_path.join("void-rules.toml"))
    }

    pub(crate) fn sync_rules_to_workspace(&mut self, project_name: &str) {
        let cfg = self.config.get();
        if let Some(pi) = cfg.projects.iter().position(|p| p.name == project_name) {
            self.do_seed_project(pi);
        }
    }
}

```

## src/tui/app/build.rs

```rs
use super::*;

impl App {
    pub(crate) fn start_docker_build(
        &mut self,
        label: &str,
        shell_command: String,
        launch_project_idx: usize,
        launch_container_idx: usize,
    ) {
        if self.build_task.is_some() {
            self.push_log("a docker build is already running", true);
            return;
        }

        self.build_output.clear();
        self.build_scroll = 0;
        if self.build_project_idx.is_none() {
            self.build_project_idx = self.selected_project_idx();
        }
        self.active_session = None;
        self.focus = Focus::ImageBuild;
        self.push_log(format!("starting {label} in shell"), false);
        self.push_log(format!("$ {shell_command}"), false);

        if let Some(pi) = self.build_project_idx {
            let items = self.sidebar_items();
            if let Some(pos) = items
                .iter()
                .position(|item| *item == SidebarItem::Build(pi))
            {
                self.sidebar_idx = pos;
            }
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.build_task = Some(BuildTaskState {
            label: label.to_string(),
            shell_command: shell_command.clone(),
            cancel_flag: cancel_flag.clone(),
        });

        let tx = self.build_event_tx.clone();
        let label = label.to_string();
        tokio::spawn(async move {
            run_build_shell_command(
                label,
                shell_command,
                launch_project_idx,
                launch_container_idx,
                cancel_flag,
                tx,
            )
            .await;
        });
    }

    pub(crate) fn cancel_build(&mut self) {
        let Some(task) = self.build_task.as_ref() else {
            return;
        };
        task.cancel_flag.store(true, Ordering::SeqCst);
        self.push_log(format!("cancelling {}...", task.label), true);
    }

    pub(crate) fn push_build_output(&mut self, line: impl Into<String>, is_error: bool) {
        self.build_output.push_back((line.into(), is_error));
        if self.build_output.len() > 400 {
            self.build_output.pop_front();
        }
        if self.build_scroll > 0 {
            self.build_scroll = self.build_scroll.saturating_add(1);
        }
    }

    pub fn build_commands_for(
        docker_dir: &Path,
        image: &str,
    ) -> (Vec<String>, Option<Vec<String>>) {
        let parts: Vec<&str> = image.splitn(2, ':').collect();
        let name = parts[0].split('/').last().unwrap_or(parts[0]);
        let tag = parts.get(1).copied().unwrap_or("ubuntu-24.04");
        let dockerfile_root = docker_dir;
        let base_dockerfile = dockerfile_root.join(format!("{tag}.Dockerfile"));
        let mut base_cmd = vec![
            "build".to_string(),
            "-t".to_string(),
            image.to_string(),
            "-f".to_string(),
            base_dockerfile.display().to_string(),
            docker_dir.display().to_string(),
        ];

        let agent_cmd = name.strip_prefix("void-claw-").map(|agent| {
            base_cmd[2] = format!("my-agent:{tag}");
            vec![
                "build".to_string(),
                "-t".to_string(),
                image.to_string(),
                "-f".to_string(),
                dockerfile_root
                    .join(agent)
                    .join(format!("{tag}.Dockerfile"))
                    .display()
                    .to_string(),
                docker_dir.display().to_string(),
            ]
        });

        (base_cmd, agent_cmd)
    }

    pub(crate) fn do_seed_project(&mut self, pi: usize) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        if crate::config::effective_sync_mode(&proj_cfg, &cfg.defaults) == SyncMode::Direct {
            return;
        }
        match crate::sync::seed(&proj_cfg, &cfg.workspace, &cfg.defaults) {
            Ok(report) => {
                let mut msg = format!(
                    "seed '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("seed failed: {e}"), true),
        }
    }

    pub(crate) fn do_pushback_project(&mut self, pi: usize) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        if crate::config::effective_sync_mode(&proj_cfg, &cfg.defaults) == SyncMode::Direct {
            self.push_log(
                format!(
                    "pushback '{}': disabled for projects.sync.mode='direct'",
                    proj_cfg.name
                ),
                false,
            );
            return;
        }
        match crate::sync::pushback(&proj_cfg, &cfg.workspace, &cfg.defaults) {
            Ok(report) => {
                let mut msg = format!(
                    "pushback '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("pushback failed: {e}"), true),
        }
    }

    pub(crate) fn do_pushback_files(&mut self, pi: usize, changed: &[PathBuf]) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        match crate::sync::pushback_files(&proj_cfg, &cfg.workspace, &cfg.defaults, changed) {
            Ok(report) => {
                let mut msg = format!(
                    "pushback '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("pushback failed: {e}"), true),
        }
    }

    pub(crate) fn do_seed_files(&mut self, pi: usize, changed: &[PathBuf]) {
        let cfg = self.config.get();
        let proj_cfg = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };
        match crate::sync::seed_files(&proj_cfg, &cfg.workspace, &cfg.defaults, changed) {
            Ok(report) => {
                let mut msg = format!(
                    "seed '{}': {} copied, {} skipped, {} errors",
                    report.project,
                    report.files_copied,
                    report.files_skipped,
                    report.errors.len()
                );
                if !report.warnings.is_empty() {
                    msg.push_str(&format!(" ({} warnings)", report.warnings.len()));
                }
                let is_err = !report.errors.is_empty();
                for e in &report.errors {
                    self.push_log(format!("  {}: {}", e.path.display(), e.message), true);
                }
                self.push_log(msg, is_err);
                if let Some(proj) = self.projects.get_mut(pi) {
                    proj.last_report = Some(report);
                }
            }
            Err(e) => self.push_log(format!("seed failed: {e}"), true),
        }
    }

    pub(crate) fn start_project_watch(&mut self, pi: usize) {
        if self.is_project_watching(pi) {
            return;
        }
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        if crate::config::effective_sync_mode(proj, &cfg.defaults) == SyncMode::Direct {
            self.push_log(
                format!(
                    "watch start '{}': disabled for projects.sync.mode='direct'",
                    proj.name
                ),
                false,
            );
            return;
        }
        self.do_seed_project(pi);
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        let workspace = crate::config::effective_workspace_path(proj, &cfg.workspace);
        let exclude_matcher = match crate::sync::build_project_exclude_matcher(proj, &cfg.defaults)
        {
            Ok(matcher) => matcher,
            Err(e) => {
                self.push_log(format!("watch start failed for '{}': {e}", proj.name), true);
                return;
            }
        };
        let canonical_files = compute_tree_file_map(&proj.canonical_path, &exclude_matcher);
        let workspace_files = compute_tree_file_map(&workspace, &exclude_matcher);
        self.project_watch.insert(
            pi,
            ProjectWatchState {
                enabled: true,
                spinner_phase: 0,
                canonical_files,
                workspace_files,
            },
        );
        self.push_log(format!("watch enabled for '{}'", proj.name), false);
    }

    pub(crate) fn stop_project_watch(&mut self, pi: usize) {
        let Some(state) = self.project_watch.get_mut(&pi) else {
            return;
        };
        if !state.enabled {
            return;
        }
        state.enabled = false;
        let cfg = self.config.get();
        if let Some(proj) = cfg.projects.get(pi) {
            self.push_log(format!("watch stopped for '{}'", proj.name), false);
        }
    }

    pub(crate) fn do_launch_container(&mut self, ctr_idx: usize) {
        let pi = match self.selected_project_idx() {
            Some(i) => i,
            None => {
                self.push_log("no project selected", true);
                return;
            }
        };
        self.do_launch_container_on_project(pi, ctr_idx);
    }

    pub(crate) fn open_image_build_prompt(&mut self, pi: usize, ctr_idx: usize, image: &str) {
        self.build_project_idx = Some(pi);
        self.build_container_idx = Some(ctr_idx);
        self.build_cursor = 0;
        self.build_output.clear();
        self.build_scroll = 0;
        self.active_session = None;
        self.active_settings_project = None;
        self.container_picker = None;
        self.focus = Focus::ImageBuild;
        self.push_log(
            format!("docker image '{image}' not found locally; build required"),
            true,
        );
    }

    pub(crate) fn preflight_image_or_prompt_build<F>(
        &mut self,
        pi: usize,
        ctr_idx: usize,
        image: &str,
        image_exists: F,
    ) -> bool
    where
        F: FnOnce(&str) -> std::io::Result<bool>,
    {
        match image_exists(image) {
            Ok(true) => true,
            Ok(false) => {
                self.open_image_build_prompt(pi, ctr_idx, image);
                false
            }
            Err(e) => {
                // If we can't check, preserve legacy behavior: attempt to run and
                // surface the real docker error in the session/logs.
                self.push_log(
                    format!("warning: failed to check docker image '{image}': {e}"),
                    true,
                );
                true
            }
        }
    }
}

```

## src/tui/app/core.rs

```rs
use super::*;

impl App {
    pub(crate) fn sidebar_item_is_selectable(item: &SidebarItem) -> bool {
        !matches!(item, SidebarItem::Project(_))
    }

    pub(crate) fn first_selectable_sidebar_idx(items: &[SidebarItem]) -> usize {
        items
            .iter()
            .position(Self::sidebar_item_is_selectable)
            .unwrap_or(0)
    }

    pub fn new(
        config: SharedConfig,
        loaded_config_path: PathBuf,
        token: String,
        session_registry: SessionRegistry,
        exec_pending_rx: mpsc::Receiver<PendingItem>,
        stop_pending_rx: mpsc::Receiver<ContainerStopItem>,
        net_pending_rx: mpsc::Receiver<PendingNetworkItem>,
        audit_rx: mpsc::Receiver<AuditEntry>,
        state: StateManager,
        proxy_state: ProxyState,
        _proxy_addr: String,
        ca_cert_path: String,
    ) -> Result<Self> {
        let cfg = config.get();

        let projects = cfg
            .projects
            .iter()
            .map(|p| ProjectStatus {
                name: p.name.clone(),
                last_report: None,
            })
            .collect();

        let mut log = state
            .recent_audit(200)
            .unwrap_or_default()
            .into_iter()
            .map(LogEntry::Audit)
            .collect::<VecDeque<_>>();

        log.push_front(LogEntry::Msg {
            text: format!("loaded config from {}", loaded_config_path.display()),
            is_error: false,
            timestamp: chrono::Utc::now(),
        });

        let (build_event_tx, build_event_rx) = mpsc::unbounded_channel();

        let rules_path = &cfg.manager.global_rules_file;
        let (hostdo_rule_count, network_rule_count) = crate::rules::load(rules_path)
            .map(|r| (r.hostdo.commands.len(), r.network.rules.len()))
            .unwrap_or((0, 0));
        log.push_front(LogEntry::Msg {
            text: format!(
                "Loaded rules from {} (hostdo: {}, network: {})",
                rules_path.display(),
                hostdo_rule_count,
                network_rule_count
            ),
            is_error: false,
            timestamp: chrono::Utc::now(),
        });

        Ok(Self {
            config,
            loaded_config_path,
            token,
            session_registry,
            ca_cert_path,
            proxy_state,
            projects,
            pending_exec: vec![],
            pending_stop: vec![],
            pending_net: vec![],
            log,
            log_scroll: 0,
            focus: Focus::Sidebar,
            sidebar_idx: Self::first_selectable_sidebar_idx(
                &cfg.projects
                    .iter()
                    .enumerate()
                    .flat_map(|(pi, _)| {
                        [
                            SidebarItem::Project(pi),
                            SidebarItem::Launch(pi),
                            SidebarItem::Settings(pi),
                        ]
                    })
                    .chain(std::iter::once(SidebarItem::NewProject))
                    .collect::<Vec<_>>(),
            ),
            sidebar_offset: 0,
            active_session: None,
            preview_session: None,
            active_settings_project: None,
            settings_cursor: 0,
            container_picker: None,
            build_container_idx: None,
            build_project_idx: None,
            build_cursor: 0,
            build_output: VecDeque::new(),
            build_scroll: 0,
            sessions: vec![],
            new_project: None,
            exec_pending_rx,
            stop_pending_rx,
            net_pending_rx,
            audit_rx,
            build_event_rx,
            build_event_tx,
            build_task: None,
            should_quit: false,
            log_fullscreen: false,
            terminal_fullscreen: false,
            ctrl_c_times: Vec::new(),
            last_terminal_esc: None,
            scroll_mode: false,
            terminal_scroll: 0,
            project_watch: HashMap::new(),
            last_watch_tick: std::time::Instant::now(),
        })
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let cfg = self.config.get();
        let mut items = Vec::new();
        for (pi, proj) in cfg.projects.iter().enumerate() {
            items.push(SidebarItem::Project(pi));
            for (si, session) in self.sessions.iter().enumerate() {
                if session.project == proj.name {
                    items.push(SidebarItem::Session(si));
                }
            }
            if self.build_project_idx == Some(pi) && self.build_is_running() {
                items.push(SidebarItem::Build(pi));
            }
            items.push(SidebarItem::Launch(pi));
            items.push(SidebarItem::Settings(pi));
        }
        items.push(SidebarItem::NewProject);
        items
    }

    pub fn selected_project_idx(&self) -> Option<usize> {
        match self.sidebar_items().get(self.sidebar_idx) {
            Some(SidebarItem::Project(pi)) => Some(*pi),
            Some(SidebarItem::Session(si)) => {
                let cfg = self.config.get();
                let name = self.sessions.get(*si)?.project.as_str();
                cfg.projects.iter().position(|p| p.name == name)
            }
            Some(SidebarItem::Settings(pi)) => Some(*pi),
            Some(SidebarItem::Launch(pi)) => Some(*pi),
            Some(SidebarItem::Build(pi)) => Some(*pi),
            Some(SidebarItem::NewProject) => None,
            None => None,
        }
    }

    pub fn is_project_watching(&self, project_idx: usize) -> bool {
        self.project_watch
            .get(&project_idx)
            .map(|s| s.enabled)
            .unwrap_or(false)
    }

    pub fn project_watch_spinner(&self, project_idx: usize) -> Option<&'static str> {
        if !self.is_project_watching(project_idx) {
            return None;
        }
        const FRAMES: [&str; 2] = ["●", "○"];
        let phase = self
            .project_watch
            .get(&project_idx)
            .map(|s| s.spinner_phase)
            .unwrap_or(0);
        Some(FRAMES[phase % FRAMES.len()])
    }

    pub fn pending_for_session(&self, session_idx: usize) -> Vec<usize> {
        let project = match self.sessions.get(session_idx) {
            Some(s) => s.project.as_str(),
            None => return vec![],
        };
        self.pending_exec
            .iter()
            .enumerate()
            .filter(|(_, item)| item.project == project)
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn active_exec_modal_idx(&self) -> Option<usize> {
        let si = self.active_session?;
        self.pending_for_session(si).into_iter().next()
    }

    pub(crate) fn session_is_loading(&self, session_idx: usize) -> bool {
        let Some(session) = self.sessions.get(session_idx) else {
            return false;
        };
        if session.is_exited() {
            return false;
        }
        let term = session.term.lock();
        let mut content = term.renderable_content();
        !content
            .display_iter
            .any(|indexed| !indexed.cell.c.is_whitespace())
    }

    pub(crate) fn close_session(&mut self, idx: usize) {
        if idx >= self.sessions.len() {
            return;
        }
        if let Some(tok) = self.sessions.get(idx).map(|s| s.session_token.clone()) {
            self.session_registry.remove(&tok);
        }
        if let Some(session) = self.sessions.get(idx) {
            if !session.is_exited() {
                session.terminate();
            }
        }
        self.sessions.remove(idx);
        self.remap_session_indices_after_removal(idx);
        let items = self.sidebar_items();
        if self.sidebar_idx >= items.len() {
            self.sidebar_idx = items.len().saturating_sub(1);
        }
    }

    pub(crate) fn clear_terminal_fullscreen_for_removed_session(&mut self, removed_idx: usize) {
        if self.active_session == Some(removed_idx) {
            self.terminal_fullscreen = false;
            self.last_terminal_esc = None;
        }
    }

    pub(crate) fn remap_session_indices_after_removal(&mut self, removed_idx: usize) {
        self.clear_terminal_fullscreen_for_removed_session(removed_idx);
        match self.active_session {
            Some(si) if si == removed_idx => {
                self.active_session = None;
                self.focus = Focus::Sidebar;
            }
            Some(si) if si > removed_idx => {
                self.active_session = Some(si - 1);
            }
            _ => {}
        }
        match self.preview_session {
            Some(si) if si == removed_idx => {
                self.preview_session = None;
            }
            Some(si) if si > removed_idx => {
                self.preview_session = Some(si - 1);
            }
            _ => {}
        }
    }

    pub(crate) fn terminate_all_sessions(&mut self) {
        for session in &self.sessions {
            if !session.is_exited() {
                session.terminate();
            }
        }
    }

    pub(crate) fn handle_stop_request(
        &mut self,
        project: &str,
        container_id: &str,
    ) -> ContainerStopDecision {
        let normalized = container_id.trim();
        let Some(idx) = self.sessions.iter().position(|session| {
            session.project == project
                && (session.container_id == normalized
                    || session.container_id.starts_with(normalized)
                    || normalized.starts_with(&session.container_id))
        }) else {
            self.push_log(
                format!(
                    "killme request for project '{}' could not find container {}",
                    project, normalized
                ),
                true,
            );
            return ContainerStopDecision::NotFound;
        };

        let label = self.sessions[idx].tab_label();
        if self.sessions[idx].is_exited() {
            self.push_log(
                format!(
                    "killme request for '{}' ignored; container already exited",
                    label
                ),
                false,
            );
            return ContainerStopDecision::Stopped;
        }

        self.push_log(format!("killme requested for '{}'", label), false);
        self.sessions[idx].terminate();
        self.sessions[idx]
            .exited
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if self.active_session == Some(idx) {
            self.active_session = None;
            self.focus = Focus::Sidebar;
        }
        ContainerStopDecision::Stopped
    }

    pub(crate) fn push_log(&mut self, text: impl Into<String>, is_error: bool) {
        self.log.push_front(LogEntry::Msg {
            text: text.into(),
            is_error,
            timestamp: chrono::Utc::now(),
        });
        if self.log.len() > 500 {
            self.log.pop_back();
        }
    }

    pub(crate) fn log_project_rules_status(&mut self, project: &crate::config::ProjectConfig) {
        let rules_path = project.canonical_path.join("void-rules.toml");
        if !rules_path.exists() {
            self.push_log(
                format!(
                    "Searched for rules at {} but void-rules.toml was not found",
                    rules_path.display()
                ),
                false,
            );
            return;
        }

        match crate::rules::load(&rules_path) {
            Ok(r) => self.push_log(
                format!(
                    "Loaded rules from {} (hostdo: {}, network: {})",
                    rules_path.display(),
                    r.hostdo.commands.len(),
                    r.network.rules.len()
                ),
                false,
            ),
            Err(e) => self.push_log(
                format!("Failed loading rules from {}: {}", rules_path.display(), e),
                true,
            ),
        }
    }
}

```

## src/tui/app/helpers.rs

```rs
use super::*;

pub(crate) fn maybe_encode_sgr_mouse_for_session(
    session: &crate::container::ContainerSession,
    mouse: MouseEvent,
) -> Option<Vec<u8>> {
    // Only forward mouse events when the terminal app has explicitly enabled mouse reporting.
    // Without this gating, shells and other apps would see raw escape sequences.
    let mode = *session.term.lock().mode();
    if !mode
        .intersects(TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION)
    {
        return None;
    }

    // Only emit SGR mouse sequences for now; this matches most modern TUIs (including OpenCode).
    if !mode.contains(TermMode::SGR_MOUSE) {
        return None;
    }

    encode_sgr_mouse(mouse)
}

pub(crate) fn encode_sgr_mouse(mouse: MouseEvent) -> Option<Vec<u8>> {
    let mut cb: u16 = 0;
    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
        cb |= 4;
    }
    if mouse.modifiers.contains(KeyModifiers::ALT) {
        cb |= 8;
    }
    if mouse.modifiers.contains(KeyModifiers::CONTROL) {
        cb |= 16;
    }

    let (button_code, suffix): (u16, u8) = match mouse.kind {
        MouseEventKind::Down(button) => (button_to_code(button)?, b'M'),
        MouseEventKind::Up(button) => (button_to_code(button)?, b'm'),
        MouseEventKind::Drag(button) => (button_to_code(button)? + 32, b'M'),
        MouseEventKind::ScrollUp => (64, b'M'),
        MouseEventKind::ScrollDown => (65, b'M'),
        MouseEventKind::ScrollLeft => (66, b'M'),
        MouseEventKind::ScrollRight => (67, b'M'),
        MouseEventKind::Moved => return None,
    };

    let cb = cb + button_code;
    let x = mouse.column.saturating_add(1) as u16;
    let y = mouse.row.saturating_add(1) as u16;

    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(b"\x1b[<");
    out.extend_from_slice(cb.to_string().as_bytes());
    out.push(b';');
    out.extend_from_slice(x.to_string().as_bytes());
    out.push(b';');
    out.extend_from_slice(y.to_string().as_bytes());
    out.push(suffix);
    Some(out)
}

pub(crate) fn button_to_code(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Middle => Some(1),
        MouseButton::Right => Some(2),
    }
}

pub(crate) fn shell_command_for_docker_args(args: &[String]) -> String {
    format!("docker {}", shell_words::join(args))
}

pub(crate) fn build_line_looks_like_error(line: &str) -> bool {
    let text = line.to_ascii_lowercase();
    [
        " error",
        "failed",
        "denied",
        "no such file",
        "not found",
        "permission denied",
        "unauthorized",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

async fn forward_build_stream<R>(
    reader: R,
    prefix: &'static str,
    mark_stderr: bool,
    stderr_tail: Option<Arc<Mutex<VecDeque<String>>>>,
    tx: mpsc::UnboundedSender<BuildEvent>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncBufReadExt;
    let mut lines = tokio::io::BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let is_error = mark_stderr && build_line_looks_like_error(&line);
                if is_error
                    && let Some(tail) = stderr_tail.as_ref()
                    && let Ok(mut lines) = tail.lock()
                {
                    lines.push_back(line.clone());
                    if lines.len() > 6 {
                        lines.pop_front();
                    }
                }
                let _ = tx.send(BuildEvent::Output {
                    line: format!("{prefix}{line}"),
                    is_error,
                });
            }
            Ok(None) | Err(_) => break,
        }
    }
}

pub(crate) async fn run_build_shell_command(
    label: String,
    shell_command: String,
    launch_project_idx: usize,
    launch_container_idx: usize,
    cancel_flag: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<BuildEvent>,
) {
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-lc")
        .arg(&shell_command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            let rc = libc::setpgid(0, 0);
            if rc == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(BuildEvent::Finished {
                label,
                launch_project_idx,
                launch_container_idx,
                success: false,
                cancelled: false,
                exit_code: None,
                error: Some(e.to_string()),
                diagnostic: None,
            });
            return;
        }
    };

    let stderr_tail: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let stdout_task = child.stdout.take().map(|stdout| {
        let tx = tx.clone();
        tokio::spawn(async move {
            forward_build_stream(stdout, "build: ", false, None, tx).await;
        })
    });
    let stderr_task = child.stderr.take().map(|stderr| {
        let tx = tx.clone();
        let stderr_tail = stderr_tail.clone();
        tokio::spawn(async move {
            forward_build_stream(stderr, "build: ", true, Some(stderr_tail), tx).await;
        })
    });

    let mut cancelled = false;
    let status = loop {
        if cancel_flag.load(Ordering::SeqCst) {
            cancelled = true;
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                let pgid = format!("-{}", pid);
                let _ = tokio::process::Command::new("kill")
                    .args(["-TERM", &pgid])
                    .status()
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                let _ = tokio::process::Command::new("kill")
                    .args(["-KILL", &pgid])
                    .status()
                    .await;
            }
            let _ = child.start_kill();
            break child.wait().await.ok();
        }

        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
            Err(_) => break None,
        }
    };

    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }

    let success = !cancelled && status.map(|s| s.success()).unwrap_or(false);
    let exit_code = status.and_then(|s| s.code());
    let diagnostic = stderr_tail.lock().ok().and_then(|lines| {
        (!lines.is_empty()).then(|| lines.iter().cloned().collect::<Vec<_>>().join(" | "))
    });
    let _ = tx.send(BuildEvent::Finished {
        label,
        launch_project_idx,
        launch_container_idx,
        success,
        cancelled,
        exit_code,
        error: None,
        diagnostic,
    });
}

pub(crate) fn compute_tree_file_map(
    root: &std::path::Path,
    exclude_matcher: &crate::sync::ExcludeMatcher,
) -> HashMap<PathBuf, FileSignature> {
    let mut map = HashMap::new();
    if !root.exists() {
        return map;
    }
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let rel = match e.path().strip_prefix(root) {
                Ok(r) => r,
                Err(_) => return true,
            };
            if rel.as_os_str().is_empty() {
                return true;
            }
            !exclude_matcher.is_excluded(rel, e.file_type().is_dir())
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            continue;
        }
        let rel = match entry.path().strip_prefix(root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        if let Ok(md) = entry.metadata() {
            let (mtime_secs, mtime_nanos) = md
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| (d.as_secs(), d.subsec_nanos()))
                .unwrap_or((0, 0));
            map.insert(
                rel.to_path_buf(),
                FileSignature {
                    size: md.len(),
                    mtime_secs,
                    mtime_nanos,
                },
            );
        }
    }
    map
}

pub(crate) fn diff_file_maps(
    old: &HashMap<PathBuf, FileSignature>,
    new: &HashMap<PathBuf, FileSignature>,
) -> Vec<PathBuf> {
    let mut changed = Vec::new();
    for (path, new_sig) in new {
        match old.get(path) {
            Some(old_sig) if old_sig == new_sig => {}
            _ => changed.push(path.clone()),
        }
    }
    for path in old.keys() {
        if !new.contains_key(path) {
            changed.push(path.clone());
        }
    }
    changed
}

pub(crate) fn host_bind_is_loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

pub(crate) fn docker_image_exists(image: &str) -> std::io::Result<bool> {
    let status = std::process::Command::new("docker")
        .args(["image", "inspect", image])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    Ok(status.success())
}

pub(crate) fn is_scroll_mode_toggle_key(key: KeyEvent) -> bool {
    (key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('\u{13}') && key.modifiers.is_empty())
}

pub(crate) fn next_sync_mode(mode: &SyncMode) -> SyncMode {
    match mode {
        SyncMode::WorkspaceOnly => SyncMode::Pullthrough,
        SyncMode::Pullthrough => SyncMode::Pushback,
        SyncMode::Pushback => SyncMode::Bidirectional,
        SyncMode::Bidirectional => SyncMode::Direct,
        SyncMode::Direct => SyncMode::WorkspaceOnly,
    }
}

pub(crate) fn prev_sync_mode(mode: &SyncMode) -> SyncMode {
    match mode {
        SyncMode::WorkspaceOnly => SyncMode::Direct,
        SyncMode::Direct => SyncMode::Bidirectional,
        SyncMode::Bidirectional => SyncMode::Pushback,
        SyncMode::Pushback => SyncMode::Pullthrough,
        SyncMode::Pullthrough => SyncMode::WorkspaceOnly,
    }
}

pub(crate) fn oneshot_dummy() -> tokio::sync::oneshot::Sender<NetworkDecision> {
    let (tx, _) = tokio::sync::oneshot::channel();
    tx
}

// ── Key → PTY bytes (Streamlined mapping) ────────────────────────────────────

```

## src/tui/app/input.rs

```rs
use super::*;

impl App {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.build_is_running() {
                self.cancel_build();
                return;
            }

            if self.focus == Focus::Terminal {
                if let Some(si) = self.active_session {
                    if self.session_is_loading(si) {
                        let label = self.sessions[si].tab_label();
                        self.push_log(format!("Cancelled container startup: {}", label), false);
                        self.close_session(si);
                        return;
                    }
                }
            }

            let running = self.sessions.iter().any(|s| !s.is_exited());
            if !running {
                self.should_quit = true;
                return;
            }

            if self.focus == Focus::Terminal {
                if let Some(si) = self.active_session {
                    if let Some(session) = self.sessions.get(si) {
                        session.send_input(vec![0x03]);
                    }
                }
            }

            let now = std::time::Instant::now();
            let window = std::time::Duration::from_secs(2);
            self.ctrl_c_times
                .retain(|t| now.duration_since(*t) < window);
            self.ctrl_c_times.push(now);
            if self.ctrl_c_times.len() >= 4 {
                self.should_quit = true;
            }
            return;
        }

        if let Some(idx) = self.active_exec_modal_idx() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => self.approve_exec(idx, false),
                KeyCode::Char('r') => self.approve_exec(idx, true),
                KeyCode::Char('n') | KeyCode::Esc => self.deny_exec(idx),
                KeyCode::Char('d') => self.deny_exec_forever(idx),
                _ => {}
            }
            return;
        }
        if !self.pending_net.is_empty() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => self.approve_net(0),
                KeyCode::Char('r') => self.approve_net_forever(0),
                KeyCode::Char('n') | KeyCode::Esc => self.deny_net(0),
                KeyCode::Char('d') => self.deny_net_forever(0),
                _ => {}
            }
            return;
        }

        if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.focus_sidebar_shortcut();
            return;
        }

        if self.log_fullscreen {
            match key.code {
                KeyCode::Char('o') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.log_fullscreen = false;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.log_scroll = self.log_scroll.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.log_scroll = self.log_scroll.saturating_add(1);
                }
                _ => {}
            }
            return;
        }

        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Terminal => self.handle_terminal_key(key),
            Focus::Settings => self.handle_settings_key(key),
            Focus::ContainerPicker => self.handle_picker_key(key),
            Focus::ImageBuild => self.handle_build_key(key),
            Focus::NewProject => self.handle_new_project_key(key),
        }
    }

    pub(crate) fn focus_sidebar_shortcut(&mut self) {
        self.last_terminal_esc = None;
        self.log_fullscreen = false;
        self.terminal_fullscreen = false;
        match self.focus {
            Focus::Sidebar => {}
            Focus::Terminal => {
                self.focus = Focus::Sidebar;
            }
            Focus::Settings => {
                self.active_settings_project = None;
                self.focus = Focus::Sidebar;
            }
            Focus::ContainerPicker => {
                self.container_picker = None;
                self.focus = Focus::Sidebar;
            }
            Focus::ImageBuild => {
                if self.build_is_running() {
                    self.focus = Focus::Sidebar;
                } else {
                    self.build_container_idx = None;
                    self.build_project_idx = None;
                    self.focus = Focus::Sidebar;
                }
            }
            Focus::NewProject => {
                self.new_project = None;
                self.focus = Focus::Sidebar;
            }
        }
        let items = self.sidebar_items();
        self.update_sidebar_preview(&items);
    }

    pub(crate) fn open_log_fullscreen(&mut self) {
        self.terminal_fullscreen = false;
        self.log_fullscreen = true;
    }

    pub(crate) fn open_terminal_fullscreen(&mut self) {
        self.log_fullscreen = false;
        self.terminal_fullscreen = true;
        self.last_terminal_esc = None;
    }

    pub(crate) fn close_terminal_fullscreen(&mut self) {
        self.terminal_fullscreen = false;
        self.last_terminal_esc = None;
    }

    pub(crate) fn handle_sidebar_key(&mut self, key: KeyEvent) {
        let items = self.sidebar_items();
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.sidebar_move_up(&items);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.sidebar_move_down(&items);
            }
            KeyCode::Char('o') => self.open_log_fullscreen(),
            KeyCode::Enter | KeyCode::Char('l') => self.handle_sidebar_enter(&items),
            _ => {}
        }
    }

    pub(crate) fn sidebar_move_up(&mut self, items: &[SidebarItem]) {
        self.sidebar_move_to_next_selectable(items, -1);
        self.update_sidebar_preview(items);
        self.ensure_sidebar_visible(items, 10); // Default height
    }

    pub(crate) fn sidebar_move_down(&mut self, items: &[SidebarItem]) {
        self.sidebar_move_to_next_selectable(items, 1);
        self.update_sidebar_preview(items);
        self.ensure_sidebar_visible(items, 10); // Default height
    }

    pub(crate) fn sidebar_move_to_next_selectable(&mut self, items: &[SidebarItem], dir: i8) {
        if items.is_empty() {
            return;
        }

        let len = items.len();
        let mut idx = self.sidebar_idx.min(len.saturating_sub(1));

        // Move at least one step, then keep stepping until we find a selectable row.
        for _ in 0..len {
            idx = if dir < 0 {
                if idx == 0 { len - 1 } else { idx - 1 }
            } else if idx >= len - 1 {
                0
            } else {
                idx + 1
            };

            if Self::sidebar_item_is_selectable(&items[idx]) {
                self.sidebar_idx = idx;
                return;
            }
        }
        // Degenerate case: everything is non-selectable (shouldn't happen).
        self.sidebar_idx = 0;
    }

    pub(crate) fn ensure_sidebar_visible(&mut self, items: &[SidebarItem], visible_height: usize) {
        if items.is_empty() || visible_height == 0 {
            return;
        }
        if self.sidebar_idx < self.sidebar_offset {
            self.sidebar_offset = self.sidebar_idx;
        } else if self.sidebar_idx >= self.sidebar_offset + visible_height {
            self.sidebar_offset = self.sidebar_idx - visible_height + 1;
        }
    }

    pub(crate) fn update_sidebar_preview(&mut self, items: &[SidebarItem]) {
        self.preview_session = match items.get(self.sidebar_idx) {
            Some(SidebarItem::Session(si)) => Some(*si),
            _ => None,
        };
    }

    pub(crate) fn handle_sidebar_enter(&mut self, items: &[SidebarItem]) {
        match items.get(self.sidebar_idx).cloned() {
            Some(SidebarItem::Project(_)) => {
                // do nothing
            }
            Some(SidebarItem::Settings(pi)) => {
                self.active_settings_project = Some(pi);
                self.settings_cursor = 0;
                self.focus = Focus::Settings;
            }
            Some(SidebarItem::Launch(_)) => self.open_picker(),
            Some(SidebarItem::Build(_)) => {
                self.active_session = None;
                self.focus = Focus::ImageBuild;
                self.active_settings_project = None;
            }
            Some(SidebarItem::Session(si)) => {
                if let Some(session) = self.sessions.get(si) {
                    session.clear_bell();
                }
                self.active_session = Some(si);
                self.preview_session = Some(si);
                self.scroll_mode = false;
                self.terminal_scroll = 0;
                self.focus = Focus::Terminal;
                self.active_settings_project = None;
            }
            Some(SidebarItem::NewProject) => self.open_new_project(),
            None => {}
        }
    }

    const NEW_PROJECT_ROW_COUNT: usize = 6;

    pub(crate) fn open_new_project(&mut self) {
        let cfg = self.config.get();
        self.new_project = Some(NewProjectState {
            cursor: 0,
            name: String::new(),
            canonical_dir: String::new(),
            sync_mode: cfg.defaults.sync.mode.clone(),
            project_type: crate::new_project::ProjectType::None,
            error: None,
        });
        self.focus = Focus::NewProject;
        self.active_session = None;
        self.active_settings_project = None;
        self.container_picker = None;
    }

    pub(crate) fn handle_new_project_key(&mut self, key: KeyEvent) {
        let Some(state) = self.new_project.as_mut() else {
            self.focus = Focus::Sidebar;
            return;
        };

        if matches!(state.cursor, 0 | 1)
            && let KeyCode::Char(c) = key.code
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.append_new_project_text(&c.to_string());
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.new_project = None;
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.cursor = state.cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                state.cursor = (state.cursor + 1).min(Self::NEW_PROJECT_ROW_COUNT - 1);
            }
            KeyCode::Left => match state.cursor {
                2 => state.sync_mode = prev_sync_mode(&state.sync_mode),
                3 => state.project_type = state.project_type.prev(),
                _ => {}
            },
            KeyCode::Right => match state.cursor {
                2 => state.sync_mode = next_sync_mode(&state.sync_mode),
                3 => state.project_type = state.project_type.next(),
                _ => {}
            },
            KeyCode::Backspace => match state.cursor {
                0 => {
                    state.name.pop();
                }
                1 => {
                    state.canonical_dir.pop();
                }
                _ => {}
            },
            KeyCode::Enter => match state.cursor {
                2 => state.sync_mode = next_sync_mode(&state.sync_mode),
                3 => state.project_type = state.project_type.next(),
                4 => self.submit_new_project(),
                5 => {
                    self.new_project = None;
                    self.focus = Focus::Sidebar;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub(crate) fn append_new_project_text(&mut self, text: &str) {
        let Some(state) = self.new_project.as_mut() else {
            return;
        };
        let cleaned = text.replace(['\r', '\n'], "");
        if cleaned.is_empty() {
            return;
        }
        match state.cursor {
            0 => state.name.push_str(&cleaned),
            1 => state.canonical_dir.push_str(&cleaned),
            _ => {}
        }
    }

    pub(crate) fn submit_new_project(&mut self) {
        let Some((name, canonical_raw, sync_mode, project_type)) =
            self.new_project.as_mut().map(|state| {
                state.error = None;
                (
                    state.name.trim().to_string(),
                    state.canonical_dir.trim().to_string(),
                    state.sync_mode.clone(),
                    state.project_type,
                )
            })
        else {
            return;
        };

        if name.is_empty() {
            self.set_new_project_error("project name is required".to_string());
            return;
        }
        if canonical_raw.is_empty() {
            self.set_new_project_error("canonical dir is required".to_string());
            return;
        }

        let canonical_path = match crate::config::expand_path(std::path::Path::new(&canonical_raw))
        {
            Ok(p) => p,
            Err(e) => {
                self.set_new_project_error(format!("canonical dir is invalid: {e}"));
                return;
            }
        };
        if !canonical_path.exists() {
            self.set_new_project_error(format!(
                "canonical dir does not exist: {}",
                canonical_path.display()
            ));
            return;
        }
        if !canonical_path.is_dir() {
            self.set_new_project_error(format!(
                "canonical dir is not a directory: {}",
                canonical_path.display()
            ));
            return;
        }

        let cfg = self.config.get();
        if cfg.projects.iter().any(|p| p.name == name) {
            self.set_new_project_error(format!("project name already exists: '{name}'"));
            return;
        }

        match crate::new_project::write_rules_if_missing(&canonical_path, project_type) {
            Ok(false) => {}
            Ok(true) => self.push_log(
                format!(
                    "created {}",
                    canonical_path.join("void-rules.toml").display()
                ),
                false,
            ),
            Err(e) => {
                self.set_new_project_error(format!("failed writing void-rules.toml: {e}"));
                return;
            }
        };

        if let Err(e) = crate::new_project::append_project_block(
            &self.loaded_config_path,
            &name,
            &canonical_path,
            sync_mode,
        ) {
            self.set_new_project_error(format!("failed updating config: {e}"));
            return;
        }

        let new_config = match crate::config::load(&self.loaded_config_path) {
            Ok(c) => c,
            Err(e) => {
                self.set_new_project_error(format!("config reload failed: {e}"));
                return;
            }
        };
        let new_pi = new_config.projects.iter().position(|p| p.name == name);
        self.config.set(std::sync::Arc::new(new_config));
        self.refresh_projects_cache();

        self.push_log(format!("added project '{name}'"), false);
        self.new_project = None;
        self.focus = Focus::Sidebar;

        if let Some(pi) = new_pi {
            if let Some(pos) = self
                .sidebar_items()
                .iter()
                .position(|item| *item == SidebarItem::Launch(pi))
            {
                self.sidebar_idx = pos;
            }
        }
    }

    pub(crate) fn set_new_project_error(&mut self, msg: String) {
        if let Some(state) = self.new_project.as_mut() {
            state.error = Some(msg);
        }
    }
}

```

## src/tui/app/launch.rs

```rs
use super::*;

impl App {
    pub(crate) fn do_launch_container_on_project(&mut self, pi: usize, ctr_idx: usize) {
        let cfg = self.config.get();
        let exec_host = cfg.defaults.hostdo.server_host.trim();
        if host_bind_is_loopback(exec_host) {
            self.push_log(
                format!("cannot launch container: defaults.hostdo.server_host='{}' is loopback; set it to '0.0.0.0'", exec_host),
                true,
            );
            return;
        }
        let ctr = match cfg.containers.get(ctr_idx) {
            Some(c) => c.clone(),
            None => return,
        };
        let extra_instructions = match ctr.agent {
            crate::config::AgentKind::Claude => cfg
                .agents
                .claude
                .as_ref()
                .and_then(|agent| agent.extra_instructions.as_deref()),
            crate::config::AgentKind::Codex => cfg
                .agents
                .codex
                .as_ref()
                .and_then(|agent| agent.extra_instructions.as_deref()),
            crate::config::AgentKind::Gemini => cfg
                .agents
                .gemini
                .as_ref()
                .and_then(|agent| agent.extra_instructions.as_deref()),
            crate::config::AgentKind::Opencode | crate::config::AgentKind::None => None,
        };

        if ctr.agent == crate::config::AgentKind::Claude {
            let has_claude_json = ctr
                .mounts
                .iter()
                .any(|m| m.container == PathBuf::from("/home/ubuntu/.claude.json"));
            let has_claude_dir = ctr
                .mounts
                .iter()
                .any(|m| m.container == PathBuf::from("/home/ubuntu/.claude"));
            if !has_claude_json || !has_claude_dir {
                self.push_log("hint: Claude containers usually need mounts for '~/.claude.json' and '~/.claude'".to_string(), false);
            }
        }
        if ctr.agent == crate::config::AgentKind::Gemini {
            let has_gemini_home = ctr.mounts.iter().any(|m| {
                m.container == PathBuf::from("/home/ubuntu/.gemini")
                    || m.container == PathBuf::from("/root/.gemini")
            });
            if !has_gemini_home {
                self.push_log(
                    "hint: Gemini containers usually need a mount for '~/.gemini' to persist sign-in/session state".to_string(),
                    false,
                );
            }
        }

        let proj = match cfg.projects.get(pi) {
            Some(p) => p.clone(),
            None => return,
        };

        if !self.preflight_image_or_prompt_build(pi, ctr_idx, &ctr.image, docker_image_exists) {
            return;
        }

        let mount_source_path =
            crate::config::effective_mount_source_path(&proj, &cfg.workspace, &cfg.defaults);
        self.log_project_rules_status(&proj);

        let exec_port = cfg.defaults.hostdo.server_port;
        let exec_host = &cfg.defaults.hostdo.server_host;
        let exec_url = format!("http://{exec_host}:{exec_port}");
        let proxy_host = &cfg.defaults.proxy.proxy_host;
        let scoped_proxy = match crate::proxy::spawn_scoped_listener(
            &self.proxy_state,
            proxy_host,
            &proj.name,
            &ctr.name,
        ) {
            Ok(listener) => listener,
            Err(e) => {
                self.push_log(
                    format!("cannot launch '{}' on '{}': {e}", ctr.name, proj.name),
                    true,
                );
                return;
            }
        };
        let proxy_url = format!("http://{}", scoped_proxy.addr);
        self.push_log(
            format!("launching '{}' on '{}'", ctr.name, proj.name),
            false,
        );

        match crate::agents::inject_agent_config(
            &ctr.agent,
            &mount_source_path,
            &proj.canonical_path,
            &proj.name,
            crate::config::effective_sync_mode(&proj, &cfg.defaults) == SyncMode::Direct,
            &ctr.mount_target,
            &exec_url,
            &proxy_url,
            extra_instructions,
        ) {
            Ok(true) => self.push_log(
                format!(
                    "created starter void-rules.toml in '{}'",
                    proj.canonical_path.display()
                ),
                false,
            ),
            Ok(false) => {}
            Err(e) => self.push_log(format!("agent config injection warning: {e}"), true),
        }

        let (term_cols, term_rows) = crossterm::terminal::size().unwrap_or((120, 40));
        let pty_cols = term_cols.saturating_sub(38).max(20);
        let pty_rows = term_rows.saturating_sub(10).max(6);

        let codex_home_dir = cfg
            .logging
            .log_dir
            .join("codex-home")
            .join(crate::container::sanitize_docker_name(&proj.name));
        let has_host_codex_state_mount = ctr.mounts.iter().any(|m| {
            let p = &m.container;
            if p.file_name().and_then(|s| s.to_str()) == Some(".codex") {
                return true;
            }
            if p.file_name().and_then(|s| s.to_str()) != Some("codex") {
                return false;
            }
            p.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|s| s.to_str())
                == Some(".config")
        });

        let codex_home_host_path: Option<&std::path::Path> = if ctr.agent
            == crate::config::AgentKind::Codex
            && !ctr.env_passthrough.iter().any(|v| v == "CODEX_HOME")
            && !has_host_codex_state_mount
        {
            Some(codex_home_dir.as_path())
        } else {
            None
        };

        let gemini_home_dir = cfg
            .logging
            .log_dir
            .join("gemini-home")
            .join(crate::container::sanitize_docker_name(&proj.name));
        let has_host_gemini_state_mount = ctr.mounts.iter().any(|m| {
            let p = &m.container;
            if p.file_name().and_then(|s| s.to_str()) == Some(".gemini") {
                return true;
            }
            if p.file_name().and_then(|s| s.to_str()) != Some("gemini") {
                return false;
            }
            p.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|s| s.to_str())
                == Some(".config")
        });

        let gemini_home_host_path: Option<&std::path::Path> =
            if ctr.agent == crate::config::AgentKind::Gemini && !has_host_gemini_state_mount {
                Some(gemini_home_dir.as_path())
            } else {
                None
            };

        #[cfg(target_os = "macos")]
        if cfg.defaults.proxy.strict_network {
            self.push_log(
                "strict_network on macOS requires Docker `--privileged`; void-claw applies it automatically for this container launch",
                false,
            );
        }

        let session_token = uuid::Uuid::new_v4().simple().to_string();
        self.session_registry.insert(
            session_token.clone(),
            crate::server::SessionIdentity {
                project: proj.name.clone(),
                container_id: String::new(),
                mount_target: ctr.mount_target.display().to_string(),
            },
        );

        match crate::container::spawn(
            &ctr,
            &proj.name,
            &mount_source_path,
            codex_home_host_path,
            gemini_home_host_path,
            &session_token,
            &self.token,
            &exec_url,
            &proxy_url,
            &self.ca_cert_path,
            Some(scoped_proxy),
            cfg.defaults.proxy.strict_network,
            pty_rows,
            pty_cols,
        ) {
            Ok((session, launch_notes)) => {
                let new_si = self.sessions.len();
                self.sessions.push(session);
                if let Some(s) = self.sessions.get(new_si) {
                    self.session_registry.insert(
                        s.session_token.clone(),
                        crate::server::SessionIdentity {
                            project: s.project.clone(),
                            container_id: s.container_id.clone(),
                            mount_target: s.mount_target.clone(),
                        },
                    );
                }
                self.active_session = Some(new_si);
                self.scroll_mode = false;
                self.terminal_scroll = 0;
                self.focus = Focus::Terminal;
                for note in launch_notes {
                    self.push_log(note, false);
                }
                if let Some(pos) = self
                    .sidebar_items()
                    .iter()
                    .position(|item| *item == SidebarItem::Session(new_si))
                {
                    self.sidebar_idx = pos;
                }
            }
            Err(e) => {
                self.push_log(
                    format!("launch '{}' on '{}' failed: {e}", ctr.name, proj.name),
                    true,
                );
            }
        }
    }
}

```

## src/tui/app/runtime.rs

```rs
use super::*;

impl App {
    pub(crate) fn drain_channels(&mut self) {
        for _ in 0..32 {
            match self.exec_pending_rx.try_recv() {
                Ok(item) => self.pending_exec.push(item),
                Err(_) => break,
            }
        }
        for _ in 0..32 {
            match self.stop_pending_rx.try_recv() {
                Ok(item) => self.pending_stop.push(item),
                Err(_) => break,
            }
        }
        for _ in 0..32 {
            match self.net_pending_rx.try_recv() {
                Ok(item) => self.pending_net.push(item),
                Err(_) => break,
            }
        }
        for _ in 0..32 {
            match self.audit_rx.try_recv() {
                Ok(entry) => {
                    self.log.push_front(LogEntry::Audit(entry));
                    if self.log.len() > 500 {
                        self.log.pop_back();
                    }
                }
                Err(_) => break,
            }
        }
        for _ in 0..256 {
            match self.build_event_rx.try_recv() {
                Ok(BuildEvent::Output { line, is_error }) => {
                    self.push_build_output(line, is_error);
                }
                Ok(BuildEvent::Finished {
                    label,
                    launch_project_idx,
                    launch_container_idx,
                    success,
                    cancelled,
                    exit_code,
                    error,
                    diagnostic,
                }) => {
                    self.build_task = None;
                    if let Some(error) = error {
                        self.build_project_idx = None;
                        self.push_log(format!("{label} failed: {error}"), true);
                        if let Some(diagnostic) = diagnostic {
                            self.push_log(format!("  build detail: {diagnostic}"), true);
                        }
                        self.focus = Focus::ImageBuild;
                        continue;
                    }
                    if cancelled {
                        self.build_project_idx = None;
                        self.push_log(format!("{label} cancelled"), true);
                        self.focus = Focus::ImageBuild;
                        continue;
                    }
                    if success {
                        self.build_project_idx = None;
                        self.push_log(format!("{label} finished successfully"), false);
                        self.build_container_idx = None;
                        self.do_launch_container_on_project(
                            launch_project_idx,
                            launch_container_idx,
                        );
                    } else {
                        self.build_project_idx = None;
                        let suffix = exit_code
                            .map(|code| format!(" (exit code {code})"))
                            .unwrap_or_default();
                        self.push_log(format!("{label} failed{suffix}"), true);
                        if let Some(diagnostic) = diagnostic {
                            self.push_log(format!("  build detail: {diagnostic}"), true);
                        }
                        self.focus = Focus::ImageBuild;
                    }
                }
                Err(_) => break,
            }
        }

        for i in (0..self.sessions.len()).rev() {
            if !self.sessions[i].is_exited() {
                continue;
            }
            let exited_for = self.sessions[i].launched_at.elapsed();
            if !self.sessions[i].exit_reported {
                self.sessions[i].exit_reported = true;
                let label = self.sessions[i].tab_label();
                match crate::container::inspect_container_exit(&self.sessions[i].docker_name) {
                    Ok(Some((exit_code, error))) => {
                        let suffix = exit_code
                            .map(|code| format!(" (exit code {code})"))
                            .unwrap_or_default();
                        if error.is_empty() {
                            self.push_log(format!("{label} exited immediately{suffix}"), true);
                        } else {
                            self.push_log(
                                format!("{label} exited immediately{suffix}: {error}"),
                                true,
                            );
                        }
                    }
                    Ok(None) => {
                        self.push_log(format!("{label} exited immediately"), true);
                    }
                    Err(e) => {
                        self.push_log(
                            format!(
                                "{label} exited immediately; failed to inspect exit status: {e}"
                            ),
                            true,
                        );
                    }
                }
                continue;
            }
            if exited_for < std::time::Duration::from_secs(15) {
                continue;
            }
            let label = self.sessions[i].tab_label();
            self.push_log(format!("container '{}' exited", label), false);
            let tok = self.sessions[i].session_token.clone();
            self.sessions.remove(i);
            self.session_registry.remove(&tok);
            self.remap_session_indices_after_removal(i);
            if self.active_session.is_none() && self.focus != Focus::ImageBuild {
                self.focus = Focus::Sidebar;
            }
        }

        for idx in (0..self.pending_stop.len()).rev() {
            let Some((project, container_id)) = self
                .pending_stop
                .get(idx)
                .map(|item| (item.project.clone(), item.container_id.clone()))
            else {
                continue;
            };
            let decision = self.handle_stop_request(&project, &container_id);
            if let Some(tx) = self.pending_stop[idx].response_tx.take() {
                let _ = tx.send(decision);
            }
            self.pending_stop.remove(idx);
        }

        if self.focus == Focus::Terminal {
            if let Some(si) = self.active_session {
                if let Some(session) = self.sessions.get(si) {
                    session.clear_bell();
                }
            }
        }

        let max = self.sidebar_items().len().saturating_sub(1);
        if self.sidebar_idx > max {
            self.sidebar_idx = max;
        }
    }

    pub(crate) fn tick_watchers(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_watch_tick) < std::time::Duration::from_secs(1) {
            return;
        }
        self.last_watch_tick = now;

        let mut active_projects = Vec::new();
        for (pi, state) in &mut self.project_watch {
            if state.enabled {
                state.spinner_phase = state.spinner_phase.wrapping_add(1);
                active_projects.push(*pi);
            }
        }

        let cfg = self.config.get();
        for pi in active_projects {
            let Some(proj) = cfg.projects.get(pi).cloned() else {
                continue;
            };
            let mode = crate::config::effective_sync_mode(&proj, &cfg.defaults);
            let workspace = crate::config::effective_workspace_path(&proj, &cfg.workspace);
            let exclude_matcher =
                match crate::sync::build_project_exclude_matcher(&proj, &cfg.defaults) {
                    Ok(matcher) => matcher,
                    Err(e) => {
                        self.push_log(
                            format!(
                                "watch skipped for '{}': cannot load excludes: {e}",
                                proj.name
                            ),
                            true,
                        );
                        continue;
                    }
                };
            let canonical_files_now = compute_tree_file_map(&proj.canonical_path, &exclude_matcher);
            let workspace_files_now = compute_tree_file_map(&workspace, &exclude_matcher);

            let (canonical_changed, workspace_changed) = match self.project_watch.get(&pi) {
                Some(state) => (
                    diff_file_maps(&state.canonical_files, &canonical_files_now),
                    diff_file_maps(&state.workspace_files, &workspace_files_now),
                ),
                None => (vec![], vec![]),
            };

            match mode {
                SyncMode::WorkspaceOnly => {}
                SyncMode::Pushback => {
                    if !workspace_changed.is_empty() {
                        self.do_pushback_files(pi, &workspace_changed);
                    }
                }
                SyncMode::Bidirectional => {
                    if !canonical_changed.is_empty() {
                        self.do_seed_files(pi, &canonical_changed);
                    }
                    if !workspace_changed.is_empty() {
                        self.do_pushback_files(pi, &workspace_changed);
                    }
                }
                SyncMode::Pullthrough => {
                    if !canonical_changed.is_empty() {
                        self.do_seed_files(pi, &canonical_changed);
                    }
                }
                SyncMode::Direct => {}
            }

            if let Some(state) = self.project_watch.get_mut(&pi) {
                state.canonical_files =
                    compute_tree_file_map(&proj.canonical_path, &exclude_matcher);
                state.workspace_files = compute_tree_file_map(&workspace, &exclude_matcher);
            }
        }
    }

    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.focus != Focus::Terminal {
            return;
        }
        if self.active_exec_modal_idx().is_some() || !self.pending_net.is_empty() {
            return;
        }
        let Some(si) = self.active_session else {
            return;
        };
        let Some(session) = self.sessions.get(si) else {
            return;
        };

        match mouse.kind {
            MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => {
                // If the inner terminal app requested SGR mouse reporting, prefer forwarding
                // scroll events so internal scrollbars (e.g. OpenCode) work.
                if !self.scroll_mode {
                    if let Some(bytes) = maybe_encode_sgr_mouse_for_session(session, mouse) {
                        session.send_input(bytes);
                        return;
                    }
                }

                // Otherwise, treat the scroll wheel as a viewport scroll gesture, without
                // requiring explicit scroll-mode activation.
                let max_scrollback = session.term.lock().history_size();
                let lines_per_tick = 3usize;
                match mouse.kind {
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollLeft => {
                        self.terminal_scroll = self.terminal_scroll.saturating_add(lines_per_tick);
                    }
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollRight => {
                        self.terminal_scroll = self.terminal_scroll.saturating_sub(lines_per_tick);
                    }
                    _ => {}
                }
                self.terminal_scroll = self.terminal_scroll.min(max_scrollback);
                self.scroll_mode = self.terminal_scroll > 0;
            }
            _ => {
                // When the user is scrolling the outer viewport, don't forward clicks/drags into
                // the PTY (it would be surprising and could trigger actions in the inner app).
                if self.scroll_mode {
                    return;
                }
                if let Some(bytes) = maybe_encode_sgr_mouse_for_session(session, mouse) {
                    session.send_input(bytes);
                }
            }
        }
    }
}

```

## src/tui/app/settings.rs

```rs
use super::*;

impl App {
    pub(crate) fn refresh_projects_cache(&mut self) {
        let cfg = self.config.get();
        let mut last_reports: std::collections::HashMap<String, Option<SyncReport>> =
            std::collections::HashMap::new();
        for p in &self.projects {
            last_reports.insert(p.name.clone(), p.last_report.clone());
        }

        self.projects = cfg
            .projects
            .iter()
            .map(|p| ProjectStatus {
                name: p.name.clone(),
                last_report: last_reports.get(&p.name).cloned().unwrap_or(None),
            })
            .collect();
    }

    pub(crate) fn settings_action_rows_for(
        mode: SyncMode,
        watching: bool,
    ) -> Vec<SettingsActionRow> {
        if mode == SyncMode::Direct {
            return vec![SettingsActionRow {
                key: 'r',
                label: "Reload rules".to_string(),
                desc: "Rescan and reload void-rules.toml for this project.",
                action: SettingsAction::ReloadRules,
            }];
        }

        vec![
            SettingsActionRow {
                key: 's',
                label: "Seed workspace now".to_string(),
                desc: "Copy canonical files into workspace using sync rules.",
                action: SettingsAction::Seed,
            },
            SettingsActionRow {
                key: 'p',
                label: "Pushback workspace now".to_string(),
                desc: "Copy workspace edits back to canonical using sync rules.",
                action: SettingsAction::Pushback,
            },
            SettingsActionRow {
                key: if watching { 't' } else { 'w' },
                label: if watching {
                    "Stop file system watching".to_string()
                } else {
                    "Watch file system (runs Seed first)".to_string()
                },
                desc: if watching {
                    "Disable continuous sync for this project."
                } else {
                    "Continuously apply sync behavior based on sync mode."
                },
                action: SettingsAction::WatchToggle,
            },
            SettingsActionRow {
                key: 'r',
                label: "Reload rules".to_string(),
                desc: "Rescan and reload void-rules.toml for this project.",
                action: SettingsAction::ReloadRules,
            },
            SettingsActionRow {
                key: 'x',
                label: "Clear workspace".to_string(),
                desc: "Delete the entire workspace directory for a clean re-seed.",
                action: SettingsAction::Clear,
            },
        ]
    }

    pub(crate) fn settings_action_rows(&self, project_idx: usize) -> Vec<SettingsActionRow> {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(project_idx) else {
            return Vec::new();
        };
        let mode = crate::config::effective_sync_mode(proj, &cfg.defaults);
        let watching = self.is_project_watching(project_idx);
        Self::settings_action_rows_for(mode, watching)
    }

    pub(crate) fn handle_settings_key(&mut self, key: KeyEvent) {
        let Some(pi) = self.active_settings_project else {
            self.focus = Focus::Sidebar;
            return;
        };

        let actions_len = self.settings_action_rows(pi).len();
        if actions_len == 0 {
            self.focus = Focus::Sidebar;
            self.active_settings_project = None;
            return;
        }
        if self.settings_cursor >= actions_len {
            self.settings_cursor = actions_len.saturating_sub(1);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => {
                self.focus = Focus::Sidebar;
                self.active_settings_project = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.settings_cursor > 0 {
                    self.settings_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.settings_cursor + 1 < actions_len {
                    self.settings_cursor += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => self.run_settings_action(pi),
            KeyCode::Char('r') | KeyCode::Char('R') => self.do_reload_rules(pi),
            KeyCode::Char('s')
            | KeyCode::Char('S')
            | KeyCode::Char('p')
            | KeyCode::Char('P')
            | KeyCode::Char('w')
            | KeyCode::Char('W')
            | KeyCode::Char('t')
            | KeyCode::Char('T')
            | KeyCode::Char('x')
            | KeyCode::Char('X') => {
                let cfg = self.config.get();
                if let Some(proj) = cfg.projects.get(pi) {
                    if crate::config::effective_sync_mode(proj, &cfg.defaults) != SyncMode::Direct {
                        match key.code {
                            KeyCode::Char('s') | KeyCode::Char('S') => self.do_seed_project(pi),
                            KeyCode::Char('p') | KeyCode::Char('P') => self.do_pushback_project(pi),
                            KeyCode::Char('w') | KeyCode::Char('W') => self.start_project_watch(pi),
                            KeyCode::Char('t') | KeyCode::Char('T') => self.stop_project_watch(pi),
                            KeyCode::Char('x') | KeyCode::Char('X') => self.do_clear_workspace(pi),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn run_settings_action(&mut self, pi: usize) {
        let actions = self.settings_action_rows(pi);
        let Some(row) = actions.get(self.settings_cursor) else {
            return;
        };
        match row.action {
            SettingsAction::Seed => self.do_seed_project(pi),
            SettingsAction::Pushback => self.do_pushback_project(pi),
            SettingsAction::WatchToggle => {
                if self.is_project_watching(pi) {
                    self.stop_project_watch(pi);
                } else {
                    self.start_project_watch(pi);
                }
            }
            SettingsAction::ReloadRules => self.do_reload_rules(pi),
            SettingsAction::Clear => self.do_clear_workspace(pi),
        }
    }

    pub(crate) fn do_reload_rules(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        let proj = proj.clone();
        self.log_project_rules_status(&proj);
    }

    pub(crate) fn do_clear_workspace(&mut self, pi: usize) {
        let cfg = self.config.get();
        let Some(proj) = cfg.projects.get(pi) else {
            return;
        };
        if crate::config::effective_sync_mode(proj, &cfg.defaults) == SyncMode::Direct {
            self.push_log(
                format!(
                    "clear '{}': disabled for projects.sync.mode='direct' (would affect canonical directory)",
                    proj.name
                ),
                true,
            );
            return;
        }
        let workspace_path = crate::config::effective_workspace_path(proj, &cfg.workspace);
        if !workspace_path.exists() {
            self.push_log(
                format!("clear '{}': workspace directory does not exist", proj.name),
                false,
            );
            return;
        }
        match std::fs::remove_dir_all(&workspace_path) {
            Ok(()) => self.push_log(
                format!(
                    "clear '{}': removed {}",
                    proj.name,
                    workspace_path.display()
                ),
                false,
            ),
            Err(e) => self.push_log(format!("clear '{}' failed: {}", proj.name, e), true),
        }
    }

    pub(crate) fn handle_terminal_key(&mut self, key: KeyEvent) {
        if self.build_is_running() && self.active_session.is_none() {
            self.handle_build_scroll_key(key);
            return;
        }

        if key.code == KeyCode::Char('g') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.terminal_fullscreen {
                self.close_terminal_fullscreen();
            } else {
                self.open_terminal_fullscreen();
            }
            return;
        }

        if self.scroll_mode {
            self.handle_scroll_mode_key(key);
            return;
        }

        if key.code == KeyCode::Esc {
            let now = std::time::Instant::now();
            let threshold = std::time::Duration::from_millis(400);
            if self
                .last_terminal_esc
                .map(|prev| now.duration_since(prev) <= threshold)
                .unwrap_or(false)
            {
                self.last_terminal_esc = None;
                if self.terminal_fullscreen {
                    self.close_terminal_fullscreen();
                } else {
                    self.should_quit = true;
                }
                return;
            }
            self.last_terminal_esc = Some(now);
            return;
        } else {
            self.last_terminal_esc = None;
        }

        if let Some(si) = self.active_session {
            if self.session_is_loading(si) {
                return;
            }
        }

        if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::ALT) {
            self.open_log_fullscreen();
            return;
        }

        if is_scroll_mode_toggle_key(key) {
            self.scroll_mode = true;
            return;
        }

        if let Some(si) = self.active_session {
            if let Some(bytes) = key_to_bytes(key) {
                if let Some(session) = self.sessions.get(si) {
                    session.send_input(bytes);
                }
            }
        }
    }

    pub(crate) fn handle_scroll_mode_key(&mut self, key: KeyEvent) {
        let half_page = self
            .active_session
            .and_then(|si| self.sessions.get(si))
            .map(|s| s.term.lock().screen_lines().max(2) / 2)
            .unwrap_or(15);

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.terminal_scroll = self.terminal_scroll.saturating_add(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.terminal_scroll = self.terminal_scroll.saturating_sub(1)
            }
            KeyCode::PageUp => {
                self.terminal_scroll = self.terminal_scroll.saturating_add(half_page)
            }
            KeyCode::PageDown => {
                self.terminal_scroll = self.terminal_scroll.saturating_sub(half_page)
            }
            KeyCode::Home | KeyCode::Char('g') => self.terminal_scroll = usize::MAX,
            KeyCode::End | KeyCode::Char('G') => self.terminal_scroll = 0,
            KeyCode::Esc | KeyCode::Char('q') => self.exit_scroll_mode(),
            _ => self.exit_scroll_mode(),
        }
    }

    pub(crate) fn exit_scroll_mode(&mut self) {
        self.scroll_mode = false;
        self.terminal_scroll = 0;
    }

    pub(crate) fn handle_build_scroll_key(&mut self, key: KeyEvent) {
        let max_scroll = self.build_output.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.build_scroll = self.build_scroll.saturating_add(1).min(max_scroll)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.build_scroll = self.build_scroll.saturating_sub(1)
            }
            KeyCode::PageUp => {
                self.build_scroll = self.build_scroll.saturating_add(15).min(max_scroll)
            }
            KeyCode::PageDown => self.build_scroll = self.build_scroll.saturating_sub(15),
            KeyCode::Home | KeyCode::Char('g') => self.build_scroll = max_scroll,
            KeyCode::End | KeyCode::Char('G') => self.build_scroll = 0,
            KeyCode::Esc => self.focus = Focus::Sidebar,
            _ => {}
        }
    }

    pub(crate) fn open_picker(&mut self) {
        let cfg = self.config.get();
        if cfg.containers.is_empty() {
            self.push_log("no containers defined in config", true);
            return;
        }
        self.container_picker = Some(0);
        self.focus = Focus::ContainerPicker;
    }

    pub(crate) fn handle_picker_key(&mut self, key: KeyEvent) {
        let cfg = self.config.get();
        let n = cfg.containers.len();
        let idx = self.container_picker.as_mut().unwrap();

        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => {
                self.container_picker = None;
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if *idx > 0 {
                    *idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *idx + 1 < n {
                    *idx += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                let ctr_idx = *idx;
                self.container_picker = None;
                self.focus = Focus::Sidebar;
                self.do_launch_container(ctr_idx);
            }
            _ => {}
        }
    }

    const BUILD_ACTION_COUNT: usize = 2;

    pub(crate) fn handle_build_key(&mut self, key: KeyEvent) {
        if self.build_is_running() {
            let max_scroll = self.build_output.len();
            match key.code {
                KeyCode::Esc | KeyCode::Char('h') => self.focus = Focus::Sidebar,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.build_scroll = self.build_scroll.saturating_add(1).min(max_scroll)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.build_scroll = self.build_scroll.saturating_sub(1)
                }
                KeyCode::PageUp => {
                    self.build_scroll = self.build_scroll.saturating_add(15).min(max_scroll)
                }
                KeyCode::PageDown => self.build_scroll = self.build_scroll.saturating_sub(15),
                KeyCode::Home | KeyCode::Char('g') => self.build_scroll = max_scroll,
                KeyCode::End | KeyCode::Char('G') => self.build_scroll = 0,
                _ => {}
            }
            return;
        }

        if matches!(key.code, KeyCode::Char('r') | KeyCode::Char('R')) {
            self.build_cursor = 0;
            self.run_build_action();
            return;
        }
        if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')) {
            self.build_cursor = 1;
            self.run_build_action();
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => {
                self.build_container_idx = None;
                self.build_project_idx = None;
                self.focus = Focus::Sidebar;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.build_cursor > 0 {
                    self.build_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.build_cursor + 1 < Self::BUILD_ACTION_COUNT {
                    self.build_cursor += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => self.run_build_action(),
            _ => {}
        }
    }

    pub(crate) fn run_build_action(&mut self) {
        let cfg = self.config.get();
        let Some(ctr_idx) = self.build_container_idx else {
            return;
        };
        let Some(ctr) = cfg.containers.get(ctr_idx) else {
            return;
        };
        let (base_cmd, agent_cmd) = Self::build_commands_for(&cfg.docker_dir, &ctr.image);

        let requested = match self.build_cursor {
            0 => match agent_cmd.as_ref() {
                Some(agent_cmd) => Some((
                    "build + launch",
                    format!(
                        "{} && {}",
                        shell_command_for_docker_args(&base_cmd),
                        shell_command_for_docker_args(agent_cmd)
                    ),
                )),
                None => Some(("build + launch", shell_command_for_docker_args(&base_cmd))),
            },
            1 => {
                self.build_container_idx = None;
                self.build_project_idx = None;
                self.focus = Focus::Sidebar;
                return;
            }
            _ => None,
        };

        let Some((label, shell_command)) = requested else {
            return;
        };

        self.build_project_idx = self.selected_project_idx();
        let Some(launch_project_idx) = self.build_project_idx else {
            self.push_log("cannot start build: no project selected", true);
            return;
        };
        self.start_docker_build(label, shell_command, launch_project_idx, ctr_idx);
    }

    pub fn build_is_running(&self) -> bool {
        self.build_task.is_some()
    }

    pub fn active_build_command(&self) -> Option<&str> {
        self.build_task
            .as_ref()
            .map(|task| task.shell_command.as_str())
    }
}

```

## src/tui/mod.rs

```rs
mod app;
pub mod render;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::TermMode;
use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableMouseCapture, Event, EventStream,
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    style::ResetColor,
    terminal::{
        EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

use crate::container::ContainerSession;
use crate::rules::{NetworkPolicy, NetworkRule};
use crate::server::SessionRegistry;
use crate::server::{ApprovalDecision, ContainerStopDecision, ContainerStopItem, PendingItem};
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, StateManager};
use crate::sync::SyncReport;
use crate::{
    config::SyncMode,
    proxy::{NetworkDecision, PendingNetworkItem, ProxyState},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsAction {
    Seed,
    Pushback,
    WatchToggle,
    ReloadRules,
    Clear,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SettingsActionRow {
    pub key: char,
    pub label: String,
    pub desc: &'static str,
    action: SettingsAction,
}

#[derive(Debug, Clone)]
/// A log line shown in the TUI log pane.
pub enum LogEntry {
    Audit(AuditEntry),
    Msg {
        text: String,
        is_error: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, PartialEq)]
/// Selectable entries in the left sidebar.
pub enum SidebarItem {
    Project(usize),
    Session(usize),
    Settings(usize),
    Launch(usize),
    Build(usize),
    NewProject,
}

#[derive(Debug, Clone, PartialEq)]
/// The currently focused UI region.
pub enum Focus {
    Sidebar,
    Terminal,
    Settings,
    ContainerPicker,
    ImageBuild,
    NewProject,
}

#[derive(Debug, Clone)]
/// Transient state for the new-project wizard.
pub struct NewProjectState {
    pub cursor: usize,
    pub name: String,
    pub canonical_dir: String,
    pub sync_mode: SyncMode,
    pub project_type: crate::new_project::ProjectType,
    pub error: Option<String>,
}

/// Top-level TUI application state and event loop ownership.
pub struct App {
    pub config: SharedConfig,
    pub loaded_config_path: PathBuf,
    pub token: String,
    pub session_registry: SessionRegistry,
    pub ca_cert_path: String,
    proxy_state: ProxyState,

    pub projects: Vec<ProjectStatus>,
    pub pending_exec: Vec<PendingItem>,
    pub pending_stop: Vec<ContainerStopItem>,
    pub pending_net: Vec<PendingNetworkItem>,
    pub log: VecDeque<LogEntry>,
    pub log_scroll: usize,

    pub focus: Focus,
    pub sidebar_idx: usize,
    pub sidebar_offset: usize,
    pub active_session: Option<usize>,
    pub preview_session: Option<usize>,
    pub active_settings_project: Option<usize>,
    pub settings_cursor: usize,

    pub container_picker: Option<usize>,
    pub build_container_idx: Option<usize>,
    pub build_project_idx: Option<usize>,
    pub build_cursor: usize,
    pub build_output: VecDeque<(String, bool)>,
    pub build_scroll: usize,
    pub sessions: Vec<ContainerSession>,
    pub new_project: Option<NewProjectState>,

    pub exec_pending_rx: mpsc::Receiver<PendingItem>,
    pub stop_pending_rx: mpsc::Receiver<ContainerStopItem>,
    pub net_pending_rx: mpsc::Receiver<PendingNetworkItem>,
    pub audit_rx: mpsc::Receiver<AuditEntry>,
    build_event_rx: mpsc::UnboundedReceiver<BuildEvent>,
    build_event_tx: mpsc::UnboundedSender<BuildEvent>,
    build_task: Option<BuildTaskState>,

    pub should_quit: bool,
    pub log_fullscreen: bool,
    pub terminal_fullscreen: bool,
    ctrl_c_times: Vec<std::time::Instant>,
    last_terminal_esc: Option<std::time::Instant>,
    pub scroll_mode: bool,
    pub terminal_scroll: usize,
    project_watch: HashMap<usize, ProjectWatchState>,
    last_watch_tick: std::time::Instant,
}

/// Cached project metadata and latest sync report for the sidebar.
pub struct ProjectStatus {
    pub name: String,
    pub last_report: Option<SyncReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSignature {
    size: u64,
    mtime_secs: u64,
    mtime_nanos: u32,
}

struct ProjectWatchState {
    enabled: bool,
    spinner_phase: usize,
    canonical_files: HashMap<PathBuf, FileSignature>,
    workspace_files: HashMap<PathBuf, FileSignature>,
}

#[derive(Debug)]
enum BuildEvent {
    Output {
        line: String,
        is_error: bool,
    },
    Finished {
        label: String,
        launch_project_idx: usize,
        launch_container_idx: usize,
        success: bool,
        cancelled: bool,
        exit_code: Option<i32>,
        error: Option<String>,
        diagnostic: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct BuildTaskState {
    label: String,
    shell_command: String,
    cancel_flag: Arc<AtomicBool>,
}

fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let b = c as u8;
                if b.is_ascii_alphabetic() {
                    Some(vec![b & 0x1f])
                } else {
                    Some(c.to_string().into_bytes())
                }
            } else {
                let mut buf = [0u8; 4];
                Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
            }
        }
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Esc => Some(b"\x1b".to_vec()),
        KeyCode::F(n) if (1..=12).contains(&n) => {
            let f_keys: [&[u8]; 12] = [
                b"\x1bOP",
                b"\x1bOQ",
                b"\x1bOR",
                b"\x1bOS",
                b"\x1b[15~",
                b"\x1b[17~",
                b"\x1b[18~",
                b"\x1b[19~",
                b"\x1b[20~",
                b"\x1b[21~",
                b"\x1b[23~",
                b"\x1b[24~",
            ];
            Some(f_keys[(n - 1) as usize].to_vec())
        }
        _ => None,
    }
}

// ── Event loop ────────────────────────────────────────────────────────────────

pub async fn run(mut app: App) -> Result<()> {
    // Must run *before* `enable_raw_mode()`: the guard restores full termios on
    // drop, so capturing termios while already in raw mode would "restore" the
    // raw settings after shutdown and permanently corrupt the user's shell.
    let _termios_guard = disable_xon_xoff();
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        cursor::Hide,
        EnableMouseCapture
    )?;
    let mut restore_guard = TerminalRestoreGuard::new();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    restore_terminal_output(terminal.backend_mut())?;
    terminal.show_cursor()?;
    restore_guard.disarm();

    result
}

fn restore_terminal_output<W: std::io::Write>(writer: &mut W) -> std::io::Result<()> {
    execute!(
        writer,
        LeaveAlternateScreen,
        cursor::Show,
        DisableMouseCapture,
        DisableBracketedPaste,
        EnableLineWrap,
        ResetColor
    )
}

struct TerminalRestoreGuard {
    armed: bool,
}

impl TerminalRestoreGuard {
    fn new() -> Self {
        Self { armed: true }
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = restore_terminal_output(&mut stdout);
    }
}

#[cfg(unix)]
fn disable_xon_xoff() -> Option<TermiosGuard> {
    disable_xon_xoff_on_fd(libc::STDIN_FILENO)
}

#[cfg(unix)]
fn disable_xon_xoff_on_fd(fd: i32) -> Option<TermiosGuard> {
    use std::mem::MaybeUninit;
    unsafe {
        let mut orig = MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(fd, orig.as_mut_ptr()) != 0 {
            return None;
        }
        let orig = orig.assume_init();
        let ixon_was_enabled = (orig.c_iflag & libc::IXON) != 0;
        let mut t = orig;
        t.c_iflag &= !libc::IXON;
        if libc::tcsetattr(fd, libc::TCSANOW, &t) != 0 {
            return None;
        }
        Some(TermiosGuard {
            fd,
            ixon_was_enabled,
        })
    }
}

#[cfg(not(unix))]
fn disable_xon_xoff() -> Option<()> {
    None
}

#[cfg(unix)]
struct TermiosGuard {
    fd: i32,
    ixon_was_enabled: bool,
}

#[cfg(unix)]
impl Drop for TermiosGuard {
    fn drop(&mut self) {
        unsafe {
            let mut cur = std::mem::MaybeUninit::<libc::termios>::uninit();
            if libc::tcgetattr(self.fd, cur.as_mut_ptr()) != 0 {
                return;
            }
            let mut cur = cur.assume_init();
            if self.ixon_was_enabled {
                cur.c_iflag |= libc::IXON;
            } else {
                cur.c_iflag &= !libc::IXON;
            }
            let _ = libc::tcsetattr(self.fd, libc::TCSANOW, &cur);
        }
    }
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut events = EventStream::new();
    let tick = tokio::time::Duration::from_millis(50);

    loop {
        app.drain_channels();
        app.tick_watchers();
        terminal.draw(|frame| render::render(frame, app))?;

        if app.should_quit {
            app.terminate_all_sessions();
            break;
        }

        let timeout = tokio::time::sleep(tick);

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => app.handle_key(key),
                    Some(Ok(Event::Mouse(mouse))) => app.handle_mouse(mouse),
                    Some(Ok(Event::Paste(text))) => {
                        if app.focus == Focus::NewProject {
                            app.append_new_project_text(&text);
                        } else if let Some(si) = app.active_session {
                            if let Some(session) = app.sessions.get(si) {
                                session.send_input(text.into_bytes());
                            }
                        }
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        let pty_cols = cols.saturating_sub(38).max(20);
                        let pty_rows = rows.saturating_sub(10).max(6);
                        for session in &mut app.sessions {
                            let _ = session.resize(pty_rows, pty_cols);
                        }
                    }
                    None => break,
                    _ => {}
                }
            }
            _ = timeout => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;

```

## src/tui/render.rs

```rs
#![allow(unused_imports)]

use super::*;
use crate::state::DecisionKind;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags as TermFlags;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Rgb};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::time::{SystemTime, UNIX_EPOCH};

mod overlays;
mod root;
mod sidebar;
mod terminal;

pub(crate) use overlays::*;
pub use root::render;
pub(crate) use root::render_scrollbar;
pub(crate) use sidebar::*;
pub(crate) use terminal::*;

mod panes {
    use super::*;

    #[path = "build.rs"]
    mod build;
    #[path = "text.rs"]
    mod text;

    pub(crate) use build::*;
    pub(crate) use text::*;
}

pub(crate) use panes::*;

```

## src/tui/render/overlays.rs

```rs
use super::*;

pub(crate) fn render_exec_approval_overlay(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    item_idx: usize,
) {
    let Some(item) = app.pending_exec.get(item_idx) else {
        return;
    };

    let popup_area = centered_rect(72, 56, 12, area);
    frame.render_widget(Clear, popup_area);

    let match_str = match &item.matched_command {
        Some(name) => format!("rule: {name}"),
        None => "unlisted command".to_string(),
    };

    let action_line = Line::from(vec![
        Span::styled(
            "[y/↵] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Approve  ", Style::default().fg(Color::White)),
        Span::styled(
            "[r] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always allow  ", Style::default().fg(Color::White)),
        Span::styled(
            "[n/Esc] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Deny  ", Style::default().fg(Color::White)),
        Span::styled(
            "[d] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always deny", Style::default().fg(Color::White)),
    ]);

    let queue_total = app
        .pending_exec
        .iter()
        .filter(|i| i.project == item.project)
        .count();
    let queue_pos = app
        .pending_exec
        .iter()
        .filter(|i| i.project == item.project)
        .position(|i| i.id == item.id)
        .map(|i| i + 1)
        .unwrap_or(1);
    let source_container = item
        .container_id
        .clone()
        .unwrap_or_else(|| "unknown-container".to_string());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  APPROVAL REQUIRED",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Command : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.argv.join(" "),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Project : ", Style::default().fg(Color::DarkGray)),
            Span::styled(item.project.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Source  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("workspace={}  container={}", item.project, source_container),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Queue   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{}/{} for workspace '{}' (exec total: {}, net total: {})",
                    queue_pos,
                    queue_total.max(1),
                    item.project,
                    app.pending_exec.len(),
                    app.pending_net.len()
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Host cwd: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.cwd.display().to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Match   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(match_str, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        action_line,
        Line::from(""),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Exec Approval Required ")
                .title_alignment(Alignment::Center)
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        popup_area,
    );
}

// ── Network approval overlay ──────────────────────────────────────────────────

pub(crate) fn render_net_approval_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let Some(item) = app.pending_net.first() else {
        return;
    };

    let show_proxy_details = item.source_status != "listener_bound_source";
    let popup_area = centered_rect(72, 56, if show_proxy_details { 13 } else { 12 }, area);
    frame.render_widget(Clear, popup_area);

    let action_line = Line::from(vec![
        Span::styled(
            "[y/↵] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Allow  ", Style::default().fg(Color::White)),
        Span::styled(
            "[r] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always allow  ", Style::default().fg(Color::White)),
        Span::styled(
            "[n/Esc] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Deny  ", Style::default().fg(Color::White)),
        Span::styled(
            "[d] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always deny", Style::default().fg(Color::White)),
    ]);

    let queue_total = app.pending_net.len();
    let source_workspace = item
        .source_project
        .clone()
        .unwrap_or_else(|| "unknown-workspace".to_string());
    let source_container = item
        .source_container
        .clone()
        .unwrap_or_else(|| "unknown-container".to_string());

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  NETWORK REQUEST",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Method  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.method.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Host    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(item.host.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Path    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(item.path.clone(), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Source  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "workspace={}  container={}",
                    source_workspace, source_container
                ),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Queue   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "1/{} (exec total: {}, net total: {})",
                    queue_total.max(1),
                    app.pending_exec.len(),
                    app.pending_net.len()
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        action_line,
        Line::from(""),
    ];
    if show_proxy_details {
        lines.insert(
            7,
            Line::from(vec![
                Span::styled("  Proxy   : ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(
                        "source_status={}  proxy_auth={}",
                        item.source_status, item.has_proxy_authorization
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        );
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Network Approval Required ")
                .title_alignment(Alignment::Center)
                .title_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        ),
        popup_area,
    );
}

// ── Fullscreen log ────────────────────────────────────────────────────────────

pub(crate) fn render_log_fullscreen(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Log (fullscreen) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let lines: Vec<Line> = app
        .log
        .iter()
        .map(|entry| match entry {
            LogEntry::Audit(e) => {
                let ts = e.timestamp.format("%H:%M:%S").to_string();
                let decision_color = match e.decision {
                    crate::state::DecisionKind::Auto => Color::Green,
                    crate::state::DecisionKind::Approved
                    | crate::state::DecisionKind::Remembered => Color::Cyan,
                    crate::state::DecisionKind::Denied
                    | crate::state::DecisionKind::DeniedByPolicy
                    | crate::state::DecisionKind::TimedOut => Color::Red,
                };
                let exit_str = match e.exit_code {
                    Some(c) => format!(" exit={c}"),
                    None => String::new(),
                };
                Line::from(vec![
                    Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:<6} ", e.decision.as_str()),
                        Style::default()
                            .fg(decision_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<16} ", e.project),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(e.argv.join(" ")),
                    Span::styled(exit_str, Style::default().fg(Color::DarkGray)),
                ])
            }
            LogEntry::Msg {
                text,
                is_error,
                timestamp,
            } => {
                let ts = timestamp.format("%H:%M:%S").to_string();
                let (prefix, color) = if *is_error {
                    ("ERR   ", Color::Red)
                } else {
                    ("INFO  ", Color::Green)
                };
                Line::from(vec![
                    Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{prefix:<6} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(text.clone(), Style::default().fg(Color::White)),
                ])
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((app.log_scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn render_status_bar_log(frame: &mut Frame, _app: &mut App, area: Rect) {
    frame.render_widget(
        Paragraph::new(Span::styled(
            " [↑↓/jk]scroll  [o/Esc/q]close",
            Style::default().fg(Color::DarkGray),
        )),
        area,
    );
}

// ── Layout helpers ────────────────────────────────────────────────────────────

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, min_height: u16, r: Rect) -> Rect {
    let height = ((r.height * percent_y) / 100).max(min_height).min(r.height);
    let width = (r.width * percent_x) / 100;
    Rect {
        x: (r.width.saturating_sub(width)) / 2 + r.x,
        y: (r.height.saturating_sub(height)) / 2 + r.y,
        width,
        height,
    }
}

```

## src/tui/render/panes/build.rs

```rs
use super::*;

pub(crate) fn render_container_picker(frame: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let selected_ctr = app.container_picker.unwrap_or(0);
    let project_name = app
        .selected_project_idx()
        .and_then(|pi| app.projects.get(pi))
        .map(|p| p.name.as_str())
        .unwrap_or("(no project)");

    let tone = |c| maybe_dim(c, dimmed);
    let workspace_path = app
        .selected_project_idx()
        .and_then(|pi| cfg.projects.get(pi))
        .map(|proj| crate::config::effective_workspace_path(proj, &cfg.workspace));
    let block = Block::default()
        .title(format!(" Run Container for '{}' ", project_name))
        .title_style(
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::Cyan)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Choose an agent to launch below. Your host dir ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                workspace_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<workspace>".to_string()),
                Style::default()
                    .fg(tone(Color::White))
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                " will be mounted inside the agent container at ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                "/workspace",
                Style::default()
                    .fg(tone(Color::White))
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                ", and the agent will start automatically.",
                Style::default().fg(tone(Color::DarkGray)),
            ),
        ]),
        Line::from(""),
    ];

    for (i, c) in cfg.containers.iter().enumerate() {
        let marker = if i == selected_ctr { "▶ " } else { "  " };
        let name_style = if i == selected_ctr {
            Style::default()
                .fg(tone(Color::White))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone(Color::White))
        };

        let spans = vec![
            Span::styled(
                format!("  {marker}"),
                Style::default().fg(tone(Color::Cyan)),
            ),
            Span::styled(c.name.clone(), name_style),
        ];
        lines.push(Line::from(spans));

        lines.push(Line::from(Span::styled(
            format!("      {}", c.image),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [^B] Back to sidebar",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Image build pane ─────────────────────────────────────────────────────────

pub(crate) fn render_image_build(frame: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let ctr_idx = app.build_container_idx.unwrap_or(0);
    let image = cfg
        .containers
        .get(ctr_idx)
        .map(|c| c.image.as_str())
        .unwrap_or("<unknown>");

    let docker_dir = cfg.docker_dir.as_path();
    let (base_cmd, agent_cmd) = App::build_commands_for(docker_dir, image);
    let base_cmd_str = format!("docker {}", base_cmd.join(" "));
    let agent_cmd_str = agent_cmd
        .as_ref()
        .map(|cmd| format!("docker {}", cmd.join(" ")));

    let parts: Vec<&str> = image.splitn(2, ':').collect();
    let name = parts[0].split('/').last().unwrap_or(parts[0]);
    let tag = parts.get(1).copied().unwrap_or("ubuntu-24.04");
    let dockerfile_root = docker_dir;
    let base_dockerfile = dockerfile_root.join(format!("{tag}.Dockerfile"));
    let agent_dockerfile = name.strip_prefix("void-claw-").map(|agent| {
        dockerfile_root
            .join(agent)
            .join(format!("{tag}.Dockerfile"))
    });

    let tone = |c| maybe_dim(c, dimmed);
    let focused = app.focus == Focus::ImageBuild;
    let border_style = if focused {
        Style::default().fg(tone(Color::Cyan))
    } else {
        Style::default().fg(tone(Color::DarkGray))
    };

    let block = Block::default()
        .title(" Image Build Required ")
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(border_style);

    let cursor = app.build_cursor;

    let run_all_cmd_str = match agent_cmd_str.as_ref() {
        Some(agent_cmd_str) => format!("{base_cmd_str} && {agent_cmd_str}"),
        None => base_cmd_str.clone(),
    };
    let actions: [(&str, &str, Option<&str>, &str); 2] = [
        (
            "r",
            "Run all build commands and launch container (Recommended)",
            Some(&run_all_cmd_str),
            "",
        ),
        ("c", "Cancel", None, "Return to sidebar"),
    ];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Image '{image}' was not found locally."),
            Style::default().fg(tone(Color::Yellow)),
        )),
        Line::from(Span::styled(
            "  Docker images must be built before containers can be launched.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Dockerfiles",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Base  : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                base_dockerfile.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Agent : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                agent_dockerfile
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "(n/a for custom image tag)".to_string()),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Select an action to run, or copy the commands below to run manually.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (i, (label, name, cmd, desc)) in actions.iter().enumerate() {
        let selected = i == cursor;
        let marker = if selected { "▶ " } else { "  " };
        let name_style = if selected {
            Style::default()
                .fg(tone(Color::White))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone(Color::White))
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {marker}"),
                Style::default().fg(tone(Color::Cyan)),
            ),
            Span::styled(format!("{label}) "), Style::default().fg(tone(Color::Cyan))),
            Span::styled(*name, name_style),
        ]));
        if let Some(cmd) = cmd {
            lines.push(Line::from(vec![
                Span::styled("      $ ", Style::default().fg(tone(Color::Green))),
                Span::styled(*cmd, Style::default().fg(tone(Color::DarkGray))),
            ]));
        }
        lines.push(Line::from(Span::styled(
            format!("      {desc}"),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [^B] Back to sidebar",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn render_build_output(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let image = app
        .build_container_idx
        .and_then(|idx| cfg.containers.get(idx))
        .map(|c| c.image.as_str())
        .unwrap_or("<unknown>");
    let tone = |c| maybe_dim(c, dimmed);
    let focused = app.focus == Focus::ImageBuild;
    let border_style = if focused {
        Style::default().fg(tone(Color::Cyan))
    } else {
        Style::default().fg(tone(Color::DarkGray))
    };

    let block = Block::default()
        .title(format!(" docker build {image} "))
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut header_lines: Vec<Line> = vec![];
    let max_cols = inner.width.saturating_sub(1) as usize;
    if let Some(cmd) = app.active_build_command() {
        let cmd = clamp_for_width(&strip_ansi_and_control(cmd), max_cols);
        header_lines.push(Line::from(vec![
            Span::styled("$ ", Style::default().fg(tone(Color::Green))),
            Span::styled(cmd, Style::default().fg(tone(Color::DarkGray))),
        ]));
        header_lines.push(Line::from(""));
    }

    let visible_rows = (inner.height as usize).saturating_sub(header_lines.len());
    let total = app.build_output.len();
    let max_scroll = total.saturating_sub(visible_rows);
    let scroll = app.build_scroll.min(max_scroll);
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows);

    let mut lines = header_lines;
    for (line, is_error) in app.build_output.iter().skip(start).take(end - start) {
        let clean = clamp_for_width(&strip_ansi_and_control(line), max_cols);
        lines.push(Line::from(Span::styled(
            clean,
            Style::default().fg(if *is_error {
                tone(Color::Red)
            } else {
                tone(Color::White)
            }),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    if app.build_scroll > 0 && max_scroll > 0 {
        render_scrollbar(frame, inner, max_scroll, scroll, true);
    }
}

pub(crate) fn strip_ansi_and_control(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        if ch == '\r' {
            continue;
        }
        if ch.is_control() && ch != '\t' {
            continue;
        }
        if ch == '\t' {
            out.push_str("    ");
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn clamp_for_width(input: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let mut out = String::new();
    for (i, ch) in input.chars().enumerate() {
        if i >= max_cols {
            break;
        }
        out.push(ch);
    }
    out
}

```

## src/tui/render/panes/text.rs

```rs
use super::*;

pub(crate) fn render_log(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let lines: Vec<Line> = app
        .log
        .iter()
        .map(|entry| match entry {
            LogEntry::Audit(e) => {
                let ts = e.timestamp.format("%H:%M:%S").to_string();
                let decision_color = match e.decision {
                    DecisionKind::Auto => Color::Green,
                    DecisionKind::Approved | DecisionKind::Remembered => Color::Cyan,
                    DecisionKind::Denied
                    | DecisionKind::DeniedByPolicy
                    | DecisionKind::TimedOut => Color::Red,
                };
                let exit_str = match e.exit_code {
                    Some(c) => format!(" exit={c}"),
                    None => String::new(),
                };
                Line::from(vec![
                    Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:<6} ", e.decision.as_str()),
                        Style::default()
                            .fg(decision_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<16} ", e.project),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(e.argv.join(" ")),
                    Span::styled(exit_str, Style::default().fg(Color::DarkGray)),
                ])
            }
            LogEntry::Msg {
                text,
                is_error,
                timestamp,
            } => {
                let ts = timestamp.format("%H:%M:%S").to_string();
                let (prefix, color) = if *is_error {
                    ("ERR   ", Color::Red)
                } else {
                    ("INFO  ", Color::Green)
                };
                Line::from(vec![
                    Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{prefix:<6} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(text.clone(), Style::default().fg(Color::White)),
                ])
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((app.log_scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Status bar ────────────────────────────────────────────────────────────────

pub(crate) fn render_status_bar(frame: &mut Frame, app: &mut App, area: Rect) {
    let keys = match app.focus {
        Focus::Sidebar => {
            if app.build_is_running() {
                " [↑↓/jk]navigate  [↵/l]select  [^C]cancel build  [o]log  [^Q]quit"
            } else {
                " [↑↓/jk]navigate  [↵/l]select  [o]log  [^Q]quit"
            }
        }
        Focus::Terminal if app.scroll_mode => {
            " SCROLL: [↑↓/jk]line  [PgUp/PgDn]page  [g/G]top/bottom  [Esc/q]exit scroll"
        }
        Focus::Terminal => {
            " [wheel]scroll  [^S]scroll  [^B]sidebar  [Alt+o]log  [^Q]quit  (keys forwarded to container)"
        }
        Focus::Settings => " [↑↓/jk]navigate  [↵/l]select  [^B]back  [^Q]quit",
        Focus::ContainerPicker => " [↑↓/jk]navigate  [↵/l]launch  [^B]back  [^Q]quit",
        Focus::ImageBuild => {
            " [r]run+launch  [c]cancel  [↑↓/jk]navigate  [↵/l]select  [^B]sidebar  [^Q]quit"
        }
        Focus::NewProject => {
            " [↑↓/jk]navigate  [type]edit  [←→]cycle  [↵/l]select  [Esc/^B]back  [^Q]quit"
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(keys, Style::default().fg(Color::DarkGray))),
        area,
    );
}

// ── New project pane ─────────────────────────────────────────────────────────

pub(crate) fn render_new_project(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let Some(state) = app.new_project.as_ref() else {
        render_idle(frame, area);
        return;
    };

    let tone = |c| maybe_dim(c, dimmed);
    let block = Block::default()
        .title(" New Project ")
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::Cyan)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sync_mode = match state.sync_mode {
        crate::config::SyncMode::WorkspaceOnly => "workspace_only",
        crate::config::SyncMode::Pushback => "pushback",
        crate::config::SyncMode::Bidirectional => "bidirectional",
        crate::config::SyncMode::Pullthrough => "pullthrough",
        crate::config::SyncMode::Direct => "direct",
    };

    let rows: [(&str, String); 6] = [
        ("Project name", state.name.clone()),
        ("Canonical dir", state.canonical_dir.clone()),
        ("Sync mode", sync_mode.to_string()),
        (
            "Project type",
            state.project_type.display_name().to_string(),
        ),
        ("Create", "Add project + write rules".to_string()),
        ("Cancel", "Back to sidebar".to_string()),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Config: {}", app.loaded_config_path.display()),
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(Span::styled(
            "  Writes canonical/void-rules.toml only if it does not exist.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Fields",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (i, (label, value)) in rows.iter().enumerate() {
        let selected = i == state.cursor;
        let marker = if selected { "▶ " } else { "  " };
        let label_style = if selected {
            Style::default()
                .fg(tone(Color::White))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone(Color::White))
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {marker}"),
                Style::default().fg(tone(Color::Cyan)),
            ),
            Span::styled(
                format!("{label}: "),
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(value.clone(), label_style),
        ]));
    }

    if let Some(err) = state.error.as_ref() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default()
                .fg(tone(Color::Red))
                .add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Esc] Cancel  [Enter] Create/Select  [←→] Cycle lists",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub(crate) fn render_new_project_preview(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let tone = |c| maybe_dim(c, dimmed);
    let block = Block::default()
        .title(" New Project ")
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::DarkGray)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sync_mode = match cfg.defaults.sync.mode {
        crate::config::SyncMode::WorkspaceOnly => "workspace_only",
        crate::config::SyncMode::Pushback => "pushback",
        crate::config::SyncMode::Bidirectional => "bidirectional",
        crate::config::SyncMode::Pullthrough => "pullthrough",
        crate::config::SyncMode::Direct => "direct",
    };

    let rows: [(&str, &str); 6] = [
        ("Project name", "<empty>"),
        ("Canonical dir", "<empty>"),
        ("Sync mode", sync_mode),
        (
            "Project type",
            crate::new_project::ProjectType::None.display_name(),
        ),
        ("Create", "Add project + write rules"),
        ("Cancel", "Back to sidebar"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Config: {}", app.loaded_config_path.display()),
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(Span::styled(
            "  Press [Enter] to open the form in edit mode.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Fields",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (label, value) in rows {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default().fg(tone(Color::Cyan))),
            Span::styled(
                format!("{label}: "),
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(value.to_string(), Style::default().fg(tone(Color::White))),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Container picker pane ─────────────────────────────────────────────────────

```

## src/tui/render/root.rs

```rs
use super::*;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::{App, Focus, SidebarItem};

const LOG_HEIGHT: u16 = 6;
const STATUS_HEIGHT: u16 = 1;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if app.log_fullscreen {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(STATUS_HEIGHT)])
            .split(area);
        render_log_fullscreen(frame, app, split[0]);
        render_status_bar_log(frame, app, split[1]);
        return;
    }

    if app.terminal_fullscreen {
        if let Some(si) = app.active_session.filter(|&si| si < app.sessions.len()) {
            let has_modal = !app.pending_net.is_empty()
                || app
                    .active_session
                    .map(|active| !app.pending_for_session(active).is_empty())
                    .unwrap_or(false);

            render_terminal(frame, app, area, si, has_modal, true);
            render_terminal_overlays(frame, app, area, si);
        } else {
            render_idle(frame, area);
        }
        return;
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(LOG_HEIGHT),
            Constraint::Length(STATUS_HEIGHT),
        ])
        .split(area);

    let main_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.config.get().defaults.ui.sidebar_width.max(1)),
            Constraint::Min(0),
        ])
        .split(outer[0]);

    render_sidebar(frame, app, main_row[0]);
    render_right_pane(frame, app, main_row[1]);
    render_log(frame, app, outer[1]);
    render_status_bar(frame, app, outer[2]);
}

pub fn render_scrollbar(
    frame: &mut Frame,
    track_area: Rect,
    max_scroll: usize,
    current_scroll: usize,
    invert: bool,
) {
    if max_scroll == 0 || track_area.height == 0 {
        return;
    }
    let track_h = track_area.height as usize;
    let total_content = max_scroll + track_h;
    let thumb_size = (track_h * track_h / total_content).max(1);
    let track_range = track_h.saturating_sub(thumb_size);
    let thumb_top = if invert {
        track_range.saturating_sub(current_scroll * track_range / max_scroll)
    } else {
        current_scroll * track_range / max_scroll
    };
    let x = track_area.right().saturating_sub(1);
    for row in 0..track_h {
        let in_thumb = row >= thumb_top && row < thumb_top + thumb_size;
        let (ch, style) = if in_thumb {
            ("┃", Style::default().fg(Color::Yellow))
        } else {
            ("│", Style::default().fg(Color::DarkGray))
        };
        let bar_area = Rect::new(x, track_area.y + row as u16, 1, 1);
        frame.render_widget(Span::styled(ch, style), bar_area);
    }
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

pub(crate) fn render_sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Sidebar;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Projects ")
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);

    let items = app.sidebar_items();
    if !items.is_empty() {
        let visible = area.height.saturating_sub(2).max(1) as usize;
        let selected = app.sidebar_idx.min(items.len().saturating_sub(1));
        let max_offset = items.len().saturating_sub(visible);

        let mut offset = app.sidebar_offset.min(max_offset);
        if selected < offset {
            offset = selected;
        } else if selected >= offset.saturating_add(visible) {
            offset = selected.saturating_add(1).saturating_sub(visible);
        }
        app.sidebar_offset = offset.min(max_offset);
    } else {
        app.sidebar_offset = 0;
    }
    let cfg = app.config.get();
    let visible = area.height.saturating_sub(2).max(1) as usize;
    let offset = app.sidebar_offset.min(items.len());
    let list_items: Vec<ListItem> = items
        .iter()
        .skip(offset)
        .take(visible)
        .map(|item| match item {
            SidebarItem::Project(pi) => {
                let proj = &app.projects[*pi];
                let sync_suffix = match &proj.last_report {
                    Some(r) => format!(" {}", r.timestamp.format("%H:%M")),
                    None => String::new(),
                };
                let is_direct = cfg
                    .projects
                    .get(*pi)
                    .map(|p| {
                        crate::config::effective_sync_mode(p, &cfg.defaults)
                            == crate::config::SyncMode::Direct
                    })
                    .unwrap_or(false);
                let (dot, dot_color) = if is_direct {
                    ("●", Color::Green)
                } else {
                    match app.project_watch_spinner(*pi) {
                        Some(frame) => (frame, Color::Green),
                        None => ("○", Color::DarkGray),
                    }
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{dot} "), Style::default().fg(dot_color)),
                    Span::styled(
                        proj.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(sync_suffix, Style::default().fg(Color::DarkGray)),
                ]))
            }
            SidebarItem::Session(si) => {
                let session = &app.sessions[*si];
                let is_active = app.active_session == Some(*si);
                let (prefix, name_color) = if session.is_exited() {
                    ("  ✗ ", Color::DarkGray)
                } else if is_active {
                    ("  ▶ ", Color::Cyan)
                } else {
                    ("  · ", Color::White)
                };
                let bell = session.has_bell();
                let short_id: String = session.docker_name.chars().take(12).collect();
                let mut spans = vec![
                    Span::styled(prefix, Style::default().fg(name_color)),
                    Span::styled(
                        session.container_name.clone(),
                        Style::default().fg(name_color),
                    ),
                    Span::styled(format!(" {short_id}"), Style::default().fg(Color::DarkGray)),
                ];
                if bell {
                    spans.push(Span::styled(
                        " [!]",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                ListItem::new(Line::from(spans))
            }
            SidebarItem::Settings(_) => ListItem::new(Line::from(vec![
                Span::styled("  ⚙ ", Style::default().fg(Color::Yellow)),
                Span::styled("Settings", Style::default().fg(Color::DarkGray)),
            ])),
            SidebarItem::Launch(_) => ListItem::new(Line::from(vec![
                Span::styled("  + ", Style::default().fg(Color::Green)),
                Span::styled("Run Container...", Style::default().fg(Color::DarkGray)),
            ])),
            SidebarItem::Build(_) => {
                let image = app
                    .build_container_idx
                    .and_then(|idx| cfg.containers.get(idx))
                    .map(|c| c.image.as_str())
                    .unwrap_or("<unknown>");
                let marker = if app.build_is_running() {
                    loading_spinner_frame()
                } else {
                    "$"
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {marker} "), Style::default().fg(Color::Yellow)),
                    Span::styled("docker build", Style::default().fg(Color::Yellow)),
                    Span::styled(format!("  {image}"), Style::default().fg(Color::DarkGray)),
                ]))
            }
            SidebarItem::NewProject => ListItem::new(Line::from(vec![
                Span::styled("+ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "New Project...",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])),
        })
        .collect();

    let mut list_state = ListState::default();
    if !items.is_empty() {
        let selected = app.sidebar_idx.min(items.len().saturating_sub(1));
        // Project rows are non-selectable section headers. If the app state ever points at one
        // (e.g. via older persisted state), render with no highlight.
        if matches!(items.get(selected), Some(SidebarItem::Project(_))) {
            list_state.select(None);
        } else {
            let rel_selected = selected.saturating_sub(offset);
            list_state.select(Some(rel_selected.min(list_items.len().saturating_sub(1))));
        }
    }

    frame.render_stateful_widget(
        List::new(list_items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(""),
        area,
        &mut list_state,
    );

    if items.len() > visible && inner.height > 0 {
        let max_offset = items.len().saturating_sub(visible).max(1);
        let offset = app.sidebar_offset.min(max_offset);
        render_scrollbar(frame, inner, max_offset, offset, false);
    }
}

// ── Right pane ────────────────────────────────────────────────────────────────

```

## src/tui/render/sidebar.rs

```rs
use super::*;

pub(crate) fn render_right_pane(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Sidebar {
        let selected = app.sidebar_items().get(app.sidebar_idx).cloned();
        match selected {
            Some(SidebarItem::Session(si)) if si < app.sessions.len() => {
                let has_modal =
                    !app.pending_for_session(si).is_empty() || !app.pending_net.is_empty();
                // Sidebar-selected session is a preview, so keep it visually
                // muted even when no modal is active.
                let preview_dimmed = true;
                render_terminal(frame, app, area, si, preview_dimmed || has_modal, false);
                render_terminal_overlays(frame, app, area, si);
            }
            Some(SidebarItem::Settings(pi)) => {
                render_project_settings(frame, app, area, pi, true);
            }
            Some(SidebarItem::Launch(_)) => {
                render_container_picker(frame, app, area, true);
            }
            Some(SidebarItem::Build(_))
                if app.build_is_running() && build_output_is_selected(app) =>
            {
                render_build_output(frame, app, area, true);
            }
            Some(SidebarItem::NewProject) => {
                if app.new_project.is_some() {
                    render_new_project(frame, app, area, true);
                } else {
                    render_new_project_preview(frame, app, area, true);
                }
            }
            _ => render_idle(frame, area),
        }
        return;
    }

    if app.focus == Focus::Settings {
        let pi = app
            .active_settings_project
            .or_else(|| app.selected_project_idx())
            .unwrap_or(0);
        render_project_settings(frame, app, area, pi, false);
        return;
    }

    if app.focus == Focus::ContainerPicker {
        render_container_picker(frame, app, area, false);
        return;
    }

    if app.build_is_running() && build_output_is_selected(app) {
        render_build_output(frame, app, area, false);
        return;
    }

    if app.focus == Focus::ImageBuild {
        render_image_build(frame, app, area, false);
        return;
    }

    if app.focus == Focus::NewProject {
        render_new_project(frame, app, area, false);
        return;
    }

    let has_modal = app
        .active_session
        .map(|si| !app.pending_for_session(si).is_empty() || !app.pending_net.is_empty())
        .unwrap_or(false);

    match app.active_session {
        Some(si) if si < app.sessions.len() => {
            render_terminal(frame, app, area, si, has_modal, false);
            render_terminal_overlays(frame, app, area, si);
        }
        _ => render_idle(frame, area),
    }
}

pub(crate) fn render_terminal_overlays(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    session_idx: usize,
) {
    let pending_exec = app.pending_for_session(session_idx);
    if !pending_exec.is_empty() {
        render_exec_approval_overlay(frame, app, area, pending_exec[0]);
        return;
    }

    if !app.pending_net.is_empty() {
        render_net_approval_overlay(frame, app, area);
    }
}

pub(crate) fn build_output_is_selected(app: &App) -> bool {
    matches!(
        app.sidebar_items().get(app.sidebar_idx),
        Some(SidebarItem::Build(_))
    )
}

// ── Idle screen ───────────────────────────────────────────────────────────────

pub(crate) fn render_idle(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Select a project and press [↵] to launch a container.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Select a running container and press [↵] to attach.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

pub(crate) fn render_project_settings(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    project_idx: usize,
    dimmed: bool,
) {
    let cfg = app.config.get();
    let Some(proj) = cfg.projects.get(project_idx) else {
        render_idle(frame, area);
        return;
    };

    let workspace_path = crate::config::effective_workspace_path(proj, &cfg.workspace);
    let mode = crate::config::effective_sync_mode(proj, &cfg.defaults);
    let watching = app.is_project_watching(project_idx);
    let tone = |c| maybe_dim(c, dimmed);
    let focused = app.focus == Focus::Settings;
    let border_style = if focused {
        Style::default().fg(tone(Color::Cyan))
    } else {
        Style::default().fg(tone(Color::DarkGray))
    };
    let block = Block::default()
        .title(format!(" {} Settings ", proj.name))
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(border_style);

    let actions = App::settings_action_rows_for(mode.clone(), watching);
    let cursor = if actions.is_empty() {
        0
    } else {
        app.settings_cursor.min(actions.len().saturating_sub(1))
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Canonical repo: ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                proj.canonical_path.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Workspace dir : ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                workspace_path.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Sync mode     : ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(mode.to_string(), Style::default().fg(tone(Color::White))),
        ]),
        Line::from(vec![
            Span::styled(
                "  File watch    : ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                if watching { "enabled" } else { "disabled" },
                Style::default().fg(if watching {
                    tone(Color::Green)
                } else {
                    tone(Color::DarkGray)
                }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (i, action) in actions.iter().enumerate() {
        let selected = focused && i == cursor;
        let marker = if selected { "▶ " } else { "  " };
        let name_style = if selected {
            Style::default()
                .fg(tone(Color::White))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone(Color::White))
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {marker}"),
                Style::default().fg(tone(Color::Cyan)),
            ),
            Span::styled(format!("[{}] {}", action.key, action.label), name_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("      {}", action.desc),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

    let rules_path = proj.canonical_path.join("void-rules.toml");
    let rules_status: Vec<Span> = if !rules_path.exists() {
        vec![
            Span::styled(
                "  void-rules.toml: ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled("Not Found", Style::default().fg(tone(Color::Yellow))),
        ]
    } else {
        match crate::rules::load(&rules_path) {
            Ok(r) => vec![
                Span::styled(
                    "  void-rules.toml: ",
                    Style::default().fg(tone(Color::DarkGray)),
                ),
                Span::styled("Loaded", Style::default().fg(tone(Color::Green))),
                Span::styled(
                    format!(
                        "  hostdo: {}, network: {}",
                        r.hostdo.commands.len(),
                        r.network.rules.len()
                    ),
                    Style::default().fg(tone(Color::White)),
                ),
            ],
            Err(_) => vec![
                Span::styled(
                    "  void-rules.toml: ",
                    Style::default().fg(tone(Color::DarkGray)),
                ),
                Span::styled("Error", Style::default().fg(tone(Color::Red))),
            ],
        }
    };

    lines.push(Line::from(""));
    lines.push(Line::from(rules_status));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [^B] Back to sidebar",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Terminal view ─────────────────────────────────────────────────────────────

pub(crate) fn render_terminal_fullscreen_header(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    title_style: Style,
) {
    let exit_hint = " CTRL+G to exit ";
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(exit_hint.len() as u16),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(title.to_string(), title_style)),
        split[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            exit_hint,
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right),
        split[1],
    );
}

pub(crate) fn render_terminal_title_hint(frame: &mut Frame, area: Rect) {
    let hint = " CTRL+G to fullscreen";
    let hint_width = hint.len() as u16;
    if area.width <= hint_width {
        return;
    }
    let hint_area = Rect::new(area.x + area.width - hint_width, area.y, hint_width, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray)))
            .alignment(Alignment::Right),
        hint_area,
    );
}

```

## src/tui/render/terminal.rs

```rs
use super::*;

pub(crate) fn render_terminal(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    session_idx: usize,
    dimmed: bool,
    fullscreen: bool,
) {
    let (term, container_id, tab_label, session_exited) = match app.sessions.get(session_idx) {
        Some(s) => (
            std::sync::Arc::clone(&s.term),
            s.container_id.clone(),
            s.tab_label(),
            s.is_exited(),
        ),
        None => return,
    };

    let focused = app.focus == Focus::Terminal;
    let in_scroll_mode = focused && app.scroll_mode;
    let border_style = if in_scroll_mode {
        Style::default().fg(Color::Yellow)
    } else if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let short_id = if container_id.len() > 12 {
        &container_id[..12]
    } else {
        &container_id
    };
    let tab_title = if in_scroll_mode {
        format!(" {} [{}] -- SCROLL -- ", tab_label, short_id)
    } else {
        format!(" {} [{}] ", tab_label, short_id)
    };
    let title_style = if in_scroll_mode {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    };
    let content_area = if fullscreen {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        render_terminal_fullscreen_header(frame, split[0], tab_title.as_str(), title_style);
        split[1]
    } else {
        area
    };

    if content_area.height == 0 || content_area.width == 0 {
        return;
    }

    let block = if fullscreen {
        Block::default()
    } else {
        Block::default()
            .title(tab_title.as_str())
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_style(border_style)
    };

    let inner = if fullscreen {
        content_area
    } else {
        block.inner(content_area)
    };
    frame.render_widget(block, content_area);
    if focused && !fullscreen {
        render_terminal_title_hint(frame, content_area);
    }

    if let Some(session) = app.sessions.get_mut(session_idx) {
        let _ = session.resize(inner.height, inner.width);
    }

    let mut term = term.lock();
    if !session_exited && !term_has_content(&term) {
        let spinner = loading_spinner_frame();
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{spinner} Starting container..."),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Waiting for terminal output",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
        return;
    }

    let desired_offset = if app.scroll_mode {
        app.terminal_scroll
    } else {
        0
    };
    let max_scrollback = term.history_size();
    let desired_offset = desired_offset.min(max_scrollback);
    let current_offset = term.grid().display_offset();
    if desired_offset != current_offset {
        let delta = desired_offset as i32 - current_offset as i32;
        term.scroll_display(Scroll::Delta(delta));
    }
    let actual_scroll = term.grid().display_offset();

    let rows = inner.height as usize;
    let cols = inner.width as usize;
    let mut content = term.renderable_content();

    let default_fg = resolve_ansi_color(AnsiColor::Named(NamedColor::Foreground), content.colors);
    let default_bg = resolve_ansi_color(AnsiColor::Named(NamedColor::Background), content.colors);
    let mut default_style = Style::default().fg(default_fg).bg(default_bg);
    if dimmed {
        if let Some(fg) = default_style.fg {
            default_style = default_style.fg(attenuate_color(fg));
        }
        if let Some(bg) = default_style.bg {
            default_style = default_style.bg(attenuate_color(bg));
        }
    }

    let cursor_point = content.cursor.point;
    let show_cursor = focused
        && !dimmed
        && actual_scroll == 0
        && content
            .mode
            .contains(alacritty_terminal::term::TermMode::SHOW_CURSOR);

    #[derive(Clone)]
    struct CellOut {
        ch: char,
        style: Style,
        skip: bool,
    }

    let mut grid: Vec<CellOut> = vec![
        CellOut {
            ch: ' ',
            style: default_style,
            skip: false,
        };
        rows * cols
    ];

    for indexed in content.display_iter.by_ref() {
        let Some(vp) =
            alacritty_terminal::term::point_to_viewport(content.display_offset, indexed.point)
        else {
            continue;
        };
        let row = vp.line;
        let col = vp.column.0;
        if col >= cols {
            continue;
        }
        let row_offset = term.screen_lines().saturating_sub(rows);
        if row < row_offset || row >= row_offset + rows {
            continue;
        }
        let rr = row - row_offset;
        let idx = rr * cols + col;

        let cell = indexed.cell;
        let mut ch = cell.c;
        let skip = cell.flags.contains(TermFlags::WIDE_CHAR_SPACER);
        if cell.flags.contains(TermFlags::HIDDEN) {
            ch = ' ';
        }

        let mut fg_src = cell.fg;
        let bg_src = cell.bg;
        let missing_default_palette = content.colors[NamedColor::Foreground].is_none()
            && content.colors[NamedColor::Background].is_none();
        if missing_default_palette
            && matches!(fg_src, AnsiColor::Spec(Rgb { r: 0, g: 0, b: 0 }))
            && matches!(bg_src, AnsiColor::Named(NamedColor::Background))
            && cell.flags.contains(TermFlags::BOLD)
        {
            fg_src = AnsiColor::Named(NamedColor::Foreground);
        }
        if cell.flags.contains(TermFlags::BOLD)
            && !cell.flags.contains(TermFlags::DIM)
            && !cell.flags.contains(TermFlags::DIM_BOLD)
        {
            fg_src = brighten_bold_ansi_color(fg_src);
        }

        let mut fg = resolve_ansi_color(fg_src, content.colors);
        let mut bg = resolve_ansi_color(bg_src, content.colors);
        if cell.flags.contains(TermFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        let mut style = Style::default().fg(fg).bg(bg);
        if cell.flags.contains(TermFlags::BOLD) {
            style = style.add_modifier(Modifier::BOLD);
        }
        if cell.flags.contains(TermFlags::ITALIC) {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if cell.flags.contains(TermFlags::ALL_UNDERLINES) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if cell.flags.contains(TermFlags::STRIKEOUT) {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if cell.flags.contains(TermFlags::DIM) || cell.flags.contains(TermFlags::DIM_BOLD) {
            style = style.add_modifier(Modifier::DIM);
            if let Some(fg) = style.fg {
                style = style.fg(attenuate_color(fg));
            }
            if let Some(bg) = style.bg {
                style = style.bg(attenuate_color(bg));
            }
        }
        if dimmed {
            if let Some(fg) = style.fg {
                style = style.fg(attenuate_color(fg));
            }
            if let Some(bg) = style.bg {
                style = style.bg(attenuate_color(bg));
            }
        }

        if show_cursor && indexed.point == cursor_point && rr < rows && col < cols {
            style = style.add_modifier(Modifier::REVERSED);
        }

        grid[idx] = CellOut { ch, style, skip };
    }

    let mut rendered: Vec<Line> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut spans: Vec<Span> = Vec::new();
        let mut cur_style: Option<Style> = None;
        let mut cur_text = String::new();
        for c in 0..cols {
            let cell = &grid[r * cols + c];
            if cell.skip {
                continue;
            }
            if cur_style == Some(cell.style) {
                cur_text.push(cell.ch);
            } else {
                if let Some(style) = cur_style.take() {
                    spans.push(Span::styled(std::mem::take(&mut cur_text), style));
                }
                cur_style = Some(cell.style);
                cur_text.push(cell.ch);
            }
        }
        if let Some(style) = cur_style.take() {
            spans.push(Span::styled(cur_text, style));
        }
        rendered.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(rendered), inner);

    if app.scroll_mode && max_scrollback > 0 {
        render_scrollbar(frame, inner, max_scrollback, actual_scroll, true);
    }
}

pub(crate) fn resolve_ansi_color(
    color: AnsiColor,
    colors: &alacritty_terminal::term::color::Colors,
) -> Color {
    match color {
        AnsiColor::Spec(Rgb { r, g, b }) => Color::Rgb(r, g, b),
        AnsiColor::Named(named) => {
            if let Some(rgb) = colors[named] {
                if matches!(named, NamedColor::Foreground | NamedColor::BrightForeground) {
                    let fg_is_blackish = rgb.r <= 0x10 && rgb.g <= 0x10 && rgb.b <= 0x10;
                    let bg_is_blackish = colors[NamedColor::Background]
                        .map(|bg| bg.r <= 0x10 && bg.g <= 0x10 && bg.b <= 0x10)
                        .unwrap_or(true);
                    if fg_is_blackish && bg_is_blackish {
                        return Color::Rgb(0xff, 0xff, 0xff);
                    }
                }
                return Color::Rgb(rgb.r, rgb.g, rgb.b);
            }
            match named {
                NamedColor::Foreground => Color::White,
                NamedColor::Background => Color::Black,
                NamedColor::BrightForeground => Color::White,
                NamedColor::DimForeground => Color::Gray,
                _ => Color::Reset,
            }
        }
        AnsiColor::Indexed(idx) => {
            if let Some(rgb) = colors[idx as usize] {
                return Color::Rgb(rgb.r, rgb.g, rgb.b);
            }
            let (r, g, b) = xterm_256_to_rgb(idx);
            Color::Rgb(r, g, b)
        }
    }
}

pub(crate) fn brighten_bold_ansi_color(color: AnsiColor) -> AnsiColor {
    match color {
        AnsiColor::Named(named) => AnsiColor::Named(match named {
            NamedColor::Black => NamedColor::BrightBlack,
            NamedColor::Red => NamedColor::BrightRed,
            NamedColor::Green => NamedColor::BrightGreen,
            NamedColor::Yellow => NamedColor::BrightYellow,
            NamedColor::Blue => NamedColor::BrightBlue,
            NamedColor::Magenta => NamedColor::BrightMagenta,
            NamedColor::Cyan => NamedColor::BrightCyan,
            NamedColor::White => NamedColor::BrightWhite,
            other => other,
        }),
        AnsiColor::Indexed(idx) if idx <= 7 => AnsiColor::Indexed(idx + 8),
        other => other,
    }
}

pub(crate) fn xterm_256_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ansi_16_to_rgb(idx),
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i / 6) % 6;
            let b = i % 6;
            (color_cube(r), color_cube(g), color_cube(b))
        }
        232..=255 => {
            let shade = 8 + (idx - 232) * 10;
            (shade, shade, shade)
        }
    }
}

pub(crate) fn ansi_16_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0x00, 0x00, 0x00),
        1 => (0xcd, 0x00, 0x00),
        2 => (0x00, 0xcd, 0x00),
        3 => (0xcd, 0xcd, 0x00),
        4 => (0x00, 0x00, 0xee),
        5 => (0xcd, 0x00, 0xcd),
        6 => (0x00, 0xcd, 0xcd),
        7 => (0xe5, 0xe5, 0xe5),
        8 => (0xb0, 0xb0, 0xb0),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x5c, 0x5c, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        _ => (0xff, 0xff, 0xff),
    }
}

pub(crate) fn color_cube(v: u8) -> u8 {
    match v {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

pub(crate) fn term_has_content<T: alacritty_terminal::event::EventListener>(
    term: &alacritty_terminal::term::Term<T>,
) -> bool {
    let content = term.renderable_content();
    for indexed in content.display_iter {
        let ch = indexed.cell.c;
        if !ch.is_whitespace() {
            return true;
        }
    }
    false
}

pub(crate) fn loading_spinner_frame() -> &'static str {
    const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as usize)
        .unwrap_or(0);
    FRAMES[(ms / 120) % FRAMES.len()]
}

pub(crate) fn attenuate_color(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(scale_channel(r), scale_channel(g), scale_channel(b)),
        Color::Black => Color::Black,
        Color::Red => Color::DarkGray,
        Color::Green => Color::DarkGray,
        Color::Yellow => Color::DarkGray,
        Color::Blue => Color::DarkGray,
        Color::Magenta => Color::DarkGray,
        Color::Cyan => Color::DarkGray,
        Color::Gray => Color::DarkGray,
        Color::DarkGray => Color::DarkGray,
        Color::LightRed => Color::DarkGray,
        Color::LightGreen => Color::DarkGray,
        Color::LightYellow => Color::DarkGray,
        Color::LightBlue => Color::DarkGray,
        Color::LightMagenta => Color::DarkGray,
        Color::LightCyan => Color::DarkGray,
        Color::White => Color::Gray,
        Color::Indexed(n) => {
            if n >= 8 {
                Color::DarkGray
            } else {
                Color::Indexed(n)
            }
        }
        Color::Reset => Color::Reset,
    }
}

pub(crate) fn scale_channel(v: u8) -> u8 {
    ((v as f32) * 0.45).round() as u8
}

pub(crate) fn maybe_dim(color: Color, dimmed: bool) -> Color {
    if dimmed {
        attenuate_color(color)
    } else {
        color
    }
}

```

## src/tui/tests.rs

```rs
use super::{App, Focus, SidebarItem, restore_terminal_output};
use crate::ca::CaStore;
use crate::config::Config;
use crate::proxy::ProxyState;
use crate::shared_config::SharedConfig;
use crate::state::StateManager;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::sync::Arc;
use tokio::sync::mpsc;

#[test]
fn restore_terminal_output_emits_reset_sequences() {
    let mut buf = Vec::new();
    restore_terminal_output(&mut buf).expect("restore commands should serialize");
    let out = String::from_utf8_lossy(&buf);
    assert!(out.contains("\u{1b}[?1049l"), "missing leave alt-screen");
    assert!(out.contains("\u{1b}[?25h"), "missing show cursor");
    assert!(out.contains("\u{1b}[?1000l"), "missing disable mouse");
    assert!(
        out.contains("\u{1b}[?2004l"),
        "missing disable bracketed paste"
    );
    assert!(out.contains("\u{1b}[?7h"), "missing enable line wrap");
    assert!(out.contains("\u{1b}[0m"), "missing reset color");
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("void-claw-{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn encode_sgr_mouse_click_down_left() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<0;1;1M");
}

#[test]
fn encode_sgr_mouse_click_up_left() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 2,
        row: 3,
        modifiers: KeyModifiers::empty(),
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<0;3;4m");
}

#[test]
fn encode_sgr_mouse_drag_left() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 9,
        row: 8,
        modifiers: KeyModifiers::empty(),
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<32;10;9M");
}

#[test]
fn encode_sgr_mouse_scroll_down_with_shift() {
    let mouse = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 4,
        row: 5,
        modifiers: KeyModifiers::SHIFT,
    };
    let bytes = super::app::encode_sgr_mouse(mouse).expect("encodes");
    assert_eq!(String::from_utf8_lossy(&bytes), "\u{1b}[<69;5;6M");
}

#[test]
fn encode_sgr_mouse_ignores_move() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::empty(),
    };
    assert!(super::app::encode_sgr_mouse(mouse).is_none());
}

fn build_test_app() -> App {
    let root = unique_temp_dir("tui-build-flow");
    let global_rules_file = root.join("global-rules.toml");
    let workspace_root = root.join("workspace");
    let docker_dir = root.join("docker-root");
    let project_path = root.join("project-a");
    std::fs::create_dir_all(&workspace_root).expect("create workspace");
    std::fs::create_dir_all(&docker_dir).expect("create docker dir");
    std::fs::create_dir_all(&project_path).expect("create project path");

    let raw = format!(
        r#"
[manager]
global_rules_file = "{}"

[workspace]
root = "{}"

docker_dir = "{}"

[[projects]]
name = "project-a"
canonical_path = "{}"

[[containers]]
name = "test"
image = "missing-image:latest"
"#,
        global_rules_file.display(),
        workspace_root.display(),
        docker_dir.display(),
        project_path.display()
    );
    let config: Config = toml::from_str(&raw).expect("parse minimal config");
    let shared = SharedConfig::new(Arc::new(config));

    let (_exec_tx, exec_rx) = mpsc::channel(8);
    let (_stop_tx, stop_rx) = mpsc::channel(8);
    let (net_tx, net_rx) = mpsc::channel(8);
    let (_audit_tx, audit_rx) = mpsc::channel(8);

    let ca = Arc::new(CaStore::load_or_create(&root.join("ca")).expect("create CA"));
    let proxy_state = ProxyState::new(ca, shared.clone(), net_tx).expect("proxy state");
    let state = StateManager::open(&root.join("state")).expect("state manager");

    App::new(
        shared,
        root.join("config.toml"),
        "token".to_string(),
        crate::server::SessionRegistry::default(),
        exec_rx,
        stop_rx,
        net_rx,
        audit_rx,
        state,
        proxy_state,
        "127.0.0.1:0".to_string(),
        root.join("ca/ca.crt").display().to_string(),
    )
    .expect("App::new")
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn build_commands_use_configured_docker_root() {
    let docker_dir = std::path::Path::new("/tmp/void-claw-docker-root");
    let (base_cmd, agent_cmd) =
        App::build_commands_for(docker_dir, "void-claw-codex:ubuntu-24.04");

    assert_eq!(
        base_cmd,
        vec![
            "build".to_string(),
            "-t".to_string(),
            "my-agent:ubuntu-24.04".to_string(),
            "-f".to_string(),
            "/tmp/void-claw-docker-root/ubuntu-24.04.Dockerfile".to_string(),
            "/tmp/void-claw-docker-root".to_string(),
        ]
    );
    assert_eq!(
        agent_cmd,
        Some(vec![
            "build".to_string(),
            "-t".to_string(),
            "void-claw-codex:ubuntu-24.04".to_string(),
            "-f".to_string(),
            "/tmp/void-claw-docker-root/codex/ubuntu-24.04.Dockerfile".to_string(),
            "/tmp/void-claw-docker-root".to_string(),
        ])
    );
}

#[test]
fn preflight_missing_image_opens_image_build_pane() {
    let mut app = build_test_app();
    let proceed = app.preflight_image_or_prompt_build(0, 0, "missing-image:latest", |_| Ok(false));
    assert!(!proceed);
    assert_eq!(app.focus, Focus::ImageBuild);
    assert_eq!(app.build_project_idx, Some(0));
    assert_eq!(app.build_container_idx, Some(0));
    assert_eq!(app.build_cursor, 0);
}

#[test]
fn sidebar_selection_tracks_session_preview() {
    let mut app = build_test_app();
    let items = vec![
        SidebarItem::Project(0),
        SidebarItem::Launch(0),
        SidebarItem::Session(2),
    ];

    app.sidebar_idx = 2;
    app.update_sidebar_preview(&items);
    assert_eq!(app.preview_session, Some(2));

    app.sidebar_idx = 1;
    app.update_sidebar_preview(&items);
    assert_eq!(app.preview_session, None);
}

#[test]
fn ctrl_g_toggles_terminal_fullscreen() {
    let mut app = build_test_app();
    app.focus = Focus::Terminal;
    app.active_session = Some(0);

    app.handle_terminal_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert!(app.terminal_fullscreen);
    assert!(!app.log_fullscreen);

    app.handle_terminal_key(key(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert!(!app.terminal_fullscreen);
}

#[test]
fn double_escape_exits_terminal_fullscreen() {
    let mut app = build_test_app();
    app.focus = Focus::Terminal;
    app.active_session = Some(0);
    app.terminal_fullscreen = true;

    app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.terminal_fullscreen);

    app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.terminal_fullscreen);
}

#[test]
fn double_escape_quits_when_not_fullscreen() {
    let mut app = build_test_app();
    app.focus = Focus::Terminal;
    app.active_session = Some(0);

    app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.should_quit);

    app.handle_terminal_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.should_quit);
}

#[test]
fn removing_active_session_clears_terminal_fullscreen() {
    let mut app = build_test_app();
    app.active_session = Some(0);
    app.terminal_fullscreen = true;
    app.last_terminal_esc = Some(std::time::Instant::now());

    app.clear_terminal_fullscreen_for_removed_session(0);

    assert!(!app.terminal_fullscreen);
    assert!(app.last_terminal_esc.is_none());
}

#[cfg(unix)]
#[test]
fn termios_guard_only_restores_ixon() {
    use super::disable_xon_xoff_on_fd;

    fn get_termios(fd: i32) -> libc::termios {
        unsafe {
            let mut t = std::mem::MaybeUninit::<libc::termios>::uninit();
            assert_eq!(libc::tcgetattr(fd, t.as_mut_ptr()), 0);
            t.assume_init()
        }
    }

    fn set_termios(fd: i32, t: &libc::termios) {
        unsafe {
            assert_eq!(libc::tcsetattr(fd, libc::TCSANOW, t), 0);
        }
    }

    unsafe {
        let mut master: libc::c_int = 0;
        let mut slave: libc::c_int = 0;
        assert_eq!(
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut()
            ),
            0
        );

        // Ensure IXON is enabled so we can observe disable+restore.
        let mut t0 = get_termios(slave);
        t0.c_iflag |= libc::IXON;
        set_termios(slave, &t0);

        let echo_was_enabled = (t0.c_lflag & libc::ECHO) != 0;
        let expected_echo_enabled = !echo_was_enabled;

        {
            let _guard = disable_xon_xoff_on_fd(slave).expect("guard should be created for PTY");
            let t_mid = get_termios(slave);
            assert_eq!((t_mid.c_iflag & libc::IXON) != 0, false);

            // Mutate an unrelated bit while guard is alive; the guard must not
            // overwrite it on drop.
            let mut t1 = t_mid;
            if echo_was_enabled {
                t1.c_lflag &= !libc::ECHO;
            } else {
                t1.c_lflag |= libc::ECHO;
            }
            set_termios(slave, &t1);
        }

        let t_after = get_termios(slave);
        assert_eq!((t_after.c_iflag & libc::IXON) != 0, true);
        assert_eq!(
            (t_after.c_lflag & libc::ECHO) != 0,
            expected_echo_enabled,
            "TermiosGuard must not restore unrelated flags like ECHO"
        );

        let _ = libc::close(master);
        let _ = libc::close(slave);
    }
}

#[test]
fn sidebar_navigation_wraps_and_scrolls() {
    let mut app = build_test_app();
    // build_test_app only adds 1 project ("project-a")
    // sidebar_items() should return [Project(0), Launch(0), Settings(0), NewProject]

    // Project rows are section headers: they render, but can't be selected/highlighted.
    app.sidebar_idx = 0;

    // Down -> Launch(0)
    app.handle_sidebar_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 1);

    // Up -> Wrap to NewProject (index 3), skipping Project(0)
    app.handle_sidebar_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 3);

    // Up -> Settings(0)
    app.handle_sidebar_key(key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 2);

    // Down -> NewProject
    app.handle_sidebar_key(key(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.sidebar_idx, 3);
}

```

## docker/claude/ubuntu-24.04.Dockerfile

```Dockerfile
# void-claw + Claude Code CLI — Ubuntu 24.04 LTS
#
# Build (from repo root — must have already built the base image):
#   docker build -t void-claw-claude:ubuntu-24.04 -f docker/claude/ubuntu-24.04.Dockerfile .
#
# Or build both in one step:
#   docker build -t my-agent:ubuntu-24.04 -f docker/ubuntu-24.04.Dockerfile . \
#   && docker build -t void-claw-claude:ubuntu-24.04 -f docker/claude/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install Claude Code CLI.

USER ubuntu

RUN curl -fsSL https://claude.ai/install.sh | bash

# Ensure claude is on PATH for all shell types (login, non-login,
# non-interactive scripts).  The installer adds it to .bashrc, but
# that is only sourced by interactive bash shells.
ENV PATH="/home/ubuntu/.local/bin:${PATH}"

CMD ["claude"]

```

## docker/codex/ubuntu-24.04.Dockerfile

```Dockerfile
# void-claw + OpenAI Codex CLI — Ubuntu 24.04 LTS
#
# Build (from repo root — must have already built the base image):
#   docker build -t void-claw-codex:ubuntu-24.04 -f docker/codex/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install Codex plus an explicit arch alias package.
# The generic package provides the `codex` bin, while the alias package
# guarantees the platform payload exists even if optional dependency
# resolution is skipped during global install.
USER root
RUN set -eu; \
    case "$(dpkg --print-architecture)" in \
        amd64) codex_payload_alias="@openai/codex-linux-x64@npm:@openai/codex@linux-x64" ;; \
        arm64) codex_payload_alias="@openai/codex-linux-arm64@npm:@openai/codex@linux-arm64" ;; \
        *) echo "unsupported architecture for Codex" >&2; exit 1 ;; \
    esac; \
    npm install -g @openai/codex "$codex_payload_alias"
USER ubuntu

CMD ["codex"]

```

## docker/gemini/ubuntu-24.04.Dockerfile

```Dockerfile
# void-claw + Google Gemini CLI — Ubuntu 24.04 LTS
#
# Build (from repo root — must have already built the base image):
#   docker build -t void-claw-gemini:ubuntu-24.04 -f docker/gemini/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install Google Gemini CLI.
USER root
RUN npm install -g @google/gemini-cli
USER ubuntu

CMD ["gemini"]

```

## docker/opencode/ubuntu-24.04.Dockerfile

```Dockerfile
# void-claw + opencode — Ubuntu 24.04 LTS
#
# opencode npm package name: verify at https://opencode.ai before building.
#
# Build (from repo root — must have already built the base image):
#   docker build -t void-claw-opencode:ubuntu-24.04 -f docker/opencode/ubuntu-24.04.Dockerfile .

FROM my-agent:ubuntu-24.04

# Install opencode CLI.
USER root
RUN npm install -g opencode-ai
USER ubuntu

CMD ["opencode"]

```

## docker/scripts/hostdo.py

```py
#!/usr/bin/env python3
"""
hostdo — void-claw container-side command bridge (Python implementation).

Routes commands through the void-claw host execution server for policy
enforcement and developer approval.  Requires only the Python 3 standard
library — no third-party packages.

Environment variables:
  VOID_CLAW_URL      Base URL of the void-claw manager (default: http://127.0.0.1:7878)
  VOID_CLAW_TOKEN    Bearer token shown by the void-claw TUI           (required)
  VOID_CLAW_SESSION_TOKEN  Per-session token injected by void-claw     (required)

Exit code mirrors the executed command; exits 1 on infrastructure errors.

Requires Python 3 (stdlib only — no third-party packages).
"""

import json
import os
import sys
import urllib.request
import urllib.error
import urllib.parse

# 6-minute timeout: 5-minute approval window + headroom for slow commands.
_TIMEOUT = 360


def _no_proxy_opener() -> urllib.request.OpenerDirector:
    """
    Return a URL opener that bypasses HTTP_PROXY / HTTPS_PROXY env vars.

    The void-claw control channel must never be routed through the MITM proxy
    that void-claw itself is managing — doing so would create a dependency loop
    and cause the approval request to be intercepted before it reaches the
    manager.
    """
    return urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _default_gateway_ip() -> str:
    """
    Best-effort IPv4 default gateway lookup from /proc/net/route.
    """
    try:
        with open("/proc/net/route", "r", encoding="utf-8") as f:
            next(f, None)  # header
            for line in f:
                cols = line.strip().split()
                if len(cols) < 4:
                    continue
                destination_hex = cols[1]
                gateway_hex = cols[2]
                flags_hex = cols[3]
                if destination_hex != "00000000":
                    continue
                flags = int(flags_hex, 16)
                if (flags & 0x2) == 0:  # RTF_GATEWAY
                    continue
                g = int(gateway_hex, 16)
                octets = [
                    str(g & 0xFF),
                    str((g >> 8) & 0xFF),
                    str((g >> 16) & 0xFF),
                    str((g >> 24) & 0xFF),
                ]
                return ".".join(octets)
    except Exception:
        pass
    return ""


def _candidate_base_urls(base_url: str) -> list[str]:
    """
    Build candidate manager URLs.
    If host.docker.internal is unreachable in this runtime, fallback to the
    container's default gateway IP (and common bridge gateway as last resort).
    """
    parsed = urllib.parse.urlparse(base_url)
    host = parsed.hostname or ""
    port = parsed.port or 80
    scheme = parsed.scheme or "http"

    out = [base_url]
    if host == "host.docker.internal":
        gw = _default_gateway_ip()
        if gw:
            out.append(f"{scheme}://{gw}:{port}")
        # Common Linux default bridge fallback.
        out.append(f"{scheme}://172.17.0.1:{port}")

    # Stable dedupe.
    seen = set()
    uniq = []
    for u in out:
        if u not in seen:
            seen.add(u)
            uniq.append(u)
    return uniq


def main() -> None:
    argv = sys.argv[1:]
    if not argv:
        print("hostdo: no command specified", file=sys.stderr)
        print("usage: hostdo <command> [args...]", file=sys.stderr)
        sys.exit(1)

    base_url = os.environ.get("VOID_CLAW_URL", "http://127.0.0.1:7878").rstrip("/")

    token = os.environ.get("VOID_CLAW_TOKEN", "")
    if not token:
        print("hostdo: VOID_CLAW_TOKEN is not set", file=sys.stderr)
        print("  Set it to the token shown in the void-claw TUI.", file=sys.stderr)
        sys.exit(1)

    session_token = os.environ.get("VOID_CLAW_SESSION_TOKEN", "")
    if not session_token:
        print("hostdo: VOID_CLAW_SESSION_TOKEN is not set", file=sys.stderr)
        print(
            "  This container was likely started with an older void-claw image.",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        cwd = os.getcwd()
    except OSError as exc:
        print(f"hostdo: cannot determine working directory: {exc}", file=sys.stderr)
        sys.exit(1)

    body = json.dumps({
        "argv": argv,
        "cwd": cwd,
    }).encode()

    opener = _no_proxy_opener()

    data = None
    last_err = None
    attempted = []
    for candidate_base in _candidate_base_urls(base_url):
        attempted.append(candidate_base)
        req = urllib.request.Request(
            f"{candidate_base}/exec",
            data=body,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "X-Hostdo-Pid": str(os.getpid()),
                "x-void-claw-session-token": session_token,
            },
            method="POST",
        )
        try:
            with opener.open(req, timeout=_TIMEOUT) as resp:
                data = json.loads(resp.read())
                break
        except urllib.error.HTTPError as exc:
            try:
                err = json.loads(exc.read())
                reason = err.get("reason", str(exc))
            except Exception:
                reason = str(exc)
            print(f"hostdo: denied — {reason}", file=sys.stderr)
            sys.exit(1)
        except urllib.error.URLError as exc:
            last_err = exc
            continue
        except TimeoutError:
            print("hostdo: request timed out (6 minutes)", file=sys.stderr)
            sys.exit(1)

    if data is None:
        reason = getattr(last_err, "reason", last_err)
        print(f"hostdo: request failed: {reason}", file=sys.stderr)
        print(
            "  Is void-claw running? Is VOID_CLAW_URL correct? "
            f"({base_url})",
            file=sys.stderr,
        )
        if len(attempted) > 1:
            print("  Tried endpoints:", file=sys.stderr)
            for u in attempted:
                print(f"    - {u}", file=sys.stderr)
        sys.exit(1)

    stdout: str = data.get("stdout", "")
    stderr: str = data.get("stderr", "")
    exit_code: int = int(data.get("exit_code", 1))

    if stdout:
        sys.stdout.write(stdout)
        sys.stdout.flush()
    if stderr:
        sys.stderr.write(stderr)
        sys.stderr.flush()

    sys.exit(exit_code)


if __name__ == "__main__":
    main()

```

## docker/scripts/killme.py

```py
#!/usr/bin/env python3
"""
killme — void-claw container exit command.

Requests that the void-claw manager stop the current container session.

Environment variables:
  VOID_CLAW_URL      Base URL of the void-claw manager (default: http://127.0.0.1:7878)
  VOID_CLAW_TOKEN    Bearer token shown by the void-claw TUI           (required)
  VOID_CLAW_SESSION_TOKEN  Per-session token injected by void-claw     (required)
"""

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

_TIMEOUT = 30


def _no_proxy_opener() -> urllib.request.OpenerDirector:
    return urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _candidate_base_urls(base_url: str) -> list[str]:
    parsed = urllib.parse.urlparse(base_url)
    host = parsed.hostname or ""
    port = parsed.port or 80
    scheme = parsed.scheme or "http"

    out = [base_url]
    if host == "host.docker.internal":
        out.append(f"{scheme}://172.17.0.1:{port}")

    seen = set()
    uniq = []
    for u in out:
        if u not in seen:
            seen.add(u)
            uniq.append(u)
    return uniq


def main() -> None:
    base_url = os.environ.get("VOID_CLAW_URL", "http://127.0.0.1:7878").rstrip("/")

    token = os.environ.get("VOID_CLAW_TOKEN", "")
    if not token:
        print("killme: VOID_CLAW_TOKEN is not set", file=sys.stderr)
        sys.exit(1)

    session_token = os.environ.get("VOID_CLAW_SESSION_TOKEN", "")
    if not session_token:
        print("killme: VOID_CLAW_SESSION_TOKEN is not set", file=sys.stderr)
        sys.exit(1)

    body = json.dumps({}).encode()

    opener = _no_proxy_opener()
    last_err = None

    for candidate_base in _candidate_base_urls(base_url):
        req = urllib.request.Request(
            f"{candidate_base}/container/stop",
            data=body,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "x-void-claw-session-token": session_token,
            },
            method="POST",
        )
        try:
            with opener.open(req, timeout=_TIMEOUT) as resp:
                data = json.loads(resp.read())
                if data.get("ok"):
                    sys.exit(0)
                print("killme: unexpected response from manager", file=sys.stderr)
                sys.exit(1)
        except urllib.error.HTTPError as exc:
            try:
                err = json.loads(exc.read())
                reason = err.get("reason", str(exc))
            except Exception:
                reason = str(exc)
            print(f"killme: denied — {reason}", file=sys.stderr)
            sys.exit(1)
        except urllib.error.URLError as exc:
            last_err = exc
            continue
        except TimeoutError:
            print("killme: request timed out", file=sys.stderr)
            sys.exit(1)

    reason = getattr(last_err, "reason", last_err)
    print(f"killme: request failed: {reason}", file=sys.stderr)
    print(
        f"  Is void-claw running? Is VOID_CLAW_URL correct? ({base_url})",
        file=sys.stderr,
    )
    sys.exit(1)


if __name__ == "__main__":
    main()

```

## docker/ubuntu-24.04.Dockerfile

```Dockerfile
FROM rust:1.88-slim-bookworm AS tun2proxy-build

ENV GIT_HASH=crates-io

RUN cargo install --locked --bin tun2proxy-bin tun2proxy

# void-claw base — Ubuntu 24.04 LTS (Noble Numbat)
#
# Build (from repo root):
#   docker build -t my-agent:ubuntu-24.04 -f docker/ubuntu-24.04.Dockerfile .
#
# Run command is generated by the void-claw TUI for your project.
#
# ── Network filtering and CA trust ───────────────────────────────────────────
# void-claw routes container traffic through its MITM proxy and injects the CA
# cert path via environment variables at run time.  Most runtimes are covered
# automatically:
#
#   SSL_CERT_FILE / CURL_CA_BUNDLE          — OpenSSL, curl, Go, Ruby, PHP
#   NODE_EXTRA_CA_CERTS / DENO_CERT         — Node.js, npm, yarn, bun, Deno
#   REQUESTS_CA_BUNDLE / AWS_CA_BUNDLE      — Python (requests/httpx/pip), AWS SDK
#   GIT_SSL_CAINFO                          — git over HTTPS
#   GRPC_DEFAULT_SSL_ROOTS_FILE_PATH        — gRPC (all language runtimes)
#
# zc-init.sh also runs update-ca-certificates (when root) to update the system
# store, which covers .NET / C# (HttpClient, RestSharp, etc.).  Omit --user
# from your docker run flags to allow this; with --user 1000:1000 the system
# store update is skipped (env-var-based runtimes still work fine).
#
# Java / JVM (OkHttp, Apache HttpClient, java.net.http, Ktor, etc.) maintains
# its own keystore and ignores the system store and env vars.  The CA cert is
# only bind-mounted at run time, so keytool must run at container start, not
# during the image build.  Extend zc-init.sh in your downstream Dockerfile:
#
#   keytool -importcert -noprompt -alias void-claw -cacerts \
#     -storepass changeit \
#     -file /usr/local/share/ca-certificates/void-claw-ca.crt 2>/dev/null || true
#
# Rust / rustls + webpki-roots: uses a compiled-in Mozilla bundle and ignores
# all env vars and the system store.  To intercept traffic from such binaries,
# either patch the application to load extra roots at startup, or rebuild with
# the native-tls / openssl-sys backend so SSL_CERT_FILE is honoured.

FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      nano \
      git \
      python3 \
      python3-pip \
      python3-venv \
      unzip \
      bubblewrap \
      build-essential \
      iproute2 \
      iptables \
      gosu \
    && rm -rf /var/lib/apt/lists/*

# Install a current Node runtime for agent CLIs (Gemini CLI requires Node 20+).
# The NodeSource `nodejs` package already carries npm, and Ubuntu's `npm`
# package conflicts with it.
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get update && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY scripts/hostdo.py /usr/local/bin/hostdo
RUN chmod 755 /usr/local/bin/hostdo
COPY scripts/killme.py /usr/local/bin/killme
RUN chmod 755 /usr/local/bin/killme
ARG TUN2PROXY_RELEASE=latest
RUN set -eu; \
    if [ -x /usr/local/cargo/bin/tun2proxy-bin ]; then \
        install -m 755 /usr/local/cargo/bin/tun2proxy-bin /usr/local/bin/tun2proxy-bin; \
    else \
        case "$(dpkg --print-architecture)" in \
            amd64) asset="tun2proxy-x86_64-unknown-linux-gnu.zip" ;; \
            arm64) asset="tun2proxy-aarch64-unknown-linux-gnu.zip" ;; \
            *) echo "void-claw: unsupported architecture for tun2proxy release fallback" >&2; exit 1 ;; \
        esac; \
        if [ "$TUN2PROXY_RELEASE" = "latest" ]; then \
            release_json="$(curl -fsSL https://api.github.com/repos/tun2proxy/tun2proxy/releases/latest)"; \
        else \
            release_json="$(curl -fsSL "https://api.github.com/repos/tun2proxy/tun2proxy/releases/tags/${TUN2PROXY_RELEASE}")"; \
        fi; \
        download_url="$(printf '%s' "$release_json" | python3 -c 'import json,sys; obj=json.load(sys.stdin); name=sys.argv[1]; print(next(a["browser_download_url"] for a in obj["assets"] if a["name"] == name))' "$asset")"; \
        tmpdir="$(mktemp -d)"; \
        curl -fsSL "$download_url" -o "$tmpdir/tun2proxy.zip"; \
        python3 -c 'import pathlib, sys, zipfile; zf=zipfile.ZipFile(sys.argv[1]); zf.extractall(sys.argv[2])' "$tmpdir/tun2proxy.zip" "$tmpdir"; \
        install -m 755 "$tmpdir/tun2proxy-bin" /usr/local/bin/tun2proxy-bin; \
        rm -rf "$tmpdir"; \
    fi; \
    test -x /usr/local/bin/tun2proxy-bin

# Trusts the void-claw MITM CA cert bind-mounted at run time (root only).
RUN cat > /usr/local/bin/zc-init.sh << 'SCRIPT'
#!/bin/sh
set -e
ZC_CERT="/usr/local/share/ca-certificates/void-claw-ca.crt"
if [ -f "$ZC_CERT" ] && [ "$(id -u)" = "0" ]; then
    update-ca-certificates 2>/dev/null || true
fi

# Best-effort: ensure bind-mounted auth/session state under /home/ubuntu is
# readable by uid 1000 (common issue on macOS/Linux).
if [ "$(id -u)" = "0" ]; then
    if [ -f /home/ubuntu/.claude.json ]; then
        chown 1000:1000 /home/ubuntu/.claude.json 2>/dev/null || true
        chmod 600 /home/ubuntu/.claude.json 2>/dev/null || true
    fi
    if [ -d /home/ubuntu/.claude ]; then
        chown -R 1000:1000 /home/ubuntu/.claude 2>/dev/null || true
        chmod 700 /home/ubuntu/.claude 2>/dev/null || true
        chmod -R u+rwX /home/ubuntu/.claude 2>/dev/null || true
    fi
fi

# Copy staged Claude credentials into place.  void-claw mounts the host
# keychain credential at a staging path to avoid nested bind-mount conflicts.
ZC_STAGED_CREDS="/tmp/.zc-claude-credentials.json"
if [ -f "$ZC_STAGED_CREDS" ]; then
    CLAUDE_DIR="/home/ubuntu/.claude"
    mkdir -p "$CLAUDE_DIR" 2>/dev/null || true
    cp "$ZC_STAGED_CREDS" "$CLAUDE_DIR/.credentials.json" 2>/dev/null || true
    chown 1000:1000 "$CLAUDE_DIR/.credentials.json" 2>/dev/null || true
    chmod 600 "$CLAUDE_DIR/.credentials.json" 2>/dev/null || true
fi

# Optional: strict network enforcement.
# TUN mode captures outbound TCP at the routing layer and forwards it through
# the void-claw scoped HTTP proxy via tun2proxy.  This does not rely on the
# application honoring HTTP(S)_PROXY or ALL_PROXY.  UDP/QUIC are intentionally
# blocked; DNS to Docker's embedded resolver is allowed.
if [ "${VOID_CLAW_STRICT_NETWORK:-0}" = "1" ]; then
    if [ "$(id -u)" != "0" ]; then
        echo "void-claw: strict_network requires root" >&2
        exit 1
    fi
    if [ -z "${VOID_CLAW_SCOPED_PROXY_ADDR:-}" ] || [ -z "${VOID_CLAW_URL:-}" ]; then
        echo "void-claw: strict_network missing VOID_CLAW_SCOPED_PROXY_ADDR or VOID_CLAW_URL" >&2
        exit 1
    fi

    if [ ! -c /dev/net/tun ]; then
        echo "void-claw: strict_network requires /dev/net/tun (Docker Desktop usually needs --privileged)" >&2
        exit 1
    fi

    # ── Pick the iptables binary that actually works ──────────────────────
    # Ubuntu 24.04 defaults to iptables-nft.  Docker Desktop's LinuxKit VM
    # may only support iptables-legacy.  Probe both and use the first that
    # succeeds at listing the filter/OUTPUT chain.
    IPT=""
    IP6T=""
    for candidate in iptables-legacy iptables; do
        if command -v "$candidate" >/dev/null 2>&1 \
           && "$candidate" -w -t filter -L OUTPUT -n >/dev/null 2>&1; then
            IPT="$candidate"
            break
        fi
    done
    for candidate in ip6tables-legacy ip6tables; do
        if command -v "$candidate" >/dev/null 2>&1 \
           && "$candidate" -w -t filter -L OUTPUT -n >/dev/null 2>&1; then
            IP6T="$candidate"
            break
        fi
    done
    if [ -z "$IPT" ]; then
        echo "void-claw: strict_network requires a working iptables (tried iptables-legacy, iptables)" >&2
        exit 1
    fi

    proxy_host="${VOID_CLAW_SCOPED_PROXY_ADDR%:*}"
    proxy_port="${VOID_CLAW_SCOPED_PROXY_ADDR##*:}"
    exec_hostport="$(printf '%s' "$VOID_CLAW_URL" | sed -E 's#^[a-zA-Z]+://##')"
    exec_scheme="$(printf '%s' "$VOID_CLAW_URL" | sed -E 's#^([a-zA-Z]+)://.*#\1#')"
    [ -n "$exec_scheme" ] || exec_scheme="http"
    exec_host="${exec_hostport%:*}"
    exec_port="${exec_hostport##*:}"

    proxy_ip="$(getent ahostsv4 "$proxy_host" 2>/dev/null | awk '{print $1; exit}')"
    exec_ip="$(getent ahostsv4 "$exec_host" 2>/dev/null | awk '{print $1; exit}')"
    if [ -z "$proxy_ip" ] || [ -z "$exec_ip" ]; then
        echo "void-claw: strict_network failed to resolve proxy ($proxy_host) or exec ($exec_host) hosts" >&2
        exit 1
    fi

    # Keep hostdo control traffic on the direct exec-bridge path.
    # In strict mode with virtual DNS, hostnames can resolve to synthetic
    # 198.18.0.0/15 addresses; using the resolved bridge IP avoids that.
    export VOID_CLAW_URL="${exec_scheme}://${exec_ip}:${exec_port}"

    echo "void-claw: strict_network using $IPT; proxy=$proxy_ip:$proxy_port exec=$exec_ip:$exec_port" >&2

    # ── Layer 1: strict egress filter ────────────────────────────────────
    # Allow only: loopback, tun0 (captured traffic), established/related,
    # Docker DNS, proxy, exec. Traffic written to tun0 is not raw egress;
    # tun2proxy consumes it in userspace and opens the real upstream sockets.
    # Anything escaping that path on a physical interface is rejected here.
    $IPT -w -t filter -N ZC_EGRESS 2>/dev/null || $IPT -w -t filter -F ZC_EGRESS
    $IPT -w -t filter -C OUTPUT -j ZC_EGRESS 2>/dev/null || $IPT -w -t filter -I OUTPUT 1 -j ZC_EGRESS
    $IPT -w -t filter -F ZC_EGRESS
    $IPT -w -t filter -A ZC_EGRESS -o lo -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -o tun0 -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -p udp -d 127.0.0.11 --dport 53 -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -p tcp -d 127.0.0.11 --dport 53 -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -p tcp -d "$proxy_ip" --dport "$proxy_port" -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -p tcp -d "$exec_ip" --dport "$exec_port" -j ACCEPT
    $IPT -w -t filter -A ZC_EGRESS -j REJECT 2>/dev/null || $IPT -w -t filter -A ZC_EGRESS -j DROP

    # IPv6: reject everything to avoid long hangs on AAAA/QUIC attempts.
    if [ -n "$IP6T" ]; then
        $IP6T -w -t filter -N ZC_EGRESS 2>/dev/null || $IP6T -w -t filter -F ZC_EGRESS
        $IP6T -w -t filter -C OUTPUT -j ZC_EGRESS 2>/dev/null || $IP6T -w -t filter -I OUTPUT 1 -j ZC_EGRESS
        $IP6T -w -t filter -F ZC_EGRESS
        $IP6T -w -t filter -A ZC_EGRESS -o lo -j ACCEPT
        $IP6T -w -t filter -A ZC_EGRESS -o tun0 -j ACCEPT
        $IP6T -w -t filter -A ZC_EGRESS -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
        $IP6T -w -t filter -A ZC_EGRESS -j REJECT 2>/dev/null || $IP6T -w -t filter -A ZC_EGRESS -j DROP
    fi

    # ── Layer 2: tun2proxy routing ───────────────────────────────────────
    # Bypass the scoped proxy and exec bridge addresses so tun2proxy can talk
    # to them directly.  Docker's DNS (127.0.0.11) remains reachable.
    TUN2PROXY_BIN="$(command -v tun2proxy-bin 2>/dev/null || true)"
    if [ -z "$TUN2PROXY_BIN" ]; then
        for candidate in \
            /usr/local/bin/tun2proxy-bin \
            /usr/local/cargo/bin/tun2proxy-bin \
            /root/.cargo/bin/tun2proxy-bin
        do
            if [ -x "$candidate" ]; then
                TUN2PROXY_BIN="$candidate"
                break
            fi
        done
    fi
    if [ -z "$TUN2PROXY_BIN" ]; then
        echo "void-claw: tun2proxy-bin not found in PATH or common cargo locations" >&2
        exit 1
    fi
    "$TUN2PROXY_BIN" \
        --setup \
        --proxy "http://${proxy_ip}:${proxy_port}" \
        --dns virtual \
        --bypass "$proxy_ip" \
        --bypass "$exec_ip" \
        >/tmp/zc-tun2proxy.log 2>&1 &
    ZC_TUN2PROXY_PID=$!
    sleep 1
    if ! kill -0 "$ZC_TUN2PROXY_PID" 2>/dev/null; then
        echo "void-claw: tun2proxy failed to start" >&2
        cat /tmp/zc-tun2proxy.log >&2 || true
        exit 1
    fi

    # With TUN capture active, application-level proxy env vars are not needed
    # and can cause double-proxying in clients that partially support them.
    unset HTTP_PROXY HTTPS_PROXY ALL_PROXY NO_PROXY
    unset http_proxy https_proxy all_proxy no_proxy

    echo "void-claw: strict_network ready (tun2proxy + iptables filter)" >&2
fi

clear 2>/dev/null || true

if [ "$(id -u)" = "0" ]; then
    exec gosu 1000:1000 "$@"
fi
exec "$@"
SCRIPT
RUN chmod 755 /usr/local/bin/zc-init.sh

# ubuntu:24.04 ships with an 'ubuntu' user at uid/gid 1000.
USER ubuntu
WORKDIR /workspace

ENTRYPOINT ["/usr/local/bin/zc-init.sh"]
CMD ["/bin/bash"]

```

## void-claw.example.toml
```t
# void-claw.example.toml
#
# Main manager config for void-claw.
# Per-project command rules, network policy, and agent instructions live in the
# project's own void-rules.toml file, not here.

# Directory used to resolve Dockerfiles and the Docker build context.
# The init flow uses `./docker` when it exists, otherwise it falls back to
# `~/.config/void-claw/docker`.
docker_dir = __VOID_CLAW_DOCKER_DIR__

[manager]
# Global rules file used by the manager for persisted approvals.
# Use the canonical key below. `rules_file` still parses as an alias, but
# `global_rules_file` is the clearer name to document.
global_rules_file = "~/.config/void-claw/void-rules.toml"

[workspace]
# Root directory for managed workspace copies.
# When a project does not set `workspace_path`, void-claw uses:
#   <workspace.root>/<project.name>
root = "~/agent_workspace"

[agents.gemini]
# Optional extra instructions appended into the generated Gemini guidance.
# extra_instructions = ""

# ---------------------------------------------------------------------------
# Defaults applied to projects unless a project overrides them.
# ---------------------------------------------------------------------------

[defaults.sync]
# Sync mode for managed workspaces. One of:
#   workspace_only  - no sync back to canonical
#   pushback        - seed from canonical, then push changes back
#   bidirectional   - sync both directions
#   pullthrough     - canonical wins on conflict
#   direct          - mount canonical_path directly, no managed copy
mode = "pushback"

# Whether deletes in the workspace can propagate back to canonical.
delete_propagation = false

# Whether renames in the workspace can propagate back to canonical.
rename_propagation = false

# How symlinks are handled during workspace copy.
# One of: reject, copy, follow
symlink_policy = "reject"

# Conflict winner when both sides changed.
# One of: preserve_canonical, preserve_workspace
conflict_policy = "preserve_canonical"

# Global exclude patterns for workspace population.
# These are layered with:
# - [[projects]].exclude_patterns
# - exclude_patterns in the project's void-rules.toml
global_exclude_patterns = [
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

[defaults.ui]
# Width of the left sidebar in the main TUI, in columns.
# The default is 32; increase this if your project names are longer.
sidebar_width = 32

[defaults.hostdo]
# Local exec bridge used by the `hostdo` helper inside containers.
server_port = 7878

# Must be reachable from Docker containers.
# Do not leave this on 127.0.0.1 or other loopback-only values.
server_host = "0.0.0.0"

# Env var name that carries the shared auth token for the exec bridge.
token_env_var = "VOID_CLAW_TOKEN"

# Executables that are always denied.
# These are common shell escape or privilege escalation paths.
denied_executables = ["sh", "bash", "zsh", "fish", "csh", "ksh", "sudo", "su", "doas"]

# Additional string fragments that are denied if they appear in any argument.
denied_argument_fragments = []

# Optional server-side aliases for hostdo.
# Values can be:
# - a plain string
# - a table with `cmd` and optional `cwd`
#
# `cwd` can use:
# - "$CANONICAL" for the host repo path
# - "$WORKSPACE" for the effective workspace path
#
# Examples:
# command_aliases = { tests = "cargo test" }
# command_aliases = { lint = { cmd = "cargo clippy", cwd = "$CANONICAL" } }
# command_aliases = { build = { cmd = "npm run build", cwd = "$WORKSPACE" } }
# command_aliases = {}

[defaults.proxy]
# Local proxy used for outbound HTTP/HTTPS policy enforcement.
proxy_port = 8081

# Must be reachable from Docker containers.
proxy_host = "0.0.0.0"

# If true, force all outbound HTTP/HTTPS traffic through proxy.
strict_network = true

# ---------------------------------------------------------------------------
# Logging and telemetry.
# ---------------------------------------------------------------------------

[logging]
# Directory for runtime state and logs.
# This file stores:
# - `void-claw.log` (daily rotating text logs)
# - runtime state used by the manager and proxy
log_dir = "~/.local/share/void-claw"

# Stable instance id written by void-claw on first startup.
# This is exported as `service.instance.id` in OpenTelemetry.
# instance_id = "generated-on-first-run"

# Optional OpenTelemetry export.
# Remove or leave commented out to disable.
#
# [logging.otlp]
# Collector endpoint.
# Examples:
#   gRPC:      http://localhost:4317
#   HTTP/proto http://localhost:4318/v1/traces
# endpoint = "http://localhost:4317"
#
# One of: grpc, http
# protocol = "grpc"
#
# Which events to export.
# One of:
#   all        - every hostdo and proxy event
#   approvals  - only events that required a developer prompt
#   none       - disable OTel spans
# level = "approvals"
#
# Example collector config:
# [logging.otlp]
# endpoint = "http://localhost:4317"
# protocol = "grpc"
# level = "approvals"

# ---------------------------------------------------------------------------
# Named environment profiles.
# These are referenced by hostdo command rules in void-rules.toml via
# `env_profile`.
# ---------------------------------------------------------------------------

[env_profiles.node]
vars = { NODE_ENV = "development" }

# Add more named env profiles as needed.
# [env_profiles.python]
# vars = { PYTHONUNBUFFERED = "1" }

# ---------------------------------------------------------------------------
# Managed projects are optional.
# Add one [[projects]] block per canonical repository when you want Void Claw
# to manage a workspace. The CLI can still launch without any projects defined.
# Project-specific hostdo overrides, sync overrides, and exclude patterns belong
# inside each [[projects]] block when you add one later.
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# Shared container defaults.
# These are merged into every container profile.
# ---------------------------------------------------------------------------

[defaults.containers]
# Optional default mount point inside the container.
# If omitted, void-claw uses /workspace.
# mount_target = "/workspace"

# Optional default agent type for profiles that do not set one.
# One of: none, claude, codex, gemini, opencode
# agent = "none"

# Optional mounts shared by all profiles.
# `container` must be an absolute path inside the container.
# `mode` is either "ro" or "rw".
#
# [[defaults.containers.mounts]]
# host = "~/.gitconfig"
# container = "/home/ubuntu/.gitconfig"
# mode = "ro"

# Env vars passed through to every container.
# These must be variable names only, not NAME=value pairs.
env_passthrough = ["TERM", "COLORTERM", "COLORFGBG"]

# Optional hosts that bypass MITM handling for every profile.
# Usually better to keep this empty and set bypasses per profile instead.
# bypass_proxy = []

# ---------------------------------------------------------------------------
# Container profiles.
# Put reusable image settings here. Each [[containers]] entry picks one profile.
# ---------------------------------------------------------------------------

[container_profiles.claude]
image = "void-claw-claude:ubuntu-24.04"

# Optional override for this profile's mount point.
# mount_target = "/workspace"

# Controls which agent bootstrap files void-claw writes into the workspace.
# One of: none, claude, codex, opencode
agent = "claude"

# Optional profile-specific env passthrough.
# env_passthrough = ["ANTHROPIC_API_KEY"]

# Hosts that should bypass MITM TLS interception for this profile.
bypass_proxy = [
  "api.anthropic.com",
  "claude.ai",
  "platform.claude.com",
  "downloads.claude.ai",
  "storage.googleapis.com",
]

# Persist Claude auth/session state across launches.
[[container_profiles.claude.mounts]]
host = "~/.claude.json"
container = "/home/ubuntu/.claude.json"
mode = "rw"

[[container_profiles.claude.mounts]]
host = "~/.claude"
container = "/home/ubuntu/.claude"
mode = "rw"

[container_profiles.codex]
image = "void-claw-codex:ubuntu-24.04"
agent = "codex"

# Optional profile-specific env passthrough.
# env_passthrough = ["OPENAI_API_KEY"]

bypass_proxy = [
  "chatgpt.com",
  ".chatgpt.com",
  "chat.openai.com",
  "auth.openai.com",
]

# Persist Codex auth/session state across launches.
[[container_profiles.codex.mounts]]
host = "~/.codex"
container = "/home/ubuntu/.codex"
mode = "rw"

[[container_profiles.codex.mounts]]
host = "~/.config/codex"
container = "/home/ubuntu/.config/codex"
mode = "rw"

[container_profiles.gemini]
image = "void-claw-gemini:ubuntu-24.04"
agent = "gemini"

# Optional profile-specific env passthrough.
# env_passthrough = ["GEMINI_API_KEY", "GOOGLE_API_KEY", "GOOGLE_GENAI_USE_VERTEXAI", "GOOGLE_CLOUD_PROJECT"]

bypass_proxy = [
  "*.googleapis.com",
  "generativelanguage.googleapis.com",
  "aistudio.google.com",
  "accounts.google.com",
  "oauth2.googleapis.com",
  "www.googleapis.com",
]

# Persist Gemini auth/session state across launches.
[[container_profiles.gemini.mounts]]
host = "~/.gemini"
container = "/home/ubuntu/.gemini"
mode = "rw"

[container_profiles.opencode]
image = "void-claw-opencode:ubuntu-24.04"
agent = "opencode"

# Optional profile-specific env passthrough.
# env_passthrough = ["OPENROUTER_API_KEY"]

bypass_proxy = [
  "models.dev",
  "githubusercontent.com",
  ".githubusercontent.com",
]

# Persist opencode auth/session state across launches.
[[container_profiles.opencode.mounts]]
host = "~/.opencode"
container = "/home/ubuntu/.opencode"
mode = "rw"

[[container_profiles.opencode.mounts]]
host = "~/.config/opencode"
container = "/home/ubuntu/.config/opencode"
mode = "rw"

# ---------------------------------------------------------------------------
# Launchable containers shown in the TUI.
# These should only define the display name and the profile to use.
# ---------------------------------------------------------------------------

[[containers]]
name = "claude-agent"
profile = "claude"

[[containers]]
name = "codex-agent"
profile = "codex"

[[containers]]
name = "opencode-agent"
profile = "opencode"

[[containers]]
name = "gemini-agent"
profile = "gemini"

```
