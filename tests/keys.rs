use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use lnav_rs::keys::*;


#[test]
fn encodes_case_sensitive_chars() {
    let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    let big = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
    assert_eq!(encode(d).as_deref(), Some("d"));
    assert_eq!(encode(big).as_deref(), Some("D"));
}

#[test]
fn default_d_maps_to_hide() {
    assert_eq!(defaults().get("d").map(String::as_str), Some("hide"));
    assert_eq!(defaults().get("D").map(String::as_str), Some("delete"));
    assert_eq!(defaults().get("q").map(String::as_str), Some("quit"));
    assert_eq!(
        defaults().get("s").map(String::as_str),
        Some("view sidebar toggle")
    );
    assert_eq!(
        defaults().get("backspace").map(String::as_str),
        Some("hide line")
    );
    assert_eq!(defaults().get("p").map(String::as_str), Some("pin line"));
    assert_eq!(
        sidebar_defaults().get("d").map(String::as_str),
        Some("filter delete")
    );
    assert_eq!(
        sidebar_defaults().get("backspace").map(String::as_str),
        Some("filter delete line")
    );
    assert_eq!(
        sidebar_defaults().get("space").map(String::as_str),
        Some("filter set toggle")
    );
    assert_eq!(
        defaults().get("esc").map(String::as_str),
        Some("view current off")
    );
    assert!(!defaults().contains_key("t"));
}
