mod list;
mod overlay;
mod status;
mod suggest;
mod theme_picker;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::details;

/// Split `area` into content + optional 1-column scrollbar strip on the right.
pub(crate) fn split_scrollbar(area: Rect, enabled: bool) -> (Rect, Option<Rect>) {
    if !enabled || area.width < 2 {
        return (area, None);
    }
    let chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(1)]).split(area);
    (chunks[0], Some(chunks[1]))
}

pub(crate) fn draw_scrollbar(
    frame: &mut Frame,
    area: Rect,
    content_len: usize,
    position: usize,
    viewport_len: usize,
    thumb: Style,
    track: Style,
) {
    let scrollable = content_len.saturating_sub(viewport_len);
    if scrollable == 0 || area.height == 0 {
        return;
    }
    let mut state = ScrollbarState::new(scrollable).position(position);
    let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .thumb_style(thumb)
        .track_style(track);
    frame.render_stateful_widget(bar, area, &mut state);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.hit = crate::app::HitAreas::default();

    let suggest_h = if app.input_mode == InputMode::Command && app.theme_picker.is_none() {
        suggest::desired_height(app, area.height.saturating_sub(4))
    } else {
        0
    };

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(suggest_h),
        Constraint::Length(1),
    ])
    .split(area);

    let body = chunks[0];
    let overlay_height = if app.show_overlay && app.theme_picker.is_none() {
        overlay_desired_height(app, body.height)
    } else {
        0
    };

    if overlay_height > 0 {
        let split = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(overlay_height),
        ])
        .split(body);
        list::draw(frame, app, split[0]);
        overlay::draw(frame, app, split[1]);
    } else {
        list::draw(frame, app, body);
    }

    if suggest_h > 0 {
        suggest::draw(frame, app, chunks[1]);
    }
    status::draw(frame, app, chunks[2]);

    if app.theme_picker.is_some() {
        theme_picker::draw(frame, app, body);
    }
}

fn overlay_desired_height(app: &App, available: u16) -> u16 {
    let Some(entry) = app.selected_entry() else {
        return 0;
    };
    let content_lines =
        details::build_lines(entry, &app.theme, &app.config, &app.overlay_folded).len();
    details::desired_height(content_lines, available, app.config.details_max_height)
}
