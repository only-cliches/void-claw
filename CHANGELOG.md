# Changelog

This changelog is derived from git history and the current working tree.

## [0.3.0] - Unreleased

### Added
- Project/package rename from `void-claw` to `harness-hat` across the Rust crate, manager binary, Docker templates, helper scripts, example config, rules file, README, and PWA metadata.
- `hostdo --image <image> ...` support for short-lived Docker runners, with image-specific approval rules and validation for requested Docker image names.
- Automatic Docker image checks for image-backed `hostdo` commands, including pull progress reporting while an image is downloading.
- Long-running `hostdo` job tracking for image-backed commands, including job polling from the `hostdo` helper and cancellable execution.
- Optional `hostdo --timeout <seconds>` requests, persisted `timeout_secs` rule updates, and `[defaults.hostdo].max_timeout_secs` enforcement.
- Streaming terminal output for `hostdo` commands and Docker runners, using the same terminal emulation path as agent terminals.
- Active hostdo and network requests now appear as selectable child rows under their container in the sidebar.
- Hostdo activity detail panes show command, image, timeout, status, elapsed timing, and terminal history; network detail panes show method, domain, path, protocol, payload metadata, payload preview, status, and connection history.
- `Ctrl+C` cancellation for selected in-flight hostdo and network activities.
- Status coloring for activity detail panes and sidebar rows: yellow while running, green for success, and red for failure/cancellation.
- Temporary completion highlighting for finished activity rows, with fading delayed while the row remains selected.
- `[network].denylist` rules for permanent network denies, with deny matches taking precedence over allow matches.
- Persistence for "always deny" network decisions into `harness-rules.toml`.
- Rules-file internal write tracking for manager-generated approvals and starter rules, avoiding false tamper alerts for expected writes.

### Changed
- Hostdo activity titles now show the actual command only, omitting `hostdo` options such as `--image` and `--timeout`.
- Hostdo command timers now measure the command phase only; Docker image checking and pulling are reported separately from the command timeout.
- Hostdo activity elapsed timers stop when the command finishes.
- Docker build and hostdo/detail panes now use more consistent controls, spacing, and footer behavior.
- Sidebar selection now preserves the selected item when activity rows appear, disappear, or fade above it.
- Activity fade timers reset when a fading row is selected again.
- The completion bell indicator is only restored for terminal bell events emitted by an agent.
- Network rule counts in the UI now include both allowlist and denylist entries.

### Fixed
- Hostdo detail panes now show both stdout and stderr instead of only stderr.
- Selected completed activity rows remain visible until selection moves away.
- Image-backed `hostdo` commands no longer make image download time appear to breach the command timeout.
- Docker build panes no longer advertise inactive `[c]` or `[r]` footer shortcuts.
- Network "always deny" approvals now create explicit persisted rules instead of relying on implicit prompt/default behavior.

## [0.2.0] - April 14, 2026

### Added
- Host command alias `cwd` resolution supports `$WORKSPACE` with subdirectories (for example: `$WORKSPACE/some-dir`).
- Tests for alias/cwd resolution and direct-mode behavior were expanded (including workspace alias parsing and mount/cwd mapping behavior).
- New binary split:
  - `void-claw-manager` for the interactive TUI manager.
  - `void-claw` for command passthrough (`void-claw -- ...`).
- Passthrough image selection via Dockerfile stem (`--image <name>` -> `<docker_dir>/<name>.dockerfile`) with explicit missing-file error messaging.
- New Docker templates:
  - `docker/void-claw-base.dockerfile`
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
- Repository/product naming has been aligned to `void-claw`.

### Removed
- Workspace mirroring and file-sync workflow from the TUI and runtime loop.
- The legacy sync subsystem (`src/sync`) and watcher-driven sync codepaths.
- Unused `walkdir` dependency and stale sync-related code.
- Obsolete `src-files-dump.md` artifact.
- Legacy per-agent Dockerfile subdirectories under `docker/{claude,codex,gemini,opencode}`.
- Legacy `docker/ubuntu-24.04.Dockerfile` base filename (replaced by `docker/void-claw-base.dockerfile`).

## [0.1.0]
- Initial release.
