use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::config::{self, Config, WorkspaceConfig};
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
pub fn check_denied(argv: &[String], proj: &WorkspaceConfig, config: &Config) -> Option<DenyReason> {
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
    use crate::config::{Config, WorkspaceConfig, WorkspaceHostdo};

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
        let proj = WorkspaceConfig::default();
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
        let mut proj = WorkspaceConfig::default();
        let mut config = Config::default();
        config.defaults.hostdo.denied_executables = vec!["cat".to_string()];

        assert!(check_denied(&["cat".into(), "secret.txt".into()], &proj, &config).is_some());
        assert!(check_denied(&["ls".into(), "file.txt".into()], &proj, &config).is_none());

        // Per-project deny
        proj.hostdo = Some(WorkspaceHostdo {
            denied_executables: Some(vec!["ls".to_string()]),
            denied_argument_fragments: None,
            command_aliases: None,
        });
        assert!(check_denied(&["ls".into(), "file.txt".into()], &proj, &config).is_some());
    }
}
