use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarOrientation, Wrap};
use unicode_width::UnicodeWidthStr;

use super::{draw_scrollbar, draw_scrollbar_corner, split_scrollbars};

use crate::app::App;
use crate::details::DetailLine;
use crate::highlight;
use crate::keys;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect, content: &[DetailLine]) {
    app.pointer.hit.overlay = area;
    let content_len = content.len();
    let focused = app.is_details_focused();
    let wrap = app.config.wrap_details;
    let search_in_overlay = app.search.in_details;
    let search_regex = if search_in_overlay {
        app.search.regex.clone()
    } else {
        None
    };
    let cursor = app.details.cursor;
    let overlay_bg = app.theme.overlay_bg();
    let foreground = app.theme.foreground.fg;
    let selection_bg = app.theme.selection_bg();
    let selection_fg = app.theme.selection_fg();
    let fg_tone = app.theme.foreground;
    let border_tone = if focused {
        app.theme.window_focus_border
    } else {
        app.theme.border
    };

    app.details.content_len = content_len;
    if content_len == 0 {
        app.details.cursor = 0;
    } else if app.details.cursor >= content_len {
        app.details.cursor = content_len - 1;
    }

    let title = " details ";
    let show_help = focused && app.details.help;
    let binding = |command, fallback| {
        keys::binding_for_command(
            &app.config.keys.bindings,
            Some(&app.config.keys.details),
            command,
        )
        .unwrap_or(fallback)
    };
    let hint = format!(
        " {}/{} move · {} fold · {} focus · {} copy · {} search · {} close · {} hide ",
        binding("nav down", "j"),
        binding("nav up", "k"),
        binding("fold toggle", "fold"),
        binding("focus toggle", "focus"),
        binding("copy", "copy"),
        binding("search", "search"),
        binding("view current off", "Esc"),
        binding("help", "help"),
    );

    let mut border = app.theme.tone_fg_style(border_tone);
    if focused {
        border = border.add_modifier(Modifier::BOLD);
    }

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .style(Style::default().bg(overlay_bg).fg(foreground));
    if show_help {
        block = block.title_bottom(hint);
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let show_h_bar = app.config.details_scrollbar_horizontal && !wrap;
    let bars = split_scrollbars(
        inner,
        app.config.details_scrollbar_vertical,
        show_h_bar,
    );
    let content_area = bars.content;
    app.pointer.hit.overlay_inner = content_area;
    app.pointer.hit.overlay_scrollbar_vertical = bars.vertical.unwrap_or_default();
    app.pointer.hit.overlay_scrollbar_horizontal = bars.horizontal.unwrap_or_default();
    app.details.viewport_height = content_area.height as usize;

    let content_w = if wrap {
        0
    } else {
        content
            .iter()
            .map(|line| UnicodeWidthStr::width(line.plain_text().as_str()))
            .max()
            .unwrap_or(0)
    };
    app.details.content_width = content_w;
    let viewport_w = content_area.width as usize;
    app.clamp_details_scroll_x(viewport_w, content_w);
    let scroll_x = if wrap { 0 } else { app.details.scroll_x };

    if focused {
        app.ensure_overlay_cursor_visible(app.config.scroll_moves_selection);
    } else {
        app.ensure_overlay_cursor_visible(false);
    }
    let scroll = app.details.scroll;
    let viewport = content_area.height as usize;
    let thumb = app.theme.tone_fg_style(app.theme.window_focus_border);
    let track = app.theme.tone_fg_style(app.theme.dim);

    let match_style = app.theme.search_highlight_style(overlay_bg);
    let cursor_style = Style::default()
        .fg(selection_fg)
        .bg(selection_bg)
        .add_modifier(Modifier::BOLD);
    let cursor_match_style = app.theme.search_highlight_style(selection_bg);
    let base_style = Style::default().fg(fg_tone.fg).bg(overlay_bg);

    let cursor_pad = if wrap {
        viewport_w
    } else {
        content_w.max(scroll_x + viewport_w)
    };
    let lines: Vec<Line> = content
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let is_cursor = focused && i == cursor;
            render_detail_line(
                line,
                is_cursor,
                DetailRenderOptions {
                    regex: search_regex.as_ref(),
                    base_style,
                    match_style,
                    cursor_style,
                    cursor_match_style,
                    width: cursor_pad,
                },
            )
        })
        .collect();
    let mut paragraph = Paragraph::new(lines)
        .style(Style::default().bg(overlay_bg))
        .scroll((scroll as u16, scroll_x as u16));
    if wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, content_area);

    if let Some(bar) = bars.vertical {
        draw_scrollbar(
            frame,
            bar,
            content_len,
            scroll,
            viewport,
            ScrollbarOrientation::VerticalRight,
            thumb,
            track,
        );
    }
    if let Some(bar) = bars.horizontal {
        draw_scrollbar(
            frame,
            bar,
            content_w,
            scroll_x,
            viewport_w.max(1),
            ScrollbarOrientation::HorizontalBottom,
            thumb,
            track,
        );
    }
    if let Some(corner) = bars.corner {
        draw_scrollbar_corner(frame, corner, Style::default().bg(overlay_bg));
    }
}

struct DetailRenderOptions<'a> {
    regex: Option<&'a regex::Regex>,
    base_style: Style,
    match_style: Style,
    cursor_style: Style,
    cursor_match_style: Style,
    width: usize,
}

fn render_detail_line(
    line: &DetailLine,
    is_cursor: bool,
    options: DetailRenderOptions<'_>,
) -> Line<'static> {
    let DetailRenderOptions {
        regex,
        base_style,
        match_style,
        cursor_style,
        cursor_match_style,
        width,
    } = options;
    let text = line.plain_text();
    let mut spans = if let Some(re) = regex.filter(|r| r.is_match(&text)) {
        let base = if is_cursor { cursor_style } else { base_style };
        let hi = if is_cursor {
            cursor_match_style
        } else {
            match_style
        };
        let mut out = Vec::new();
        highlight::push_highlighted(&mut out, text.clone(), base, hi, Some(re));
        out
    } else if is_cursor {
        vec![Span::styled(text.clone(), cursor_style)]
    } else {
        line.spans.clone()
    };

    // Pad cursor row so the selection bar spans the full width.
    if is_cursor && width > 0 {
        let used: usize = spans
            .iter()
            .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
            .sum();
        if used < width {
            spans.push(Span::styled(" ".repeat(width - used), cursor_style));
        }
    }

    Line::from(spans)
}
