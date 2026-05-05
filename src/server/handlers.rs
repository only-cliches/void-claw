use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tracing::instrument;

use crate::activity::ActivityEvent;
use crate::config::AliasValue;
use crate::shared_config::SharedConfig;
use crate::state::{AuditEntry, StateManager};

/// A command request waiting for developer approval in the TUI.
pub struct PendingItem {
    /// Unique identifier for this pending item, used for TUI interaction and tracking.
    pub id: String,
    pub activity_id: String,
    pub cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub project: String,
    pub container_id: Option<String>,
    pub argv: Vec<String>,
    pub image: Option<String>,
    pub timeout_secs: u64,
    /// Host-side cwd used to actually execute the command.
    pub cwd: PathBuf,
    /// Container/request cwd used for rule matching and persistence.
    pub rule_cwd: PathBuf,
    pub matched_command: Option<String>,
    /// Sender for the `ApprovalDecision` once the TUI processes this item.
    pub response_tx: Option<oneshot::Sender<ApprovalDecision>>,
}

/// The decision returned by the TUI for a pending command request.
pub enum ApprovalDecision {
    /// Approve the command. `remember: true` means the approval will be persisted
    /// for future identical commands.
    Approve { remember: bool },
    /// Deny the command.
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
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Response payload returned by the hostdo HTTP endpoint.
#[derive(Debug, Serialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Long-running hostdo job state returned by `/exec` and `/exec/jobs/:id`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecJobState {
    Running,
    Complete,
    Failed,
}

/// More specific state for a running image-backed hostdo job.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecJobPhase {
    CheckingImage,
    PullingImage,
    RunningCommand,
}

/// Best-effort progress detail for an image pull.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecJobProgress {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Status payload for a long-running hostdo job.
#[derive(Debug, Clone, Serialize)]
pub struct ExecJobStatus {
    pub state: ExecJobState,
    pub job_id: String,
    #[serde(skip_serializing)]
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<ExecJobPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ExecJobProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
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

/// Represents the identity of a running container session.
#[derive(Debug, Clone)]
pub struct SessionIdentity {
    pub project: String,
    pub container_id: String,
    pub mount_target: String,
}

/// A registry for active container sessions, mapping session tokens to their identities.
/// Provides thread-safe access to session information.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<Mutex<HashMap<String, SessionIdentity>>>,
}

impl SessionRegistry {
    /// Inserts a new session identity into the registry.
    /// Acquires a lock to safely modify the internal map.
    pub fn insert(&self, session_token: String, identity: SessionIdentity) {
        if let Ok(mut map) = self.inner.lock() {
            map.insert(session_token, identity);
        }
    }

    /// Removes a session identity from the registry.
    /// Acquires a lock to safely modify the internal map.
    pub fn remove(&self, session_token: &str) {
        if let Ok(mut map) = self.inner.lock() {
            map.remove(session_token);
        }
    }

    /// Retrieves a session identity from the registry.
    /// Acquires a lock to safely read from the internal map.
    pub fn get(&self, session_token: &str) -> Option<SessionIdentity> {
        self.inner
            .lock()
            .ok()
            .and_then(|map| map.get(session_token).cloned())
    }
}

/// Shared server state for hostdo requests and other manager operations.
/// This state is shared across all HTTP handlers.
#[derive(Clone)]
pub struct ServerState {
    pub config: SharedConfig,
    pub state: StateManager,
    /// Channel to send `PendingItem`s to the TUI for developer approval.
    pub pending_tx: mpsc::Sender<PendingItem>,
    /// Channel to send `ContainerStopItem`s to the TUI to handle container termination.
    pub stop_tx: mpsc::Sender<ContainerStopItem>,
    /// Channel to send `AuditEntry` events for logging and display in the TUI.
    pub audit_tx: mpsc::Sender<AuditEntry>,
    /// The secret token used for authenticating requests from containers.
    pub token: String,
    /// Registry of currently active container sessions.
    pub sessions: SessionRegistry,
    /// Status registry for long-running image-backed hostdo jobs.
    pub exec_jobs: ExecJobRegistry,
    pub activity_tx: mpsc::UnboundedSender<ActivityEvent>,
}

/// In-memory status registry for long-running hostdo jobs.
#[derive(Clone, Default)]
pub struct ExecJobRegistry {
    inner: Arc<Mutex<HashMap<String, ExecJobStatus>>>,
}

