pub mod json;
pub mod logfmt;
pub mod otel;

use crate::model::{Field, LineFormat, LogEntry, LogLevel};
use crate::timestamp;

type ParsedLine = (LogLevel, Option<String>, Option<String>, Vec<Field>);

pub fn parse_line(line_no: usize, raw: String) -> LogEntry {
    if let Some((level, ts, message, fields)) = json::parse_json_line(&raw) {
        return make_entry(
            line_no,
            raw,
            LineFormat::Json,
            level,
            ts,
            message,
            fields,
        );
    }

    if let Some((level, ts, message, fields)) = otel::parse_otel_inspect(&raw) {
        return make_entry(
            line_no,
            raw,
            LineFormat::Otel,
            level,
            ts,
            message,
            fields,
        );
    }

    if let Some((level, ts, message, fields)) = logfmt::parse_logfmt(&raw) {
        return make_entry(
            line_no,
            raw,
            LineFormat::Logfmt,
            level,
            ts,
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
    let timestamp_parsed = timestamp.as_deref().and_then(|ts| {
        timestamp::parse(ts).or_else(|| parse_epoch_timestamp(ts))
    });
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

/// OTel inspect timestamps are often epoch microseconds (sometimes with a fraction).
fn parse_epoch_timestamp(ts: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let n: f64 = ts.parse().ok()?;
    if !n.is_finite() || n <= 0.0 {
        return None;
    }
    // Heuristic: ns (>= 1e18), µs (>= 1e14), ms (>= 1e11), else seconds.
    let secs = if n >= 1e18 {
        n / 1e9
    } else if n >= 1e14 {
        n / 1e6
    } else if n >= 1e11 {
        n / 1e3
    } else {
        n
    };
    let whole = secs.floor() as i64;
    let nsecs = ((secs - whole as f64) * 1e9).round() as u32;
    chrono::DateTime::from_timestamp(whole, nsecs)
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
