use lnav_rs::model::{LineFormat, LogLevel};
use lnav_rs::parse::json::parse_json_line;
use lnav_rs::parse::logfmt::parse_logfmt;
use lnav_rs::parse::parse_line;

#[test]
fn parses_json_line() {
    let (level, ts, msg, fields) = parse_json_line(
        r#"{"level":"error","msg":"boom","time":"2024-01-01T00:00:00Z","code":500}"#,
    )
    .unwrap();
    assert_eq!(level, LogLevel::Error);
    assert_eq!(ts.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(msg.as_deref(), Some("boom"));
    assert_eq!(fields.len(), 4);
}


#[test]
fn parses_basic_logfmt() {
    let (level, ts, msg, fields) =
        parse_logfmt(r#"time=2024-01-01T00:00:00Z level=info msg="hello world" user=ada"#)
            .unwrap();
    assert_eq!(level, LogLevel::Info);
    assert_eq!(ts.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(msg.as_deref(), Some("hello world"));
    assert_eq!(fields.len(), 4);
}


#[test]
fn parses_sample_jsonl() {
    let raw = include_str!("../examples/sample.jsonl");
    let entries: Vec<_> = raw
        .lines()
        .enumerate()
        .map(|(i, line)| parse_line(i + 1, line.to_string()))
        .collect();
    assert_eq!(entries.len(), 7);
    assert!(entries.iter().all(|e| e.format == LineFormat::Json));
    assert_eq!(entries[4].level, LogLevel::Error);
    assert!(entries[4].fields.iter().any(|f| f.key == "service"));
    assert!(entries[0].timestamp_parsed.is_some());
}

#[test]
fn parses_sample_logfmt() {
    let raw = include_str!("../examples/sample.logfmt");
    let entries: Vec<_> = raw
        .lines()
        .enumerate()
        .map(|(i, line)| parse_line(i + 1, line.to_string()))
        .collect();
    assert_eq!(entries.len(), 6);
    assert!(entries.iter().all(|e| e.format == LineFormat::Logfmt));
    assert_eq!(entries[4].level, LogLevel::Error);
}
