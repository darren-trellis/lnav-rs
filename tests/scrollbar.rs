use teleminator::app::mouse::*;

#[test]
fn scroll_index_maps_track_ends() {
    assert_eq!(scroll_index_at(11, 0, 100, 10), 0);
    assert_eq!(scroll_index_at(11, 10, 100, 10), 90);
    assert_eq!(scroll_index_at(11, 5, 100, 10), 45);
}

#[test]
fn scroll_index_maps_horizontal_track() {
    assert_eq!(scroll_index_at(21, 0, 200, 20), 0);
    assert_eq!(scroll_index_at(21, 20, 200, 20), 180);
}
