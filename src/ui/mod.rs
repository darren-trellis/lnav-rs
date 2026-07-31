mod list;
mod overlay;
mod sidebar;
mod status;
mod suggest;
mod theme_picker;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

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
    app.pointer.hit = crate::app::HitAreas::default();

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
    let show_sidebar = app.config.sidebar && app.theme_picker.is_none();
    let sidebar_w = if show_sidebar {
        sidebar::desired_width(body.width)
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

    let detail_content = if app.details.visible && app.theme_picker.is_none() {
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
        list::draw(frame, app, split[0]);
        overlay::draw(
            frame,
            app,
            split[1],
            detail_content.as_deref().unwrap_or_default(),
        );
    } else {
        list::draw(frame, app, main);
    }

    if let Some(area) = sidebar_area {
        sidebar::draw(frame, app, area);
    } else if app.is_sidebar_focused() {
        app.focus_list();
    }

    if suggest_h > 0 {
        suggest::draw(frame, app, chunks[1]);
    }
    status::draw(frame, app, chunks[2]);

    if app.theme_picker.is_some() {
        theme_picker::draw(frame, app, body);
    }
}
