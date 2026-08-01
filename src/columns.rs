use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::config::{Align, Column};
use crate::model::{FieldValue, LogEntry};
use crate::theme::ColorSpec;
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
    /// Vertical rule between columns (`│` × theme `ui.border_width`).
    ColumnBorder,
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
    /// Per-column border color override (`ColumnBorder` only).
    pub border_color: Option<ColorSpec>,
}

/// Vertical rule between list columns (`│` × width, with padding spaces).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnBorderStyle {
    pub width: usize,
    pub padding: crate::config::Padding,
    pub color: Option<ColorSpec>,
}

impl ColumnBorderStyle {
    pub fn resolve(column: &Column, defaults: &Self) -> Self {
        Self {
            width: column.border_width.unwrap_or(defaults.width),
            padding: column.border_padding.unwrap_or(defaults.padding),
            color: column.border.clone().or_else(|| defaults.color.clone()),
        }
    }
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
    let mut widths: Vec<usize> = columns.iter().map(|c| c.width.unwrap_or(0)).collect();

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

pub fn render_segments(
    columns: &[Column],
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
    default_border: &ColumnBorderStyle,
) -> Vec<Segment> {
    let widths = measure_widths(columns, &[(entry, opts.view_line)], opts.timestamp_format);
    render_segments_sized(columns, &widths, entry, opts, default_border)
}

/// Render using precomputed column widths (for aligned multi-row tables).
pub fn render_segments_sized(
    columns: &[Column],
    widths: &[usize],
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
    default_border: &ColumnBorderStyle,
) -> Vec<Segment> {
    let mut out = Vec::new();
    for (i, col) in columns.iter().enumerate() {
        if i > 0 {
            out.push(column_separator(ColumnBorderStyle::resolve(col, default_border)));
        }
        let width = widths.get(i).copied().or(col.width);
        out.push(render_column(col, width, entry, opts));
    }
    out
}

pub fn render(
    columns: &[Column],
    entry: &LogEntry,
    opts: &FormatOptions<'_>,
    default_border: &ColumnBorderStyle,
) -> String {
    render_segments(columns, entry, opts, default_border)
        .into_iter()
        .map(|s| s.text)
        .collect()
}

pub fn column_separator(border: ColumnBorderStyle) -> Segment {
    if border.width == 0 {
        Segment {
            kind: SegmentKind::Literal,
            text: " ".into(),
            border_color: None,
        }
    } else {
        Segment {
            kind: SegmentKind::ColumnBorder,
            text: format!(
                "{}{}{}",
                " ".repeat(border.padding.left),
                "│".repeat(border.width),
                " ".repeat(border.padding.right)
            ),
            border_color: border.color,
        }
    }
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
    Segment {
        kind,
        text,
        border_color: None,
    }
}

fn column_value(source: &str, entry: &LogEntry, opts: &FormatOptions<'_>) -> (SegmentKind, String) {
    match source {
        "level" => (SegmentKind::Level, entry.level.as_str().to_string()),
        "timestamp" | "time" | "ts" => (
            SegmentKind::Timestamp,
            format_timestamp(entry, opts.timestamp_format),
        ),
        "message" | "msg" => (SegmentKind::Message, entry.summary_message().to_string()),
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
        return crate::text::truncate_width(value, width);
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

    let parsed;
    let root = match &field.value {
        FieldValue::Nested(value) => value,
        FieldValue::String(value) => {
            let Ok(value) = serde_json::from_str::<Value>(value) else {
                return String::new();
            };
            parsed = value;
            &parsed
        }
        _ => return String::new(),
    };
    dig_json(root, &rest).unwrap_or_default()
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
