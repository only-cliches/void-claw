use super::*;

pub(crate) fn render_right_pane(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.focus == Focus::Sidebar {
        let selected = app.sidebar_items().get(app.sidebar_idx).cloned();
        match selected {
            Some(SidebarItem::Session(si)) if si < app.sessions.len() => {
                let has_modal =
                    !app.pending_for_session(si).is_empty() || !app.pending_net.is_empty();
                // Sidebar-selected session is a preview, so keep it visually
                // muted even when no modal is active.
                let preview_dimmed = true;
                render_terminal(frame, app, area, si, preview_dimmed || has_modal, false);
                render_terminal_overlays(frame, app, area, si);
            }
            Some(SidebarItem::Activity(id)) => {
                render_activity_detail(frame, app, area, id.as_str(), true);
            }
            Some(SidebarItem::Settings(pi)) => {
                render_project_settings(frame, app, area, pi, true);
            }
            Some(SidebarItem::Launch(_)) => {
                render_container_picker(frame, app, area, true);
            }
            Some(SidebarItem::Build(_))
                if app.build_is_running() && build_output_is_selected(app) =>
            {
                render_build_output(frame, app, area, true);
            }
            Some(SidebarItem::NewWorkspace) => {
                if app.new_project.is_some() {
                    render_new_project(frame, app, area, true);
                } else {
                    render_new_project_preview(frame, app, area, true);
                }
            }
            _ => render_idle(frame, area),
        }
        return;
    }

    if app.focus == Focus::Settings {
        let pi = app
            .active_settings_project
            .or_else(|| app.selected_project_idx())
            .unwrap_or(0);
        render_project_settings(frame, app, area, pi, false);
        return;
    }

    if app.focus == Focus::ContainerPicker {
        render_container_picker(frame, app, area, false);
        return;
    }

    if app.focus == Focus::Activity {
        if let Some(id) = app.active_activity.clone() {
            render_activity_detail(frame, app, area, id.as_str(), false);
        } else {
            render_idle(frame, area);
        }
        return;
    }

    if app.focus == Focus::ImageBuild {
        if app.build_is_running() {
            render_build_output(frame, app, area, false);
        } else {
            render_image_build(frame, app, area, false);
        }
        return;
    }

    if app.build_is_running() && build_output_is_selected(app) {
        render_build_output(frame, app, area, false);
        return;
    }

    if app.focus == Focus::NewWorkspace {
        render_new_project(frame, app, area, false);
        return;
    }

    let has_modal = app
        .active_session
        .map(|si| !app.pending_for_session(si).is_empty() || !app.pending_net.is_empty())
        .unwrap_or(false);

    match app.active_session {
        Some(si) if si < app.sessions.len() => {
            render_terminal(frame, app, area, si, has_modal, false);
            render_terminal_overlays(frame, app, area, si);
        }
        _ => render_idle(frame, area),
    }
}

pub(crate) fn render_terminal_overlays(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    session_idx: usize,
) {
    let pending_exec = app.pending_for_session(session_idx);
    if !pending_exec.is_empty() {
        render_exec_approval_overlay(frame, app, area, pending_exec[0]);
        return;
    }

    if !app.pending_net.is_empty() {
        render_net_approval_overlay(frame, app, area);
    }
}

pub(crate) fn build_output_is_selected(app: &App) -> bool {
    matches!(
        app.sidebar_items().get(app.sidebar_idx),
        Some(SidebarItem::Build(_))
    )
}

