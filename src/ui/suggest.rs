use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;

pub fn desired_height(app: &App, max: u16) -> u16 {
    if app.completions.items.is_empty() {
        return 0;
    }
    let n = app.completions.items.len().min(8) as u16;
    // +2 for borders
    (n + 2).min(max)
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.height < 2 || app.completions.items.is_empty() {
        return;
    }

    let block = {
        let theme = &app.theme;
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border.fg))
            .title(" completions · Tab cycle · ↑↓ then Tab/Enter · click ")
            .style(
                Style::default()
                    .bg(theme.overlay_bg)
                    .fg(theme.foreground.fg),
            )
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected_idx = app.completions.selected.unwrap_or(0);
    let start = selected_idx.saturating_sub(inner.height as usize / 2);
    let end = (start + inner.height as usize).min(app.completions.items.len());
    app.hit.suggest_inner = inner;
    app.hit.suggest_start = start;

    let theme = &app.theme;
    let row_w = inner.width as usize;
    let mut lines = Vec::new();
    for idx in start..end {
        let item = &app.completions.items[idx];
        let selected = app.completions.selected == Some(idx);
        let marker = if selected { "▸ " } else { "  " };
        let label = format!("{marker}{:<18}", item.label);
        let help_budget = row_w.saturating_sub(UnicodeWidthStr::width(label.as_str()) + 1);
        let help = truncate(&item.help, help_budget);

        let style = if selected {
            theme.selection_style()
        } else {
            theme.tone_style(theme.foreground, theme.overlay_bg)
        };
        let help_style = if selected {
            style
        } else {
            theme.tone_style(theme.dim, theme.overlay_bg)
        };

        let mut spans = vec![
            Span::styled(label, style.add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {help}"), help_style),
        ];
        let used = spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum::<usize>();
        if used < row_w {
            spans.push(Span::styled(" ".repeat(row_w - used), style));
        }

        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.overlay_bg)),
        inner,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
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
