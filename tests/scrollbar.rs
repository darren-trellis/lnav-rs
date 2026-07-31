use ratatui::layout::Rect;

use lnav_rs::app::mouse::*;


#[test]
fn scroll_index_maps_track_ends() {
    let bar = Rect::new(10, 5, 1, 11);
    assert_eq!(scroll_index_at(bar, 5, 100, 10), 0);
    assert_eq!(scroll_index_at(bar, 15, 100, 10), 90);
    assert_eq!(scroll_index_at(bar, 10, 100, 10), 45);
}
