use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarOrientation};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, SidebarItem};
use crate::config;
use crate::text;

const MIN_MAIN: u16 = 20;

pub fn desired_width(area_width: u16, configured: usize) -> u16 {
    let min = config::default_sidebar_width_min() as u16;
    let configured = (configured as u16).max(min);
    let available = area_width.saturating_sub(MIN_MAIN).max(min);
    configured.min(available)
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.is_sidebar_focused();
    let border_tone = if focused {
        app.theme.window_focus_border
    } else {
        app.theme.border
    };
    let mut border = app.theme.tone_fg_style(border_tone);
    if focused {
        border = border.add_modifier(Modifier::BOLD);
    }

    let n_filters = app.filters.len();
    let n_hidden = app.view.hidden.len();
    let filters_off = !app.filtering_enabled && n_filters > 0;
    // Title sits on the top border between the corner glyphs.
    let title_max = area.width.saturating_sub(2) as usize;
    let title = match (n_filters, n_hidden, filters_off) {
        (0, 0, _) => " sidebar ".to_string(),
        (_, 0, true) => " filters (disabled) ".to_string(),
        (f, 0, false) => format!(" filters ({f}) "),
        (0, h, _) => format!(" hidden ({h}) "),
        (_, h, true) => {
            let full = format!(" filters (disabled) · hidden ({h}) ");
            let short = format!(" filters (disabled) · H{h} ");
            if UnicodeWidthStr::width(full.as_str()) <= title_max {
                full
            } else {
                short
            }
        }
        (f, h, false) => {
            let full = format!(" filters ({f}) · hidden ({h}) ");
            let short = format!(" F{f} · H{h} ");
            if UnicodeWidthStr::width(full.as_str()) <= title_max {
                full
            } else {
                short
            }
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .style(
            Style::default()
                .bg(app.theme.overlay_bg())
                .fg(app.theme.foreground.fg),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);
    let bars = super::split_scrollbars(
        inner,
        app.config.sidebar_scrollbar_vertical,
        app.config.sidebar_scrollbar_horizontal,
    );
    let content = bars.content;
    app.pointer.hit.sidebar_inner = content;
    app.pointer.hit.sidebar_scrollbar_vertical = bars.vertical.unwrap_or_default();
    app.pointer.hit.sidebar_scrollbar_horizontal = bars.horizontal.unwrap_or_default();

    let viewport = content.height as usize;
    app.ensure_sidebar_visible(viewport, app.config.scroll_moves_selection);

    let items = app.sidebar_items();
    let width = content.width as usize;
    let content_w = app.sidebar_content_width();
    app.clamp_sidebar_scroll_x(width, content_w);
    let scroll_x = app.sidebar_scroll_x;

    if items.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            " no filters · no hidden ",
            app.theme.tone_style(app.theme.dim, app.theme.overlay_bg()),
        )))
        .style(Style::default().bg(app.theme.overlay_bg()));
        frame.render_widget(empty, content);
    } else {
        let end = (app.sidebar_scroll + viewport).min(items.len());
        let mut lines = Vec::with_capacity(end.saturating_sub(app.sidebar_scroll));

        for idx in app.sidebar_scroll..end {
            let selected = focused && idx == app.sidebar_selected;
            let item = items[idx];
            let text = app.sidebar_item_text(item);
            let dim = match item {
                SidebarItem::Filter(fi) => {
                    !app.filtering_enabled || !app.filters[fi].enabled
                }
                SidebarItem::Hidden(_) => true,
            };
            let display = text::slice_width(&text, scroll_x, width);
            let style = if selected {
                app.theme.selection_style()
            } else if dim {
                app.theme.tone_style(app.theme.dim, app.theme.overlay_bg())
            } else {
                Style::default()
                    .fg(app.theme.foreground.fg)
                    .bg(app.theme.overlay_bg())
            };
            let mut spans = vec![Span::styled(display.clone(), style)];
            let used = UnicodeWidthStr::width(display.as_str());
            if selected && used < width {
                spans.push(Span::styled(" ".repeat(width - used), style));
            }
            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines).style(Style::default().bg(app.theme.overlay_bg()));
        frame.render_widget(paragraph, content);
    }

    let theme = &app.theme;
    let thumb = theme.tone_fg_style(theme.window_focus_border);
    let track = theme.tone_fg_style(theme.dim);
    if let Some(bar) = bars.vertical {
        super::draw_scrollbar(
            frame,
            bar,
            items.len(),
            app.sidebar_scroll,
            viewport.max(1),
            ScrollbarOrientation::VerticalRight,
            thumb,
            track,
        );
    }
    if let Some(bar) = bars.horizontal {
        super::draw_scrollbar(
            frame,
            bar,
            content_w,
            scroll_x,
            width.max(1),
            ScrollbarOrientation::HorizontalBottom,
            thumb,
            track,
        );
    }
    if let Some(corner) = bars.corner {
        super::draw_scrollbar_corner(frame, corner, Style::default().bg(theme.overlay_bg()));
    }
}
