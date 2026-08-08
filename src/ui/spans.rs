use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarOrientation};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::text;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    app.ensure_spans_built();

    let trace_count = app.spans.forest.traces.len();
    let title = if trace_count == 0 {
        " spans ".to_string()
    } else {
        format!(
            " spans · {trace_count} trace{} ",
            if trace_count == 1 { "" } else { "s" }
        )
    };

    let block = {
        let theme = &app.theme;
        let list_focused = app.is_list_focused();
        let border_tone = if list_focused {
            theme.window_focus_border
        } else {
            theme.border
        };
        let mut border_style = theme.tone_fg_style(border_tone);
        if list_focused {
            border_style = border_style.add_modifier(Modifier::BOLD);
        }
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .style(
                Style::default()
                    .bg(theme.background)
                    .fg(theme.foreground.fg),
            )
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let bars = super::split_scrollbars(
        inner,
        app.config.list_scrollbar_vertical,
        app.config.list_scrollbar_horizontal,
    );
    let content = bars.content;
    app.pointer.hit.list_inner = content;
    app.pointer.hit.list_scrollbar_vertical = bars.vertical.unwrap_or_default();
    app.pointer.hit.list_scrollbar_horizontal = bars.horizontal.unwrap_or_default();
    app.pointer.hit.list_pin_rows = 0;

    let viewport = content.height as usize;
    app.ensure_span_selection_visible();

    let content_w = app
        .spans
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    app.spans.content_width = content_w;
    let width = content.width as usize;
    let max_x = content_w.saturating_sub(width.max(1));
    if app.spans.scroll_x > max_x {
        app.spans.scroll_x = max_x;
    }

    if app.spans.lines.is_empty() {
        return;
    }

    let end = (app.spans.scroll + viewport).min(app.spans.lines.len());
    let mut lines = Vec::with_capacity(end.saturating_sub(app.spans.scroll));
    let scroll_x = app.spans.scroll_x;
    let list_focused = app.is_list_focused();
    let selection_bg = app.theme.selection_bg();
    let selection_fg = app.theme.selection_fg();
    let background = app.theme.background;

    for idx in app.spans.scroll..end {
        let line = &app.spans.lines[idx];
        let cursor_row = idx == app.spans.selected && list_focused;
        let base_bg = if cursor_row {
            selection_bg
        } else {
            background
        };

        let styled: Vec<Span<'static>> = line
            .spans
            .iter()
            .map(|s| {
                let mut style = s.style.bg(base_bg);
                if cursor_row {
                    style = style.fg(selection_fg);
                }
                Span::styled(s.content.to_string(), style)
            })
            .collect();

        let mut visible = text::slice_spans(&styled, scroll_x, width);
        let used = text::spans_width(&visible);
        if used < width {
            visible.push(Span::styled(
                " ".repeat(width - used),
                Style::default().bg(base_bg),
            ));
        }
        lines.push(Line::from(visible));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(background)),
        content,
    );

    let thumb = app.theme.tone_fg_style(app.theme.window_focus_border);
    let track = app.theme.tone_fg_style(app.theme.dim);
    if let Some(area) = bars.vertical {
        super::draw_scrollbar(
            frame,
            area,
            app.spans.lines.len(),
            app.spans.scroll,
            viewport,
            ScrollbarOrientation::VerticalRight,
            thumb,
            track,
        );
    }
    if let Some(area) = bars.horizontal {
        super::draw_scrollbar(
            frame,
            area,
            content_w,
            app.spans.scroll_x,
            width,
            ScrollbarOrientation::HorizontalBottom,
            thumb,
            track,
        );
    }
    if let Some(area) = bars.corner {
        super::draw_scrollbar_corner(frame, area, Style::default().bg(background));
    }
}
