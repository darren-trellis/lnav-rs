use teleminator::history::*;
use std::fs;

#[test]
fn push_skips_empty_and_consecutive_dupes() {
    let mut h = History::default();
    h.push("  ");
    h.push("filter list");
    h.push("filter list");
    h.push("theme list");
    assert_eq!(h.entries(), &["filter list", "theme list"]);
}

#[test]
fn up_down_restores_staging() {
    let mut h = History::default();
    h.push("one");
    h.push("two");
    assert_eq!(h.up("draft").as_deref(), Some("two"));
    assert_eq!(h.up("draft").as_deref(), Some("one"));
    assert_eq!(h.up("draft").as_deref(), Some("one"));
    assert_eq!(h.down().as_deref(), Some("two"));
    assert_eq!(h.down().as_deref(), Some("draft"));
    assert_eq!(h.down(), None);
}

#[test]
fn roundtrip_file() {
    let dir = std::env::temp_dir().join(format!("teleminator-hist-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("search_history");
    let mut h = History::default();
    h.push("a");
    h.push("b");
    h.save_to(&path).unwrap();
    let loaded = History::load_from(&path).unwrap();
    assert_eq!(loaded.entries(), &["a", "b"]);
    let _ = fs::remove_dir_all(&dir);
}
