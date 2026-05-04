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

    if app.build_is_running() && build_output_is_selected(app) {
        render_build_output(frame, app, area, false);
        return;
    }

    if app.focus == Focus::ImageBuild {
        render_image_build(frame, app, area, false);
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
                        r.network.allowlist.len()
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
        "[Ctrl+S] for scroll"
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
