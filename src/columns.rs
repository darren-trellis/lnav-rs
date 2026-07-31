use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::config::{Align, Column};
use crate::model::{FieldValue, LogEntry};
use crate::timestamp;

#[derive(Debug, Clone)]
pub struct FormatOptions<'a> {
    pub timestamp_format: &'a str,
    /// 1-based index in the current visible list (lnav view line).
    pub view_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Literal,
    Level,
    Timestamp,
    Message,
    Raw,
    LineNo,
    Format,
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentKind,
    pub text: String,
}

/// Compute per-column display widths for a set of rows so columns share an X origin.
///
/// Explicit `width` on a column is kept; otherwise the width is the max natural
/// content width among `rows` (visible viewport).
pub fn measure_widths(
    columns: &[Column],
    rows: &[(&LogEntry, usize)],
    timestamp_format: &str,
) -> Vec<usize> {
    let mut widths: Vec<usize> = columns
        .iter()
        .map(|c| c.width.unwrap_or(0))
        .collect();

    for (entry, view_line) in rows {
        let opts = FormatOptions {
            timestamp_format,
            view_line: *view_line,
        };
        for (i, col) in columns.iter().enumerate() {
            if col.width.is_some() {
                continue;
            }
            let (_, raw) = column_value(&col.source, entry, &opts);
            widths[i] = widths[i].max(UnicodeWidthStr::width(raw.as_str()));
        }
    }
    widths
}

/// Render a log line from configured columns into styled segments.
pub fn render_segments(
    columns: &[Column],
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
) -> Vec<Segment> {
    let widths = measure_widths(columns, &[(entry, opts.view_line)], opts.timestamp_format);
    render_segments_sized(columns, &widths, entry, opts)
}

/// Render using precomputed column widths (for aligned multi-row tables).
pub fn render_segments_sized(
    columns: &[Column],
    widths: &[usize],
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
) -> Vec<Segment> {
    let mut out = Vec::new();
    for (i, col) in columns.iter().enumerate() {
        if i > 0 {
            out.push(Segment {
                kind: SegmentKind::Literal,
                text: " ".into(),
            });
        }
        let width = widths.get(i).copied().or(col.width);
        out.push(render_column(col, width, entry, opts));
    }
    out
}

pub fn render(columns: &[Column], entry: &LogEntry, opts: &FormatOptions<'_>) -> String {
    render_segments(columns, entry, opts)
        .into_iter()
        .map(|s| s.text)
        .collect()
}

fn render_column(
    col: &Column,
    width: Option<usize>,
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
) -> Segment {
    let (kind, raw) = column_value(&col.source, entry, opts);
    let fitted = fit_width(&raw, width, col.align);
    let text = if col.padding.is_zero() {
        fitted
    } else {
        format!(
            "{}{fitted}{}",
            " ".repeat(col.padding.left),
            " ".repeat(col.padding.right)
        )
    };
    Segment { kind, text }
}

fn column_value(
    source: &str,
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
) -> (SegmentKind, String) {
    match source {
        "level" => (SegmentKind::Level, entry.level.as_str().to_string()),
        "timestamp" | "time" | "ts" => (
            SegmentKind::Timestamp,
            format_timestamp(entry, opts.timestamp_format),
        ),
        "message" | "msg" => (
            SegmentKind::Message,
            entry.summary_message().to_string(),
        ),
        "raw" => (SegmentKind::Raw, entry.raw.clone()),
        "line" => (SegmentKind::LineNo, opts.view_line.to_string()),
        "format" => {
            let f = match entry.format {
                crate::model::LineFormat::Json => "json",
                crate::model::LineFormat::Logfmt => "logf",
                crate::model::LineFormat::Plain => "text",
            };
            (SegmentKind::Format, f.to_string())
        }
        path => (SegmentKind::Field, lookup_field(entry, path)),
    }
}

fn fit_width(value: &str, width: Option<usize>, align: Align) -> String {
    let Some(width) = width else {
        return value.to_string();
    };
    if width == 0 {
        return String::new();
    }
    let vw = UnicodeWidthStr::width(value);
    if vw == width {
        return value.to_string();
    }
    if vw > width {
        return truncate_width(value, width);
    }
    let pad = width - vw;
    match align {
        Align::Left => format!("{value}{}", " ".repeat(pad)),
        Align::Center => {
            // Prefer the leftover space on the left when pad is odd so short
            // labels (INFO/WARN in a 5-wide column) don't look left-aligned.
            let right = pad / 2;
            let left = pad - right;
            format!("{}{value}{}", " ".repeat(left), " ".repeat(right))
        }
        Align::Right => format!("{}{value}", " ".repeat(pad)),
    }
}

