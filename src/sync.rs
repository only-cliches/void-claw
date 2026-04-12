use anyhow::Result;
use chrono::{DateTime, Utc};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{
    self, ConflictPolicy, DefaultsConfig, ProjectConfig, SymlinkPolicy, SyncMode, WorkspaceSection,
};

/// zero-rules.toml is always overwritten from canonical on seed and never
/// copied back to canonical on pushback.
const PROTECTED_RULE_FILE: &str = "zero-rules.toml";

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
    exclude_set: GlobSet,
    gitignore: Gitignore,
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
    // Never push zero-rules.toml back to canonical; warn if it was modified.
    if rel == Path::new(PROTECTED_RULE_FILE) {
        if src.exists() && canonical_rules_path.exists() {
            let ws_bytes = std::fs::read(src).unwrap_or_default();
            let canon_bytes = std::fs::read(canonical_rules_path).unwrap_or_default();
            if ws_bytes != canon_bytes {
                report.warnings.push(
                    "zero-rules.toml was modified in workspace — changes discarded (edit the canonical copy instead)".to_string()
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
fn copy_symlink(src: &Path, dest: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    let target = std::fs::read_link(src)?;
    if dest.exists() || dest.is_symlink() {
        std::fs::remove_file(dest)?;
    }
    symlink(target, dest)?;
    Ok(())
}

#[cfg(not(unix))]
fn copy_symlink(_src: &Path, _dest: &Path) -> Result<()> {
    anyhow::bail!("symlink copy is not supported on this platform")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApprovalMode, ConflictPolicy, DefaultsConfig, ProjectConfig, WorkspaceSection};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("agent-zero-sync-{prefix}-{nanos}"));
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
        let ws = WorkspaceSection { root: ws_root.clone() };
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
        let ws = WorkspaceSection { root: ws_root.clone() };
        let defaults = DefaultsConfig::default();

        let report = seed_files(&proj, &ws, &defaults, &[PathBuf::from("a.txt")]).expect("seed partial");
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
        let ws = WorkspaceSection { root: ws_root.clone() };
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
        let ws = WorkspaceSection { root: ws_root.clone() };
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
        let ws = WorkspaceSection { root: ws_root.clone() };
        let mut defaults = DefaultsConfig::default();
        defaults.sync.conflict_policy = ConflictPolicy::PreserveCanonical;

        let report = pushback(&proj, &ws, &defaults).expect("pushback");
        assert_eq!(report.files_copied, 0);
        assert_eq!(fs::read_to_string(&canon_file).unwrap(), "canonical version");
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
        let ws = WorkspaceSection { root: ws_root.clone() };
        let mut defaults = DefaultsConfig::default();
        defaults.sync.conflict_policy = ConflictPolicy::PreserveWorkspace;

        let report = pushback(&proj, &ws, &defaults).expect("pushback");
        assert_eq!(report.files_copied, 1);
        assert_eq!(fs::read_to_string(&canon_file).unwrap(), "workspace version");
    }
}
