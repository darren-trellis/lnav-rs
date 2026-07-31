mod list;
mod overlay;
mod status;
mod suggest;
mod theme_picker;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::{App, InputMode};

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
    let content_lines = if entry.fields.is_empty() {
        3
    } else {
        entry.fields.len() + 2
    };
    let max = (available as usize).saturating_sub(4).max(4);
    (content_lines.min(max).max(4)) as u16
}
