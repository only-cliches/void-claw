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

use crate::config::AliasValue;
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, StateManager};

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
        .route("/exec", post(super::core::exec_handler))
        .route("/container/stop", post(stop_handler))
        .with_state(Arc::new(server_state));

    axum::serve(listener, router).await?;
    Ok(())
}

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

pub(super) fn require_session_identity(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<SessionIdentity, Response> {
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

pub(super) async fn record_audit(state: &ServerState, entry: AuditEntry) {
    let _ = state.audit_tx.send(entry.clone()).await;
    let state_clone = state.state.clone();
    tokio::task::spawn_blocking(move || {
        let _ = state_clone.log_audit(&entry);
    });
}

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

/// Resolved alias: the expanded argv and an optional cwd override.
pub(super) struct ResolvedAlias {
    pub(super) argv: Vec<String>,
    pub(super) cwd_override: Option<PathBuf>,
}

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
}
