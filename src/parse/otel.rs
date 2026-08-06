use regex::Regex;
use std::sync::OnceLock;

use crate::model::{Field, FieldValue, LogLevel};

use super::ParsedLine;

/// Parse a Node `util.inspect` OpenTelemetry ReadableSpan dump into fields.
///
/// Expects brace-balanced text containing `traceId` and a top-level `id`.
pub fn parse_otel_inspect(text: &str) -> Option<ParsedLine> {
    let trimmed = text.trim();
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return None;
    }
    // Valid JSON is handled by the JSON parser first.
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return None;
    }

    let trace_id = capture(trace_id_re(), trimmed)?;
    let span_id = capture(top_id_re(), trimmed)?;
    if trace_id.is_empty() || span_id.is_empty() {
        return None;
    }

    let name = capture(name_re(), trimmed);
    let parent_span_id = capture(parent_span_id_re(), trimmed);
    let duration = capture(duration_re(), trimmed);
    let status = capture(status_code_re(), trimmed);
    let timestamp = capture(timestamp_re(), trimmed);

    let mut fields = vec![
        Field {
            key: "trace_id".into(),
            value: FieldValue::String(trace_id),
        },
        Field {
            key: "span_id".into(),
            value: FieldValue::String(span_id),
        },
    ];
    if let Some(parent) = parent_span_id.filter(|s| !s.is_empty()) {
        fields.push(Field {
            key: "parent_span_id".into(),
            value: FieldValue::String(parent),
        });
    }
    if let Some(name) = &name {
        fields.push(Field {
            key: "name".into(),
            value: FieldValue::String(name.clone()),
        });
    }
    if let Some(duration) = duration {
        fields.push(Field {
            key: "duration".into(),
            value: FieldValue::Number(duration),
        });
    }
    let level = match status.as_deref() {
        Some("2") => LogLevel::Error,
        _ => LogLevel::Info,
    };
    if let Some(status) = status {
        fields.push(Field {
            key: "status".into(),
            value: FieldValue::String(status),
        });
    }
    if let Some(ts) = &timestamp {
        fields.push(Field {
            key: "timestamp".into(),
            value: FieldValue::Number(ts.clone()),
        });
    }

    let message = name.clone();

    Some((level, timestamp, message, fields))
}

fn capture(re: &Regex, text: &str) -> Option<String> {
    re.captures(text)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn trace_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*traceId:\s*'([^']+)'").unwrap())
}

fn top_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*id:\s*'([^']+)'").unwrap())
}

fn parent_span_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)parentSpanContext:\s*\{.*?spanId:\s*'([^']+)'").unwrap()
    })
}

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Prefer the span name line; skip instrumentationScope's `name:` by requiring
    // it after traceId typically — take the last `name:` at indent 2, or first
    // top-level-ish name that isn't inside instrumentationScope.
    RE.get_or_init(|| Regex::new(r"(?m)^  name:\s*'([^']+)'").unwrap())
}

fn duration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*duration:\s*([0-9]+(?:\.[0-9]+)?)").unwrap())
}

fn status_code_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"status:\s*\{\s*code:\s*([0-9]+)").unwrap())
}

fn timestamp_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*timestamp:\s*([0-9]+(?:\.[0-9]+)?)").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  resource: {
    attributes: {
      'service.name': 'node'
    }
  },
  instrumentationScope: { name: 'backend-ts', version: undefined, schemaUrl: undefined },
  traceId: 'b4c1a73453220bf99e1581fffdb4d79d',
  parentSpanContext: {
    traceId: 'b4c1a73453220bf99e1581fffdb4d79d',
    spanId: '5613e9c06d088aee',
    traceFlags: 1
  },
  name: 'jwt.verify',
  id: '808bf6eaed2f48a3',
  kind: 0,
  timestamp: 1785975608200301.5,
  duration: 272282.75,
  attributes: {},
  status: { code: 1 },
  events: [],
  links: []
}"#;

    #[test]
    fn extracts_ids_name_duration() {
        let (level, ts, msg, fields) = parse_otel_inspect(SAMPLE).expect("otel");
        assert_eq!(level, LogLevel::Info);
        assert_eq!(msg.as_deref(), Some("jwt.verify"));
        assert!(ts.is_some());
        let get = |k: &str| {
            fields
                .iter()
                .find(|f| f.key == k)
                .map(|f| f.value.display())
        };
        assert_eq!(
            get("trace_id").as_deref(),
            Some("b4c1a73453220bf99e1581fffdb4d79d")
        );
        assert_eq!(get("span_id").as_deref(), Some("808bf6eaed2f48a3"));
        assert_eq!(get("parent_span_id").as_deref(), Some("5613e9c06d088aee"));
        assert_eq!(get("name").as_deref(), Some("jwt.verify"));
        assert_eq!(get("duration").as_deref(), Some("272282.75"));
        assert_eq!(get("status").as_deref(), Some("1"));
    }

    #[test]
    fn rejects_json() {
        assert!(parse_otel_inspect(r#"{"traceId":"a","id":"b"}"#).is_none());
    }

    #[test]
    fn root_span_without_parent() {
        let text = r#"{
  traceId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  name: 'HTTP GET',
  id: 'bbbbbbbbbbbbbbbb',
  duration: 1000,
  status: { code: 0 }
}"#;
        let (_, _, msg, fields) = parse_otel_inspect(text).unwrap();
        assert_eq!(msg.as_deref(), Some("HTTP GET"));
        assert!(fields.iter().all(|f| f.key != "parent_span_id"));
    }
}
