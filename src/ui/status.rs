use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, InputMode, PendingOp};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    app.hit.status = area;
    let theme = &app.theme;
    let visible = app.visible_len();
    let total = app.source.len();
    let pos = if visible == 0 { 0 } else { app.selected + 1 };
    let follow = if app.follow { "FOLLOW" } else { "PAUSED" };
    let filter_tag = if !app.filters.is_empty() {
        if app.filtering_enabled {
            format!("  F{}", app.filters.len())
        } else {
            "  Foff".into()
        }
    } else {
        String::new()
    };

    let right = if visible == total {
        format!("{pos}/{total}  {follow}{filter_tag}  {}  lnav-rs", app.theme.name)
    } else {
        format!(
            "{pos}/{visible} ({total})  {follow}{filter_tag}  {}  lnav-rs",
            app.theme.name
        )
    };

    let style = Style::default()
        .bg(theme.status_bg)
        .fg(theme.status_fg)
        .add_modifier(Modifier::BOLD);

    let mut cursor_col: Option<u16> = None;

    let spans = match app.input_mode {
        InputMode::Search => {
            let query = format!("/{}", app.search_query);
            cursor_col = Some(UnicodeWidthStr::width(query.as_str()) as u16);
            let suffix = if app.search_query.is_empty() {
                String::new()
            } else if app.search_error.is_some() {
                "  invalid regex".into()
            } else if app.search_matches.is_empty() {
                "  no matches".into()
            } else {
                let n = app.search_matches.len();
                let cur = app.search_cursor.map(|c| c + 1).unwrap_or(1).min(n);
                format!("  {cur}/{n}")
            };
            vec![
                Span::styled(query, style),
                Span::styled(suffix, style),
            ]
        }
        InputMode::Command => {
            let typed = format!(":{}", app.command_buffer);
            cursor_col = Some(UnicodeWidthStr::width(typed.as_str()) as u16);
            command_spans(app, style)
        }
        InputMode::Normal => {
            let mut spans = Vec::new();
            if let Some(n) = app.count {
                spans.push(Span::styled(
                    format!("{n}"),
                    theme
                        .tone_style(theme.search_match, theme.status_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if let Some(op) = app.pending_op {
                let tag = match op {
                    PendingOp::Hide => "d",
                    PendingOp::Delete => "D",
                };
                spans.push(Span::styled(
                    format!("{tag} "),
                    theme
                        .tone_style(theme.search_match, theme.status_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if app.count.is_some() {
                spans.push(Span::styled(" ", style));
            }
            spans.push(Span::styled(
                app.status_message
                    .clone()
                    .unwrap_or_else(|| ":help · 5j · dd/DD · d{{motion}}".into()),
                style,
            ));
            spans
        }
    };

    let left_width: usize = spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let right_w = UnicodeWidthStr::width(right.as_str());
    let pad = (area.width as usize)
        .saturating_sub(left_width + right_w)
        .max(1);

    let mut line_spans = spans;
    line_spans.push(Span::styled(" ".repeat(pad), style));
    line_spans.push(Span::styled(right, style));

    frame.render_widget(
        Paragraph::new(Line::from(line_spans)).style(style),
        area,
    );

    if let Some(col) = cursor_col {
        let max_x = area.width.saturating_sub(1);
        let x = area.x.saturating_add(col).min(area.x.saturating_add(max_x));
        frame.set_cursor_position((x, area.y));
    }
}

fn command_spans<'a>(app: &'a App, style: Style) -> Vec<Span<'a>> {
    let theme = &app.theme;
    let mut spans = vec![Span::styled(format!(":{}", app.command_buffer), style)];

    // Ghost text: remaining suffix of the selected completion.
    if let Some(sel) = app.completions.selected() {
        if sel.replace_from <= app.command_buffer.len() {
            let typed = &app.command_buffer[sel.replace_from..];
            if sel.text.starts_with(typed) && sel.text.len() > typed.len() {
                let ghost = sel.text[typed.len()..].to_string();
                spans.push(Span::styled(
                    ghost,
                    theme
                        .tone_style(theme.dim, theme.status_bg)
                        .add_modifier(Modifier::DIM),
                ));
            }
        }
    }

    spans
}
