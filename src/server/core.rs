use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tracing::Instrument;

use crate::activity::{Activity, ActivityEvent, ActivityKind, ActivityState};
use crate::config::{self, ApprovalMode, AuditExportLevel};
use crate::exec::{self, CommandMatch, DenyReason};
use crate::rules::{DEFAULT_TIMEOUT_SECS, RuleCommand};
use crate::server::{
    ApprovalDecision, ErrorResponse, ExecJobPhase, ExecJobProgress, ExecJobState, ExecJobStatus,
    ExecRequest, ExecResponse, PendingItem, ServerState, deny as server_deny, record_audit,
    require_session_identity, resolve_exec_argv_aliases, resolve_host_cwd,
};
use crate::state::{AuditEntry, DecisionKind};

const IMAGE_PULL_TIMEOUT_SECS: u64 = 30 * 60;

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
    let supports_exec_jobs = supports_exec_jobs(&headers);

    let identity = match require_session_identity(&state, &headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let identity_project = identity.project.clone();
    let identity_container_id = identity.container_id.clone();
    let identity_mount_target = identity.mount_target.clone();

    let cfg = state.config.get();
    let requested_timeout_secs = req.timeout_secs;
    let max_timeout_secs = cfg.defaults.hostdo.max_timeout_secs;

    if let Some(reason) = validate_requested_timeout(requested_timeout_secs, max_timeout_secs) {
        record_audit(
            &state,
            AuditEntry {
                project: identity_project.clone(),
                argv: req.argv.clone(),
                cwd: req.cwd.clone(),
                decision: DecisionKind::DeniedByPolicy,
                exit_code: None,
                duration_ms: None,
                timestamp: Utc::now(),
            },
        )
        .await;
        return server_deny(reason);
    }

    let parsed = match req.image.as_deref() {
        Some(image) => exec::parse_exec_target_with_image(image, &req.argv),
        None => exec::parse_exec_target(&req.argv),
    };
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(reason) => {
            record_audit(
                &state,
                AuditEntry {
                    project: identity_project.clone(),
                    argv: req.argv.clone(),
                    cwd: req.cwd.clone(),
                    decision: DecisionKind::DeniedByPolicy,
                    exit_code: None,
                    duration_ms: None,
                    timestamp: Utc::now(),
                },
            )
            .await;
            return server_deny(reason.to_string());
        }
    };
    let exec_target = parsed.target;
    let request_argv = parsed.argv;

    // Find project config.
    let proj = match cfg.workspaces.iter().find(|p| p.name == identity_project) {
        Some(p) => p,
        None => {
            return server_deny(format!("unknown project '{}'", identity_project));
        }
    };
    // Extract path strings before alias resolution (aliases may reference them).
    let canonical_path = proj.canonical_path.display().to_string();
    let workspace_path_buf =
        config::effective_mount_source_path(proj, &cfg.workspace, &cfg.defaults);

    let resolved = match resolve_exec_argv_aliases(
        &request_argv,
        &config::effective_command_aliases(proj, &cfg.defaults),
        &proj.canonical_path,
        &workspace_path_buf,
    ) {
        Ok(v) => v,
        Err(reason) => return server_deny(reason),
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
            | DenyReason::DeniedArgumentFragment(_)
            | DenyReason::InvalidImage(_) => DecisionKind::DeniedByPolicy,
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
        return server_deny(reason.to_string());
    }

    // Load composed rules from harness-rules.toml files (global + all projects).
    let mut rules =
        match config::load_composed_rules_for_workspace(&cfg, Some(identity_project.as_str())) {
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

    // Expand $WORKSPACE in rule cwds so matching works.
    rules.expand_cwd_vars(&canonical_path);

    // Command matching against the composed rules.
    let cmd_match = exec::find_matching_command(&exec_argv, &exec_target, &rules);
    let explicit_cmd = match &cmd_match {
        CommandMatch::Explicit(cmd) => Some(*cmd),
        CommandMatch::Unlisted => None,
    };
    let requested_timeout_exceeds_rule =
        explicit_cmd.is_some_and(|cmd| requested_timeout_exceeds_rule(requested_timeout_secs, cmd));
    let policy_cmd = explicit_cmd
        .filter(|cmd| cmd.approval_mode != ApprovalMode::Auto || !requested_timeout_exceeds_rule);

    // For unlisted commands (which require approval), default the CWD to the
    // canonical project directory rather than the workspace copy.
    let host_cwd = if policy_cmd.is_none() && !has_cwd_override {
        resolve_host_cwd(
            &request_cwd,
            Some(identity_mount_target.as_str()),
            &proj.canonical_path,
        )
    } else {
        host_cwd
    };

    // Determine approval mode.
    let approval_mode = match policy_cmd {
        Some(cmd) => cmd.approval_mode.clone(),
        None => rules.hostdo.default_policy.clone(),
    };

    if approval_mode == ApprovalMode::Deny {
        let reason = if policy_cmd.is_some() {
            "command denied by rule".to_string()
        } else if requested_timeout_exceeds_rule {
            "requested timeout exceeds auto-approved command rule and default_policy is deny"
                .to_string()
        } else {
            "command not in allowlist and default_policy is deny".to_string()
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
        return server_deny(reason);
    }

    let timeout_secs =
        match effective_timeout_secs(requested_timeout_secs, policy_cmd, max_timeout_secs) {
            Ok(timeout_secs) => timeout_secs,
            Err(reason) => {
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
                return server_deny(reason);
            }
        };
    let env_profile = policy_cmd.and_then(|cmd| cmd.env_profile.clone());
    let matched_command_name = policy_cmd.map(|cmd| cmd.display_name());
    let runner_mount_target = PathBuf::from(&identity_mount_target);
    let runner_cwd = resolve_runner_container_cwd(
        &request_cwd,
        &host_cwd,
        &runner_mount_target,
        &workspace_path_buf,
    );
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let initial_activity_state = match &approval_mode {
        ApprovalMode::Prompt => ActivityState::PendingApproval,
        ApprovalMode::Auto => ActivityState::Running,
        ApprovalMode::Deny => ActivityState::Denied,
    };
    let activity = Activity::new(
        identity_project.clone(),
        Some(identity_container_id.clone()),
        ActivityKind::Hostdo {
            argv: exec_argv.clone(),
            image: exec_target.image().map(str::to_string),
            timeout_secs,
        },
        initial_activity_state,
        cancel_flag.clone(),
    );
    let activity_id = activity.id.clone();
    let _ = state.activity_tx.send(ActivityEvent::Started(activity));

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
            "exec.image" = tracing::field::Empty,
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
        if let Some(image) = exec_target.image() {
            span.record("exec.image", image);
        }
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

        let run = CommandRun {
            state: state.clone(),
            target: exec_target.clone(),
            argv: exec_argv.clone(),
            host_cwd: host_cwd.clone(),
            env_vars: exec::resolve_env(env_profile.as_deref(), &cfg),
            timeout_secs,
            workspace_path: workspace_path_buf.clone(),
            runner_mount_target: runner_mount_target.clone(),
            runner_cwd: runner_cwd.clone(),
            audit_project: identity_project.clone(),
            audit_cwd: req.cwd.clone(),
            decision_kind: DecisionKind::Auto,
            decision_label: "auto".to_string(),
            allow_background_job: supports_exec_jobs,
            activity_id: activity_id.clone(),
            cancel_flag: cancel_flag.clone(),
        };
        return async move { execute_or_start_job(run).await }
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
        image: exec_target.image().map(str::to_string),
        activity_id: activity_id.clone(),
        cancel_flag: cancel_flag.clone(),
        timeout_secs,
        cwd: host_cwd.clone(),
        rule_cwd: request_cwd.clone(),
        matched_command: matched_command_name,
        response_tx: Some(response_tx),
    };

    if state.pending_tx.send(pending).await.is_err() {
        return server_deny("manager is shutting down".to_string());
    }

    // Await the developer decision (and execution) under the span.
    let env_vars = exec::resolve_env(env_profile.as_deref(), &cfg);
    async move {
        let approval_start = Instant::now();

        // Wait for developer decision (5 minute timeout).
        let decision = match tokio::time::timeout(Duration::from_secs(300), response_rx).await {
            Ok(Ok(d)) => d,
            Ok(Err(_)) | Err(_) => {
                let _ = state.activity_tx.send(ActivityEvent::Finished {
                    id: activity_id.clone(),
                    state: ActivityState::Failed,
                    status: Some("approval timed out".to_string()),
                });
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
                return server_deny("approval timed out (5 minutes)".to_string());
            }
        };

        let approval_wait_ms = approval_start.elapsed().as_millis() as i64;
        tracing::Span::current().record("approval_wait_ms", approval_wait_ms);

        match decision {
            ApprovalDecision::Deny => {
                let cancelled = cancel_flag.load(std::sync::atomic::Ordering::SeqCst);
                let _ = state.activity_tx.send(ActivityEvent::Finished {
                    id: activity_id.clone(),
                    state: if cancelled {
                        ActivityState::Cancelled
                    } else {
                        ActivityState::Denied
                    },
                    status: Some(if cancelled {
                        "cancelled".to_string()
                    } else {
                        "denied by developer".to_string()
                    }),
                });
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
                server_deny("denied by developer".to_string())
            }
            ApprovalDecision::Approve { remember } => {
                let _ = state.activity_tx.send(ActivityEvent::State {
                    id: activity_id.clone(),
                    state: ActivityState::Running,
                    status: Some("running command".to_string()),
                });
                let decision_label = if remember { "remembered" } else { "approved" };
                let run = CommandRun {
                    state: state.clone(),
                    target: exec_target.clone(),
                    argv: exec_argv.clone(),
                    host_cwd: host_cwd.clone(),
                    env_vars,
                    timeout_secs,
                    workspace_path: workspace_path_buf.clone(),
                    runner_mount_target: runner_mount_target.clone(),
                    runner_cwd: runner_cwd.clone(),
                    audit_project: identity_project.clone(),
                    audit_cwd: req.cwd.clone(),
                    decision_kind: if remember {
                        DecisionKind::Remembered
                    } else {
                        DecisionKind::Approved
                    },
                    decision_label: decision_label.to_string(),
                    allow_background_job: supports_exec_jobs,
                    activity_id: activity_id.clone(),
                    cancel_flag: cancel_flag.clone(),
                };
                execute_or_start_job(run).await
            }
        }
    }
    .instrument(span_rec)
    .await
}

