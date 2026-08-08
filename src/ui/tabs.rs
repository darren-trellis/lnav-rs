use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, PrimaryTab};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    app.pointer.hit.tabs = area;
    let theme = &app.theme;
    let bg = theme.status_bg();
    let inactive = Style::default().bg(bg).fg(theme.dim.fg);
    let active = Style::default()
        .bg(bg)
        .fg(theme.foreground.fg)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let logs_label = " Logs ";
    let spans_label = " Spans ";
    let logs_w = UnicodeWidthStr::width(logs_label) as u16;
    let spans_w = UnicodeWidthStr::width(spans_label) as u16;

    app.pointer.hit.tab_logs = Rect {
        x: area.x,
        y: area.y,
        width: logs_w.min(area.width),
        height: area.height.min(1),
    };
    let spans_x = area.x.saturating_add(logs_w);
    app.pointer.hit.tab_spans = Rect {
        x: spans_x,
        y: area.y,
        width: spans_w.min(area.width.saturating_sub(logs_w)),
        height: area.height.min(1),
    };

    let logs_style = if app.primary_tab == PrimaryTab::Logs {
        active
    } else {
        inactive
    };
    let spans_style = if app.primary_tab == PrimaryTab::Spans {
        active
    } else {
        inactive
    };

    let mut spans = vec![
        Span::styled(logs_label.to_string(), logs_style),
        Span::styled(spans_label.to_string(), spans_style),
    ];
    let used = logs_w.saturating_add(spans_w);
    if area.width > used {
        spans.push(Span::styled(
            " ".repeat((area.width - used) as usize),
            Style::default().bg(bg),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(bg)),
        area,
    );
}