impl ExecJobRegistry {
    pub fn insert(&self, mut status: ExecJobStatus) -> ExecJobStatus {
        if status.job_id.is_empty() {
            status.job_id = uuid::Uuid::new_v4().to_string();
        }
        if let Ok(mut map) = self.inner.lock() {
            map.insert(status.job_id.clone(), status.clone());
        }
        status
    }

    pub fn get(&self, job_id: &str) -> Option<ExecJobStatus> {
        self.inner
            .lock()
            .ok()
            .and_then(|map| map.get(job_id).cloned())
    }

    pub fn update<F>(&self, job_id: &str, update: F)
    where
        F: FnOnce(&mut ExecJobStatus),
    {
        if let Ok(mut map) = self.inner.lock() {
            if let Some(status) = map.get_mut(job_id) {
                update(status);
            }
        }
    }
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

/// Initializes and runs the Axum HTTP server to listen for incoming requests.
/// This server handles `/exec` commands from containers (via `hostdo`) and `/container/stop` requests (via `killme`).
#[instrument(skip(server_state, listener))]
pub async fn run_with_listener(
    server_state: ServerState,
    listener: tokio::net::TcpListener,
) -> Result<()> {
    // The server state is wrapped in Arc so it can be shared immutably across multiple handler instances.
    let router = Router::new()
        .route("/exec", post(super::core::exec_handler))
        .route("/exec/jobs/:job_id", get(exec_job_handler))
        .route("/container/stop", post(stop_handler))
        .with_state(Arc::new(server_state));

    axum::serve(listener, router).await?;
    Ok(())
}

/// Returns status for a long-running hostdo execution job.
pub(super) async fn exec_job_handler(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    AxumPath(job_id): AxumPath<String>,
) -> Response {
    let identity = match require_session_identity(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match state.exec_jobs.get(&job_id) {
        Some(status) if status.project == identity.project => Json(status).into_response(),
        Some(_) | None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "not_found".into(),
                reason: "no exec job matched the request".into(),
            }),
        )
            .into_response(),
    }
}

/// Handles incoming requests to stop a container.
///
/// This endpoint is typically called by the `killme` script within a container.
/// It verifies the session identity and then sends a `ContainerStopItem` to the TUI
/// for processing. A timeout is applied for awaiting the TUI's decision.
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

    // Wait for the TUI to process the stop request, with a 10-second timeout.
    // This timeout duration is currently fixed but could be made configurable.
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

/// Creates a standard HTTP 403 Forbidden response with a JSON error payload.
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

