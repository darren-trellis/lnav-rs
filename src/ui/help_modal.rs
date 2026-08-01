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
enum Context {
    Base,
    Details,
    Sidebar,
}

enum Entry {
    Heading(&'static str),
    Blank,
    Item {
        context: Context,
        groups: &'static [&'static str],
        description: &'static str,
    },
}

/// Cheatsheet rows. Key columns are resolved from the live `[keys]` config.
const CHEATSHEET: &[Entry] = &[
    Entry::Heading("Navigation"),
    Entry::Item {
        context: Context::Base,
        groups: &["nav down", "nav up"],
        description: "move (accepts a count)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["page down", "page up"],
        description: "page (page_lines)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["nav top"],
        description: "top",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["nav bottom"],
        description: "bottom (follow on list)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["scroll left", "scroll right"],
        description: "scroll horizontally",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["focus toggle"],
        description: "cycle focus: list → details → sidebar",
    },
    Entry::Item {
        context: Context::Base,
        groups: &[
            "config set sidebar_width +1",
            "config set sidebar_width -1",
        ],
        description: "resize sidebar (← grow · → shrink)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &[
            "config set details_max_height +1",
            "config set details_max_height -1",
        ],
        description: "resize details max height",
    },
    Entry::Blank,
    Entry::Heading("List"),
    Entry::Item {
        context: Context::Base,
        groups: &["view details on"],
        description: "open details",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["command clear"],
        description: "clear status / cancel",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["view sidebar toggle"],
        description: "toggle sidebar",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["search"],
        description: "search (regex)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["match next", "match prev"],
        description: "next/prev match",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["hide"],
        description: "hide",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["hide line"],
        description: "hide line (same as dd)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["delete"],
        description: "delete from file",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["delete line"],
        description: "delete line (same as DD)",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["delete all"],
        description: "clear all logs from file",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["pin"],
        description: "pin/unpin sticky line",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["command"],
        description: "command mode",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["help"],
        description: "this help",
    },
    Entry::Item {
        context: Context::Base,
        groups: &["quit"],
        description: "quit",
    },
    Entry::Blank,
    Entry::Heading("Details"),
    Entry::Item {
        context: Context::Details,
        groups: &["fold toggle"],
        description: "fold/unfold tree item",
    },
    Entry::Item {
        context: Context::Details,
        groups: &["view current off"],
        description: "close details",
    },
    Entry::Item {
        context: Context::Details,
        groups: &["search"],
        description: "search in overlay",
    },
    Entry::Item {
        context: Context::Details,
        groups: &["match next", "match prev"],
        description: "next/prev match",
    },
    Entry::Item {
        context: Context::Details,
        groups: &["copy"],
        description: "copy focused value",
    },
    Entry::Item {
        context: Context::Details,
        groups: &["hide", "delete"],
        description: "hide / delete (same as list)",
    },
    Entry::Blank,
    Entry::Heading("Sidebar"),
    Entry::Item {
        context: Context::Sidebar,
        groups: &["filter set toggle"],
        description: "toggle selected filter",
    },
    Entry::Item {
        context: Context::Sidebar,
        groups: &["hide reveal"],
        description: "reveal hidden line and jump",
    },
    Entry::Item {
        context: Context::Sidebar,
        groups: &["view current off"],
        description: "close sidebar",
    },
    Entry::Item {
        context: Context::Sidebar,
        groups: &["filter delete"],
        description: "delete filter / unhide (dd, d+motion)",
    },
    Entry::Item {
        context: Context::Sidebar,
        groups: &["filter delete line"],
        description: "delete filter / unhide line",
    },
    Entry::Item {
        context: Context::Sidebar,
        groups: &["delete", "delete line"],
        description: "delete hidden / filter matches (DD, D+motion)",
    },
];

pub fn render(keys: &KeysConfig) -> Vec<String> {
    let mut lines = Vec::with_capacity(CHEATSHEET.len());
    for entry in CHEATSHEET {
        match entry {
            Entry::Heading(title) => lines.push((*title).to_string()),
            Entry::Blank => lines.push(String::new()),
            Entry::Item {
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
    context: Context,
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
    context: Context,
) -> (&BTreeMap<String, String>, Option<&BTreeMap<String, String>>) {
    match context {
        Context::Base => (&keys.bindings, None),
        Context::Details => (&keys.bindings, Some(&keys.details)),
        Context::Sidebar => (&keys.bindings, Some(&keys.sidebar)),
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
