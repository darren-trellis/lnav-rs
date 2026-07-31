use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::model::{LineFormat, LogEntry};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let viewport = inner.height as usize;
    app.ensure_visible(viewport);

    let theme = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(format!(" {} ", app.source.path().display()))
        .style(Style::default().bg(theme.background).fg(theme.foreground));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.source.len() == 0 {
        let empty = Paragraph::new(Line::from(Span::styled(
            " waiting for log lines… ",
            Style::default().fg(theme.dim),
        )))
        .style(Style::default().bg(theme.background));
        frame.render_widget(empty, inner);
        return;
    }

    let end = (app.scroll + viewport).min(app.source.len());
    let mut lines = Vec::with_capacity(end - app.scroll);

    for idx in app.scroll..end {
        let entry = &app.source.entries()[idx];
        let selected = idx == app.selected;
        let is_match = app.search_matches.binary_search(&idx).is_ok();
        lines.push(render_line(app, entry, selected, is_match, inner.width as usize));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(theme.background));
    frame.render_widget(paragraph, inner);
}

fn render_line<'a>(
    app: &'a App,
    entry: &'a LogEntry,
    selected: bool,
    is_match: bool,
    width: usize,
) -> Line<'a> {
    let theme = &app.theme;
    let gutter = if selected { "▌" } else { " " };
    let level = format!("{:<5}", entry.level.as_str());
    let ts = entry
        .timestamp
        .as_deref()
        .map(|t| format!("{t} "))
        .unwrap_or_default();
    let fmt = match entry.format {
        LineFormat::Json => "json",
        LineFormat::Logfmt => "logf",
        LineFormat::Plain => "text",
    };
    let msg = entry.summary_message();

    let mut base = Style::default().bg(theme.background).fg(theme.foreground);
    if selected {
        base = theme.selection_style();
    } else if is_match {
        base = base.bg(theme.background).fg(theme.search_match);
    }

    let level_style = if selected {
        base.add_modifier(Modifier::BOLD)
    } else {
        theme.level_style(entry.level).bg(theme.background)
    };

    let ts_style = if selected {
        base
    } else {
        Style::default().fg(theme.timestamp).bg(theme.background)
    };

    let dim = if selected {
        base
    } else {
        Style::default().fg(theme.dim).bg(theme.background)
    };

    let prefix = format!("{gutter}{level} {ts}");
    let meta = format!(" {fmt}");
    let used = UnicodeWidthStr::width(prefix.as_str()) + UnicodeWidthStr::width(meta.as_str());
    let msg_budget = width.saturating_sub(used).max(8);
    let msg_disp = truncate(msg, msg_budget);

    Line::from(vec![
        Span::styled(gutter.to_string(), base),
        Span::styled(level, level_style),
        Span::styled(" ", base),
        Span::styled(ts, ts_style),
        Span::styled(msg_disp, base),
        Span::styled(meta, dim),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        if w + cw + 1 > max {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}
