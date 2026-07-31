use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::model::{FieldValue, LineFormat};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    app.hit.overlay = area;
    let Some(entry) = app.selected_entry() else {
        return;
    };
    let theme = &app.theme;

    let title = match entry.format {
        LineFormat::Json => " details · json ",
        LineFormat::Logfmt => " details · logfmt ",
        LineFormat::Plain => " details · raw ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border.fg))
        .title(title)
        .title_bottom(" Enter/Esc close ")
        .style(
            Style::default()
                .bg(theme.overlay_bg)
                .fg(theme.foreground.fg),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    let surface = theme.overlay_bg;

    lines.push(Line::from(vec![
        Span::styled("file ", theme.tone_style(theme.dim, surface)),
        Span::styled(
            entry.line_no.to_string(),
            theme
                .tone_style(theme.number, surface)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  level ", theme.tone_style(theme.dim, surface)),
        Span::styled(
            entry.level.as_str(),
            theme.level_style(entry.level).add_modifier(Modifier::BOLD),
        ),
    ]));

    if entry.fields.is_empty() {
        lines.push(Line::from(Span::styled(
            entry.raw.clone(),
            theme.tone_style(theme.foreground, surface),
        )));
    } else {
        for field in &entry.fields {
            let value_style = theme.field_value_style(&field.value, surface);
            let value = match &field.value {
                FieldValue::String(s) => format!("\"{s}\""),
                FieldValue::Nested(s) => s.clone(),
                other => other.display(),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<16}", field.key),
                    theme
                        .tone_style(theme.key, surface)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(value, value_style),
            ]));
        }
    }

    let mut paragraph = Paragraph::new(lines).style(Style::default().bg(theme.overlay_bg));
    if app.config.wrap_details {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, inner);
}
