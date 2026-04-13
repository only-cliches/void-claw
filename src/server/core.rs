use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tracing::Instrument;

use crate::config::{self, ApprovalMode, AuditExportLevel};
use crate::exec::{self, CommandMatch, DenyReason};
use crate::server::{
    ApprovalDecision, ErrorResponse, ExecRequest, ExecResponse, PendingItem, ServerState, deny,
    record_audit, require_session_identity, resolve_exec_argv_aliases, resolve_host_cwd,
};
use crate::state::{AuditEntry, DecisionKind};

// ── Handler ──────────────────────────────────────────────────────────────────

pub(super) async fn exec_handler(
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
        resolve_host_cwd(
            &request_cwd,
            Some(identity_mount_target.as_str()),
            &workspace_path_buf,
        )
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
    let mut rules =
        match config::load_composed_rules_for_project(&cfg, Some(identity_project.as_str())) {
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
        CommandMatch::Unlisted if !has_cwd_override => resolve_host_cwd(
            &request_cwd,
            Some(identity_mount_target.as_str()),
            &proj.canonical_path,
        ),
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
