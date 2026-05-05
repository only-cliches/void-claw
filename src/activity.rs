use std::collections::VecDeque;
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi::Processor;

use crate::container::TermSize;

pub type ActivityId = String;

pub const ACTIVITY_HISTORY_LIMIT: usize = 400;
pub const ACTIVITY_PAYLOAD_PREVIEW_LIMIT: usize = 16 * 1024;
pub const ACTIVITY_TERMINAL_HIGHLIGHT_SECS: u64 = 3;
pub const ACTIVITY_TERMINAL_FADE_SECS: u64 = 3;
pub const ACTIVITY_TERMINAL_TTL_SECS: u64 =
    ACTIVITY_TERMINAL_HIGHLIGHT_SECS + ACTIVITY_TERMINAL_FADE_SECS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityState {
    PendingApproval,
    PullingImage,
    Running,
    Forwarding,
    Complete,
    Failed,
    Denied,
    Cancelled,
}

impl ActivityState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Complete | Self::Failed | Self::Denied | Self::Cancelled
        )
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::PendingApproval => "pending approval",
            Self::PullingImage => "pulling image",
            Self::Running => "running",
            Self::Forwarding => "forwarding",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Denied => "denied",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn succeeded(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityKind {
    Hostdo {
        argv: Vec<String>,
        image: Option<String>,
        timeout_secs: u64,
    },
    Network {
        method: String,
        host: String,
        path: String,
        protocol: String,
        payload_preview: String,
        payload_truncated: bool,
        content_type: Option<String>,
        content_length: Option<usize>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ActivityEventProxy;

impl EventListener for ActivityEventProxy {
    fn send_event(&self, _event: Event) {}
}

#[derive(Clone)]
pub struct ActivityTerminal {
    pub(crate) term: Arc<FairMutex<Term<ActivityEventProxy>>>,
    parser: Arc<FairMutex<Processor>>,
}

impl fmt::Debug for ActivityTerminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivityTerminal").finish_non_exhaustive()
    }
}

impl ActivityTerminal {
    fn new() -> Self {
        let mut term_cfg = TermConfig::default();
        term_cfg.scrolling_history = 50_000;
        let term_size = TermSize {
            cols: 80,
            lines: 24,
        };
        Self {
            term: Arc::new(FairMutex::new(Term::new(
                term_cfg,
                &term_size,
                ActivityEventProxy,
            ))),
            parser: Arc::new(FairMutex::new(Processor::new())),
        }
    }

    pub(crate) fn resize(&self, rows: u16, cols: u16) {
        let mut term = self.term.lock();
        term.resize(TermSize {
            cols: cols as usize,
            lines: rows as usize,
        });
    }

    fn write(&self, bytes: &[u8]) {
        let mut term = self.term.lock();
        let mut parser = self.parser.lock();
        parser.advance(&mut *term, bytes);
    }
}

#[derive(Debug, Clone)]
pub struct Activity {
    pub id: ActivityId,
    pub project: String,
    pub container: Option<String>,
    pub kind: ActivityKind,
    pub state: ActivityState,
    pub status: Option<String>,
    pub lines: VecDeque<String>,
    pub terminal: ActivityTerminal,
    pub started_at: Instant,
    pub updated_at: Instant,
    pub finished_at: Option<Instant>,
    pub command_started_at: Option<Instant>,
    pub command_finished_at: Option<Instant>,
    pub terminal_unselected_at: Option<Instant>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl Activity {
    pub fn new(
        project: String,
        container: Option<String>,
        kind: ActivityKind,
        state: ActivityState,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        let now = Instant::now();
        let finished_at = state.is_terminal().then_some(now);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project,
            container,
            kind,
            state,
            status: None,
            lines: VecDeque::new(),
            terminal: ActivityTerminal::new(),
            started_at: now,
            updated_at: now,
            finished_at,
            command_started_at: None,
            command_finished_at: None,
            terminal_unselected_at: None,
            cancel_flag,
        }
    }

    pub fn title(&self) -> String {
        match &self.kind {
            ActivityKind::Hostdo { argv, .. } => argv.join(" "),
            ActivityKind::Network {
                method, host, path, ..
            } => {
                format!("{method} {host}{path}")
            }
        }
    }

    pub fn push_line(&mut self, line: String) {
        if self.lines.len() >= ACTIVITY_HISTORY_LIMIT {
            self.lines.pop_front();
        }
        let terminal_line = activity_terminal_line(&line);
        self.terminal.write(terminal_line.as_bytes());
        self.terminal.write(b"\r\n");
        self.lines.push_back(line);
        self.updated_at = Instant::now();
    }

    pub fn elapsed_duration(&self) -> Duration {
        self.finished_at
            .unwrap_or_else(Instant::now)
            .saturating_duration_since(self.started_at)
    }

    pub fn command_elapsed_duration(&self) -> Option<Duration> {
        let started_at = self.command_started_at?;
        Some(
            self.command_finished_at
                .unwrap_or_else(Instant::now)
                .saturating_duration_since(started_at),
        )
    }

    pub fn mark_command_started(&mut self, at: Instant) {
        if matches!(self.kind, ActivityKind::Hostdo { .. }) {
            self.command_started_at.get_or_insert(at);
            self.command_finished_at = None;
        }
    }

    pub fn mark_command_finished(&mut self, at: Instant) {
        if matches!(self.kind, ActivityKind::Hostdo { .. }) && self.command_started_at.is_some() {
            self.command_finished_at.get_or_insert(at);
        }
    }

    pub fn clear_command_timing(&mut self) {
        self.command_started_at = None;
        self.command_finished_at = None;
    }

    pub fn request_cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }
}