fn truncate_width(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    if max == 1 {
        return "…".into();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        if w + cw + 1 > max {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

/// Resolve a field path like `code` or `annotations.url` (and `items.0.id`).
fn lookup_field(entry: &LogEntry, path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut parts = path.split('.');
    let Some(first) = parts.next() else {
        return String::new();
    };
    let Some(field) = entry.fields.iter().find(|f| f.key == first) else {
        return String::new();
    };
    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        return field.value.display();
    }

    let nested = match &field.value {
        FieldValue::Nested(s) => s.as_str(),
        FieldValue::String(s) => s.as_str(),
        _ => return String::new(),
    };
    let Ok(root) = serde_json::from_str::<Value>(nested) else {
        return String::new();
    };
    dig_json(&root, &rest).unwrap_or_default()
}

fn dig_json(root: &Value, path: &[&str]) -> Option<String> {
    let mut cur = root;
    for key in path {
        cur = dig_step(cur, key)?;
    }
    Some(json_leaf_display(cur))
}

fn dig_step<'a>(val: &'a Value, key: &str) -> Option<&'a Value> {
    if let Ok(idx) = key.parse::<usize>() {
        val.get(idx)
    } else {
        val.get(key)
    }
}

fn json_leaf_display(val: &Value) -> String {
    match val {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(val).unwrap_or_else(|_| val.to_string())
        }
    }
}