pub(crate) fn render_activity_detail(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    activity_id: &str,
    dimmed: bool,
) {
    let Some(activity) = app.activity_by_id(activity_id) else {
        render_idle(frame, area);
        return;
    };
    let tone = |c| maybe_dim(c, dimmed);
    let title = match &activity.kind {
        crate::activity::ActivityKind::Hostdo { .. } => " Hostdo Activity ",
        crate::activity::ActivityKind::Network { .. } => " Network Activity ",
    };
    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::Cyan)));

    let elapsed = activity.elapsed_duration().as_secs();
    let status_color = activity_status_color(&activity.state);
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  State   : ", Style::default().fg(tone(Color::DarkGray))),
        Span::styled(
            activity.state.label(),
            Style::default()
                .fg(tone(status_color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  total: {elapsed}s"),
            Style::default().fg(tone(Color::DarkGray)),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Source  : ", Style::default().fg(tone(Color::DarkGray))),
        Span::styled(
            format!(
                "workspace={}  container={}",
                activity.project,
                activity.container.as_deref().unwrap_or("unknown-container")
            ),
            Style::default().fg(tone(Color::White)),
        ),
    ]));

    match &activity.kind {
        crate::activity::ActivityKind::Hostdo {
            argv,
            image,
            timeout_secs,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  Command : ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled(activity.title(), Style::default().fg(tone(Color::White))),
            ]));
            if let Some(image) = image {
                lines.push(Line::from(vec![
                    Span::styled("  Image   : ", Style::default().fg(tone(Color::DarkGray))),
                    Span::styled(image.clone(), Style::default().fg(tone(Color::White))),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  Timeout : ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled(
                    hostdo_timeout_label(activity, *timeout_secs),
                    Style::default().fg(tone(Color::White)),
                ),
            ]));
            if let Some(command_elapsed) = activity.command_elapsed_duration() {
                lines.push(Line::from(vec![
                    Span::styled(
                        "  Command elapsed: ",
                        Style::default().fg(tone(Color::DarkGray)),
                    ),
                    Span::styled(
                        format!("{}s / {}s", command_elapsed.as_secs(), timeout_secs),
                        Style::default().fg(tone(status_color)),
                    ),
                ]));
            }
            if argv.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  <empty command>",
                    Style::default().fg(tone(Color::Red)),
                )));
            }
        }
        crate::activity::ActivityKind::Network {
            method,
            host,
            path,
            protocol,
            payload_preview,
            payload_truncated,
            content_type,
            content_length,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  Method  : ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled(method.clone(), Style::default().fg(tone(Color::White))),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Domain  : ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled(host.clone(), Style::default().fg(tone(Color::White))),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Path    : ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled(path.clone(), Style::default().fg(tone(Color::White))),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Protocol: ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled(protocol.clone(), Style::default().fg(tone(Color::White))),
            ]));
            if let Some(content_type) = content_type {
                lines.push(Line::from(vec![
                    Span::styled("  Type    : ", Style::default().fg(tone(Color::DarkGray))),
                    Span::styled(
                        content_type.clone(),
                        Style::default().fg(tone(Color::White)),
                    ),
                ]));
            }
            if let Some(content_length) = content_length {
                lines.push(Line::from(vec![
                    Span::styled("  Payload : ", Style::default().fg(tone(Color::DarkGray))),
                    Span::styled(
                        format!(
                            "{content_length} bytes{}",
                            if *payload_truncated {
                                " (preview truncated)"
                            } else {
                                ""
                            }
                        ),
                        Style::default().fg(tone(Color::White)),
                    ),
                ]));
            }
            if !payload_preview.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Payload preview",
                    Style::default()
                        .fg(tone(Color::Magenta))
                        .add_modifier(Modifier::BOLD),
                )));
                for line in payload_preview.lines().take(8) {
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(tone(Color::DarkGray)),
                    )));
                }
            }
        }
    }

    if let Some(status) = &activity.status {
        lines.push(Line::from(vec![
            Span::styled("  Status  : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                status.clone(),
                Style::default()
                    .fg(tone(status_color))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    lines.push(Line::from(""));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let footer_height = if inner.height > 0 { 1 } else { 0 };
    let header_height = (lines.len() as u16).min(inner.height.saturating_sub(footer_height));
    let footer_height = footer_height.min(inner.height.saturating_sub(header_height));
    let output_height = inner
        .height
        .saturating_sub(header_height)
        .saturating_sub(footer_height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Length(output_height),
            Constraint::Length(footer_height),
        ])
        .split(inner);

    if header_height > 0 {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);
    }
    if output_height > 0 {
        activity.terminal.resize(chunks[1].height, chunks[1].width);
        let mut term = activity.terminal.term.lock();
        render_term_buffer(frame, chunks[1], &mut *term, dimmed, false, false, 0);
    }
    if footer_height > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  [^C] Cancel request   [Esc/^B] Back to sidebar",
                Style::default().fg(tone(Color::DarkGray)),
            ))),
            chunks[2],
        );
    }
}

fn hostdo_timeout_label(activity: &crate::activity::Activity, timeout_secs: u64) -> String {
    if activity.command_started_at.is_some() {
        return format!("{timeout_secs}s command-only");
    }

    match &activity.state {
        crate::activity::ActivityState::PullingImage => {
            format!("{timeout_secs}s command-only, starts after image is ready")
        }
        crate::activity::ActivityState::PendingApproval => {
            format!("{timeout_secs}s command-only, starts after approval")
        }
        state if state.is_terminal() => {
            format!("{timeout_secs}s command-only, command did not run")
        }
        _ => format!("{timeout_secs}s command-only, starts when command runs"),
    }
}

