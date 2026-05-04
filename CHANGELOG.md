# Changelog

This changelog is derived from git history and the current working tree.

## [0.2.0] - Unreleased

### Added
- Host command alias `cwd` resolution supports `$WORKSPACE` with subdirectories (for example: `$WORKSPACE/some-dir`).
- Tests for alias/cwd resolution and direct-mode behavior were expanded (including workspace alias parsing and mount/cwd mapping behavior).
- New binary split:
  - `harness-hat-manager` for the interactive TUI manager.
  - `harness-hat` for command passthrough (`harness-hat -- ...`).
- Passthrough image selection via Dockerfile stem (`--image <name>` -> `<docker_dir>/<name>.dockerfile`) with explicit missing-file error messaging.
- New Docker templates:
  - `docker/harness-hat-base.dockerfile`
  - `docker/default.dockerfile`

### Changed
- Terminology across the product has been updated from **Projects** to **Workspaces** in the TUI, docs, and config model.
- Config now supports `[[workspaces]]` as the primary key, while retaining compatibility with legacy `[[projects]]`.
- Runtime behavior is now direct-only: effective mount/workspace paths resolve to the canonical path, and sync mode resolves to `direct`.
- `hostdo`/rules cwd placeholders were consolidated to `$WORKSPACE` only; `$CANONICAL` references were removed from templates, tests, and examples.
- **Breaking:** network policy schema now uses Coder-style `[network].allowlist` entries (`method=... domain=... path=...`) with prompt-by-default matching; legacy `[[network.rules]]` entries are rejected.
- **Breaking:** `exclude_patterns` and `global_exclude_patterns` are no longer parsed from config/rules TOML files.
- **Breaking:** launch model is now profile-only. `container_profiles` are direct launch targets and legacy `[[containers]]` entries are rejected.
- **Breaking:** `container_profiles.<name>.image` now uses Dockerfile stem resolution (`<docker_dir>/<stem>.dockerfile`) rather than pre-baked per-agent image tags.
- Manager build/launch behavior now resolves images from Dockerfile stems consistently with passthrough CLI behavior.
- Fullscreen terminal hint text for `Ctrl+G` was removed from the UI chrome.
- README and sample config were updated to document direct mode and workspace-first naming.
- Repository/product naming has been aligned to `harness-hat`.

### Removed
- Workspace mirroring and file-sync workflow from the TUI and runtime loop.
- The legacy sync subsystem (`src/sync`) and watcher-driven sync codepaths.
- Unused `walkdir` dependency and stale sync-related code.
- Obsolete `src-files-dump.md` artifact.
- Legacy per-agent Dockerfile subdirectories under `docker/{claude,codex,gemini,opencode}`.
- Legacy `docker/ubuntu-24.04.Dockerfile` base filename (replaced by `docker/harness-hat-base.dockerfile`).

## [0.1.0]
- Initial release.