struct CommandRun {
    state: Arc<ServerState>,
    target: exec::ExecTarget,
    argv: Vec<String>,
    host_cwd: PathBuf,
    env_vars: HashMap<String, String>,
    timeout_secs: u64,
    workspace_path: PathBuf,
    runner_mount_target: PathBuf,
    runner_cwd: PathBuf,
    audit_project: String,
    audit_cwd: String,
    decision_kind: DecisionKind,
    decision_label: String,
    allow_background_job: bool,
    activity_id: String,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CommandRun {
    async fn execute(&self) -> anyhow::Result<exec::ExecResult> {
        let activity_tx = self.state.activity_tx.clone();
        let activity_id = self.activity_id.clone();
        exec::run_target_command_streaming(
            &self.target,
            &self.argv,
            &self.host_cwd,
            &self.env_vars,
            self.timeout_secs,
            &self.workspace_path,
            &self.runner_mount_target,
            &self.runner_cwd,
            self.cancel_flag.clone(),
            move |is_stderr, line| {
                let stream = if is_stderr { "stderr" } else { "stdout" };
                let _ = activity_tx.send(ActivityEvent::Line {
                    id: activity_id.clone(),
                    line: format!("{stream}: {line}"),
                });
            },
        )
        .await
    }
}

async fn execute_or_start_job(run: CommandRun) -> Response {
    if let exec::ExecTarget::DockerImage(image) = run.target.clone() {
        match exec::docker_image_present(&image).await {
            Ok(true) => {}
            Ok(false) if run.allow_background_job => return start_image_pull_job(run, image),
            Ok(false) => return pull_image_then_execute(run, image).await,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "execution_failed".into(),
                        reason: format!("checking docker image failed: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    }

    execute_immediate(run).await
}

async fn pull_image_then_execute(run: CommandRun, image: String) -> Response {
    let _ = run.state.activity_tx.send(ActivityEvent::State {
        id: run.activity_id.clone(),
        state: ActivityState::PullingImage,
        status: Some(format!("pulling Docker image '{image}'")),
    });
    let activity_tx = run.state.activity_tx.clone();
    let activity_id = run.activity_id.clone();
    let progress_image = image.clone();
    match exec::pull_docker_image_cancelable(
        &image,
        IMAGE_PULL_TIMEOUT_SECS,
        run.cancel_flag.clone(),
        move |progress| {
            let _ = activity_tx.send(ActivityEvent::Line {
                id: activity_id.clone(),
                line: format!(
                    "Pulling Docker image '{}': {}",
                    progress_image, progress.message
                ),
            });
        },
    )
    .await
    {
        Ok(result) if result.exit_code == 0 => execute_immediate(run).await,
        Ok(result) => {
            let reason = pull_failure_reason(&result);
            let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                id: run.activity_id.clone(),
                state: ActivityState::Failed,
                status: Some(reason.clone()),
            });
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "execution_failed".into(),
                    reason,
                }),
            )
                .into_response()
        }
        Err(e) => {
            let cancelled = run.cancel_flag.load(std::sync::atomic::Ordering::SeqCst);
            let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                id: run.activity_id.clone(),
                state: if cancelled {
                    ActivityState::Cancelled
                } else {
                    ActivityState::Failed
                },
                status: Some(e.to_string()),
            });
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "execution_failed".into(),
                    reason: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

