use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags as TermFlags;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Rgb};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{App, Focus, LogEntry, SidebarItem};
use crate::state::DecisionKind;

const LOG_HEIGHT: u16 = 6;
const STATUS_HEIGHT: u16 = 1;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if app.log_fullscreen {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(STATUS_HEIGHT)])
            .split(area);
        render_log_fullscreen(frame, app, split[0]);
        render_status_bar_log(frame, app, split[1]);
        return;
    }

    if app.terminal_fullscreen {
        if let Some(si) = app.active_session.filter(|&si| si < app.sessions.len()) {
            let has_modal = !app.pending_net.is_empty()
                || app
                    .active_session
                    .map(|active| !app.pending_for_session(active).is_empty())
                    .unwrap_or(false);

            render_terminal(frame, app, area, si, has_modal, true);
            render_terminal_overlays(frame, app, area, si);
        } else {
            render_idle(frame, area);
        }
        return;
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(LOG_HEIGHT),
            Constraint::Length(STATUS_HEIGHT),
        ])
        .split(area);

    let main_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.config.get().defaults.ui.sidebar_width.max(1)),
            Constraint::Min(0),
        ])
        .split(outer[0]);

    render_sidebar(frame, app, main_row[0]);
    render_right_pane(frame, app, main_row[1]);
    render_log(frame, app, outer[1]);
    render_status_bar(frame, app, outer[2]);
}

