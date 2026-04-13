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
