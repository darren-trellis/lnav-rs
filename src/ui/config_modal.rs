use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, ConfigModal};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    match &app.config_modal {
        Some(ConfigModal::Picker(_)) => draw_picker(frame, app, area),
        Some(ConfigModal::Editor(_)) => draw_editor(frame, app, area),
        None => {}
    }
}

fn draw_picker(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(ConfigModal::Picker(picker)) = &app.config_modal else {
        return;
    };
    let rows = picker.values.len().clamp(1, 12) as u16;
    let width = picker
        .values
        .iter()
        .map(|n| n.len())
        .max()
        .unwrap_or(8)
        .saturating_add(8)
        .max(picker.option_name.len() + 4)
        .clamp(16, 56) as u16;
    let h = rows + 2;
    if width == 0 || h == 0 || area.width < 10 || area.height < 4 {
        return;
    }

    let [popup] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(popup);

    frame.render_widget(Clear, popup);

    let selected = picker.selected;
    let values = picker.values.clone();
    let committed = picker.previous_value.clone();
    let title = format!(" {} ", picker.option_name);
    let block = {
        let theme = &app.theme;
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border.fg))
            .title(title)
            .style(
                Style::default()
                    .bg(theme.overlay_bg)
                    .fg(theme.foreground.fg),
            )
    };
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let start = selected.saturating_sub(inner.height as usize / 2);
    let end = (start + inner.height as usize).min(values.len());

    if let Some(ConfigModal::Picker(picker)) = &mut app.config_modal {
        picker.popup_area = popup;
        picker.list_area = inner;
        picker.list_start = start;
    }

    let theme = &app.theme;
    let row_w = inner.width as usize;
    let mut lines = Vec::new();
    for (idx, name) in values.iter().enumerate().take(end).skip(start) {
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

fn draw_editor(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(ConfigModal::Editor(editor)) = &app.config_modal else {
        return;
    };
    let title = format!(" {} ", editor.option_name);
    let content_w = editor
        .buffer
        .len()
        .max(editor.previous_value.len())
        .saturating_add(4)
        .max(editor.option_name.len() + 4)
        .clamp(16, 56);
    let width = content_w as u16;
    let h = 3u16;
    if area.width < 10 || area.height < 3 {
        return;
    }

    let [popup] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(popup);

    frame.render_widget(Clear, popup);

    let buffer = editor.buffer.clone();
    let block = {
        let theme = &app.theme;
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.window_focus_border.fg))
            .title(title)
            .style(
                Style::default()
                    .bg(theme.overlay_bg)
                    .fg(theme.foreground.fg),
            )
    };
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if let Some(ConfigModal::Editor(editor)) = &mut app.config_modal {
        editor.popup_area = popup;
    }

    let theme = &app.theme;
    let display = format!("{buffer}▌");
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            display,
            theme.tone_style(theme.foreground, theme.overlay_bg),
        )))
        .style(Style::default().bg(theme.overlay_bg)),
        inner,
    );
}