fn activity_status_color(state: &crate::activity::ActivityState) -> Color {
    match state {
        crate::activity::ActivityState::PendingApproval => Color::Yellow,
        crate::activity::ActivityState::PullingImage => Color::Yellow,
        crate::activity::ActivityState::Running => Color::Yellow,
        crate::activity::ActivityState::Forwarding => Color::Yellow,
        crate::activity::ActivityState::Complete => Color::Green,
        crate::activity::ActivityState::Failed => Color::Red,
        crate::activity::ActivityState::Denied => Color::Red,
        crate::activity::ActivityState::Cancelled => Color::Red,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{Activity, ActivityKind, ActivityState};
    use std::sync::{Arc, atomic::AtomicBool};

    #[test]
    fn activity_status_color_uses_progress_success_failure_palette() {
        assert_eq!(
            activity_status_color(&ActivityState::PendingApproval),
            Color::Yellow
        );
        assert_eq!(
            activity_status_color(&ActivityState::PullingImage),
            Color::Yellow
        );
        assert_eq!(
            activity_status_color(&ActivityState::Running),
            Color::Yellow
        );
        assert_eq!(
            activity_status_color(&ActivityState::Forwarding),
            Color::Yellow
        );
        assert_eq!(
            activity_status_color(&ActivityState::Complete),
            Color::Green
        );
        assert_eq!(activity_status_color(&ActivityState::Failed), Color::Red);
        assert_eq!(activity_status_color(&ActivityState::Denied), Color::Red);
        assert_eq!(activity_status_color(&ActivityState::Cancelled), Color::Red);
    }

    #[test]
    fn hostdo_timeout_label_explains_command_only_timer() {
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

        assert_eq!(
            hostdo_timeout_label(&activity, 120),
            "120s command-only, starts after image is ready"
        );

        activity.mark_command_started(std::time::Instant::now());
        assert_eq!(hostdo_timeout_label(&activity, 120), "120s command-only");
    }
}

// ── Idle screen ───────────────────────────────────────────────────────────────

pub(crate) fn render_idle(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Select a workspace and press [↵] to launch a container.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Select a running container and press [↵] to attach.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

pub(crate) fn render_project_settings(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    project_idx: usize,
    dimmed: bool,
) {
    let cfg = app.config.get();
    let Some(proj) = cfg.workspaces.get(project_idx) else {
        render_idle(frame, area);
        return;
    };

    let tone = |c| maybe_dim(c, dimmed);
    let focused = app.focus == Focus::Settings;
    let border_style = if focused {
        Style::default().fg(tone(Color::Cyan))
    } else {
        Style::default().fg(tone(Color::DarkGray))
    };
    let block = Block::default()
        .title(format!(" {} Settings ", proj.name))
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(border_style);

    let actions = App::settings_action_rows_for();
    let cursor = if actions.is_empty() {
        0
    } else {
        app.settings_cursor.min(actions.len().saturating_sub(1))
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Canonical repo: ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                proj.canonical_path.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (i, action) in actions.iter().enumerate() {
        let selected = focused && i == cursor;
        let marker = if selected { "▶ " } else { "  " };
        let name_style = if selected {
            Style::default()
                .fg(tone(Color::White))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone(Color::White))
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {marker}"),
                Style::default().fg(tone(Color::Cyan)),
            ),
            Span::styled(format!("[{}] {}", action.key, action.label), name_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("      {}", action.desc),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

    let rules_path = proj.canonical_path.join("harness-rules.toml");
    let rules_status: Vec<Span> = if !rules_path.exists() {
        vec![
            Span::styled(
                "  harness-rules.toml: ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled("Not Found", Style::default().fg(tone(Color::Yellow))),
        ]
    } else {
        match crate::rules::load(&rules_path) {
            Ok(r) => vec![
                Span::styled(
                    "  harness-rules.toml: ",
                    Style::default().fg(tone(Color::DarkGray)),
                ),
                Span::styled("Loaded", Style::default().fg(tone(Color::Green))),
                Span::styled(
                    format!(
                        "  hostdo: {}, network: {}",
                        r.hostdo.commands.len(),
                        r.network.allowlist.len() + r.network.denylist.len()
                    ),
                    Style::default().fg(tone(Color::White)),
                ),
            ],
            Err(_) => vec![
                Span::styled(
                    "  harness-rules.toml: ",
                    Style::default().fg(tone(Color::DarkGray)),
                ),
                Span::styled("Error", Style::default().fg(tone(Color::Red))),
            ],
        }
    };

    lines.push(Line::from(""));
    lines.push(Line::from(rules_status));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [^B] Back to sidebar",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Terminal view ─────────────────────────────────────────────────────────────

pub(crate) fn render_terminal_fullscreen_header(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    title_style: Style,
) {
    frame.render_widget(
        Paragraph::new(Span::styled(title.to_string(), title_style)),
        area,
    );
}

pub(crate) fn render_terminal_title_hint(frame: &mut Frame, area: Rect, in_scroll_mode: bool) {
    if area.width <= 2 || area.height == 0 {
        return;
    }
    let hint = if in_scroll_mode {
        "[Esc/q] exit scroll"
    } else {
        "[^S] scroll  [Esc/^B] sidebar"
    };
    let hint_style = if in_scroll_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let hint_area = Rect::new(
        area.x.saturating_add(1),
        area.y,
        area.width.saturating_sub(2),
        1,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(hint, hint_style)).alignment(Alignment::Right),
        hint_area,
    );
}
