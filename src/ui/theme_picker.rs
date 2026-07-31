use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::App;

pub fn desired_size(app: &App) -> (u16, u16) {
    let Some(picker) = &app.theme_picker else {
        return (0, 0);
    };
    let rows = picker.names.len().clamp(1, 12) as u16;
    let width = picker
        .names
        .iter()
        .map(|n| n.len())
        .max()
        .unwrap_or(8)
        .saturating_add(8)
        .clamp(28, 48) as u16;
    (width, rows + 2)
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.theme_picker else {
        return;
    };
    let (w, h) = desired_size(app);
    if w == 0 || h == 0 || area.width < 10 || area.height < 4 {
        return;
    }

    let [popup] = Layout::horizontal([Constraint::Length(w.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(popup);

    frame.render_widget(Clear, popup);

    let selected = picker.selected;
    let names = picker.names.clone();
    let committed = app.config.theme.name().to_string();
    let block = {
        let theme = &app.theme;
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border.fg))
            .title(" theme · ↑↓/hover preview · click/Enter · Esc ")
            .style(
                Style::default()
                    .bg(theme.overlay_bg)
                    .fg(theme.foreground.fg),
            )
    };
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let start = selected.saturating_sub(inner.height as usize / 2);
    let end = (start + inner.height as usize).min(names.len());

    if let Some(picker) = &mut app.theme_picker {
        picker.popup_area = popup;
        picker.list_area = inner;
        picker.list_start = start;
    }

    let theme = &app.theme;
    let row_w = inner.width as usize;
    let mut lines = Vec::new();
    for (idx, name) in names.iter().enumerate().take(end).skip(start) {
        let is_sel = idx == selected;
        let is_current = name == &committed;
        let marker = if is_sel { "▸ " } else { "  " };
        let mark = if is_current { " *" } else { "" };
        let style = if is_sel {
            theme.selection_style()
        } else {
            theme.tone_style(theme.foreground, theme.overlay_bg)
        };
        let text = format!("{marker}{name}{mark}");
        let used = UnicodeWidthStr::width(text.as_str());
        let mut spans = vec![Span::styled(text, style.add_modifier(Modifier::BOLD))];
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
