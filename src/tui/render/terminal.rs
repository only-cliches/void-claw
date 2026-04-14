use super::*;

pub(crate) fn render_terminal(
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
        if focused {
            render_terminal_title_hint(frame, split[0], in_scroll_mode);
        }
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

    let inner = if fullscreen {
        content_area
    } else {
        block.inner(content_area)
    };
    frame.render_widget(block, content_area);
    if focused && !fullscreen {
        render_terminal_title_hint(frame, content_area, in_scroll_mode);
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

pub(crate) fn resolve_ansi_color(
    color: AnsiColor,
    colors: &alacritty_terminal::term::color::Colors,
) -> Color {
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

pub(crate) fn brighten_bold_ansi_color(color: AnsiColor) -> AnsiColor {
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

pub(crate) fn xterm_256_to_rgb(idx: u8) -> (u8, u8, u8) {
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

pub(crate) fn ansi_16_to_rgb(idx: u8) -> (u8, u8, u8) {
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

pub(crate) fn color_cube(v: u8) -> u8 {
    match v {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}

pub(crate) fn term_has_content<T: alacritty_terminal::event::EventListener>(
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

pub(crate) fn loading_spinner_frame() -> &'static str {
    const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as usize)
        .unwrap_or(0);
    FRAMES[(ms / 120) % FRAMES.len()]
}

pub(crate) fn attenuate_color(color: Color) -> Color {
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

pub(crate) fn scale_channel(v: u8) -> u8 {
    ((v as f32) * 0.45).round() as u8
}

pub(crate) fn maybe_dim(color: Color, dimmed: bool) -> Color {
    if dimmed {
        attenuate_color(color)
    } else {
        color
    }
}
