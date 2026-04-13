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