async fn execute_immediate(run: CommandRun) -> Response {
    let _ = run.state.activity_tx.send(ActivityEvent::State {
        id: run.activity_id.clone(),
        state: ActivityState::Running,
        status: Some("running command".to_string()),
    });
    match run.execute().await {
        Ok(result) => {
            record_success(&run, &result).await;
            let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                id: run.activity_id.clone(),
                state: ActivityState::Complete,
                status: Some(format!("exit code {}", result.exit_code)),
            });
            Json(ExecResponse {
                exit_code: result.exit_code,
                stdout: result.stdout,
                stderr: result.stderr,
            })
            .into_response()
        }
        Err(e) => {
            let cancelled = run.cancel_flag.load(std::sync::atomic::Ordering::SeqCst);
            let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                id: run.activity_id.clone(),
                state: if cancelled {
                    ActivityState::Cancelled
                } else {
                    ActivityState::Failed
                },
                status: Some(e.to_string()),
            });
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "execution_failed".into(),
                    reason: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

fn start_image_pull_job(run: CommandRun, image: String) -> Response {
    let _ = run.state.activity_tx.send(ActivityEvent::State {
        id: run.activity_id.clone(),
        state: ActivityState::PullingImage,
        status: Some(format!("pulling Docker image '{image}'")),
    });
    let status = run.state.exec_jobs.insert(ExecJobStatus {
        state: ExecJobState::Running,
        job_id: String::new(),
        project: run.audit_project.clone(),
        phase: Some(ExecJobPhase::PullingImage),
        image: Some(image.clone()),
        message: format!("Docker image '{image}' is not present locally; pulling it now."),
        progress: Some(ExecJobProgress {
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
    });
    let job_id = status.job_id.clone();
    let registry = run.state.exec_jobs.clone();
    let activity_tx = run.state.activity_tx.clone();
    let activity_id = run.activity_id.clone();
    let cancel_flag = run.cancel_flag.clone();
    let span = tracing::Span::current();

    tokio::spawn(
        async move {
            let progress_registry = registry.clone();
            let progress_job_id = job_id.clone();
            let progress_image = image.clone();
            let pull = exec::pull_docker_image_cancelable(
                &image,
                IMAGE_PULL_TIMEOUT_SECS,
                cancel_flag.clone(),
                move |progress| {
                    let message = format!(
                        "Pulling Docker image '{}': {}",
                        progress_image, progress.message
                    );
                    let _ = activity_tx.send(ActivityEvent::Line {
                        id: activity_id.clone(),
                        line: message.clone(),
                    });
                    progress_registry.update(&progress_job_id, |status| {
                        status.state = ExecJobState::Running;
                        status.phase = Some(ExecJobPhase::PullingImage);
                        status.message = message;
                        status.poll_after_ms = Some(1000);
                        status.progress = Some(ExecJobProgress {
                            kind: "docker_pull".to_string(),
                            id: progress.id,
                            status: progress.status,
                            detail: progress.detail,
                        });
                    });
                },
            )
            .await;

            match pull {
                Ok(result) if result.exit_code == 0 => {
                    registry.update(&job_id, |status| {
                        status.state = ExecJobState::Running;
                        status.phase = Some(ExecJobPhase::RunningCommand);
                        status.message =
                            format!("Image '{image}' is ready; running {}.", run.argv.join(" "));
                        status.progress = None;
                        status.poll_after_ms = Some(1000);
                    });
                    let _ = run.state.activity_tx.send(ActivityEvent::State {
                        id: run.activity_id.clone(),
                        state: ActivityState::Running,
                        status: Some(format!("running {}", run.argv.join(" "))),
                    });
                }
                Ok(result) => {
                    let reason = pull_failure_reason(&result);
                    set_job_failed(&registry, &job_id, reason);
                    let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                        id: run.activity_id.clone(),
                        state: ActivityState::Failed,
                        status: Some(pull_failure_reason(&result)),
                    });
                    return;
                }
                Err(e) => {
                    let cancelled = run.cancel_flag.load(std::sync::atomic::Ordering::SeqCst);
                    set_job_failed(&registry, &job_id, e.to_string());
                    let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                        id: run.activity_id.clone(),
                        state: if cancelled {
                            ActivityState::Cancelled
                        } else {
                            ActivityState::Failed
                        },
                        status: Some(e.to_string()),
                    });
                    return;
                }
            }

            match run.execute().await {
                Ok(result) => {
                    record_success(&run, &result).await;
                    registry.update(&job_id, |status| {
                        status.state = ExecJobState::Complete;
                        status.phase = None;
                        status.message =
                            format!("Command finished with exit code {}.", result.exit_code);
                        status.progress = None;
                        status.poll_after_ms = None;
                        status.exit_code = Some(result.exit_code);
                        status.stdout = Some(result.stdout);
                        status.stderr = Some(result.stderr);
                        status.reason = None;
                    });
                    let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                        id: run.activity_id.clone(),
                        state: ActivityState::Complete,
                        status: Some(format!("exit code {}", result.exit_code)),
                    });
                }
                Err(e) => {
                    let cancelled = run.cancel_flag.load(std::sync::atomic::Ordering::SeqCst);
                    set_job_failed(&registry, &job_id, e.to_string());
                    let _ = run.state.activity_tx.send(ActivityEvent::Finished {
                        id: run.activity_id.clone(),
                        state: if cancelled {
                            ActivityState::Cancelled
                        } else {
                            ActivityState::Failed
                        },
                        status: Some(e.to_string()),
                    });
                }
            }
        }
        .instrument(span),
    );

    (StatusCode::ACCEPTED, Json(status)).into_response()
}

