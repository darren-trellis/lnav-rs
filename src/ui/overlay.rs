use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::model::{FieldValue, LineFormat};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let Some(entry) = app.selected_entry() else {
        return;
    };

    let title = match entry.format {
        LineFormat::Json => " details · json ",
        LineFormat::Logfmt => " details · logfmt ",
        LineFormat::Plain => " details · raw ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(title)
        .title_bottom(" Enter/Esc close ")
        .style(Style::default().bg(theme.overlay_bg).fg(theme.foreground));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("line ", Style::default().fg(theme.dim)),
        Span::styled(
            entry.line_no.to_string(),
            Style::default()
                .fg(theme.number)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  level ", Style::default().fg(theme.dim)),
        Span::styled(
            entry.level.as_str(),
            theme.level_style(entry.level).add_modifier(Modifier::BOLD),
        ),
    ]));

    if entry.fields.is_empty() {
        lines.push(Line::from(Span::styled(
            entry.raw.clone(),
            Style::default().fg(theme.foreground),
        )));
    } else {
        for field in &entry.fields {
            let value_style = theme.field_value_style(&field.value);
            let value = match &field.value {
                FieldValue::String(s) => format!("\"{s}\""),
                FieldValue::Nested(s) => s.clone(),
                other => other.display(),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<16}", field.key),
                    Style::default()
                        .fg(theme.key)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(value, value_style),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(theme.overlay_bg));
    frame.render_widget(paragraph, inner);
}
