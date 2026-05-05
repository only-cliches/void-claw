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
        if app.base_rules_changed.is_some() {
            render_base_rules_changed_overlay(frame, app, area);
        }
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
            if app.base_rules_changed.is_some() {
                render_base_rules_changed_overlay(frame, app, area);
            } else if app.remove_workspace_confirm.is_some() {
                render_remove_workspace_confirm_overlay(frame, app, area);
            }
        } else {
            render_idle(frame, area);
            if app.base_rules_changed.is_some() {
                render_base_rules_changed_overlay(frame, app, area);
            } else if app.remove_workspace_confirm.is_some() {
                render_remove_workspace_confirm_overlay(frame, app, area);
            }
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

    let gap_width = right_pane_gap_width(app);
    let main_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.config.get().defaults.ui.sidebar_width.max(1)),
            Constraint::Length(gap_width),
            Constraint::Min(0),
        ])
        .split(outer[0]);

    render_sidebar(frame, app, main_row[0]);
    render_right_pane(frame, app, main_row[2]);
    render_log(frame, app, outer[1]);
    render_status_bar(frame, app, outer[2]);
    if app.base_rules_changed.is_some() {
        render_base_rules_changed_overlay(frame, app, area);
    } else if app.remove_workspace_confirm.is_some() {
        render_remove_workspace_confirm_overlay(frame, app, area);
    }
}

pub(crate) fn right_pane_gap_width(app: &App) -> u16 {
    if app.focus == Focus::ImageBuild {
        return 1;
    }

    match app.sidebar_items().get(app.sidebar_idx) {
        Some(SidebarItem::Build(_)) => 1,
        _ => 0,
    }
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
        .title(" Workspaces ")
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
    app.refresh_terminal_activity_selection(&items);
    let list_items: Vec<ListItem> = items
        .iter()
        .skip(offset)
        .take(visible)
        .map(|item| match item {
            SidebarItem::Workspace(pi) => {
                let proj = &app.workspaces[*pi];
                ListItem::new(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Green)),
                    Span::styled(
                        proj.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
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
            SidebarItem::Activity(id) => {
                if let Some(activity) = app.activity_by_id(id) {
                    let (marker, color) = match activity.state {
                        crate::activity::ActivityState::PendingApproval => ("? ", Color::Yellow),
                        crate::activity::ActivityState::PullingImage => ("↓ ", Color::Yellow),
                        crate::activity::ActivityState::Running => ("$ ", Color::Cyan),
                        crate::activity::ActivityState::Forwarding => ("⇅ ", Color::Magenta),
                        crate::activity::ActivityState::Complete => ("✓ ", Color::Green),
                        crate::activity::ActivityState::Failed => ("! ", Color::Red),
                        crate::activity::ActivityState::Denied => ("× ", Color::Red),
                        crate::activity::ActivityState::Cancelled => ("× ", Color::DarkGray),
                    };
                    let title = truncate_middle(&activity.title(), 32);
                    ListItem::new(Line::from(vec![
                        Span::styled("    ", Style::default().fg(Color::DarkGray)),
                        Span::styled(marker, Style::default().fg(color)),
                        Span::styled(title, Style::default().fg(color)),
                    ]))
                    .style(activity_sidebar_terminal_style(activity))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled("    ? ", Style::default().fg(Color::DarkGray)),
                        Span::styled("unknown activity", Style::default().fg(Color::DarkGray)),
                    ]))
                }
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
            SidebarItem::NewWorkspace => ListItem::new(Line::from(vec![
                Span::styled("+ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "New Workspace...",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])),
        })
        .collect();

    let mut list_state = ListState::default();
    if !items.is_empty() {
        let selected = app.sidebar_idx.min(items.len().saturating_sub(1));
        // Workspace rows are non-selectable section headers. If the app state ever points at one
        // (e.g. via older persisted state), render with no highlight.
        if matches!(items.get(selected), Some(SidebarItem::Workspace(_))) {
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

fn activity_sidebar_terminal_style(activity: &crate::activity::Activity) -> Style {
    if !activity.state.is_terminal() {
        return Style::default();
    }

    let age_ms = activity
        .terminal_unselected_at
        .map(|unselected_at| unselected_at.elapsed().as_millis())
        .unwrap_or(0);
    let hold_ms = u128::from(crate::activity::ACTIVITY_TERMINAL_HIGHLIGHT_SECS) * 1_000;
    let fade_ms = u128::from(crate::activity::ACTIVITY_TERMINAL_FADE_SECS) * 1_000;
    let level = if age_ms < hold_ms {
        3
    } else if age_ms < hold_ms + fade_ms {
        2usize.saturating_sub((((age_ms - hold_ms) * 3) / fade_ms) as usize)
    } else {
        return Style::default();
    };

    let bg = terminal_activity_background(activity.state.succeeded(), level);
    Style::default().fg(Color::White).bg(bg)
}

fn terminal_activity_background(succeeded: bool, level: usize) -> Color {
    match (succeeded, level) {
        (true, 3) => Color::Rgb(24, 112, 56),
        (true, 2) => Color::Rgb(16, 80, 40),
        (true, 1) => Color::Rgb(10, 48, 28),
        (true, _) => Color::Rgb(6, 28, 18),
        (false, 3) => Color::Rgb(132, 40, 40),
        (false, 2) => Color::Rgb(96, 32, 36),
        (false, 1) => Color::Rgb(56, 24, 28),
        (false, _) => Color::Rgb(32, 18, 20),
    }
}

fn truncate_middle(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }
    let left = (max_chars - 1) / 2;
    let right = max_chars - 1 - left;
    let mut out = chars[..left].iter().collect::<String>();
    out.push('…');
    out.push_str(&chars[chars.len() - right..].iter().collect::<String>());
    out
}

// ── Right pane ────────────────────────────────────────────────────────────────