async fn record_success(run: &CommandRun, result: &exec::ExecResult) {
    tracing::Span::current().record("decision", run.decision_label.as_str());
    tracing::Span::current().record("exit_code", result.exit_code);
    tracing::Span::current().record("duration_ms", result.duration_ms as i64);
    record_audit(
        &run.state,
        AuditEntry {
            project: run.audit_project.clone(),
            argv: run.argv.clone(),
            cwd: run.audit_cwd.clone(),
            decision: run.decision_kind.clone(),
            exit_code: Some(result.exit_code),
            duration_ms: Some(result.duration_ms),
            timestamp: Utc::now(),
        },
    )
    .await;
}

fn set_job_failed(registry: &crate::server::ExecJobRegistry, job_id: &str, reason: String) {
    registry.update(job_id, |status| {
        status.state = ExecJobState::Failed;
        status.phase = None;
        status.message = reason.clone();
        status.progress = None;
        status.poll_after_ms = None;
        status.reason = Some(reason);
    });
}

fn pull_failure_reason(result: &exec::ExecResult) -> String {
    let detail = [result.stderr.trim(), result.stdout.trim()]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or("docker pull failed");
    format!(
        "docker pull failed with exit code {}: {detail}",
        result.exit_code
    )
}

fn validate_requested_timeout(requested: Option<u64>, max_timeout_secs: u64) -> Option<String> {
    if max_timeout_secs == 0 {
        return Some("defaults.hostdo.max_timeout_secs must be greater than zero".to_string());
    }
    let requested = requested?;
    validate_effective_timeout(requested, max_timeout_secs, "requested timeout").err()
}

