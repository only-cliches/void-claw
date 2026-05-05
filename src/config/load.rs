use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, value};
use tracing::instrument;

use crate::config::{
    AgentKind, AliasValue, Config, ContainerMount, DefaultsConfig, SyncMode, WorkspaceConfig,
    WorkspaceSection, default_mount_target,
};

// ── Rule loading ─────────────────────────────────────────────────────────────

/// Load and compose rules for a specific project (global + that project's
/// harness-rules.toml). Called at request time so edits take effect without
/// restart.
#[instrument(skip(config))]
pub fn load_composed_rules_for_workspace(
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
        if let Some(project) = config.workspaces.iter().find(|p| p.name == project_name) {
            let path = project.canonical_path.join("harness-rules.toml");
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
    if !config.docker_dir.as_os_str().is_empty() {
        config.docker_dir = expand_path(&config.docker_dir)?;
    }
    for proj in &mut config.workspaces {
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
    anyhow::ensure!(
        config.containers.is_empty(),
        "legacy [[containers]] is no longer supported; define launchable entries under [container_profiles.<name>] only"
    );

    let defaults = config.defaults.containers.clone();
    let mut profile_names = config
        .container_profiles
        .keys()
        .cloned()
        .collect::<Vec<String>>();
    profile_names.sort();

    let mut resolved = Vec::with_capacity(profile_names.len());
    for profile_name in profile_names {
        let profile = config
            .container_profiles
            .get(&profile_name)
            .ok_or_else(|| anyhow::anyhow!("unknown container profile '{}'", profile_name))?;

        let image_stem_raw = profile.image.as_deref().unwrap_or("default").trim();
        anyhow::ensure!(
            !image_stem_raw.is_empty(),
            "container profile '{}': image must not be empty",
            profile_name
        );
        let image_stem = image_stem_raw.to_string();
        anyhow::ensure!(
            image_stem.chars().all(|c| c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, '-' | '_' | '.')),
            "container profile '{}': image must be a lowercase stem (allowed: a-z, 0-9, '-', '_', '.')",
            profile_name
        );
        let image_tag = image_tag_for_stem(&image_stem);

        resolved.push(crate::config::ContainerDef {
            name: profile_name.clone(),
            profile: None,
            image: image_tag,
            mount_target: profile
                .mount_target
                .clone()
                .or_else(|| defaults.mount_target.clone())
                .unwrap_or_else(default_mount_target),
            agent: profile
                .agent
                .clone()
                .or_else(|| defaults.agent.clone())
                .unwrap_or(AgentKind::None),
            mounts: merge_mounts(&defaults.mounts, &profile.mounts, &[]),
            env_passthrough: merge_unique_strings(
                &defaults.env_passthrough,
                &profile.env_passthrough,
                &[],
            ),
            bypass_proxy: merge_unique_strings(&defaults.bypass_proxy, &profile.bypass_proxy, &[]),
            image_stem,
        });
    }
    config.containers = resolved;

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

pub fn image_tag_for_stem(stem: &str) -> String {
    let mut slug = String::new();
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            slug.push(ch.to_ascii_lowercase());
        } else {
            slug.push('-');
        }
    }
    if slug.is_empty() {
        slug.push_str("default");
    }
    format!("harness-hat-{slug}:local")
}

fn validate(config: &Config) -> Result<()> {
    anyhow::ensure!(
        config.defaults.hostdo.max_timeout_secs > 0,
        "defaults.hostdo.max_timeout_secs must be greater than zero"
    );

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
    for proj in &config.workspaces {
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
pub fn effective_workspace_path(proj: &WorkspaceConfig, ws: &WorkspaceSection) -> PathBuf {
    let _ = ws;
    proj.canonical_path.clone()
}

/// Host-side directory that should be mounted into the container at `mount_target`.
#[instrument(skip(proj, ws, defaults))]
pub fn effective_mount_source_path(
    proj: &WorkspaceConfig,
    ws: &WorkspaceSection,
    defaults: &DefaultsConfig,
) -> PathBuf {
    let _ = (ws, defaults);
    proj.canonical_path.clone()
}

/// Effective sync mode for a project.
#[instrument(skip(proj, defaults))]
pub fn effective_sync_mode(proj: &WorkspaceConfig, defaults: &DefaultsConfig) -> SyncMode {
    let _ = (proj, defaults);
    SyncMode::Direct
}

/// Effective denied executables.
#[instrument(skip(proj, defaults))]
pub fn effective_denied_executables(
    proj: &WorkspaceConfig,
    defaults: &DefaultsConfig,
) -> Vec<String> {
    proj.hostdo
        .as_ref()
        .and_then(|he| he.denied_executables.clone())
        .unwrap_or_else(|| defaults.hostdo.denied_executables.clone())
}

/// Effective denied argument fragments.
#[instrument(skip(proj, defaults))]
pub fn effective_denied_fragments(
    proj: &WorkspaceConfig,
    defaults: &DefaultsConfig,
) -> Vec<String> {
    proj.hostdo
        .as_ref()
        .and_then(|he| he.denied_argument_fragments.clone())
        .unwrap_or_else(|| defaults.hostdo.denied_argument_fragments.clone())
}

/// Effective hostdo command aliases for a project.
/// Merge order (later wins): global defaults → per-project config → per-project rules.
#[instrument(skip(proj, defaults))]
pub fn effective_command_aliases(
    proj: &WorkspaceConfig,
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
    // Layer on aliases from the project's harness-rules.toml (highest priority).
    let rules_path = proj.canonical_path.join("harness-rules.toml");
    if let Ok(rules) = crate::rules::load(&rules_path) {
        if !rules.hostdo.command_aliases.is_empty() {
            out.extend(rules.hostdo.command_aliases);
        }
    }
    out
}
