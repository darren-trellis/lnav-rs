use serde_json::Value;

use crate::model::{Field, FieldValue, LogLevel};

use super::ParsedLine;

pub fn parse_json_line(line: &str) -> Option<ParsedLine> {
    let trimmed = line.trim();
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return None;
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;

    let mut level = LogLevel::Unknown;
    let mut timestamp = None;
    let mut message = None;
    let mut fields = Vec::with_capacity(obj.len());

    for (key, val) in obj {
        let key_l = key.to_ascii_lowercase();
        match key_l.as_str() {
            "level" | "lvl" | "severity" | "log_level" | "loglevel" => {
                if let Some(s) = val.as_str() {
                    level = LogLevel::parse(s);
                } else if let Some(n) = val.as_u64() {
                    level = level_from_number(n);
                }
            }
            "time" | "timestamp" | "ts" | "@timestamp" | "datetime" | "date" => {
                timestamp = Some(json_to_string(val));
            }
            "msg" | "message" | "text" | "log" => {
                message = Some(json_to_string(val));
            }
            _ => {}
        }
        fields.push(Field {
            key: key.clone(),
            value: json_to_field(val),
        });
    }

    Some((level, timestamp, message, fields))
}

fn level_from_number(n: u64) -> LogLevel {
    match n {
        0..=10 => LogLevel::Trace,
        20 => LogLevel::Debug,
        30 => LogLevel::Info,
        40 => LogLevel::Warn,
        50 => LogLevel::Error,
        60.. => LogLevel::Fatal,
        _ => LogLevel::Unknown,
    }
}

fn json_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn json_to_field(value: &Value) -> FieldValue {
    match value {
        Value::Null => FieldValue::Null,
        Value::Bool(b) => FieldValue::Bool(*b),
        Value::Number(n) => FieldValue::Number(n.to_string()),
        Value::String(s) => FieldValue::String(s.clone()),
        Value::Array(_) | Value::Object(_) => FieldValue::Nested(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