fn requested_timeout_exceeds_rule(requested: Option<u64>, cmd: &RuleCommand) -> bool {
    requested.is_some_and(|timeout_secs| timeout_secs > cmd.timeout_secs)
}

fn effective_timeout_secs(
    requested: Option<u64>,
    cmd: Option<&RuleCommand>,
    max_timeout_secs: u64,
) -> Result<u64, String> {
    let timeout_secs = match (requested, cmd) {
        (Some(timeout_secs), _) => timeout_secs,
        (None, Some(cmd)) => cmd.timeout_secs,
        (None, None) => DEFAULT_TIMEOUT_SECS.min(max_timeout_secs),
    };
    validate_effective_timeout(timeout_secs, max_timeout_secs, "effective timeout")?;
    Ok(timeout_secs)
}

fn validate_effective_timeout(
    timeout_secs: u64,
    max_timeout_secs: u64,
    label: &str,
) -> Result<(), String> {
    if timeout_secs == 0 {
        return Err(format!("{label} must be greater than zero"));
    }
    if timeout_secs > max_timeout_secs {
        return Err(format!(
            "{label} {timeout_secs}s exceeds configured maximum {max_timeout_secs}s"
        ));
    }
    Ok(())
}

fn supports_exec_jobs(headers: &HeaderMap) -> bool {
    headers
        .get("x-hostdo-protocol")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("jobs"))
        })
}

