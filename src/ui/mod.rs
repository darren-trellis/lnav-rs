mod list;
mod overlay;
mod sidebar;
mod spans;
mod status;
mod suggest;
mod tabs;
pub(crate) mod config_modal;
pub mod help_modal;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::app::{App, InputMode, PrimaryTab};
use crate::details;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ScrollbarLayout {
    pub content: Rect,
    pub vertical: Option<Rect>,
    pub horizontal: Option<Rect>,
    pub corner: Option<Rect>,
}

/// Split `area` into content plus optional vertical (right) and horizontal (bottom) bars.
pub(crate) fn split_scrollbars(area: Rect, vertical: bool, horizontal: bool) -> ScrollbarLayout {
    let vertical = vertical && area.width >= 2;
    let horizontal = horizontal && area.height >= 2;
    match (vertical, horizontal) {
        (false, false) => ScrollbarLayout {
            content: area,
            ..Default::default()
        },
        (true, false) => {
            let chunks =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(1)]).split(area);
            ScrollbarLayout {
                content: chunks[0],
                vertical: Some(chunks[1]),
                ..Default::default()
            }
        }
        (false, true) => {
            let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            ScrollbarLayout {
                content: chunks[0],
                horizontal: Some(chunks[1]),
                ..Default::default()
            }
        }
        (true, true) => {
            if area.width < 2 || area.height < 2 {
                return ScrollbarLayout {
                    content: area,
                    ..Default::default()
                };
            }
            let content = Rect {
                x: area.x,
                y: area.y,
                width: area.width - 1,
                height: area.height - 1,
            };
            let vertical = Rect {
                x: area.x + area.width - 1,
                y: area.y,
                width: 1,
                height: area.height - 1,
            };
            let horizontal = Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: area.width - 1,
                height: 1,
            };
            let corner = Rect {
                x: area.x + area.width - 1,
                y: area.y + area.height - 1,
                width: 1,
                height: 1,
            };
            ScrollbarLayout {
                content,
                vertical: Some(vertical),
                horizontal: Some(horizontal),
                corner: Some(corner),
            }
        }
    }
}

/// Backward-compatible vertical-only split used by the details overlay.
pub(crate) fn split_scrollbar(area: Rect, enabled: bool) -> (Rect, Option<Rect>) {
    let layout = split_scrollbars(area, enabled, false);
    (layout.content, layout.vertical)
}

pub(crate) fn draw_scrollbar(
    frame: &mut Frame,
    area: Rect,
    content_len: usize,
    position: usize,
    viewport_len: usize,
    orientation: ScrollbarOrientation,
    thumb: Style,
    track: Style,
) {
    let scrollable = content_len.saturating_sub(viewport_len);
    let empty = match orientation {
        ScrollbarOrientation::VerticalRight | ScrollbarOrientation::VerticalLeft => {
            area.height == 0
        }
        ScrollbarOrientation::HorizontalBottom | ScrollbarOrientation::HorizontalTop => {
            area.width == 0
        }
    };
    if scrollable == 0 || empty {
        return;
    }
    let mut state = ScrollbarState::new(scrollable).position(position);
    let bar = Scrollbar::new(orientation)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_style(thumb)
        .track_style(track);
    frame.render_stateful_widget(bar, area, &mut state);
}

pub(crate) fn draw_scrollbar_corner(frame: &mut Frame, area: Rect, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(style), area);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.pointer.hit = crate::app::HitAreas::default();

    let modal_open = app.config_modal.is_some() || app.help_modal.is_some();
    let suggest_h = if app.input_mode == InputMode::Command && !modal_open {
        suggest::desired_height(app, area.height.saturating_sub(4))
    } else {
        0
    };

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(suggest_h),
        Constraint::Length(1),
    ])
    .split(area);

    tabs::draw(frame, app, chunks[0]);

    let body = chunks[1];
    let show_sidebar = app.config.sidebar;
    let sidebar_w = if show_sidebar {
        sidebar::desired_width(body.width, app.config.sidebar_width)
    } else {
        0
    };

    let (main, sidebar_area) = if sidebar_w > 0 {
        let split =
            Layout::horizontal([Constraint::Min(20), Constraint::Length(sidebar_w)]).split(body);
        (split[0], Some(split[1]))
    } else {
        (body, None)
    };
    app.pointer.hit.main = main;

    let detail_content = if app.details.visible {
        app.selected_entry()
            .map(|entry| details::build_lines(entry, &app.theme, &app.config, &app.details.folded))
    } else {
        None
    };
    let overlay_height = detail_content.as_ref().map_or(0, |content| {
        details::desired_height(content.len(), main.height, app.config.details_max_height)
    });

    if overlay_height > 0 {
        let split =
            Layout::vertical([Constraint::Min(3), Constraint::Length(overlay_height)]).split(main);
        match app.primary_tab {
            PrimaryTab::Logs => list::draw(frame, app, split[0]),
            PrimaryTab::Spans => spans::draw(frame, app, split[0]),
        }
        overlay::draw(
            frame,
            app,
            split[1],
            detail_content.as_deref().unwrap_or_default(),
        );
    } else {
        match app.primary_tab {
            PrimaryTab::Logs => list::draw(frame, app, main),
            PrimaryTab::Spans => spans::draw(frame, app, main),
        }
    }

    if let Some(area) = sidebar_area {
        sidebar::draw(frame, app, area);
    } else if app.is_sidebar_focused() {
        app.focus_list();
    }

    if suggest_h > 0 {
        suggest::draw(frame, app, chunks[2]);
    }
    status::draw(frame, app, chunks[3]);

    // Center modals over the main pane so the sidebar stays visible beside them.
    if app.config_modal.is_some() {
        config_modal::draw(frame, app, main);
    }
    if app.help_modal.is_some() {
        help_modal::draw(frame, app, main);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_scrollbars_reserves_axes() {
        let area = Rect::new(0, 0, 20, 10);

        let none = split_scrollbars(area, false, false);
        assert_eq!(none.content, area);
        assert!(none.vertical.is_none());
        assert!(none.horizontal.is_none());

        let vert = split_scrollbars(area, true, false);
        assert_eq!(vert.content.width, 19);
        assert_eq!(vert.content.height, 10);
        assert_eq!(vert.vertical.map(|r| (r.width, r.height)), Some((1, 10)));

        let horiz = split_scrollbars(area, false, true);
        assert_eq!(horiz.content.width, 20);
        assert_eq!(horiz.content.height, 9);
        assert_eq!(horiz.horizontal.map(|r| (r.width, r.height)), Some((20, 1)));

        let both = split_scrollbars(area, true, true);
        assert_eq!(both.content.width, 19);
        assert_eq!(both.content.height, 9);
        assert_eq!(both.vertical.map(|r| (r.width, r.height)), Some((1, 9)));
        assert_eq!(both.horizontal.map(|r| (r.width, r.height)), Some((19, 1)));
        assert_eq!(
            both.corner.map(|r| (r.x, r.y, r.width, r.height)),
            Some((19, 9, 1, 1))
        );
    }
}
