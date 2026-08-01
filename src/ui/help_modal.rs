use std::collections::BTreeMap;

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::keys::{self, KeysConfig};
use crate::text;

#[derive(Clone, Copy)]
enum CheatContext {
    Base,
    Details,
    Sidebar,
}

enum CheatEntry {
    Heading(&'static str),
    Blank,
    /// Look up keys for each command group; join keys in a group with `/`,
    /// groups with ` · `, then ` — description`.
    Item {
        context: CheatContext,
        groups: &'static [&'static str],
        description: &'static str,
    },
}

/// Cheatsheet rows. Key columns are resolved from the live `[keys]` config.
const CHEATSHEET: &[CheatEntry] = &[
    CheatEntry::Heading("Navigation"),
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["nav down", "nav up"],
        description: "move (accepts a count)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["page down", "page up"],
        description: "page (page_lines)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["nav top"],
        description: "top",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["nav bottom"],
        description: "bottom (follow on list)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["scroll left", "scroll right"],
        description: "scroll horizontally",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["focus toggle"],
        description: "cycle focus: list → details → sidebar",
    },
    CheatEntry::Blank,
    CheatEntry::Heading("List"),
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["view details on"],
        description: "open details",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["command clear"],
        description: "clear status / cancel",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["view sidebar toggle"],
        description: "toggle filters/hidden sidebar",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["search"],
        description: "search (regex)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["match next", "match prev"],
        description: "next/prev match",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["hide"],
        description: "hide (dd, dj, …)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["hide line"],
        description: "hide line (same as dd)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["delete"],
        description: "delete from file (DD, Dj, …)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["delete line"],
        description: "delete line (same as DD)",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["pin"],
        description: "pin/unpin sticky line",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["command"],
        description: "command mode",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["help"],
        description: "this help",
    },
    CheatEntry::Item {
        context: CheatContext::Base,
        groups: &["quit"],
        description: "quit",
    },
    CheatEntry::Blank,
    CheatEntry::Heading("Details"),
    CheatEntry::Item {
        context: CheatContext::Details,
        groups: &["fold toggle"],
        description: "fold/unfold tree item",
    },
    CheatEntry::Item {
        context: CheatContext::Details,
        groups: &["view current off"],
        description: "close details",
    },
    CheatEntry::Item {
        context: CheatContext::Details,
        groups: &["search"],
        description: "search in overlay",
    },
    CheatEntry::Item {
        context: CheatContext::Details,
        groups: &["match next", "match prev"],
        description: "next/prev match",
    },
    CheatEntry::Item {
        context: CheatContext::Details,
        groups: &["copy"],
        description: "copy focused value",
    },
    CheatEntry::Item {
        context: CheatContext::Details,
        groups: &["hide", "delete"],
        description: "hide / delete (same as list)",
    },
    CheatEntry::Blank,
    CheatEntry::Heading("Sidebar"),
    CheatEntry::Item {
        context: CheatContext::Sidebar,
        groups: &["filter set toggle"],
        description: "toggle selected filter",
    },
    CheatEntry::Item {
        context: CheatContext::Sidebar,
        groups: &["hide reveal"],
        description: "reveal hidden line and jump",
    },
    CheatEntry::Item {
        context: CheatContext::Sidebar,
        groups: &["view current off"],
        description: "close sidebar",
    },
    CheatEntry::Item {
        context: CheatContext::Sidebar,
        groups: &["filter delete"],
        description: "delete filter / unhide line (dd)",
    },
    CheatEntry::Item {
        context: CheatContext::Sidebar,
        groups: &["filter delete line"],
        description: "delete filter / unhide line",
    },
];

pub fn render(keys: &KeysConfig) -> Vec<String> {
    let mut lines = Vec::with_capacity(CHEATSHEET.len());
    for entry in CHEATSHEET {
        match entry {
            CheatEntry::Heading(title) => lines.push((*title).to_string()),
            CheatEntry::Blank => lines.push(String::new()),
            CheatEntry::Item {
                context,
                groups,
                description,
            } => {
                if let Some(line) = render_item(keys, *context, groups, description) {
                    lines.push(line);
                }
            }
        }
    }
    lines
}

fn render_item(
    keys: &KeysConfig,
    context: CheatContext,
    groups: &[&str],
    description: &str,
) -> Option<String> {
    let (base, overlay) = maps_for(keys, context);
    let mut parts = Vec::new();
    for command in groups {
        let bound = keys::bindings_for_command(base, overlay, command);
        if bound.is_empty() {
            continue;
        }
        let labels: Vec<String> = bound.iter().map(|k| keys::display_key(k)).collect();
        parts.push(labels.join("/"));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("- {} — {description}", parts.join(" · ")))
}

fn maps_for(
    keys: &KeysConfig,
    context: CheatContext,
) -> (&BTreeMap<String, String>, Option<&BTreeMap<String, String>>) {
    match context {
        CheatContext::Base => (&keys.bindings, None),
        CheatContext::Details => (&keys.bindings, Some(&keys.details)),
        CheatContext::Sidebar => (&keys.bindings, Some(&keys.sidebar)),
    }
}

pub fn content_width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()))
        .max()
        .unwrap_or(0)
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(_) = &app.help_modal else {
        return;
    };
    let lines = render(&app.config.keys);
    let content_w = content_width(&lines);
    let line_count = lines.len();

    let scroll_y = app.help_modal.as_ref().map(|m| m.scroll_y).unwrap_or(0);
    let scroll_x = app.help_modal.as_ref().map(|m| m.scroll_x).unwrap_or(0);
    let width = area.width.saturating_sub(2).clamp(24, 72);
    let height = area.height.saturating_sub(2).clamp(8, (line_count as u16) + 2);
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
        modal.line_count = line_count;
        let max_y = line_count.saturating_sub(modal.viewport_h);
        let max_x = content_w.saturating_sub(modal.viewport_w);
        modal.scroll_y = scroll_y.min(max_y);
        modal.scroll_x = scroll_x.min(max_x);
    }
    let scroll_y = app.help_modal.as_ref().map(|m| m.scroll_y).unwrap_or(0);
    let scroll_x = app.help_modal.as_ref().map(|m| m.scroll_x).unwrap_or(0);
    let viewport_h = inner.height as usize;
    let viewport_w = inner.width as usize;
    let end = (scroll_y + viewport_h).min(line_count);

    let mut out = Vec::with_capacity(end.saturating_sub(scroll_y));
    for line in lines.iter().take(end).skip(scroll_y) {
        let is_heading = !line.is_empty() && !line.starts_with('-');
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
        out.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(out).style(Style::default().bg(app.theme.overlay_bg)),
        inner,
    );
}