/// Validates the session identity from incoming request headers.
///
/// This function checks for:
/// 1. A valid `Authorization` header with a `Bearer` token matching the server's secret token.
/// 2. A non-empty `x-harness-hat-session-token` header.
/// 3. That the session token corresponds to an active session in the `SessionRegistry`.
///
/// Returns `Ok(SessionIdentity)` on success, or an `Err(Response)` with an appropriate
/// HTTP status code and error message on failure.
#[allow(clippy::result_large_err)]
pub(super) fn require_session_identity(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<SessionIdentity, Response> {
    // Extract and validate the Authorization header.
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(""); // If header is missing or invalid, it defaults to an empty string.
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

    // Extract and validate the session token.
    let session_token = headers
        .get("x-harness-hat-session-token")
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

    // Look up the session in the registry.
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

/// Records an audit entry.
///
/// The entry is sent over a channel to the TUI for display and logged to persistent storage
/// on a blocking thread to avoid impacting the main event loop.
pub(super) async fn record_audit(state: &ServerState, entry: AuditEntry) {
    let _ = state.audit_tx.send(entry.clone()).await;
    let state_clone = state.state.clone();
    tokio::task::spawn_blocking(move || {
        let _ = state_clone.log_audit(&entry);
    });
}

/// Resolves the effective host-side current working directory (CWD) for a command.
///
/// This function translates a container's CWD into the corresponding host CWD,
/// taking into account explicit mount targets and a fallback to the historical
/// `/workspace` mapping.
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

/// Represents a resolved command alias, including the expanded argv and an optional CWD override.
pub(super) struct ResolvedAlias {
    pub(super) argv: Vec<String>,
    pub(super) cwd_override: Option<PathBuf>,
}

/// Resolves command aliases for hostdo requests.
///
/// If the first argument of `argv` matches an alias, it expands the alias
/// command and appends any remaining arguments. It also resolves magic CWD
/// placeholder (`$WORKSPACE`) in alias definitions.
/// The `shell_words::split` crate is used to correctly parse shell-like alias commands.
pub(super) fn resolve_exec_argv_aliases(
    argv: &[String],
    aliases: &HashMap<String, AliasValue>,
    _canonical_path: &Path,
    workspace_path: &Path,
) -> std::result::Result<ResolvedAlias, String> {
    if argv.is_empty() {
        return Ok(ResolvedAlias {
            argv: Vec::new(),
            cwd_override: None,
        });
    }
    let Some(alias) = aliases.get(&argv[0]) else {
        // No alias found, return original argv.
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
    // Append any arguments that followed the alias.
    if argv.len() > 1 {
        expanded.extend_from_slice(&argv[1..]);
    }
    Ok(ResolvedAlias {
        argv: expanded,
        cwd_override: alias.resolve_cwd(workspace_path),
    })
}

#[cfg(test)]
mod tests {
    use super::{resolve_exec_argv_aliases, resolve_host_cwd};
    use crate::config::AliasValue;
    use crate::server::SessionRegistry;
    use crate::shared_config::SharedConfig;
    use crate::state::StateManager;
    use axum::{
        body::to_bytes,
        extract::{Path as AxumPath, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
    };
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

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
    fn alias_resolution_supports_magic_cwd_subdirs() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "test-ws-root".to_string(),
            AliasValue::WithOptions {
                cmd: "cargo test".to_string(),
                cwd: Some(PathBuf::from("$WORKSPACE/subdir")),
            },
        );
        aliases.insert(
            "test-ws".to_string(),
            AliasValue::WithOptions {
                cmd: "npm test".to_string(),
                cwd: Some(PathBuf::from("$WORKSPACE/src/app")),
            },
        );

        let canonical = Path::new("/canonical/path");
        let workspace = Path::new("/workspace/path");

        let ws_root_out = resolve_exec_argv_aliases(
            &["test-ws-root".to_string()],
            &aliases,
            canonical,
            workspace,
        )
        .expect("workspace-root alias should resolve");
        assert_eq!(ws_root_out.argv, vec!["cargo", "test"]);
        assert_eq!(
            ws_root_out.cwd_override,
            Some(PathBuf::from("/workspace/path/subdir"))
        );

        let ws_out =
            resolve_exec_argv_aliases(&["test-ws".to_string()], &aliases, canonical, workspace)
                .expect("workspace alias should resolve");
        assert_eq!(ws_out.argv, vec!["npm", "test"]);
        assert_eq!(
            ws_out.cwd_override,
            Some(PathBuf::from("/workspace/path/src/app"))
        );
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

    fn job_status(project: &str) -> super::ExecJobStatus {
        super::ExecJobStatus {
            state: super::ExecJobState::Running,
            job_id: String::new(),
            project: project.to_string(),
            phase: Some(super::ExecJobPhase::PullingImage),
            image: Some("rust".to_string()),
            message: "pulling image".to_string(),
            progress: Some(super::ExecJobProgress {
                kind: "indeterminate".to_string(),
                id: None,
                status: None,
                detail: None,
            }),
            poll_after_ms: Some(1000),
            exit_code: None,
            stdout: None,
            stderr: None,
            reason: None,
        }
    }

    fn auth_headers(token: &str, session_token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
        headers.insert(
            "x-harness-hat-session-token",
            session_token.parse().unwrap(),
        );
        headers
    }

    fn server_state(
        sessions: SessionRegistry,
        exec_jobs: super::ExecJobRegistry,
    ) -> super::ServerState {
        super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions,
            exec_jobs,
            activity_tx: mpsc::unbounded_channel().0,
        }
    }

    #[test]
    fn exec_job_registry_generates_ids_and_updates_status() {
        let registry = super::ExecJobRegistry::default();
        let inserted = registry.insert(job_status("project-a"));

        assert!(!inserted.job_id.is_empty());
        let fetched = registry.get(&inserted.job_id).unwrap();
        assert_eq!(fetched.project, "project-a");
        assert_eq!(fetched.phase, Some(super::ExecJobPhase::PullingImage));

        registry.update(&inserted.job_id, |status| {
            status.state = super::ExecJobState::Complete;
            status.phase = None;
            status.message = "done".to_string();
            status.exit_code = Some(0);
            status.stdout = Some("ok\n".to_string());
            status.stderr = Some(String::new());
        });

        let updated = registry.get(&inserted.job_id).unwrap();
        assert_eq!(updated.state, super::ExecJobState::Complete);
        assert_eq!(updated.phase, None);
        assert_eq!(updated.message, "done");
        assert_eq!(updated.exit_code, Some(0));
        assert_eq!(updated.stdout.as_deref(), Some("ok\n"));
    }

    #[tokio::test]
    async fn exec_job_handler_returns_matching_project_status() {
        let sessions = SessionRegistry::default();
        sessions.insert(
            "session-a".to_string(),
            super::SessionIdentity {
                project: "project-a".to_string(),
                container_id: "container-a".to_string(),
                mount_target: "/workspace".to_string(),
            },
        );
        let registry = super::ExecJobRegistry::default();
        let inserted = registry.insert(job_status("project-a"));
        let state = server_state(sessions, registry);
        let response = super::exec_job_handler(
            State(Arc::new(state)),
            auth_headers("test_token", "session-a"),
            AxumPath(inserted.job_id.clone()),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["state"], "running");
        assert_eq!(body["job_id"], inserted.job_id);
        assert_eq!(body["phase"], "pulling_image");
        assert_eq!(body["image"], "rust");
        assert!(body.get("project").is_none());
    }

    #[tokio::test]
    async fn exec_job_handler_hides_other_project_jobs() {
        let sessions = SessionRegistry::default();
        sessions.insert(
            "session-a".to_string(),
            super::SessionIdentity {
                project: "project-a".to_string(),
                container_id: "container-a".to_string(),
                mount_target: "/workspace".to_string(),
            },
        );
        let registry = super::ExecJobRegistry::default();
        let inserted = registry.insert(job_status("project-b"));
        let state = server_state(sessions, registry);
        let response = super::exec_job_handler(
            State(Arc::new(state)),
            auth_headers("test_token", "session-a"),
            AxumPath(inserted.job_id),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn exec_job_handler_requires_valid_session() {
        let registry = super::ExecJobRegistry::default();
        let inserted = registry.insert(job_status("project-a"));
        let state = server_state(SessionRegistry::default(), registry);
        let response = super::exec_job_handler(
            State(Arc::new(state)),
            auth_headers("test_token", "missing-session"),
            AxumPath(inserted.job_id),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_missing_auth_header() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(), // Use a real path for StateManager
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions: SessionRegistry::default(),
            exec_jobs: super::ExecJobRegistry::default(),
            activity_tx: mpsc::unbounded_channel().0,
        };
        let headers = HeaderMap::new();

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_invalid_auth_token() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "valid_token".to_string(),
            sessions: SessionRegistry::default(),
            exec_jobs: super::ExecJobRegistry::default(),
            activity_tx: mpsc::unbounded_channel().0,
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer invalid_token".parse().unwrap());
        headers.insert(
            "x-harness-hat-session-token",
            "some_session_token".parse().unwrap(),
        );

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_missing_session_token() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions: SessionRegistry::default(),
            exec_jobs: super::ExecJobRegistry::default(),
            activity_tx: mpsc::unbounded_channel().0,
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token".parse().unwrap());

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_unknown_session_token() {
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions: SessionRegistry::default(),
            exec_jobs: super::ExecJobRegistry::default(),
            activity_tx: mpsc::unbounded_channel().0,
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token".parse().unwrap());
        headers.insert(
            "x-harness-hat-session-token",
            "unknown_session".parse().unwrap(),
        );

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn require_session_identity_valid_session() {
        let sessions = SessionRegistry::default();
        sessions.insert(
            "valid_session".to_string(),
            super::SessionIdentity {
                project: "test_project".to_string(),
                container_id: "test_container".to_string(),
                mount_target: "/workspace".to_string(),
            },
        );
        let state = super::ServerState {
            config: SharedConfig::new(Arc::new(crate::config::Config::default())),
            state: StateManager::open(Path::new("/tmp")).unwrap(),
            pending_tx: mpsc::channel(1).0,
            stop_tx: mpsc::channel(1).0,
            audit_tx: mpsc::channel(1).0,
            token: "test_token".to_string(),
            sessions,
            exec_jobs: super::ExecJobRegistry::default(),
            activity_tx: mpsc::unbounded_channel().0,
        };
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token".parse().unwrap());
        headers.insert(
            "x-harness-hat-session-token",
            "valid_session".parse().unwrap(),
        );

        let result = super::require_session_identity(&state, &headers);
        assert!(result.is_ok());
        let identity = result.unwrap();
        assert_eq!(identity.project, "test_project");
        assert_eq!(identity.container_id, "test_container");
        assert_eq!(identity.mount_target, "/workspace");
    }
}
