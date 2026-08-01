use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use lnav_rs::keys::*;
use lnav_rs::ui::help_modal;

#[test]
fn encodes_case_sensitive_chars() {
    let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    let big = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
    assert_eq!(encode(d).as_deref(), Some("d"));
    assert_eq!(encode(big).as_deref(), Some("D"));
}

#[test]
fn encodes_shift_backspace() {
    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::SHIFT);
    assert_eq!(encode(key).as_deref(), Some("S-backspace"));
    let plain = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(encode(plain).as_deref(), Some("backspace"));
}

#[test]
fn encodes_shift_super_chords() {
    let left = KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT | KeyModifiers::SUPER);
    assert_eq!(encode(left).as_deref(), Some("S-D-left"));
    let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::SHIFT | KeyModifiers::SUPER);
    assert_eq!(encode(h).as_deref(), Some("S-D-h"));
    let h_upper = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT | KeyModifiers::SUPER);
    assert_eq!(encode(h_upper).as_deref(), Some("S-D-h"));
}

#[test]
fn default_resize_bindings() {
    assert_eq!(
        defaults().get("S-D-left").map(String::as_str),
        Some("resize sidebar left")
    );
    assert_eq!(
        defaults().get("S-D-j").map(String::as_str),
        Some("resize details down")
    );
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
    assert_eq!(
        defaults().get("S-backspace").map(String::as_str),
        Some("delete line")
    );
    assert_eq!(defaults().get("p").map(String::as_str), Some("pin"));
    assert_eq!(
        defaults().get("enter").map(String::as_str),
        Some("view details on")
    );
    assert_eq!(
        defaults().get("h").map(String::as_str),
        Some("scroll left")
    );
    assert_eq!(
        defaults().get("l").map(String::as_str),
        Some("scroll right")
    );
    assert_eq!(
        defaults().get("left").map(String::as_str),
        Some("scroll left")
    );
    assert_eq!(
        defaults().get("right").map(String::as_str),
        Some("scroll right")
    );
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
        sidebar_defaults().get("enter").map(String::as_str),
        Some("hide reveal")
    );
    assert_eq!(
        sidebar_defaults().get("h").map(String::as_str),
        Some("scroll left")
    );
    assert_eq!(
        sidebar_defaults().get("l").map(String::as_str),
        Some("scroll right")
    );
    assert_eq!(
        sidebar_defaults().get("left").map(String::as_str),
        Some("scroll left")
    );
    assert_eq!(
        sidebar_defaults().get("right").map(String::as_str),
        Some("scroll right")
    );
    assert_eq!(
        defaults().get("esc").map(String::as_str),
        Some("command clear")
    );
    assert_eq!(
        details_defaults().get("esc").map(String::as_str),
        Some("view current off")
    );
    assert_eq!(
        sidebar_defaults().get("esc").map(String::as_str),
        Some("view current off")
    );
    assert!(!defaults().contains_key("t"));
}

#[test]
fn display_key_prettifies_specials() {
    assert_eq!(display_key("down"), "↓");
    assert_eq!(display_key("pagedown"), "PgDn");
    assert_eq!(display_key("S-backspace"), "S-Backspace");
    assert_eq!(display_key("C-c"), "C-c");
}

#[test]
fn cheatsheet_uses_configured_bindings() {
    let mut keys = KeysConfig::with_defaults();
    keys.bindings.insert("x".into(), "quit".into());
    let lines = help_modal::render(&keys);
    let quit = lines
        .iter()
        .find(|line| line.contains("— quit"))
        .expect("quit row");
    assert!(quit.contains('x'), "{quit}");
    assert!(quit.contains('q'), "{quit}");
}

#[test]
fn bindings_for_command_lists_aliases() {
    let base = defaults();
    let keys = bindings_for_command(&base, None, "nav down");
    assert!(keys.iter().any(|k| k == "j"));
    assert!(keys.iter().any(|k| k == "down"));
}
