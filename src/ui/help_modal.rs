use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::App;

pub const HELP_LINES: &[&str] = &[
    "Navigation",
    "  j/↓  k/↑     move selection (count: 5j)",
    "  PgDn/Space   page down          PgUp  page up",
    "  g/Home       top                G/End  bottom (follow)",
    "  Tab          cycle focus: list → details → sidebar",
    "",
    "Views",
    "  Enter        open details       Esc  close pane / clear status",
    "  s            toggle filters sidebar",
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
    "  d            hide operator (dd, dj, dG, …)",
    "  D            delete from file (DD, Dj, …)",
    "  p            pin/unpin sticky line",
    "  Backspace    hide line (list) · delete filter (sidebar)",
    "  :hide/:pin clear   restore hidden / unpin all",
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

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(modal) = &app.help_modal else {
        return;
    };
    let scroll = modal.scroll;
    let width = 64u16.min(area.width.saturating_sub(2).max(24));
    let height = (HELP_LINES.len() as u16 + 2)
        .min(area.height.saturating_sub(2).max(8))
        .min(area.height);
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
        .title_bottom(" Esc close · j/k scroll ")
        .style(
            Style::default()
                .bg(app.theme.overlay_bg)
                .fg(app.theme.foreground.fg),
        );
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if let Some(modal) = &mut app.help_modal {
        modal.popup_area = popup;
        modal.viewport = inner.height as usize;
        let max_scroll = HELP_LINES.len().saturating_sub(modal.viewport);
        modal.scroll = scroll.min(max_scroll);
    }
    let scroll = app.help_modal.as_ref().map(|m| m.scroll).unwrap_or(0);
    let viewport = inner.height as usize;
    let end = (scroll + viewport).min(HELP_LINES.len());
    let row_w = inner.width as usize;

    let mut lines = Vec::with_capacity(end.saturating_sub(scroll));
    for line in HELP_LINES.iter().take(end).skip(scroll) {
        let is_heading = !line.is_empty() && !line.starts_with(' ');
        let style = if is_heading {
            app.theme
                .tone_style(app.theme.search_match, app.theme.overlay_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            app.theme.tone_style(app.theme.foreground, app.theme.overlay_bg)
        };
        let display = crate::text::truncate_width(line, row_w);
        let used = UnicodeWidthStr::width(display.as_str());
        let mut spans = vec![Span::styled(display, style)];
        if used < row_w {
            spans.push(Span::styled(
                " ".repeat(row_w - used),
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