fn format_timestamp(entry: &LogEntry, fmt: &str) -> String {
    let raw = entry.timestamp.as_deref().unwrap_or("");
    if raw.is_empty() {
        return String::new();
    }
    timestamp::format(raw, entry.timestamp_parsed.as_ref(), fmt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Align, Column};
    use crate::model::{Field, FieldValue, LineFormat, LogLevel};

    fn entry() -> LogEntry {
        let raw_ts = "2026-07-27T23:58:14.817Z".to_string();
        let parsed = timestamp::parse(&raw_ts);
        LogEntry {
            line_no: 7,
            raw: r#"{"msg":"hi"}"#.into(),
            format: LineFormat::Json,
            level: LogLevel::Error,
            timestamp: Some(raw_ts),
            timestamp_parsed: parsed,
            message: Some("hi".into()),
            fields: vec![
                Field {
                    key: "code".into(),
                    value: FieldValue::Number("500".into()),
                },
                Field {
                    key: "annotations".into(),
                    value: FieldValue::Nested(
                        r#"{"url":"https://example.com/x","items":[{"id":"a"},{"id":"b"}]}"#
                            .into(),
                    ),
                },
            ],
        }
    }

    fn cols(sources: &[(&str, Option<usize>, Align)]) -> Vec<Column> {
        sources
            .iter()
            .map(|(source, width, align)| Column {
                source: (*source).into(),
                width: *width,
                align: *align,
                padding: crate::config::Padding::default(),
            })
            .collect()
    }

    #[test]
    fn renders_default_style_columns() {
        let opts = FormatOptions {
            timestamp_format: "%H:%M:%S",
            view_line: 3,
        };
        let entry = entry();
        let local_ts = entry
            .timestamp_parsed
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%H:%M:%S")
            .to_string();
        let columns = cols(&[
            ("level", Some(5), Align::Left),
            ("timestamp", None, Align::Left),
            ("message", None, Align::Left),
            ("line", None, Align::Left),
            ("code", None, Align::Left),
        ]);
        assert_eq!(
            render(&columns, &entry, &opts),
            format!("ERROR {local_ts} hi 3 500")
        );
    }

    #[test]
    fn width_and_align() {
        let opts = FormatOptions {
            timestamp_format: "raw",
            view_line: 1,
        };
        let columns = cols(&[("level", Some(8), Align::Right)]);
        assert_eq!(render(&columns, &entry(), &opts), "   ERROR");
    }

    #[test]
    fn center_aligns_in_fixed_width() {
        let opts = FormatOptions {
            timestamp_format: "raw",
            view_line: 1,
        };
        let err = entry();
        let columns = cols(&[("level", Some(5), Align::Center)]);
        assert_eq!(render(&columns, &err, &opts), "ERROR");

        let mut info = entry();
        info.level = LogLevel::Info;
        let columns = cols(&[("level", Some(5), Align::Center)]);
        assert_eq!(render(&columns, &info, &opts), " INFO");

        let columns = cols(&[("level", Some(6), Align::Center)]);
        assert_eq!(render(&columns, &info, &opts), " INFO ");

        let columns = cols(&[("level", Some(8), Align::Center)]);
        assert_eq!(render(&columns, &err, &opts), "  ERROR ");
    }

    #[test]
    fn truncates_wide_columns() {
        let opts = FormatOptions {
            timestamp_format: "raw",
            view_line: 1,
        };
        let columns = cols(&[("message", Some(2), Align::Left)]);
        let mut e = entry();
        e.message = Some("hello".into());
        assert_eq!(render(&columns, &e, &opts), "h…");
    }

    #[test]
    fn resolves_dotted_nested_fields() {
        let opts = FormatOptions {
            timestamp_format: "raw",
            view_line: 1,
        };
        let columns = cols(&[("annotations.url", None, Align::Left)]);
        assert_eq!(
            render(&columns, &entry(), &opts),
            "https://example.com/x"
        );
        let columns = cols(&[("annotations.items.1.id", None, Align::Left)]);
        assert_eq!(render(&columns, &entry(), &opts), "b");
    }

    #[test]
    fn segments_preserve_kinds() {
        let opts = FormatOptions {
            timestamp_format: "%H:%M:%S",
            view_line: 1,
        };
        let columns = cols(&[
            ("level", None, Align::Left),
            ("timestamp", None, Align::Left),
            ("message", None, Align::Left),
        ]);
        let segs = render_segments(&columns, &entry(), &opts);
        assert_eq!(segs[0].kind, SegmentKind::Level);
        assert_eq!(segs[1].kind, SegmentKind::Literal);
        assert_eq!(segs[2].kind, SegmentKind::Timestamp);
        assert_eq!(segs[4].kind, SegmentKind::Message);
    }

    #[test]
    fn padding_wraps_fitted_content() {
        use crate::config::Padding;

        let opts = FormatOptions {
            timestamp_format: "raw",
            view_line: 1,
        };
        let columns = vec![Column {
            source: "level".into(),
            width: Some(5),
            align: Align::Left,
            padding: Padding::both(1),
        }];
        assert_eq!(render(&columns, &entry(), &opts), " ERROR ");
    }

    #[test]
    fn auto_widths_align_across_rows() {
        let columns = cols(&[
            ("level", Some(5), Align::Left),
            ("message", None, Align::Left),
            ("code", None, Align::Left),
        ]);
        let mut short = entry();
        short.message = Some("hi".into());
        short.fields[0].value = FieldValue::Number("1".into());
        let mut long = entry();
        long.message = Some("[Network] Response".into());
        long.fields[0].value = FieldValue::Number("500".into());

        let rows = [(&short as &LogEntry, 1usize), (&long, 2)];
        let widths = measure_widths(&columns, &rows, "raw");
        assert_eq!(widths[0], 5);
        assert_eq!(widths[1], UnicodeWidthStr::width("[Network] Response"));
        assert_eq!(widths[2], 3); // "500"

        let short_row: String = render_segments_sized(
            &columns,
            &widths,
            &short,
            &FormatOptions {
                timestamp_format: "raw",
                view_line: 1,
            },
        )
        .into_iter()
        .map(|s| s.text)
        .collect();
        let long_row: String = render_segments_sized(
            &columns,
            &widths,
            &long,
            &FormatOptions {
                timestamp_format: "raw",
                view_line: 2,
            },
        )
        .into_iter()
        .map(|s| s.text)
        .collect();

        let code_start = 5 + 1 + widths[1] + 1;
        assert_eq!(
            UnicodeWidthStr::width(&short_row[..code_start]),
            UnicodeWidthStr::width(&long_row[..code_start])
        );
        assert!(short_row[code_start..].starts_with('1'));
        assert!(long_row[code_start..].starts_with("500"));
        assert_eq!(
            UnicodeWidthStr::width(short_row.as_str()),
            UnicodeWidthStr::width(long_row.as_str())
        );
    }
}
