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
