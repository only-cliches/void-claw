use super::*;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::{App, Focus, SidebarItem};

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

pub fn render_scrollbar(
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

pub(crate) fn render_sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
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
