use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tracing::Instrument;

use crate::config::{self, AliasValue, ApprovalMode, AuditExportLevel};
use crate::exec::{self, CommandMatch, DenyReason};
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, DecisionKind, StateManager};

/// A command request waiting for developer approval in the TUI.
pub struct PendingItem {
    pub id: String,
    pub project: String,
    pub container_id: Option<String>,
    pub argv: Vec<String>,
    /// Host-side cwd used to actually execute the command.
    pub cwd: PathBuf,
    /// Container/request cwd used for rule matching and persistence.
    pub rule_cwd: PathBuf,
    pub matched_command: Option<String>,
    pub response_tx: Option<oneshot::Sender<ApprovalDecision>>,
}

/// The decision returned by the TUI for a pending command request.
pub enum ApprovalDecision {
    Approve { remember: bool },
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

#[derive(Debug, Clone)]
pub struct SessionIdentity {
    pub project: String,
    pub container_id: String,
    pub mount_target: String,
}

#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<String, SessionIdentity>>>,
}

impl SessionRegistry {
    pub fn insert(&self, session_token: String, identity: SessionIdentity) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(session_token, identity);
        }
    }

    pub fn remove(&self, session_token: &str) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(session_token);
        }
    }

    pub fn get(&self, session_token: &str) -> Option<SessionIdentity> {
        self.inner
            .lock()
            .ok()
            .and_then(|map| map.get(session_token).cloned())
    }
}

/// Shared server state for hostdo requests.
#[derive(Clone)]
pub struct ServerState {
    pub config: SharedConfig,
    pub state: StateManager,
    pub pending_tx: mpsc::Sender<PendingItem>,
    pub stop_tx: mpsc::Sender<ContainerStopItem>,
    pub audit_tx: mpsc::Sender<AuditEntry>,
    pub token: String,
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

pub async fn run_with_listener(
    server_state: ServerState,
    listener: tokio::net::TcpListener,
) -> Result<()> {
    let router = Router::new()
        .route("/exec", post(exec_handler))
        .route("/container/stop", post(stop_handler))
        .with_state(Arc::new(server_state));

    axum::serve(listener, router).await?;
    Ok(())
}

// ── Handler ──────────────────────────────────────────────────────────────────

async fn exec_handler(
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
            return deny(format!("unknown project '{}'", identity_project));
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
        Err(reason) => return deny(reason),
    };
    let exec_argv = resolved.argv;
    let workspace_path = workspace_path_buf.display().to_string();

    let request_cwd = PathBuf::from(&req.cwd);
    let has_cwd_override = resolved.cwd_override.is_some();
    let host_cwd = if let Some(cwd_override) = resolved.cwd_override {
        cwd_override
    } else {
        resolve_host_cwd(&request_cwd, Some(identity_mount_target.as_str()), &workspace_path_buf)
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
        return deny(reason.to_string());
    }

    // Load composed rules from zero-rules.toml files (global + all projects).
    let mut rules = match config::load_composed_rules_for_project(&cfg, Some(identity_project.as_str()))
    {
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
        CommandMatch::Unlisted if !has_cwd_override => {
            resolve_host_cwd(&request_cwd, Some(identity_mount_target.as_str()), &proj.canonical_path)
        }
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
        return deny(reason);
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
        return deny("manager is shutting down".to_string());
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
                return deny("approval timed out (5 minutes)".to_string());
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
                deny("denied by developer".to_string())
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

async fn stop_handler(
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

fn deny(reason: String) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "denied".into(),
            reason,
        }),
    )
        .into_response()
}

fn require_session_identity(state: &ServerState, headers: &HeaderMap) -> Result<SessionIdentity, Response> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
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

    let session_token = headers
        .get("x-agent-zero-session-token")
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

async fn record_audit(state: &ServerState, entry: AuditEntry) {
    let _ = state.audit_tx.send(entry.clone()).await;
    let state_clone = state.state.clone();
    tokio::task::spawn_blocking(move || {
        let _ = state_clone.log_audit(&entry);
    });
}

fn resolve_host_cwd(
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

/// Resolved alias: the expanded argv and an optional cwd override.
struct ResolvedAlias {
    argv: Vec<String>,
    cwd_override: Option<PathBuf>,
}

fn resolve_exec_argv_aliases(
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
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

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
        aliases.insert("b".to_string(), AliasValue::Simple("cargo test".to_string()));
        
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
}
