use super::*;

pub(crate) fn render_container_picker(frame: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let selected_ctr = app.container_picker.unwrap_or(0);
    let project_name = app
        .selected_project_idx()
        .and_then(|pi| app.projects.get(pi))
        .map(|p| p.name.as_str())
        .unwrap_or("(no project)");

    let tone = |c| maybe_dim(c, dimmed);
    let workspace_path = app
        .selected_project_idx()
        .and_then(|pi| cfg.projects.get(pi))
        .map(|proj| crate::config::effective_workspace_path(proj, &cfg.workspace));
    let block = Block::default()
        .title(format!(" Run Container for '{}' ", project_name))
        .title_style(
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::Cyan)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Choose an agent to launch below. Your host dir ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                workspace_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<workspace>".to_string()),
                Style::default()
                    .fg(tone(Color::White))
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                " will be mounted inside the agent container at ",
                Style::default().fg(tone(Color::DarkGray)),
            ),
            Span::styled(
                "/workspace",
                Style::default()
                    .fg(tone(Color::White))
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                ", and the agent will start automatically.",
                Style::default().fg(tone(Color::DarkGray)),
            ),
        ]),
        Line::from(""),
    ];

    for (i, c) in cfg.containers.iter().enumerate() {
        let marker = if i == selected_ctr { "▶ " } else { "  " };
        let name_style = if i == selected_ctr {
            Style::default()
                .fg(tone(Color::White))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tone(Color::White))
        };

        let spans = vec![
            Span::styled(
                format!("  {marker}"),
                Style::default().fg(tone(Color::Cyan)),
            ),
            Span::styled(c.name.clone(), name_style),
        ];
        lines.push(Line::from(spans));

        lines.push(Line::from(Span::styled(
            format!("      {}", c.image),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [^B] Back to sidebar",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Image build pane ─────────────────────────────────────────────────────────

pub(crate) fn render_image_build(frame: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let ctr_idx = app.build_container_idx.unwrap_or(0);
    let image = cfg
        .containers
        .get(ctr_idx)
        .map(|c| c.image.as_str())
        .unwrap_or("<unknown>");

    let docker_dir = cfg.docker_dir.as_path();
    let (base_cmd, agent_cmd) = App::build_commands_for(docker_dir, image);
    let base_cmd_str = format!("docker {}", base_cmd.join(" "));
    let agent_cmd_str = agent_cmd
        .as_ref()
        .map(|cmd| format!("docker {}", cmd.join(" ")));

    let parts: Vec<&str> = image.splitn(2, ':').collect();
    let name = parts[0].split('/').last().unwrap_or(parts[0]);
    let tag = parts.get(1).copied().unwrap_or("ubuntu-24.04");
    let dockerfile_root = docker_dir;
    let base_dockerfile = dockerfile_root.join(format!("{tag}.Dockerfile"));
    let agent_dockerfile = name.strip_prefix("void-claw-").map(|agent| {
        dockerfile_root
            .join(agent)
            .join(format!("{tag}.Dockerfile"))
    });

    let tone = |c| maybe_dim(c, dimmed);
    let focused = app.focus == Focus::ImageBuild;
    let border_style = if focused {
        Style::default().fg(tone(Color::Cyan))
    } else {
        Style::default().fg(tone(Color::DarkGray))
    };

    let block = Block::default()
        .title(" Image Build Required ")
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(border_style);

    let cursor = app.build_cursor;

    let run_all_cmd_str = match agent_cmd_str.as_ref() {
        Some(agent_cmd_str) => format!("{base_cmd_str} && {agent_cmd_str}"),
        None => base_cmd_str.clone(),
    };
    let actions: [(&str, &str, Option<&str>, &str); 2] = [
        (
            "r",
            "Run all build commands and launch container (Recommended)",
            Some(&run_all_cmd_str),
            "",
        ),
        ("c", "Cancel", None, "Return to sidebar"),
    ];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Image '{image}' was not found locally."),
            Style::default().fg(tone(Color::Yellow)),
        )),
        Line::from(Span::styled(
            "  Docker images must be built before containers can be launched.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Dockerfiles",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Base  : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                base_dockerfile.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Agent : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                agent_dockerfile
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "(n/a for custom image tag)".to_string()),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Select an action to run, or copy the commands below to run manually.",
            Style::default().fg(tone(Color::DarkGray)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions",
            Style::default()
                .fg(tone(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (i, (label, name, cmd, desc)) in actions.iter().enumerate() {
        let selected = i == cursor;
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
            Span::styled(format!("{label}) "), Style::default().fg(tone(Color::Cyan))),
            Span::styled(*name, name_style),
        ]));
        if let Some(cmd) = cmd {
            lines.push(Line::from(vec![
                Span::styled("      $ ", Style::default().fg(tone(Color::Green))),
                Span::styled(*cmd, Style::default().fg(tone(Color::DarkGray))),
            ]));
        }
        lines.push(Line::from(Span::styled(
            format!("      {desc}"),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

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

pub(crate) fn render_build_output(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let image = app
        .build_container_idx
        .and_then(|idx| cfg.containers.get(idx))
        .map(|c| c.image.as_str())
        .unwrap_or("<unknown>");
    let tone = |c| maybe_dim(c, dimmed);
    let focused = app.focus == Focus::ImageBuild;
    let border_style = if focused {
        Style::default().fg(tone(Color::Cyan))
    } else {
        Style::default().fg(tone(Color::DarkGray))
    };

    let block = Block::default()
        .title(format!(" docker build {image} "))
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut header_lines: Vec<Line> = vec![];
    let max_cols = inner.width.saturating_sub(1) as usize;
    if let Some(cmd) = app.active_build_command() {
        let cmd = clamp_for_width(&strip_ansi_and_control(cmd), max_cols);
        header_lines.push(Line::from(vec![
            Span::styled("$ ", Style::default().fg(tone(Color::Green))),
            Span::styled(cmd, Style::default().fg(tone(Color::DarkGray))),
        ]));
        header_lines.push(Line::from(""));
    }

    let visible_rows = (inner.height as usize).saturating_sub(header_lines.len());
    let total = app.build_output.len();
    let max_scroll = total.saturating_sub(visible_rows);
    let scroll = app.build_scroll.min(max_scroll);
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows);

    let mut lines = header_lines;
    for (line, is_error) in app.build_output.iter().skip(start).take(end - start) {
        let clean = clamp_for_width(&strip_ansi_and_control(line), max_cols);
        lines.push(Line::from(Span::styled(
            clean,
            Style::default().fg(if *is_error {
                tone(Color::Red)
            } else {
                tone(Color::White)
            }),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    if app.build_scroll > 0 && max_scroll > 0 {
        render_scrollbar(frame, inner, max_scroll, scroll, true);
    }
}

pub(crate) fn strip_ansi_and_control(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        if ch == '\r' {
            continue;
        }
        if ch.is_control() && ch != '\t' {
            continue;
        }
        if ch == '\t' {
            out.push_str("    ");
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn clamp_for_width(input: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let mut out = String::new();
    for (i, ch) in input.chars().enumerate() {
        if i >= max_cols {
            break;
        }
        out.push(ch);
    }
    out
}
