use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let total = app.source.len();
    let pos = if total == 0 { 0 } else { app.selected + 1 };
    let follow = if app.follow { "FOLLOW" } else { "PAUSED" };

    let left = match app.input_mode {
        InputMode::Search => format!("/{}", app.search_query),
        InputMode::Normal => app
            .status_message
            .clone()
            .unwrap_or_else(|| "? help".into()),
    };

    let right = format!(
        "{pos}/{total}  {follow}  {}  {}",
        app.theme.name, "lnav-rs"
    );

    let gap = area.width as usize;
    let left_w = left.chars().count();
    let right_w = right.chars().count();
    let pad = gap.saturating_sub(left_w + right_w).max(1);
    let line = format!("{left}{}{right}", " ".repeat(pad));

    let style = Style::default()
        .bg(theme.status_bg)
        .fg(theme.status_fg)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(line, style))).style(style),
        area,
    );
}