fn activity_terminal_line(line: &str) -> &str {
    line.strip_prefix("stdout: ")
        .or_else(|| line.strip_prefix("stderr: "))
        .unwrap_or(line)
}

#[derive(Debug, Clone)]
pub enum ActivityEvent {
    Started(Activity),
    State {
        id: ActivityId,
        state: ActivityState,
        status: Option<String>,
    },
    Line {
        id: ActivityId,
        line: String,
    },
    Finished {
        id: ActivityId,
        state: ActivityState,
        status: Option<String>,
    },
}

pub fn payload_preview(body: &[u8]) -> (String, bool) {
    let truncated = body.len() > ACTIVITY_PAYLOAD_PREVIEW_LIMIT;
    let preview = &body[..body.len().min(ACTIVITY_PAYLOAD_PREVIEW_LIMIT)];
    let text = if preview.is_empty() {
        String::new()
    } else {
        String::from_utf8_lossy(preview).into_owned()
    };
    (text, truncated)
}

pub async fn wait_cancelled(flag: Arc<AtomicBool>) {
    while !flag.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_activity_elapsed_uses_finished_timestamp() {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let mut activity = Activity::new(
            "project".to_string(),
            Some("container".to_string()),
            ActivityKind::Hostdo {
                argv: vec!["cargo".to_string(), "test".to_string()],
                image: None,
                timeout_secs: 60,
            },
            ActivityState::Running,
            cancel_flag,
        );
        let now = Instant::now();
        activity.started_at = now - Duration::from_secs(30);
        activity.finished_at = Some(now - Duration::from_secs(10));
        activity.updated_at = now;
        activity.state = ActivityState::Complete;

        assert_eq!(activity.elapsed_duration().as_secs(), 20);
    }

    #[test]
    fn hostdo_activity_title_omits_hostdo_options() {
        let activity = Activity::new(
            "project".to_string(),
            Some("container".to_string()),
            ActivityKind::Hostdo {
                argv: vec!["cargo".to_string(), "test".to_string()],
                image: Some("rust".to_string()),
                timeout_secs: 120,
            },
            ActivityState::Running,
            Arc::new(AtomicBool::new(false)),
        );

        assert_eq!(activity.title(), "cargo test");
    }

    #[test]
    fn command_elapsed_tracks_command_phase_only() {
        let mut activity = Activity::new(
            "project".to_string(),
            Some("container".to_string()),
            ActivityKind::Hostdo {
                argv: vec!["cargo".to_string(), "test".to_string()],
                image: Some("rust".to_string()),
                timeout_secs: 120,
            },
            ActivityState::PullingImage,
            Arc::new(AtomicBool::new(false)),
        );
        let now = Instant::now();
        activity.started_at = now - Duration::from_secs(130);
        activity.updated_at = now;

        assert_eq!(activity.elapsed_duration().as_secs(), 130);
        assert_eq!(activity.command_elapsed_duration(), None);

        activity.mark_command_started(now - Duration::from_secs(80));
        activity.mark_command_finished(now - Duration::from_secs(5));

        assert_eq!(activity.command_elapsed_duration().unwrap().as_secs(), 75);
    }

    #[test]
    fn push_line_updates_terminal_without_stream_prefix() {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let mut activity = Activity::new(
            "project".to_string(),
            Some("container".to_string()),
            ActivityKind::Hostdo {
                argv: vec!["cargo".to_string(), "test".to_string()],
                image: None,
                timeout_secs: 60,
            },
            ActivityState::Running,
            cancel_flag,
        );

        activity.push_line("stdout: \x1b[31mcompiled ok\x1b[0m".to_string());

        let term = activity.terminal.term.lock();
        let text = term
            .renderable_content()
            .display_iter
            .map(|indexed| indexed.cell.c)
            .collect::<String>();
        assert!(text.contains("compiled ok"));
        assert!(!text.contains("stdout:"));
    }
}
