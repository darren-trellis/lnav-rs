use std::fs;
use std::io::Write;

use teleminator::app::{App, Focus, PendingOp, SidebarItem, ToggleAction};
use teleminator::command;
use teleminator::config::Config;
use teleminator::tail::LogSource;

fn temp_log(name: &str, lines: &[&str]) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "teleminator-op-{}-{}",
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
    config.tail_mode = false;
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

#[test]
fn sidebar_dg_deletes_hidden_range_to_end() {
    let (dir, path) = temp_log(
        "sidebar-dg",
        &[
            r#"{"level":"info","msg":"keep"}"#,
            r#"{"level":"info","msg":"hide-a"}"#,
            r#"{"level":"info","msg":"hide-b"}"#,
            r#"{"level":"info","msg":"hide-c"}"#,
        ],
    );
    let mut app = app_for(&path);
    for line in [4usize, 3, 2] {
        app.count = Some(line);
        command::execute(&mut app, "nav top");
        command::execute(&mut app, "hide");
    }
    assert_eq!(app.hidden_count(), 3);
    assert_eq!(app.source.len(), 4);

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    // Sidebar order: hidden lines sorted — hide-a, hide-b, hide-c (sources 1,2,3).
    app.select_sidebar_item(SidebarItem::Hidden(1));
    command::execute_from_key(&mut app, "delete");
    command::execute_from_key(&mut app, "nav bottom");

    assert_eq!(app.source.len(), 1);
    assert!(app.source.entries()[0].raw.contains("keep"));
    assert_eq!(app.hidden_count(), 0);
    let on_disk = fs::read_to_string(&path).unwrap();
    assert!(on_disk.contains("keep"));
    assert!(!on_disk.contains("hide-a"));
    assert!(!on_disk.contains("hide-c"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sidebar_d5k_unhides_range() {
    let (dir, path) = temp_log(
        "sidebar-d5k",
        &[
            r#"{"level":"info","msg":"a"}"#,
            r#"{"level":"info","msg":"b"}"#,
            r#"{"level":"info","msg":"c"}"#,
            r#"{"level":"info","msg":"d"}"#,
            r#"{"level":"info","msg":"e"}"#,
            r#"{"level":"info","msg":"f"}"#,
        ],
    );
    let mut app = app_for(&path);
    // Hide all six so sidebar is only hidden rows.
    for line in (1..=6).rev() {
        app.count = Some(line);
        command::execute(&mut app, "nav top");
        command::execute(&mut app, "hide");
    }
    assert_eq!(app.hidden_count(), 6);
    assert_eq!(app.display_len(), 0);

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    app.jump_sidebar_cursor(5); // last row
    command::execute_from_key(&mut app, "filter delete"); // d
    app.count = Some(5);
    command::execute_from_key(&mut app, "nav up"); // 5k

    assert_eq!(app.hidden_count(), 0);
    assert_eq!(app.display_len(), 6);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn focus_change_cancels_pending_op() {
    let (dir, path) = temp_log(
        "focus-cancel",
        &[r#"{"level":"info","msg":"a"}"#, r#"{"level":"info","msg":"b"}"#],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "hide");
    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    command::execute_from_key(&mut app, "delete");
    assert_eq!(app.pending_op, Some(PendingOp::Delete));

    app.focus_list();
    assert!(app.pending_op.is_none(), "leaving sidebar should cancel pending D");

    command::execute_from_key(&mut app, "delete");
    assert_eq!(app.pending_op, Some(PendingOp::Delete));
    app.focus_list();
    assert_eq!(
        app.pending_op,
        Some(PendingOp::Delete),
        "same-pane focus should keep pending op"
    );

    command::execute(&mut app, "view details on");
    assert!(app.pending_op.is_none(), "list → details should cancel pending D");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn esc_cancels_sidebar_pending_op_without_closing() {
    let (dir, path) = temp_log(
        "esc-sidebar",
        &[r#"{"level":"info","msg":"a"}"#, r#"{"level":"info","msg":"b"}"#],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "hide");
    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    assert!(app.sidebar_len() > 0);
    command::execute_from_key(&mut app, "filter delete"); // d
    assert_eq!(app.pending_op, Some(PendingOp::DeleteFilter));

    command::execute_from_key(&mut app, "view current off"); // Esc

    assert!(app.pending_op.is_none());
    assert!(app.config.sidebar, "sidebar should stay open");
    assert_eq!(app.focus(), Focus::Sidebar);

    // Second Esc closes.
    command::execute_from_key(&mut app, "view current off");
    assert!(!app.config.sidebar);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn esc_cancels_details_pending_op_without_closing() {
    let (dir, path) = temp_log(
        "esc-details",
        &[r#"{"level":"info","msg":"a"}"#, r#"{"level":"info","msg":"b"}"#],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "view details on");
    assert_eq!(app.focus(), Focus::Details);
    // Pending ops from details still use the list operator (d/D fall through).
    app.pending_op = Some(PendingOp::Delete);
    app.op_anchor = 0;

    command::execute_from_key(&mut app, "view current off");

    assert!(app.pending_op.is_none());
    assert_eq!(app.focus(), Focus::Details, "details should stay focused/open");

    command::execute_from_key(&mut app, "view current off");
    assert_eq!(app.focus(), Focus::List);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn esc_clears_count_prefix_without_closing_sidebar() {
    let (dir, path) = temp_log("esc-count", &[r#"{"level":"info","msg":"a"}"#]);
    let mut app = app_for(&path);
    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    app.count = Some(5);

    app.set_current_view(ToggleAction::Off);

    assert!(app.count.is_none());
    assert!(app.config.sidebar);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn delete_all_clears_the_file() {
    let (dir, path) = temp_log(
        "delete-all",
        &[
            r#"{"level":"info","msg":"a"}"#,
            r#"{"level":"error","msg":"b"}"#,
            r#"{"level":"info","msg":"c"}"#,
        ],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "filter out error");
    assert_eq!(app.source.len(), 3);
    assert_eq!(app.display_len(), 2);

    command::execute_from_key(&mut app, "delete all");

    assert_eq!(app.source.len(), 0);
    assert_eq!(app.display_len(), 0);
    assert!(fs::read_to_string(&path).unwrap().is_empty());
    // Filter definition remains.
    assert_eq!(app.filters.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn delete_all_clears_stdin_buffer() {
    use std::os::fd::FromRawFd;
    use std::thread;
    use std::time::Duration;

    use teleminator::tail::RefreshOutcome;

    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let mut writer = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    let reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    writeln!(writer, r#"{{"level":"info","msg":"one"}}"#).unwrap();
    writeln!(writer, r#"{{"level":"info","msg":"two"}}"#).unwrap();

    let source = LogSource::open_stdin(reader).unwrap();
    let mut config = Config::default();
    config.tail_mode = false;
    config.session_filters = false;
    let mut app = App::new(source, config, std::env::temp_dir().join("teleminator-stdin-cfg.toml"))
        .unwrap();

    let mut added = 0;
    for _ in 0..50 {
        if let RefreshOutcome::Appended(n) = app.source.refresh().unwrap() {
            added += n;
            app.rebuild_visible(None);
        }
        if added >= 2 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.source.len(), 2);

    command::execute_from_key(&mut app, "delete all");
    assert_eq!(app.source.len(), 0);
    assert_eq!(app.display_len(), 0);
    assert!(
        app.status_message
            .as_deref()
            .is_some_and(|m| m.contains("cleared") && m.contains("memory"))
    );

    // Pipe still accepts new lines after the clear.
    writeln!(writer, r#"{{"level":"info","msg":"three"}}"#).unwrap();
    let mut added = 0;
    for _ in 0..50 {
        if let RefreshOutcome::Appended(n) = app.source.refresh().unwrap() {
            added += n;
            app.rebuild_visible(None);
        }
        if added >= 1 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.source.len(), 1);
    assert!(app.source.entries()[0].raw.contains("three"));
    drop(writer);
}

#[test]
fn sidebar_dg_on_filters_deletes_all_matches() {
    let (dir, path) = temp_log(
        "sidebar-dg-filters",
        &[
            r#"{"level":"info","msg":"keep"}"#,
            r#"{"level":"error","msg":"e1"}"#,
            r#"{"level":"warn","msg":"w1"}"#,
            r#"{"level":"error","msg":"e2"}"#,
        ],
    );
    let mut app = app_for(&path);
    command::execute(&mut app, "filter out error");
    command::execute(&mut app, "filter out warn");
    assert_eq!(app.filters.len(), 2);

    command::execute(&mut app, "view sidebar on");
    app.focus_sidebar();
    app.select_sidebar_item(SidebarItem::Filter(0));
    command::execute_from_key(&mut app, "delete");
    command::execute_from_key(&mut app, "nav bottom");

    assert_eq!(app.source.len(), 1);
    assert!(app.source.entries()[0].raw.contains("keep"));
    // Filters remain; only matching lines were deleted.
    assert_eq!(app.filters.len(), 2);

    let _ = fs::remove_dir_all(&dir);
}
