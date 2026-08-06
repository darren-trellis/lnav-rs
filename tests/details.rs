use std::collections::HashSet;

use teleminator::config::Config;
use teleminator::details::*;
use teleminator::model::{Field, FieldValue, LineFormat, LogEntry, LogLevel};
use teleminator::theme::Theme;


fn theme() -> Theme {
    Theme::resolve("catppuccin").unwrap()
}

fn sample_entry() -> LogEntry {
    LogEntry {
        line_no: 1,
        raw: "{}".into(),
        format: LineFormat::Json,
        level: LogLevel::Info,
        timestamp: None,
        timestamp_parsed: None,
        message: None,
        fields: vec![Field {
            key: "annotations".into(),
            value: FieldValue::Nested(serde_json::json!({
                "url": "http://x",
                "tags": ["a", "b"]
            })),
        }],
    }
}

#[test]
fn tree_expands_nested_object() {
    let cfg = Config {
        details_json_tree: true,
        ..Config::default()
    };
    let lines = build_lines(&sample_entry(), &theme(), &cfg, &HashSet::new());
    let text: Vec<String> = lines.iter().map(|l| l.plain_text()).collect();
    assert!(text.iter().any(|t| t.contains("annotations")));
    assert!(text.iter().any(|t| t.contains("url")));
    assert!(text.iter().any(|t| t.contains("http://x")));
    assert!(lines.iter().any(|l| l.foldable));
}

#[test]
fn folding_hides_children() {
    let cfg = Config {
        details_json_tree: true,
        ..Config::default()
    };
    let mut folded = HashSet::new();
    folded.insert(path_key(&["annotations".into()]));
    let lines = build_lines(&sample_entry(), &theme(), &cfg, &folded);
    let text: Vec<String> = lines.iter().map(|l| l.plain_text()).collect();
    assert!(
        text.iter()
            .any(|t| t.contains("annotations") && t.contains('…'))
    );
    assert!(!text.iter().any(|t| t.contains("url")));
    assert!(!text.iter().any(|t| t.contains("tags")));
}

#[test]
fn flat_keeps_nested_as_blob() {
    let entry = LogEntry {
        line_no: 1,
        raw: "{}".into(),
        format: LineFormat::Json,
        level: LogLevel::Info,
        timestamp: None,
        timestamp_parsed: None,
        message: None,
        fields: vec![Field {
            key: "annotations".into(),
            value: FieldValue::Nested(serde_json::json!({"url": "http://x"})),
        }],
    };
    let cfg = Config {
        details_json_tree: false,
        ..Config::default()
    };
    let lines = build_lines(&entry, &theme(), &cfg, &HashSet::new());
    assert!(lines.len() >= 2);
}

#[test]
fn height_respects_max() {
    assert_eq!(desired_height(100, 50, 10), 10);
    assert_eq!(desired_height(2, 50, 24), 4);
    assert!(desired_height(20, 10, 24) <= 7);
}

#[test]
fn copy_value_is_raw_field_value() {
    let cfg = Config {
        details_json_tree: true,
        ..Config::default()
    };
    let lines = build_lines(&sample_entry(), &theme(), &cfg, &HashSet::new());
    let url = lines
        .iter()
        .find(|l| l.path.last().map(|s| s.as_str()) == Some("url"))
        .unwrap();
    assert_eq!(url.copy_value.as_deref(), Some("http://x"));
    let annotations = lines
        .iter()
        .find(|l| l.path == ["annotations".to_string()])
        .unwrap();
    assert!(
        annotations
            .copy_value
            .as_ref()
            .is_some_and(|v| v.contains("url") && v.contains("tags"))
    );
}

#[test]
fn empty_containers_render_as_literals() {
    let entry = LogEntry {
        line_no: 1,
        raw: "{}".into(),
        format: LineFormat::Json,
        level: LogLevel::Info,
        timestamp: None,
        timestamp_parsed: None,
        message: None,
        fields: vec![
            Field {
                key: "empty_obj".into(),
                value: FieldValue::Nested(serde_json::json!({})),
            },
            Field {
                key: "empty_arr".into(),
                value: FieldValue::Nested(serde_json::json!([])),
            },
            Field {
                key: "nested".into(),
                value: FieldValue::Nested(serde_json::json!({"a": {}, "b": []})),
            },
        ],
    };
    let cfg = Config {
        details_json_tree: true,
        ..Config::default()
    };
    let lines = build_lines(&entry, &theme(), &cfg, &HashSet::new());
    let text: Vec<String> = lines.iter().map(|l| l.plain_text()).collect();
    assert!(
        text.iter()
            .any(|t| t.contains("empty_obj") && t.contains("{}"))
    );
    assert!(
        text.iter()
            .any(|t| t.contains("empty_arr") && t.contains("[]"))
    );
    assert!(text.iter().any(|t| t.contains("a: ") && t.contains("{}")));
    assert!(text.iter().any(|t| t.contains("b: ") && t.contains("[]")));
    assert!(
        !lines
            .iter()
            .any(|l| { l.path.last().map(|s| s.as_str()) == Some("empty_obj") && l.foldable })
    );
    assert!(
        !lines
            .iter()
            .any(|l| l.path.last().map(|s| s.as_str()) == Some("a") && l.foldable)
    );
}

#[test]
fn tab_width_changes_indent() {
    let cfg = Config {
        details_json_tree: true,
        details_tab_width: 2,
        ..Config::default()
    };
    let lines = build_lines(&sample_entry(), &theme(), &cfg, &HashSet::new());
    let nested = lines
        .iter()
        .map(|l| l.plain_text())
        .find(|t| t.contains("url"))
        .unwrap();
    assert!(nested.contains("├ ") || nested.contains("└ "));
    assert!(!nested.contains("├── "));
}