fn render_scrollbar(
    frame: &mut Frame,
    track_area: Rect,
    max_scroll: usize,
    current_scroll: usize,
    invert: bool,
) {
    if max_scroll == 0 || track_area.height == 0 {
        return;
    }
    let track_h = track_area.height as usize;
    let total_content = max_scroll + track_h;
    let thumb_size = (track_h * track_h / total_content).max(1);
    let track_range = track_h.saturating_sub(thumb_size);
    let thumb_top = if invert {
        track_range.saturating_sub(current_scroll * track_range / max_scroll)
    } else {
        current_scroll * track_range / max_scroll
    };
    let x = track_area.right().saturating_sub(1);
    for row in 0..track_h {
        let in_thumb = row >= thumb_top && row < thumb_top + thumb_size;
        let (ch, style) = if in_thumb {
            ("┃", Style::default().fg(Color::Yellow))
        } else {
            ("│", Style::default().fg(Color::DarkGray))
        };
        let bar_area = Rect::new(x, track_area.y + row as u16, 1, 1);
        frame.render_widget(Span::styled(ch, style), bar_area);
    }
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn render_sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Sidebar;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Projects ")
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);

    let items = app.sidebar_items();
    if !items.is_empty() {
        let visible = area.height.saturating_sub(2).max(1) as usize;
        let selected = app.sidebar_idx.min(items.len().saturating_sub(1));
        let max_offset = items.len().saturating_sub(visible);

        let mut offset = app.sidebar_offset.min(max_offset);
        if selected < offset {
            offset = selected;
        } else if selected >= offset.saturating_add(visible) {
            offset = selected.saturating_add(1).saturating_sub(visible);
        }
        app.sidebar_offset = offset.min(max_offset);
    } else {
        app.sidebar_offset = 0;
    }
    let cfg = app.config.get();
    let visible = area.height.saturating_sub(2).max(1) as usize;
    let offset = app.sidebar_offset.min(items.len());
    let list_items: Vec<ListItem> = items
        .iter()
        .skip(offset)
        .take(visible)
        .map(|item| match item {
            SidebarItem::Project(pi) => {
                let proj = &app.projects[*pi];
                let sync_suffix = match &proj.last_report {
                    Some(r) => format!(" {}", r.timestamp.format("%H:%M")),
                    None => String::new(),
                };
                let is_direct = cfg
                    .projects
                    .get(*pi)
                    .map(|p| {
                        crate::config::effective_sync_mode(p, &cfg.defaults)
                            == crate::config::SyncMode::Direct
                    })
                    .unwrap_or(false);
                let (dot, dot_color) = if is_direct {
                    ("●", Color::Green)
                } else {
                    match app.project_watch_spinner(*pi) {
                        Some(frame) => (frame, Color::Green),
                        None => ("○", Color::DarkGray),
                    }
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{dot} "), Style::default().fg(dot_color)),
                    Span::styled(
                        proj.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(sync_suffix, Style::default().fg(Color::DarkGray)),
                ]))
            }
            SidebarItem::Session(si) => {
                let session = &app.sessions[*si];
                let is_active = app.active_session == Some(*si);
                let (prefix, name_color) = if session.is_exited() {
                    ("  ✗ ", Color::DarkGray)
                } else if is_active {
                    ("  ▶ ", Color::Cyan)
                } else {
                    ("  · ", Color::White)
                };
                let bell = session.has_bell();
                let short_id: String = session.docker_name.chars().take(12).collect();
                let mut spans = vec![
                    Span::styled(prefix, Style::default().fg(name_color)),
                    Span::styled(
                        session.container_name.clone(),
                        Style::default().fg(name_color),
                    ),
                    Span::styled(format!(" {short_id}"), Style::default().fg(Color::DarkGray)),
                ];
                if bell {
                    spans.push(Span::styled(
                        " [!]",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                ListItem::new(Line::from(spans))
            }
            SidebarItem::Settings(_) => ListItem::new(Line::from(vec![
                Span::styled("  ⚙ ", Style::default().fg(Color::Yellow)),
                Span::styled("Settings", Style::default().fg(Color::DarkGray)),
            ])),
            SidebarItem::Launch(_) => ListItem::new(Line::from(vec![
                Span::styled("  + ", Style::default().fg(Color::Green)),
                Span::styled("Run Container...", Style::default().fg(Color::DarkGray)),
            ])),
            SidebarItem::Build(_) => {
                let image = app
                    .build_container_idx
                    .and_then(|idx| cfg.containers.get(idx))
                    .map(|c| c.image.as_str())
                    .unwrap_or("<unknown>");
                let marker = if app.build_is_running() {
                    loading_spinner_frame()
                } else {
                    "$"
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {marker} "), Style::default().fg(Color::Yellow)),
                    Span::styled("docker build", Style::default().fg(Color::Yellow)),
                    Span::styled(format!("  {image}"), Style::default().fg(Color::DarkGray)),
                ]))
            }
            SidebarItem::NewProject => ListItem::new(Line::from(vec![
                Span::styled("+ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "New Project...",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])),
        })
        .collect();

    let mut list_state = ListState::default();
    if !items.is_empty() {
        let selected = app.sidebar_idx.min(items.len().saturating_sub(1));
        // Project rows are non-selectable section headers. If the app state ever points at one
        // (e.g. via older persisted state), render with no highlight.
        if matches!(items.get(selected), Some(SidebarItem::Project(_))) {
            list_state.select(None);
        } else {
            let rel_selected = selected.saturating_sub(offset);
            list_state.select(Some(rel_selected.min(list_items.len().saturating_sub(1))));
        }
    }

    frame.render_stateful_widget(
        List::new(list_items)
            .block(block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(""),
        area,
        &mut list_state,
    );

    if items.len() > visible && inner.height > 0 {
        let max_offset = items.len().saturating_sub(visible).max(1);
        let offset = app.sidebar_offset.min(max_offset);
        render_scrollbar(frame, inner, max_offset, offset, false);
    }
}

// ── Right pane ────────────────────────────────────────────────────────────────

fn render_right_pane(frame: &mut Frame, app: &mut App, area: Rect) {
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
            Some(SidebarItem::Build(_)) if app.build_is_running() && build_output_is_selected(app) => {
                render_build_output(frame, app, area, true);
            }
            Some(SidebarItem::NewProject) => {
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

    if app.focus == Focus::NewProject {
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

fn render_terminal_overlays(frame: &mut Frame, app: &mut App, area: Rect, session_idx: usize) {
    let pending_exec = app.pending_for_session(session_idx);
    if !pending_exec.is_empty() {
        render_exec_approval_overlay(frame, app, area, pending_exec[0]);
        return;
    }

    if !app.pending_net.is_empty() {
        render_net_approval_overlay(frame, app, area);
    }
}

fn build_output_is_selected(app: &App) -> bool {
    matches!(
        app.sidebar_items().get(app.sidebar_idx),
        Some(SidebarItem::Build(_))
    )
}

// ── Idle screen ───────────────────────────────────────────────────────────────

fn render_idle(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Select a project and press [↵] to launch a container.",
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

fn render_project_settings(frame: &mut Frame, app: &App, area: Rect, project_idx: usize, dimmed: bool) {
    let cfg = app.config.get();
    let Some(proj) = cfg.projects.get(project_idx) else {
        render_idle(frame, area);
        return;
    };

    let workspace_path = crate::config::effective_workspace_path(proj, &cfg.workspace);
    let mode = crate::config::effective_sync_mode(proj, &cfg.defaults);
    let watching = app.is_project_watching(project_idx);
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

    let actions = App::settings_action_rows_for(mode.clone(), watching);
    let cursor = if actions.is_empty() {
        0
    } else {
        app.settings_cursor.min(actions.len().saturating_sub(1))
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Canonical repo: ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                proj.canonical_path.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Workspace dir : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                workspace_path.display().to_string(),
                Style::default().fg(tone(Color::White)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Sync mode     : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(mode.to_string(), Style::default().fg(tone(Color::White))),
        ]),
        Line::from(vec![
            Span::styled("  File watch    : ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled(
                if watching { "enabled" } else { "disabled" },
                Style::default().fg(if watching {
                    tone(Color::Green)
                } else {
                    tone(Color::DarkGray)
                }),
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
            Span::styled(format!("  {marker}"), Style::default().fg(tone(Color::Cyan))),
            Span::styled(format!("[{}] {}", action.key, action.label), name_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("      {}", action.desc),
            Style::default().fg(tone(Color::DarkGray)),
        )));
    }

    let rules_path = proj.canonical_path.join("void-claw-rules.toml");
    let rules_status: Vec<Span> = if !rules_path.exists() {
        vec![
            Span::styled("  void-claw-rules.toml: ", Style::default().fg(tone(Color::DarkGray))),
            Span::styled("Not Found", Style::default().fg(tone(Color::Yellow))),
        ]
    } else {
        match crate::rules::load(&rules_path) {
            Ok(r) => vec![
                Span::styled("  void-claw-rules.toml: ", Style::default().fg(tone(Color::DarkGray))),
                Span::styled("Loaded", Style::default().fg(tone(Color::Green))),
                Span::styled(
                    format!(
                        "  hostdo: {}, network: {}",
                        r.hostdo.commands.len(),
                        r.network.rules.len()
                    ),
                    Style::default().fg(tone(Color::White)),
                ),
            ],
            Err(_) => vec![
                Span::styled("  void-claw-rules.toml: ", Style::default().fg(tone(Color::DarkGray))),
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

fn render_terminal_fullscreen_header(frame: &mut Frame, area: Rect, title: &str, title_style: Style) {
    let exit_hint = " CTRL+G to exit ";
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(exit_hint.len() as u16),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(title.to_string(), title_style)),
        split[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            exit_hint,
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right),
        split[1],
    );
}

fn render_terminal_title_hint(frame: &mut Frame, area: Rect) {
    let hint = " CTRL+G to fullscreen";
    let hint_width = hint.len() as u16;
    if area.width <= hint_width {
        return;
    }
    let hint_area = Rect::new(area.x + area.width - hint_width, area.y, hint_width, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right),
        hint_area,
    );
}

fn render_terminal(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    session_idx: usize,
    dimmed: bool,
    fullscreen: bool,
) {
    let (term, container_id, tab_label, session_exited) = match app.sessions.get(session_idx) {
        Some(s) => (
            std::sync::Arc::clone(&s.term),
            s.container_id.clone(),
            s.tab_label(),
            s.is_exited(),
        ),
        None => return,
    };

    let focused = app.focus == Focus::Terminal;
    let in_scroll_mode = focused && app.scroll_mode;
    let border_style = if in_scroll_mode {
        Style::default().fg(Color::Yellow)
    } else if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let short_id = if container_id.len() > 12 {
        &container_id[..12]
    } else {
        &container_id
    };
    let tab_title = if in_scroll_mode {
        format!(" {} [{}] -- SCROLL -- ", tab_label, short_id)
    } else {
        format!(" {} [{}] ", tab_label, short_id)
    };
    let title_style = if in_scroll_mode {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    };
    let content_area = if fullscreen {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        render_terminal_fullscreen_header(frame, split[0], tab_title.as_str(), title_style);
        split[1]
    } else {
        area
    };

    if content_area.height == 0 || content_area.width == 0 {
        return;
    }

    let block = if fullscreen {
        Block::default()
    } else {
        Block::default()
            .title(tab_title.as_str())
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_style(border_style)
    };

    let inner = if fullscreen { content_area } else { block.inner(content_area) };
    frame.render_widget(block, content_area);
    if focused && !fullscreen {
        render_terminal_title_hint(frame, content_area);
    }

    if let Some(session) = app.sessions.get_mut(session_idx) {
        let _ = session.resize(inner.height, inner.width);
    }

    let mut term = term.lock();
    if !session_exited && !term_has_content(&term) {
        let spinner = loading_spinner_frame();
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("{spinner} Starting container..."),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Waiting for terminal output",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
        return;
    }

    let desired_offset = if app.scroll_mode {
        app.terminal_scroll
    } else {
        0
    };
    let max_scrollback = term.history_size();
    let desired_offset = desired_offset.min(max_scrollback);
    let current_offset = term.grid().display_offset();
    if desired_offset != current_offset {
        let delta = desired_offset as i32 - current_offset as i32;
        term.scroll_display(Scroll::Delta(delta));
    }
    let actual_scroll = term.grid().display_offset();

    let rows = inner.height as usize;
    let cols = inner.width as usize;
    let mut content = term.renderable_content();

    let default_fg = resolve_ansi_color(AnsiColor::Named(NamedColor::Foreground), content.colors);
    let default_bg = resolve_ansi_color(AnsiColor::Named(NamedColor::Background), content.colors);
    let mut default_style = Style::default().fg(default_fg).bg(default_bg);
    if dimmed {
        if let Some(fg) = default_style.fg {
            default_style = default_style.fg(attenuate_color(fg));
        }
        if let Some(bg) = default_style.bg {
            default_style = default_style.bg(attenuate_color(bg));
        }
    }

    let cursor_point = content.cursor.point;
    let show_cursor = focused
        && !dimmed
        && actual_scroll == 0
        && content
            .mode
            .contains(alacritty_terminal::term::TermMode::SHOW_CURSOR);

    #[derive(Clone)]
    struct CellOut {
        ch: char,
        style: Style,
        skip: bool,
    }

    let mut grid: Vec<CellOut> = vec![
        CellOut {
            ch: ' ',
            style: default_style,
            skip: false,
        };
        rows * cols
    ];

    for indexed in content.display_iter.by_ref() {
        let Some(vp) =
            alacritty_terminal::term::point_to_viewport(content.display_offset, indexed.point)
        else {
            continue;
        };
        let row = vp.line;
        let col = vp.column.0;
        if col >= cols {
            continue;
        }
        let row_offset = term.screen_lines().saturating_sub(rows);
        if row < row_offset || row >= row_offset + rows {
            continue;
        }
        let rr = row - row_offset;
        let idx = rr * cols + col;

        let cell = indexed.cell;
        let mut ch = cell.c;
        let skip = cell.flags.contains(TermFlags::WIDE_CHAR_SPACER);
        if cell.flags.contains(TermFlags::HIDDEN) {
            ch = ' ';
        }

        let mut fg_src = cell.fg;
        let bg_src = cell.bg;
        let missing_default_palette = content.colors[NamedColor::Foreground].is_none()
            && content.colors[NamedColor::Background].is_none();
        if missing_default_palette
            && matches!(fg_src, AnsiColor::Spec(Rgb { r: 0, g: 0, b: 0 }))
            && matches!(bg_src, AnsiColor::Named(NamedColor::Background))
            && cell.flags.contains(TermFlags::BOLD)
        {
            fg_src = AnsiColor::Named(NamedColor::Foreground);
        }
        if cell.flags.contains(TermFlags::BOLD)
            && !cell.flags.contains(TermFlags::DIM)
            && !cell.flags.contains(TermFlags::DIM_BOLD)
        {
            fg_src = brighten_bold_ansi_color(fg_src);
        }

        let mut fg = resolve_ansi_color(fg_src, content.colors);
        let mut bg = resolve_ansi_color(bg_src, content.colors);
        if cell.flags.contains(TermFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        let mut style = Style::default().fg(fg).bg(bg);
        if cell.flags.contains(TermFlags::BOLD) {
            style = style.add_modifier(Modifier::BOLD);
        }
        if cell.flags.contains(TermFlags::ITALIC) {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if cell.flags.contains(TermFlags::ALL_UNDERLINES) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if cell.flags.contains(TermFlags::STRIKEOUT) {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if cell.flags.contains(TermFlags::DIM) || cell.flags.contains(TermFlags::DIM_BOLD) {
            style = style.add_modifier(Modifier::DIM);
            if let Some(fg) = style.fg {
                style = style.fg(attenuate_color(fg));
            }
            if let Some(bg) = style.bg {
                style = style.bg(attenuate_color(bg));
            }
        }
        if dimmed {
            if let Some(fg) = style.fg {
                style = style.fg(attenuate_color(fg));
            }
            if let Some(bg) = style.bg {
                style = style.bg(attenuate_color(bg));
            }
        }

        if show_cursor && indexed.point == cursor_point && rr < rows && col < cols {
            style = style.add_modifier(Modifier::REVERSED);
        }

        grid[idx] = CellOut { ch, style, skip };
    }

    let mut rendered: Vec<Line> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut spans: Vec<Span> = Vec::new();
        let mut cur_style: Option<Style> = None;
        let mut cur_text = String::new();
        for c in 0..cols {
            let cell = &grid[r * cols + c];
            if cell.skip {
                continue;
            }
            if cur_style == Some(cell.style) {
                cur_text.push(cell.ch);
            } else {
                if let Some(style) = cur_style.take() {
                    spans.push(Span::styled(std::mem::take(&mut cur_text), style));
                }
                cur_style = Some(cell.style);
                cur_text.push(cell.ch);
            }
        }
        if let Some(style) = cur_style.take() {
            spans.push(Span::styled(cur_text, style));
        }
        rendered.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(rendered), inner);

    if app.scroll_mode && max_scrollback > 0 {
        render_scrollbar(frame, inner, max_scrollback, actual_scroll, true);
    }
}

fn resolve_ansi_color(color: AnsiColor, colors: &alacritty_terminal::term::color::Colors) -> Color {
    match color {
        AnsiColor::Spec(Rgb { r, g, b }) => Color::Rgb(r, g, b),
        AnsiColor::Named(named) => {
            if let Some(rgb) = colors[named] {
                if matches!(named, NamedColor::Foreground | NamedColor::BrightForeground) {
                    let fg_is_blackish = rgb.r <= 0x10 && rgb.g <= 0x10 && rgb.b <= 0x10;
                    let bg_is_blackish = colors[NamedColor::Background]
                        .map(|bg| bg.r <= 0x10 && bg.g <= 0x10 && bg.b <= 0x10)
                        .unwrap_or(true);
                    if fg_is_blackish && bg_is_blackish {
                        return Color::Rgb(0xff, 0xff, 0xff);
                    }
                }
                return Color::Rgb(rgb.r, rgb.g, rgb.b);
            }
            match named {
                NamedColor::Foreground => Color::White,
                NamedColor::Background => Color::Black,
                NamedColor::BrightForeground => Color::White,
                NamedColor::DimForeground => Color::Gray,
                _ => Color::Reset,
            }
        }
        AnsiColor::Indexed(idx) => {
            if let Some(rgb) = colors[idx as usize] {
                return Color::Rgb(rgb.r, rgb.g, rgb.b);
            }
            let (r, g, b) = xterm_256_to_rgb(idx);
            Color::Rgb(r, g, b)
        }
    }
}

fn brighten_bold_ansi_color(color: AnsiColor) -> AnsiColor {
    match color {
        AnsiColor::Named(named) => AnsiColor::Named(match named {
            NamedColor::Black => NamedColor::BrightBlack,
            NamedColor::Red => NamedColor::BrightRed,
            NamedColor::Green => NamedColor::BrightGreen,
            NamedColor::Yellow => NamedColor::BrightYellow,
            NamedColor::Blue => NamedColor::BrightBlue,
            NamedColor::Magenta => NamedColor::BrightMagenta,
            NamedColor::Cyan => NamedColor::BrightCyan,
            NamedColor::White => NamedColor::BrightWhite,
            other => other,
        }),
        AnsiColor::Indexed(idx) if idx <= 7 => AnsiColor::Indexed(idx + 8),
        other => other,
    }
}

fn xterm_256_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ansi_16_to_rgb(idx),
        16..=231 => {
            let i = idx - 16;
            let r = i / 36;
            let g = (i / 6) % 6;
            let b = i % 6;
            (color_cube(r), color_cube(g), color_cube(b))
        }
        232..=255 => {
            let shade = 8 + (idx - 232) * 10;
            (shade, shade, shade)
        }
    }
}

fn ansi_16_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0x00, 0x00, 0x00),
        1 => (0xcd, 0x00, 0x00),
        2 => (0x00, 0xcd, 0x00),
        3 => (0xcd, 0xcd, 0x00),
        4 => (0x00, 0x00, 0xee),
        5 => (0xcd, 0x00, 0xcd),
        6 => (0x00, 0xcd, 0xcd),
        7 => (0xe5, 0xe5, 0xe5),
        8 => (0xb0, 0xb0, 0xb0),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x5c, 0x5c, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        _ => (0xff, 0xff, 0xff),
    }
}

fn color_cube(v: u8) -> u8 {
    match v {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

fn term_has_content<T: alacritty_terminal::event::EventListener>(
    term: &alacritty_terminal::term::Term<T>,
) -> bool {
    let content = term.renderable_content();
    for indexed in content.display_iter {
        let ch = indexed.cell.c;
        if !ch.is_whitespace() {
            return true;
        }
    }
    false
}

fn loading_spinner_frame() -> &'static str {
    const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as usize)
        .unwrap_or(0);
    FRAMES[(ms / 120) % FRAMES.len()]
}

fn attenuate_color(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(scale_channel(r), scale_channel(g), scale_channel(b)),
        Color::Black => Color::Black,
        Color::Red => Color::DarkGray,
        Color::Green => Color::DarkGray,
        Color::Yellow => Color::DarkGray,
        Color::Blue => Color::DarkGray,
        Color::Magenta => Color::DarkGray,
        Color::Cyan => Color::DarkGray,
        Color::Gray => Color::DarkGray,
        Color::DarkGray => Color::DarkGray,
        Color::LightRed => Color::DarkGray,
        Color::LightGreen => Color::DarkGray,
        Color::LightYellow => Color::DarkGray,
        Color::LightBlue => Color::DarkGray,
        Color::LightMagenta => Color::DarkGray,
        Color::LightCyan => Color::DarkGray,
        Color::White => Color::Gray,
        Color::Indexed(n) => {
            if n >= 8 {
                Color::DarkGray
            } else {
                Color::Indexed(n)
            }
        }
        Color::Reset => Color::Reset,
    }
}

fn scale_channel(v: u8) -> u8 {
    ((v as f32) * 0.45).round() as u8
}

fn maybe_dim(color: Color, dimmed: bool) -> Color {
    if dimmed {
        attenuate_color(color)
    } else {
        color
    }
}

// ── Log panel ─────────────────────────────────────────────────────────────────

fn render_log(frame: &mut Frame, app: &mut App, area: Rect) {
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

fn render_status_bar(frame: &mut Frame, app: &mut App, area: Rect) {
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
            " [^S]scroll  [^B]sidebar  [Alt+o]log  [^Q]quit  (keys forwarded to container)"
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

fn render_new_project(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
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
            "  Writes canonical/void-claw-rules.toml only if it does not exist.",
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
            Span::styled(format!("  {marker}"), Style::default().fg(tone(Color::Cyan))),
            Span::styled(format!("{label}: "), Style::default().fg(tone(Color::DarkGray))),
            Span::styled(value.clone(), label_style),
        ]));
    }

    if let Some(err) = state.error.as_ref() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(tone(Color::Red)).add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Esc] Cancel  [Enter] Create/Select  [←→] Cycle lists",
        Style::default().fg(tone(Color::DarkGray)),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn render_new_project_preview(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
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
            Span::styled(format!("{label}: "), Style::default().fg(tone(Color::DarkGray))),
            Span::styled(value.to_string(), Style::default().fg(tone(Color::White))),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Container picker pane ─────────────────────────────────────────────────────

fn render_container_picker(frame: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let cfg = app.config.get();
    let selected_ctr = app.container_picker.unwrap_or(0);
    let project_name = app
        .selected_project_idx()
        .and_then(|pi| app.projects.get(pi))
        .map(|p| p.name.as_str())
        .unwrap_or("(no project)");

    let tone = |c| maybe_dim(c, dimmed);
    let block = Block::default()
        .title(format!(" Launch Container for '{}' ", project_name))
        .title_style(
            Style::default()
                .fg(tone(Color::Yellow))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tone(Color::Cyan)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = vec![Line::from("")];

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
            Span::styled(format!("  {marker}"), Style::default().fg(tone(Color::Cyan))),
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

fn render_image_build(frame: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
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
    let agent_dockerfile = name
        .strip_prefix("void-claw-")
        .map(|agent| {
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
            ""
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
            Span::styled(format!("  {marker}"), Style::default().fg(tone(Color::Cyan))),
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

fn render_build_output(frame: &mut Frame, app: &App, area: Rect, dimmed: bool) {
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
            Style::default().fg(if *is_error { tone(Color::Red) } else { tone(Color::White) }),
        )));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    if app.build_scroll > 0 && max_scroll > 0 {
        render_scrollbar(frame, inner, max_scroll, scroll, true);
    }
}

fn strip_ansi_and_control(input: &str) -> String {
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

fn clamp_for_width(input: &str, max_cols: usize) -> String {
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

// ── Exec approval overlay ─────────────────────────────────────────────────────

fn render_exec_approval_overlay(frame: &mut Frame, app: &App, area: Rect, item_idx: usize) {
    let Some(item) = app.pending_exec.get(item_idx) else {
        return;
    };

    let popup_area = centered_rect(72, 56, 12, area);
    frame.render_widget(Clear, popup_area);

    let match_str = match &item.matched_command {
        Some(name) => format!("rule: {name}"),
        None => "unlisted command".to_string(),
    };

    let action_line = Line::from(vec![
        Span::styled(
            "[y/↵] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Approve  ", Style::default().fg(Color::White)),
        Span::styled(
            "[r] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always allow  ", Style::default().fg(Color::White)),
        Span::styled(
            "[n/Esc] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Deny  ", Style::default().fg(Color::White)),
        Span::styled(
            "[d] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always deny", Style::default().fg(Color::White)),
    ]);

    let queue_total = app
        .pending_exec
        .iter()
        .filter(|i| i.project == item.project)
        .count();
    let queue_pos = app
        .pending_exec
        .iter()
        .filter(|i| i.project == item.project)
        .position(|i| i.id == item.id)
        .map(|i| i + 1)
        .unwrap_or(1);
    let source_container = item
        .container_id
        .clone()
        .unwrap_or_else(|| "unknown-container".to_string());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  APPROVAL REQUIRED",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Command : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.argv.join(" "),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Project : ", Style::default().fg(Color::DarkGray)),
            Span::styled(item.project.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Source  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("workspace={}  container={}", item.project, source_container),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Queue   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{}/{} for workspace '{}' (exec total: {}, net total: {})",
                    queue_pos,
                    queue_total.max(1),
                    item.project,
                    app.pending_exec.len(),
                    app.pending_net.len()
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Host cwd: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.cwd.display().to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Match   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(match_str, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        action_line,
        Line::from(""),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Exec Approval Required ")
                .title_alignment(Alignment::Center)
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        popup_area,
    );
}

// ── Network approval overlay ──────────────────────────────────────────────────

fn render_net_approval_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let Some(item) = app.pending_net.first() else {
        return;
    };

    let show_proxy_details = item.source_status != "listener_bound_source";
    let popup_area = centered_rect(72, 56, if show_proxy_details { 13 } else { 12 }, area);
    frame.render_widget(Clear, popup_area);

    let action_line = Line::from(vec![
        Span::styled(
            "[y/↵] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Allow  ", Style::default().fg(Color::White)),
        Span::styled(
            "[r] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always allow  ", Style::default().fg(Color::White)),
        Span::styled(
            "[n/Esc] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Deny  ", Style::default().fg(Color::White)),
        Span::styled(
            "[d] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled("Always deny", Style::default().fg(Color::White)),
    ]);

    let queue_total = app.pending_net.len();
    let source_workspace = item
        .source_project
        .clone()
        .unwrap_or_else(|| "unknown-workspace".to_string());
    let source_container = item
        .source_container
        .clone()
        .unwrap_or_else(|| "unknown-container".to_string());

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  NETWORK REQUEST",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Method  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                item.method.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Host    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(item.host.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Path    : ", Style::default().fg(Color::DarkGray)),
            Span::styled(item.path.clone(), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Source  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "workspace={}  container={}",
                    source_workspace, source_container
                ),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Queue   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "1/{} (exec total: {}, net total: {})",
                    queue_total.max(1),
                    app.pending_exec.len(),
                    app.pending_net.len()
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        action_line,
        Line::from(""),
    ];
    if show_proxy_details {
        lines.insert(
            7,
            Line::from(vec![
                Span::styled("  Proxy   : ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!(
                        "source_status={}  proxy_auth={}",
                        item.source_status, item.has_proxy_authorization
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        );
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(" Network Approval Required ")
                .title_alignment(Alignment::Center)
                .title_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        ),
        popup_area,
    );
}

// ── Fullscreen log ────────────────────────────────────────────────────────────

fn render_log_fullscreen(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Log (fullscreen) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let lines: Vec<Line> = app
        .log
        .iter()
        .map(|entry| match entry {
            LogEntry::Audit(e) => {
                let ts = e.timestamp.format("%H:%M:%S").to_string();
                let decision_color = match e.decision {
                    crate::state::DecisionKind::Auto => Color::Green,
                    crate::state::DecisionKind::Approved
                    | crate::state::DecisionKind::Remembered => Color::Cyan,
                    crate::state::DecisionKind::Denied
                    | crate::state::DecisionKind::DeniedByPolicy
                    | crate::state::DecisionKind::TimedOut => Color::Red,
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

fn render_status_bar_log(frame: &mut Frame, _app: &mut App, area: Rect) {
    frame.render_widget(
        Paragraph::new(Span::styled(
            " [↑↓/jk]scroll  [o/Esc/q]close",
            Style::default().fg(Color::DarkGray),
        )),
        area,
    );
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, min_height: u16, r: Rect) -> Rect {
    let height = ((r.height * percent_y) / 100).max(min_height).min(r.height);
    let width = (r.width * percent_x) / 100;
    Rect {
        x: (r.width.saturating_sub(width)) / 2 + r.x,
        y: (r.height.saturating_sub(height)) / 2 + r.y,
        width,
        height,
    }
}