fn resolve_runner_container_cwd(
    request_cwd: &Path,
    host_cwd: &Path,
    mount_target: &Path,
    workspace_host_path: &Path,
) -> PathBuf {
    if host_cwd == workspace_host_path || host_cwd.starts_with(workspace_host_path) {
        if let Ok(rel) = host_cwd.strip_prefix(workspace_host_path) {
            return mount_target.join(rel);
        }
    }

    if request_cwd.is_absolute() {
        request_cwd.to_path_buf()
    } else {
        mount_target.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        effective_timeout_secs, requested_timeout_exceeds_rule, supports_exec_jobs,
        validate_requested_timeout,
    };
    use crate::rules::{ApprovalMode, ConcurrencyPolicy, DEFAULT_TIMEOUT_SECS, RuleCommand};
    use axum::http::HeaderMap;

    fn rule(timeout_secs: u64) -> RuleCommand {
        RuleCommand {
            name: None,
            argv: vec!["cargo".into(), "test".into()],
            image: None,
            cwd: "$WORKSPACE".into(),
            env_profile: None,
            timeout_secs,
            concurrency: ConcurrencyPolicy::Queue,
            approval_mode: ApprovalMode::Auto,
        }
    }

    #[test]
    fn supports_exec_jobs_requires_protocol_header() {
        assert!(!supports_exec_jobs(&HeaderMap::new()));
    }

    #[test]
    fn supports_exec_jobs_accepts_case_insensitive_comma_list() {
        let mut headers = HeaderMap::new();
        headers.insert("x-hostdo-protocol", "legacy, Jobs".parse().unwrap());

        assert!(supports_exec_jobs(&headers));
    }

    #[test]
    fn supports_exec_jobs_rejects_partial_token() {
        let mut headers = HeaderMap::new();
        headers.insert("x-hostdo-protocol", "jobs-v2".parse().unwrap());

        assert!(!supports_exec_jobs(&headers));
    }

    #[test]
    fn requested_timeout_validation_rejects_zero_and_over_max() {
        assert!(
            validate_requested_timeout(Some(0), 300)
                .unwrap()
                .contains("greater than zero")
        );
        assert!(
            validate_requested_timeout(Some(301), 300)
                .unwrap()
                .contains("exceeds configured maximum")
        );
        assert!(validate_requested_timeout(Some(300), 300).is_none());
        assert!(validate_requested_timeout(None, 300).is_none());
    }

    #[test]
    fn effective_timeout_uses_request_rule_or_default() {
        let cmd = rule(180);
        assert_eq!(
            effective_timeout_secs(Some(90), Some(&cmd), 300).unwrap(),
            90
        );
        assert_eq!(effective_timeout_secs(None, Some(&cmd), 300).unwrap(), 180);
        assert_eq!(
            effective_timeout_secs(None, None, 300).unwrap(),
            DEFAULT_TIMEOUT_SECS
        );
    }

    #[test]
    fn effective_timeout_respects_global_max() {
        let cmd = rule(600);
        let err = effective_timeout_secs(None, Some(&cmd), 300).unwrap_err();
        assert!(err.contains("exceeds configured maximum"));
    }

    #[test]
    fn requested_timeout_can_exceed_command_rule_timeout() {
        let cmd = rule(60);
        assert!(requested_timeout_exceeds_rule(Some(120), &cmd));
        assert!(!requested_timeout_exceeds_rule(Some(60), &cmd));
        assert!(!requested_timeout_exceeds_rule(None, &cmd));
    }
}
