use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::text;

pub const HELP_LINES: &[&str] = &[
    "Navigation",
    "  j/↓  k/↑     move selection (count: 5j)",
    "  PgDn/Space   page down          PgUp  page up  (page_lines)",
    "  g/Home       top                G/End  bottom (follow)",
    "  Tab          cycle focus: list → details → sidebar",
    "",
    "Views",
    "  Enter        open details · reveal hidden (sidebar)",
    "  Esc          clear status (close pane in details/sidebar)",
    "  s            toggle filters/hidden sidebar",
    "  h/l ←/→      scroll list/sidebar horizontally",
    "  c            copy focused details value",
    "  Space        fold (details) · toggle filter (sidebar)",
    "",
    "Search & filter",
    "  /            search (regex)     n/N  next/prev match",
    "  :filter in   keep matching      :filter out  hide matching",
    "  :filter set  enable/disable one filter (Space in sidebar)",
    "  :filter on/off/toggle   master filtering switch",
    "",
    "Edit view",
    "  d            hide (dd, dj, …) · unhide (sidebar)",
    "  D            delete from file (DD, Dj, …)",
    "  p            pin/unpin sticky line",
    "  Backspace    hide line (list) · delete/unhide (sidebar)",
    "  S-Backspace  delete line (same as DD)",
    "  :hide clear|unhide|reveal   restore / unhide / jump",
    "  :pin clear   unpin all",
    "",
    "Config",
    "  :config set KEY [VAL]   set option (omit VAL for picker)",
    "  :config get/save/load   inspect / write / reload config",
    "  :config set theme NAME  change theme",
    "",
    "Other",
    "  :            command mode       ?  this help",
    "  q            quit",
    "",
    "Keys: [keys] · [keys.details] · [keys.sidebar]",
];

pub fn content_width() -> usize {
    HELP_LINES
        .iter()
        .map(|line| UnicodeWidthStr::width(*line))
        .max()
        .unwrap_or(0)
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(modal) = &app.help_modal else {
        return;
    };
    let scroll_y = modal.scroll_y;
    let scroll_x = modal.scroll_x;
    let content_w = content_width();
    let width = area.width.saturating_sub(2).clamp(24, 72);
    let height = area.height.saturating_sub(2).clamp(8, (HELP_LINES.len() as u16) + 2);
    if width < 20 || height < 5 {
        return;
    }

    let [popup] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(popup);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(app.theme.window_focus_border.fg)
                .add_modifier(Modifier::BOLD),
        )
        .title(" help ")
        .title_bottom(" Esc close · j/k ↕ · h/l ↔ ")
        .style(
            Style::default()
                .bg(app.theme.overlay_bg)
                .fg(app.theme.foreground.fg),
        );
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if let Some(modal) = &mut app.help_modal {
        modal.popup_area = popup;
        modal.viewport_h = inner.height as usize;
        modal.viewport_w = inner.width as usize;
        modal.content_w = content_w;
        let max_y = HELP_LINES.len().saturating_sub(modal.viewport_h);
        let max_x = content_w.saturating_sub(modal.viewport_w);
        modal.scroll_y = scroll_y.min(max_y);
        modal.scroll_x = scroll_x.min(max_x);
    }
    let scroll_y = app.help_modal.as_ref().map(|m| m.scroll_y).unwrap_or(0);
    let scroll_x = app.help_modal.as_ref().map(|m| m.scroll_x).unwrap_or(0);
    let viewport_h = inner.height as usize;
    let viewport_w = inner.width as usize;
    let end = (scroll_y + viewport_h).min(HELP_LINES.len());

    let mut lines = Vec::with_capacity(end.saturating_sub(scroll_y));
    for line in HELP_LINES.iter().take(end).skip(scroll_y) {
        let is_heading = !line.is_empty() && !line.starts_with(' ');
        let style = if is_heading {
            app.theme
                .tone_style(app.theme.search_match, app.theme.overlay_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            app.theme.tone_style(app.theme.foreground, app.theme.overlay_bg)
        };
        let display = text::slice_width(line, scroll_x, viewport_w);
        let used = UnicodeWidthStr::width(display.as_str());
        let mut spans = vec![Span::styled(display, style)];
        if used < viewport_w {
            spans.push(Span::styled(
                " ".repeat(viewport_w - used),
                Style::default().bg(app.theme.overlay_bg),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(app.theme.overlay_bg)),
        inner,
    );
}
