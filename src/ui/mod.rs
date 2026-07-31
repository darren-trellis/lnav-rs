mod list;
mod overlay;
mod status;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    let overlay_height = if app.show_overlay {
        overlay_desired_height(app, chunks[0].height)
    } else {
        0
    };

    if overlay_height > 0 {
        let split = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(overlay_height),
        ])
        .split(chunks[0]);
        list::draw(frame, app, split[0]);
        overlay::draw(frame, app, split[1]);
    } else {
        list::draw(frame, app, chunks[0]);
    }

    status::draw(frame, app, chunks[1]);
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
