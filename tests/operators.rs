use std::fs;
use std::io::Write;

use lnav_rs::app::{App, Focus, SidebarItem};
use lnav_rs::command;
use lnav_rs::config::Config;
use lnav_rs::tail::LogSource;

fn temp_log(name: &str, lines: &[&str]) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "lnav-rs-op-{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("log.jsonl");
    {
        let mut f = fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
    }
    (dir, path)
}

fn app_for(path: &std::path::Path) -> App {
    let mut config = Config::default();
    config.follow = false;
    config.session_filters = false;
    let source = LogSource::open_file(path).unwrap();
    let config_path = path.parent().unwrap().join("config.toml");
    App::new(source, config, config_path).unwrap()
}

#[test]
fn pin_survives_delete_of_earlier_line() {
    let (dir, path) = temp_log(
        "pin",
        &[
            r#"{"level":"info","msg":"alpha"}"#,
            r#"{"level":"info","msg":"bravo"}"#,
            r#"{"level":"info","msg":"charlie"}"#,
        ],
    );
    let mut app = app_for(&path);
    assert_eq!(app.source.len(), 3);

    // Pin charlie (display/source index 2).
    app.count = Some(3);
    command::execute(&mut app, "nav top");
    assert!(
        app.selected_entry()
            .is_some_and(|e| e.raw.contains("charlie"))
    );
    command::execute(&mut app, "pin");
    assert_eq!(app.pin_count(), 1);
    assert!(app.is_display_pinned(0));
    assert_eq!(app.source_at_display(0), Some(2));

    // Delete alpha (first body row after the pin band).
    app.count = None;
    command::execute(&mut app, "nav top");
    assert!(
        app.selected_entry()
            .is_some_and(|e| e.raw.contains("alpha"))
    );
    command::execute(&mut app, "delete");

    assert_eq!(app.source.len(), 2);
    assert_eq!(app.pin_count(), 1);
    assert!(app.is_display_pinned(0));
    let pinned_src = app.source_at_display(0).unwrap();
    assert!(
        app.source.entries()[pinned_src].raw.contains("charlie"),
        "pinned row should still be charlie after earlier delete"
    );
    assert_eq!(pinned_src, 1, "charlie should remap from source 2 -> 1");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn hide_survives_delete_of_unrelated_line() {
    let (dir, path) = temp_log(
        "hide",
        &[
            r#"{"level":"info","msg":"alpha"}"#,
            r#"{"level":"info","msg":"bravo"}"#,
            r#"{"level":"info","msg":"charlie"}"#,
        ],
    );
    let mut app = app_for(&path);

    // Hide charlie.
    app.count = Some(3);
    command::execute(&mut app, "nav top");
    command::execute(&mut app, "hide");
    assert_eq!(app.hidden_count(), 1);
    assert_eq!(app.display_len(), 2);

    // Delete alpha.
    app.count = Some(1);
    command::execute(&mut app, "nav top");
    assert!(
        app.selected_entry()
            .is_some_and(|e| e.raw.contains("alpha"))
    );
    command::execute(&mut app, "delete");

    assert_eq!(app.source.len(), 2);
    assert_eq!(app.hidden_count(), 1, "unrelated hide should be remapped, not cleared");
    assert_eq!(app.display_len(), 1);
    assert!(app.source.entries()[0].raw.contains("bravo"));
    // charlie remaps from source 2 -> 1 and stays hidden (not in the display list).
    assert!(app.source.entries()[1].raw.contains("charlie"));
    assert!(app.display_of_source(1).is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sidebar_dd_deletes_hidden_line_from_file() {
    let (dir, path) = temp_log(
        "sidebar-del",
        &[
            r#"{"level":"info","msg":"keep"}"#,
            r#"{"level":"error","msg":"drop-me"}"#,
            r#"{"level":"info","msg":"also-keep"}"#,
        ],
    );
    let mut app = app_for(&path);

    app.count = Some(2);
    command::execute(&mut app, "nav top");
    assert!(
        app.selected_entry()
            .is_some_and(|e| e.raw.contains("drop-me"))
    );
    command::execute(&mut app, "hide");
    assert_eq!(app.hidden_count(), 1);
    assert_eq!(app.source.len(), 3);

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    assert_eq!(app.focus(), Focus::Sidebar);
    app.select_sidebar_item(SidebarItem::Hidden(1));
    assert_eq!(app.sidebar_selection(), Some(SidebarItem::Hidden(1)));

    // DD via keybinding invoke.
    command::execute_from_key(&mut app, "delete");
    command::execute_from_key(&mut app, "delete");

    assert_eq!(app.source.len(), 2);
    assert!(app.source.entries().iter().all(|e| !e.raw.contains("drop-me")));
    assert!(app.source.entries().iter().any(|e| e.raw.contains("keep")));
    assert!(app.source.entries().iter().any(|e| e.raw.contains("also-keep")));
    assert_eq!(app.hidden_count(), 0);

    let on_disk = fs::read_to_string(&path).unwrap();
    assert!(!on_disk.contains("drop-me"));
    assert!(on_disk.contains("also-keep"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sidebar_delete_line_on_hidden_is_immediate() {
    let (dir, path) = temp_log(
        "sidebar-del-line",
        &[
            r#"{"level":"info","msg":"alpha"}"#,
            r#"{"level":"info","msg":"gone"}"#,
        ],
    );
    let mut app = app_for(&path);
    app.count = Some(2);
    command::execute(&mut app, "nav top");
    command::execute(&mut app, "hide");

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    app.select_sidebar_item(SidebarItem::Hidden(1));
    command::execute(&mut app, "delete line");

    assert_eq!(app.source.len(), 1);
    assert!(app.source.entries()[0].raw.contains("alpha"));
    assert!(!fs::read_to_string(&path).unwrap().contains("gone"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sidebar_dd_on_filter_deletes_matching_lines() {
    let (dir, path) = temp_log(
        "sidebar-filter-del",
        &[
            r#"{"level":"info","msg":"keep"}"#,
            r#"{"level":"error","msg":"boom"}"#,
            r#"{"level":"warn","msg":"careful"}"#,
            r#"{"level":"error","msg":"again"}"#,
        ],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "filter out error");
    assert_eq!(app.filters.len(), 1);
    assert_eq!(app.display_len(), 2);

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    app.select_sidebar_item(SidebarItem::Filter(0));

    command::execute_from_key(&mut app, "delete");
    command::execute_from_key(&mut app, "delete");

    assert_eq!(app.source.len(), 2);
    assert!(app.source.entries().iter().all(|e| !e.raw.contains("error")));
    assert!(app.source.entries().iter().any(|e| e.raw.contains("keep")));
    assert!(app.source.entries().iter().any(|e| e.raw.contains("careful")));
    // Filter itself remains; dd still removes the filter definition.
    assert_eq!(app.filters.len(), 1);
    let on_disk = fs::read_to_string(&path).unwrap();
    assert!(!on_disk.contains("boom"));
    assert!(on_disk.contains("keep"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sidebar_dd_on_include_filter_deletes_matches() {
    let (dir, path) = temp_log(
        "sidebar-filter-in-del",
        &[
            r#"{"level":"info","msg":"noise"}"#,
            r#"{"level":"error","msg":"keep-me-not"}"#,
            r#"{"level":"info","msg":"more-noise"}"#,
        ],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "filter in error");
    assert_eq!(app.display_len(), 1);

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    app.select_sidebar_item(SidebarItem::Filter(0));
    command::execute(&mut app, "delete line");

    assert_eq!(app.source.len(), 2);
    assert!(app.source.entries().iter().all(|e| !e.raw.contains("error")));

    let _ = fs::remove_dir_all(&dir);
}
