mod json;
mod logfmt;

use crate::model::{Field, LineFormat, LogEntry, LogLevel};
use crate::timestamp;

type ParsedLine = (LogLevel, Option<String>, Option<String>, Vec<Field>);

pub fn parse_line(line_no: usize, raw: String) -> LogEntry {
    if let Some((level, timestamp, message, fields)) = json::parse_json_line(&raw) {
        return make_entry(
            line_no,
            raw,
            LineFormat::Json,
            level,
            timestamp,
            message,
            fields,
        );
    }

    if let Some((level, timestamp, message, fields)) = logfmt::parse_logfmt(&raw) {
        return make_entry(
            line_no,
            raw,
            LineFormat::Logfmt,
            level,
            timestamp,
            message,
            fields,
        );
    }

    let level = detect_plain_level(&raw);
    make_entry(
        line_no,
        raw,
        LineFormat::Plain,
        level,
        None,
        None,
        Vec::new(),
    )
}

fn make_entry(
    line_no: usize,
    raw: String,
    format: LineFormat,
    level: LogLevel,
    timestamp: Option<String>,
    message: Option<String>,
    fields: Vec<crate::model::Field>,
) -> LogEntry {
    let timestamp_parsed = timestamp.as_deref().and_then(timestamp::parse);
    LogEntry {
        line_no,
        raw,
        format,
        level,
        timestamp,
        timestamp_parsed,
        message,
        fields,
    }
}

fn detect_plain_level(line: &str) -> LogLevel {
    for token in line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        let level = LogLevel::parse(token);
        if level != LogLevel::Unknown {
            return level;
        }
    }
    LogLevel::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LineFormat;

    #[test]
    fn parses_sample_jsonl() {
        let raw = include_str!("../../examples/sample.jsonl");
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
        let raw = include_str!("../../examples/sample.logfmt");
        let entries: Vec<_> = raw
            .lines()
            .enumerate()
            .map(|(i, line)| parse_line(i + 1, line.to_string()))
            .collect();
        assert_eq!(entries.len(), 6);
        assert!(entries.iter().all(|e| e.format == LineFormat::Logfmt));
        assert_eq!(entries[4].level, LogLevel::Error);
    }
}
