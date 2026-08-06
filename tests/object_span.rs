use teleminator::model::{LineFormat, LogEntry, LogLevel};
use teleminator::object_span::*;
use teleminator::parse::parse_line;

fn entries(lines: &[&str]) -> Vec<LogEntry> {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| parse_line(i + 1, (*l).to_string()))
        .collect()
}

#[test]
fn single_line_json_object() {
    let e = entries(&[r#"{"level":"info","msg":"hi"}"#, r#"{"level":"error"}"#]);
    assert_eq!(*object_span(&e, 0).start(), 0);
    assert_eq!(*object_span(&e, 0).end(), 0);
    assert_eq!(e[0].format, LineFormat::Json);
    assert_eq!(e[0].level, LogLevel::Info);
}

#[test]
fn multiline_json_object() {
    let e = entries(&[
        "{",
        r#"  "level": "info","#,
        r#"  "msg": "hi""#,
        "}",
        r#"{"level":"error"}"#,
    ]);
    assert_eq!(object_span(&e, 0), 0..=3);
    assert_eq!(object_span(&e, 4), 4..=4);
}

#[test]
fn plain_line_is_single() {
    let e = entries(&["just a line", "another"]);
    assert_eq!(object_span(&e, 0), 0..=0);
}
