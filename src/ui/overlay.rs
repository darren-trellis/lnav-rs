use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::{draw_scrollbar, split_scrollbar};

use crate::app::App;
use crate::details::{self, DetailLine};
use crate::highlight;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    app.hit.overlay = area;
    let Some(entry) = app.selected_entry() else {
        return;
    };
    let content = details::build_lines(entry, &app.theme, &app.config, &app.overlay_folded);
    let content_len = content.len();
    let focused = app.overlay_focused;
    let wrap = app.config.wrap_details;
    let search_in_overlay = app.search_in_overlay;
    let search_regex = if search_in_overlay {
        app.search_regex.clone()
    } else {
        None
    };
    let cursor = app.overlay_cursor;
    let overlay_bg = app.theme.overlay_bg;
    let foreground = app.theme.foreground.fg;
    let selection_bg = app.theme.selection_bg;
    let selection_fg = app.theme.selection_fg;
    let fg_tone = app.theme.foreground;
    let border_tone = if focused {
        app.theme.window_focus_border
    } else {
        app.theme.border
    };

    app.overlay_content_len = content_len;
    if content_len == 0 {
        app.overlay_cursor = 0;
    } else if app.overlay_cursor >= content_len {
        app.overlay_cursor = content_len - 1;
    }

    let title = " details ";
    let show_help = focused && app.overlay_help;
    let hint = " j/k move · Space fold · Tab focus · c copy · / search · Esc close · ? hide ";

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
    let show_bar = app.config.scrollbar;
    let (content_area, bar_area) = split_scrollbar(inner, show_bar);
    app.hit.overlay_scrollbar = bar_area.unwrap_or_default();
    app.overlay_inner_height = content_area.height as usize;

    let max_scroll = content_len.saturating_sub(content_area.height as usize);
    if app.overlay_scroll > max_scroll {
        app.overlay_scroll = max_scroll;
    }
    if focused {
        app.ensure_overlay_cursor_visible();
    }
    let scroll = app.overlay_scroll;
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

    let lines: Vec<Line> = content
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let is_cursor = focused && i == cursor;
            render_detail_line(
                &line,
                search_regex.as_ref(),
                base_style,
                match_style,
                is_cursor,
                cursor_style,
                cursor_match_style,
                content_area.width as usize,
            )
        })
        .collect();
    let mut paragraph = Paragraph::new(lines)
        .style(Style::default().bg(overlay_bg))
        .scroll((scroll as u16, 0));
    if wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, content_area);

    if let Some(bar) = bar_area {
        draw_scrollbar(
            frame,
            bar,
            content_len,
            scroll,
            viewport,
            thumb,
            track,
        );
    }
}

fn render_detail_line(
    line: &DetailLine,
    regex: Option<&regex::Regex>,
    base_style: Style,
    match_style: Style,
    is_cursor: bool,
    cursor_style: Style,
    cursor_match_style: Style,
    width: usize,
) -> Line<'static> {
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
