use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::details;
use crate::model::LineFormat;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    app.hit.overlay = area;
    let Some(entry) = app.selected_entry() else {
        return;
    };
    let format = entry.format;
    let content = details::build_lines(entry, &app.theme, &app.config);
    let content_len = content.len();
    let focused = app.overlay_focused;
    let wrap = app.config.wrap_details;
    let theme_border = app.theme.border.fg;
    let theme_match = app.theme.search_match.fg;
    let overlay_bg = app.theme.overlay_bg;
    let foreground = app.theme.foreground.fg;

    app.overlay_content_len = content_len;

    let title = match format {
        LineFormat::Json => {
            if focused {
                " details · json · focused "
            } else {
                " details · json "
            }
        }
        LineFormat::Logfmt => {
            if focused {
                " details · logfmt · focused "
            } else {
                " details · logfmt "
            }
        }
        LineFormat::Plain => {
            if focused {
                " details · raw · focused "
            } else {
                " details · raw "
            }
        }
    };
    let hint = if focused {
        " j/k scroll · Esc close "
    } else {
        " Enter focus · Esc close "
    };

    let border = if focused {
        Style::default()
            .fg(theme_match)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme_border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .title_bottom(hint)
        .style(Style::default().bg(overlay_bg).fg(foreground));

    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.overlay_inner_height = inner.height as usize;

    let max_scroll = content_len.saturating_sub(inner.height as usize);
    if app.overlay_scroll > max_scroll {
        app.overlay_scroll = max_scroll;
    }
    let scroll = app.overlay_scroll;

    let lines: Vec<Line> = content.into_iter().map(|l| l.to_line()).collect();
    let mut paragraph = Paragraph::new(lines)
        .style(Style::default().bg(overlay_bg))
        .scroll((scroll as u16, 0));
    if wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, inner);
}
