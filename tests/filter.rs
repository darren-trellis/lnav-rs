use lnav_rs::config::CaseMode;
use lnav_rs::filter::*;
use lnav_rs::model::{LineFormat, LogEntry, LogLevel};


fn entry(raw: &str) -> LogEntry {
    LogEntry {
        line_no: 1,
        raw: raw.into(),
        format: LineFormat::Plain,
        level: LogLevel::Info,
        timestamp: None,
        timestamp_parsed: None,
        message: None,
        fields: Vec::new(),
    }
}

#[test]
fn filter_in_requires_match_when_present() {
    let filters =
        vec![Filter::new(FilterKind::Include, "error", CaseMode::Insensitive).unwrap()];
    assert!(!entry_passes(&filters, true, &entry("info ok")));
    assert!(entry_passes(&filters, true, &entry("got error here")));
    assert!(entry_passes(&filters, true, &entry("got ERROR here")));
}

#[test]
fn filter_out_hides_matches() {
    let filters =
        vec![Filter::new(FilterKind::Exclude, "spam", CaseMode::Insensitive).unwrap()];
    assert!(!entry_passes(&filters, true, &entry("spam message")));
    assert!(entry_passes(&filters, true, &entry("real message")));
}

#[test]
fn include_and_exclude_combine() {
    let filters = vec![
        Filter::new(FilterKind::Include, "http", CaseMode::Insensitive).unwrap(),
        Filter::new(FilterKind::Exclude, "health", CaseMode::Insensitive).unwrap(),
    ];
    assert!(entry_passes(&filters, true, &entry("http request /api")));
    assert!(!entry_passes(&filters, true, &entry("http health")));
    assert!(!entry_passes(&filters, true, &entry("db query")));
}

#[test]
fn smartcase_sensitive_when_uppercase() {
    let f = Filter::new(FilterKind::Include, "ERROR", CaseMode::Smart).unwrap();
    assert!(f.matches(&entry("got ERROR here")));
    assert!(!f.matches(&entry("got error here")));
}

#[test]
fn smartcase_insensitive_when_lowercase() {
    let f = Filter::new(FilterKind::Include, "error", CaseMode::Smart).unwrap();
    assert!(f.matches(&entry("got ERROR here")));
    assert!(f.matches(&entry("got error here")));
}
