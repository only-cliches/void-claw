use super::*;

pub(crate) fn render_log(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let lines: Vec<Line> = app
        .log
        .iter()
        .map(|entry| match entry {
            LogEntry::Audit(e) => {
                let ts = e.timestamp.format("%H:%M:%S").to_string();
                let decision_color = match e.decision {
                    DecisionKind::Auto => Color::Green,
                    DecisionKind::Approved | DecisionKind::Remembered => Color::Cyan,
                    DecisionKind::Denied
                    | DecisionKind::DeniedByPolicy
                    | DecisionKind::TimedOut => Color::Red,
                };
                let exit_str = match e.exit_code {
                    Some(c) => format!(" exit={c}"),
                    None => String::new(),
                };
                Line::from(vec![
                    Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:<6} ", e.decision.as_str()),
                        Style::default()
                            .fg(decision_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<16} ", e.project),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(e.argv.join(" ")),
                    Span::styled(exit_str, Style::default().fg(Color::DarkGray)),
                ])
            }
            LogEntry::Msg {
                text,
                is_error,
                timestamp,
            } => {
                let ts = timestamp.format("%H:%M:%S").to_string();
                let (prefix, color) = if *is_error {
                    ("ERR   ", Color::Red)
                } else {
                    ("INFO  ", Color::Green)
                };
                Line::from(vec![
                    Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{prefix:<6} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(text.clone(), Style::default().fg(Color::White)),
                ])
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((app.log_scroll as u16, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ── Status bar ────────────────────────────────────────────────────────────────

pub(crate) fn render_status_bar(frame: &mut Frame, app: &mut App, area: Rect) {
    let keys = match app.focus {
        Focus::Sidebar => {
            if app.build_is_running() {
                " [↑↓/jk]navigate  [↵/l]select  [^C]cancel build  [o]log  [^Q]quit"
            } else {
                " [↑↓/jk]navigate  [↵/l]select  [o]log  [^Q]quit"
            }
        }
        Focus::Terminal if app.scroll_mode => {
            " SCROLL: [↑↓/jk]line  [PgUp/PgDn]page  [g/G]top/bottom  [Esc/q]exit scroll"
        }
        Focus::Terminal => {
            " [wheel]scroll  [^S]scroll  [^B]sidebar  [Alt+o]log  [^Q]quit  (keys forwarded to container)"
        }
        Focus::Settings => " [↑↓/jk]navigate  [↵/l]select  [^B]back  [^Q]quit",
        Focus::ContainerPicker => " [↑↓/jk]navigate  [↵/l]launch  [^B]back  [^Q]quit",
        Focus::ImageBuild => {
            " [r]run+launch  [c]cancel  [↑↓/jk]navigate  [↵/l]select  [^B]sidebar  [^Q]quit"
        }
        Focus::NewProject => {
            " [↑↓/jk]navigate  [type]edit  [←→]cycle  [↵/l]select  [Esc/^B]back  [^Q]quit"
        }
    };
    frame.render_widget(
        Paragraph::new(Span::styled(keys, Style::default().fg(Color::DarkGray))),
        area,
    );
}

// ── New project pane ─────────────────────────────────────────────────────────

pub(crate) fn render_new_project(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let Some(state) = app.new_project.as_ref() else {
        render_idle(frame, area);
        return;
    };

    let tone = |c| maybe_dim(c, dimmed);
    let block = Block::default()
        .title(" New Project ")
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::Cyan)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sync_mode = match state.sync_mode {
        crate::config::SyncMode::WorkspaceOnly => "workspace_only",
        crate::config::SyncMode::Pushback => "pushback",
        crate::config::SyncMode::Bidirectional => "bidirectional",
        crate::config::SyncMode::Pullthrough => "pullthrough",
        crate::config::SyncMode::Direct => "direct",
    };

    let rows: [(&str, String); 6] = [
        ("Project name", state.name.clone()),
        ("Canonical dir", state.canonical_dir.clone()),
        ("Sync mode", sync_mode.to_string()),
        (
            "Project type",
            state.project_type.display_name().to_string(),
        ),
        ("Create", "Add project + write rules".to_string()),
        ("Cancel", "Back to sidebar".to_string()),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Config: {}", app.loaded_config_path.display()),
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(Span::styled(
            "  Writes canonical/zero-rules.toml only if it does not exist.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Fields",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (i, (label, value)) in rows.iter().enumerate() {
        let selected = i == state.cursor;
        let marker = if selected { "▶ " } else { "  " };
        let label_style = if selected {
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
            Span::styled(
                format!("{label}: "),
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(value.clone(), label_style),
        ]));
    }

    if let Some(err) = state.error.as_ref() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default()
                .fg(tone(Color::Red))
                .add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Esc] Cancel  [Enter] Create/Select  [←→] Cycle lists",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub(crate) fn render_new_project_preview(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let tone = |c| maybe_dim(c, dimmed);
    let block = Block::default()
        .title(" New Project ")
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::DarkGray)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sync_mode = match cfg.defaults.sync.mode {
        crate::config::SyncMode::WorkspaceOnly => "workspace_only",
        crate::config::SyncMode::Pushback => "pushback",
        crate::config::SyncMode::Bidirectional => "bidirectional",
        crate::config::SyncMode::Pullthrough => "pullthrough",
        crate::config::SyncMode::Direct => "direct",
    };

    let rows: [(&str, &str); 6] = [
        ("Project name", "<empty>"),
        ("Canonical dir", "<empty>"),
        ("Sync mode", sync_mode),
        (
            "Project type",
            crate::new_project::ProjectType::None.display_name(),
        ),
        ("Create", "Add project + write rules"),
        ("Cancel", "Back to sidebar"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Config: {}", app.loaded_config_path.display()),
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(Span::styled(
            "  Press [Enter] to open the form in edit mode.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Fields",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (label, value) in rows {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default().fg(tone(Color::Cyan))),
            Span::styled(
                format!("{label}: "),
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(value.to_string(), Style::default().fg(tone(Color::White))),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Container picker pane ─────────────────────────────────────────────────────
