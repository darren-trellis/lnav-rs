use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, SidebarItem};
use crate::text;

const WIDTH: u16 = 28;

pub fn desired_width(area_width: u16) -> u16 {
    WIDTH.min(area_width.saturating_sub(20).max(12))
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
    let title = match (n_filters, n_hidden) {
        (0, 0) => " sidebar ".to_string(),
        (f, 0) => format!(" filters ({f}) "),
        (0, h) => format!(" hidden ({h}) "),
        (f, h) => format!(" F{f} · H{h} "),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .style(
            Style::default()
                .bg(app.theme.overlay_bg)
                .fg(app.theme.foreground.fg),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.pointer.hit.sidebar_inner = inner;

    let viewport = inner.height as usize;
    app.ensure_sidebar_visible(viewport, app.config.scroll_moves_selection);

    let items = app.sidebar_items();
    if items.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            " no filters · no hidden ",
            app.theme.tone_style(app.theme.dim, app.theme.overlay_bg),
        )))
        .style(Style::default().bg(app.theme.overlay_bg));
        frame.render_widget(empty, inner);
        return;
    }

    let end = (app.sidebar_scroll + viewport).min(items.len());
    let mut lines = Vec::with_capacity(end.saturating_sub(app.sidebar_scroll));
    let width = inner.width as usize;

    for idx in app.sidebar_scroll..end {
        let selected = focused && idx == app.sidebar_selected;
        let (text, dim) = match items[idx] {
            SidebarItem::Filter(fi) => {
                let filter = &app.filters[fi];
                let mark = if filter.enabled { "*" } else { " " };
                (
                    format!("{mark}{fi}:{} /{}/", filter.label(), filter.pattern),
                    !filter.enabled,
                )
            }
            SidebarItem::Hidden(src) => {
                let preview = app
                    .source
                    .entries()
                    .get(src)
                    .map(|e| e.raw.replace('\n', " "))
                    .unwrap_or_default();
                (format!("·{} {preview}", src + 1), true)
            }
        };
        let display = text::truncate_width(&text, width);
        let style = if selected {
            app.theme.selection_style()
        } else if dim {
            app.theme.tone_style(app.theme.dim, app.theme.overlay_bg)
        } else {
            Style::default()
                .fg(app.theme.foreground.fg)
                .bg(app.theme.overlay_bg)
        };
        let mut spans = vec![Span::styled(display.clone(), style)];
        let used = UnicodeWidthStr::width(display.as_str());
        if selected && used < width {
            spans.push(Span::styled(" ".repeat(width - used), style));
        }
        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(app.theme.overlay_bg));
    frame.render_widget(paragraph, inner);
}
