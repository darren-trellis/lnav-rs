use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::details::{self, DetailLine};
use crate::highlight;
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
    let search_in_overlay = app.search_in_overlay;
    let search_regex = if search_in_overlay {
        app.search_regex.clone()
    } else {
        None
    };
    let current_match = if search_in_overlay {
        app.search_cursor
            .and_then(|c| app.search_matches.get(c).copied())
    } else {
        None
    };
    let theme_border = app.theme.border.fg;
    let match_fg = app.theme.search_match.fg;
    let match_bg = app.theme.search_match.bg;
    let overlay_bg = app.theme.overlay_bg;
    let foreground = app.theme.foreground.fg;
    let fg_tone = app.theme.foreground;

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
        " j/k scroll · / search · Esc close "
    } else {
        " Enter focus · Esc close "
    };

    let border = if focused {
        Style::default()
            .fg(match_fg)
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

    let match_style = Style::default()
        .fg(match_fg)
        .bg(match_bg.unwrap_or(overlay_bg))
        .add_modifier(Modifier::BOLD);
    let current_style = Style::default()
        .fg(overlay_bg)
        .bg(match_fg)
        .add_modifier(Modifier::BOLD);
    let base_style = Style::default().fg(fg_tone.fg).bg(overlay_bg);

    let lines: Vec<Line> = content
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            render_detail_line(
                &line,
                search_regex.as_ref(),
                base_style,
                match_style,
                current_match == Some(i),
                current_style,
            )
        })
        .collect();
    let mut paragraph = Paragraph::new(lines)
        .style(Style::default().bg(overlay_bg))
        .scroll((scroll as u16, 0));
    if wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, inner);
}

fn render_detail_line(
    line: &DetailLine,
    regex: Option<&regex::Regex>,
    base_style: Style,
    match_style: Style,
    is_current: bool,
    current_style: Style,
) -> Line<'static> {
    let text = line.plain_text();
    let Some(re) = regex else {
        return line.to_line();
    };
    if !re.is_match(&text) {
        return line.to_line();
    }
    let base = if is_current { current_style } else { base_style };
    let hi = if is_current { current_style } else { match_style };
    let mut spans = Vec::new();
    highlight::push_highlighted(&mut spans, text, base, hi, Some(re));
    Line::from(spans)
}
