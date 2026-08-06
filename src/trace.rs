use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use serde_json::Value;

use crate::model::{FieldValue, LogEntry};
use crate::theme::Theme;

/// One span extracted from a log line.
#[derive(Debug, Clone)]
pub struct SpanNode {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start: Option<DateTime<Utc>>,
    pub duration_ns: Option<u64>,
    pub status: Option<String>,
    pub source_index: usize,
    pub children: Vec<usize>,
}

/// A trace: root spans plus a flat span list linked by `children`.
#[derive(Debug, Clone)]
pub struct Trace {
    pub trace_id: String,
    pub spans: Vec<SpanNode>,
    pub roots: Vec<usize>,
}

impl Trace {
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn earliest_start(&self) -> Option<DateTime<Utc>> {
        self.spans.iter().filter_map(|s| s.start).min()
    }

    pub fn total_duration_ns(&self) -> Option<u64> {
        let mut best = None;
        for &root in &self.roots {
            if let Some(d) = subtree_duration_ns(&self.spans, root) {
                best = Some(best.map_or(d, |b: u64| b.max(d)));
            }
        }
        best
    }
}

fn subtree_duration_ns(spans: &[SpanNode], idx: usize) -> Option<u64> {
    let span = spans.get(idx)?;
    if let Some(d) = span.duration_ns {
        return Some(d);
    }
    let mut total = 0u64;
    let mut any = false;
    for &child in &span.children {
        if let Some(d) = subtree_duration_ns(spans, child) {
            total = total.saturating_add(d);
            any = true;
        }
    }
    any.then_some(total)
}

#[derive(Debug, Clone, Default)]
pub struct TraceForest {
    pub traces: Vec<Trace>,
}

/// Flat display row in the spans tree.
#[derive(Debug, Clone)]
pub struct SpanLine {
    pub spans: Vec<Span<'static>>,
    /// Fold key (`trace:<id>` or `span:<trace>/<span>`).
    pub path: String,
    pub foldable: bool,
    /// Underlying log line, when this row is a span.
    pub source_index: Option<usize>,
}

impl SpanLine {
    pub fn plain_text(&self) -> String {
        self.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect()
    }
}

/// Build a forest from log entries at the given source indices.
pub fn build_forest(entries: &[LogEntry], indices: &[usize]) -> TraceForest {
    let mut by_trace: HashMap<String, Vec<ExtractedSpan>> = HashMap::new();

    for &src in indices {
        let Some(entry) = entries.get(src) else {
            continue;
        };
        let Some(extracted) = extract_span(entry, src) else {
            continue;
        };
        by_trace
            .entry(extracted.trace_id.clone())
            .or_default()
            .push(extracted);
    }

    let mut traces: Vec<Trace> = by_trace
        .into_iter()
        .map(|(trace_id, spans)| build_trace(trace_id, spans))
        .collect();

    traces.sort_by(|a, b| {
        match (a.earliest_start(), b.earliest_start()) {
            (Some(sa), Some(sb)) => sa.cmp(&sb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.trace_id.cmp(&b.trace_id),
        }
    });

    TraceForest { traces }
}

#[derive(Debug)]
struct ExtractedSpan {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    name: String,
    start: Option<DateTime<Utc>>,
    duration_ns: Option<u64>,
    status: Option<String>,
    source_index: usize,
}

fn build_trace(trace_id: String, extracted: Vec<ExtractedSpan>) -> Trace {
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    let mut spans: Vec<SpanNode> = extracted
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            id_to_idx.insert(e.span_id.clone(), i);
            SpanNode {
                span_id: e.span_id,
                parent_span_id: e.parent_span_id,
                name: e.name,
                start: e.start,
                duration_ns: e.duration_ns,
                status: e.status,
                source_index: e.source_index,
                children: Vec::new(),
            }
        })
        .collect();

    let mut roots = Vec::new();
    for i in 0..spans.len() {
        let parent = spans[i].parent_span_id.clone();
        match parent.and_then(|p| id_to_idx.get(&p).copied()) {
            Some(parent_idx) if parent_idx != i => {
                spans[parent_idx].children.push(i);
            }
            _ => roots.push(i),
        }
    }

    let starts: Vec<Option<DateTime<Utc>>> = spans.iter().map(|s| s.start).collect();
    let cmp = |a: usize, b: usize| match (starts[a], starts[b]) {
        (Some(sa), Some(sb)) => sa.cmp(&sb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.cmp(&b),
    };
    for span in &mut spans {
        span.children.sort_by(|&a, &b| cmp(a, b));
    }
    roots.sort_by(|&a, &b| cmp(a, b));

    Trace {
        trace_id,
        spans,
        roots,
    }
}

/// Render the forest into display lines, respecting folds.
pub fn build_lines(
    forest: &TraceForest,
    folded: &HashSet<String>,
    theme: &Theme,
    tab_width: usize,
) -> Vec<SpanLine> {
    let surface = theme.background;
    let tab = tab_width.max(2);
    let mut lines = Vec::new();

    if forest.traces.is_empty() {
        lines.push(SpanLine {
            spans: vec![Span::styled(
                " no spans found (need trace_id + span_id on log lines) ".to_string(),
                theme.tone_style(theme.dim, surface),
            )],
            path: String::new(),
            foldable: false,
            source_index: None,
        });
        return lines;
    }

    for trace in &forest.traces {
        let path = format!("trace:{}", trace.trace_id);
        let is_folded = folded.contains(&path);
        let marker = if is_folded { "▸" } else { "▾" };
        let count = trace.span_count();
        let dur = trace
            .total_duration_ns()
            .map(format_duration)
            .unwrap_or_default();
        let short_id = shorten_id(&trace.trace_id, 12);
        let mut row = vec![
            Span::styled(
                format!("{marker} "),
                theme.tone_style(theme.dim, surface),
            ),
            Span::styled(
                "trace ".to_string(),
                theme.tone_style(theme.dim, surface),
            ),
            Span::styled(
                short_id,
                theme
                    .tone_style(theme.number, surface)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {count} span{}", if count == 1 { "" } else { "s" }),
                theme.tone_style(theme.foreground, surface),
            ),
        ];
        if !dur.is_empty() {
            row.push(Span::styled(
                format!("  {dur}"),
                theme.tone_style(theme.dim, surface),
            ));
        }
        lines.push(SpanLine {
            spans: row,
            path,
            foldable: true,
            source_index: None,
        });

        if is_folded {
            continue;
        }

        for &root in &trace.roots {
            push_span_lines(
                &mut lines,
                trace,
                root,
                1,
                tab,
                folded,
                theme,
                surface,
                true,
            );
        }
    }

    lines
}

fn push_span_lines(
    lines: &mut Vec<SpanLine>,
    trace: &Trace,
    idx: usize,
    depth: usize,
    tab: usize,
    folded: &HashSet<String>,
    theme: &Theme,
    surface: ratatui::style::Color,
    is_last: bool,
) {
    let Some(span) = trace.spans.get(idx) else {
        return;
    };
    let path = format!("span:{}/{}", trace.trace_id, span.span_id);
    let has_children = !span.children.is_empty();
    let is_folded = has_children && folded.contains(&path);
    let indent = " ".repeat(depth * tab);
    let branch = if depth == 0 {
        ""
    } else if is_last {
        "└─ "
    } else {
        "├─ "
    };
    let marker = if has_children {
        if is_folded { "▸ " } else { "▾ " }
    } else {
        "  "
    };

    let mut row = vec![
        Span::styled(
            format!("{indent}{branch}{marker}"),
            theme.tone_style(theme.dim, surface),
        ),
        Span::styled(
            span.name.clone(),
            theme
                .tone_style(theme.foreground, surface)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(dur) = span.duration_ns {
        row.push(Span::styled(
            format!("  {}", format_duration(dur)),
            theme.tone_style(theme.dim, surface),
        ));
    }
    if let Some(status) = &span.status {
        let style = status_style(status, theme, surface);
        row.push(Span::styled(format!("  {status}"), style));
    }

    lines.push(SpanLine {
        spans: row,
        path: path.clone(),
        foldable: has_children,
        source_index: Some(span.source_index),
    });

    if is_folded {
        return;
    }

    let n = span.children.len();
    for (i, &child) in span.children.iter().enumerate() {
        push_span_lines(
            lines,
            trace,
            child,
            depth + 1,
            tab,
            folded,
            theme,
            surface,
            i + 1 == n,
        );
    }
}

fn status_style(status: &str, theme: &Theme, surface: ratatui::style::Color) -> Style {
    let lower = status.to_ascii_lowercase();
    if lower.contains("error") || lower.contains("fault") || lower == "2" {
        theme.level_style(crate::model::LogLevel::Error)
    } else if lower.contains("ok") || lower.contains("unset") || lower == "0" || lower == "1" {
        theme.tone_style(theme.dim, surface)
    } else {
        theme.tone_style(theme.foreground, surface)
    }
}

pub fn format_duration(ns: u64) -> String {
    if ns < 1_000 {
        format!("{ns}ns")
    } else if ns < 1_000_000 {
        format!("{:.1}µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.1}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", ns as f64 / 1_000_000_000.0)
    }
}

fn shorten_id(id: &str, max: usize) -> String {
    if id.len() <= max {
        id.to_string()
    } else {
        format!("{}…", &id[..max.saturating_sub(1)])
    }
}

fn extract_span(entry: &LogEntry, source_index: usize) -> Option<ExtractedSpan> {
    let trace_id = lookup_string(entry, TRACE_ID_KEYS)?;
    // Bare `id` is used by OTel ReadableSpan dumps / JSON; only accept it once
    // a trace id is present so unrelated JSON `id` fields are ignored.
    let span_id = lookup_string(entry, SPAN_ID_KEYS)
        .or_else(|| lookup_string(entry, &["id"]))?;
    if trace_id.is_empty() || span_id.is_empty() {
        return None;
    }
    let parent_span_id = lookup_string(entry, PARENT_SPAN_ID_KEYS).filter(|s| !s.is_empty());
    let name = lookup_string(entry, SPAN_NAME_KEYS)
        .filter(|s| !s.is_empty())
        .or_else(|| entry.message.clone())
        .unwrap_or_else(|| span_id.clone());
    let duration_ns = lookup_duration_ns(entry);
    let status = lookup_string(entry, STATUS_KEYS);
    let start = entry.timestamp_parsed;

    Some(ExtractedSpan {
        trace_id,
        span_id,
        parent_span_id,
        name,
        start,
        duration_ns,
        status,
        source_index,
    })
}

const TRACE_ID_KEYS: &[&str] = &[
    "trace_id",
    "traceid",
    "trace.id",
    "dd.trace_id",
    "otel.trace_id",
    "oteltraceid",
    "otel.traceid",
];

const SPAN_ID_KEYS: &[&str] = &[
    "span_id",
    "spanid",
    "span.id",
    "dd.span_id",
    "otel.span_id",
    "otelspanid",
    "otel.spanid",
];

const PARENT_SPAN_ID_KEYS: &[&str] = &[
    "parent_span_id",
    "parentspanid",
    "parent_id",
    "parentid",
    "parent.id",
    "parentSpanContext.spanId",
    "dd.parent_id",
    "otel.parent_id",
    "otel.parent_span_id",
];

const SPAN_NAME_KEYS: &[&str] = &[
    "span_name",
    "spanname",
    "operation",
    "operation_name",
    "operationname",
    "resource",
    "resource_name",
    "resourcename",
    "name",
    "otel.span_name",
    "otel.name",
];

const STATUS_KEYS: &[&str] = &[
    "span_status",
    "status_code",
    "status.code",
    "otel.status_code",
    "otel.status",
    "status",
];

fn lookup_string(entry: &LogEntry, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = field_by_path(entry, key) {
            let s = value_as_string(&v)?;
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn lookup_duration_ns(entry: &LogEntry) -> Option<u64> {
    const NS_KEYS: &[&str] = &[
        "duration_ns",
        "duration.ns",
        "otel.duration_ns",
        "span.duration_ns",
    ];
    for key in NS_KEYS {
        if let Some(v) = field_by_path(entry, key)
            && let Some(n) = value_as_u64(&v)
        {
            return Some(n);
        }
    }

    const US_KEYS: &[&str] = &["duration_us", "duration.us", "duration_µs"];
    for key in US_KEYS {
        if let Some(v) = field_by_path(entry, key)
            && let Some(n) = value_as_u64(&v)
        {
            return Some(n.saturating_mul(1_000));
        }
    }

    const MS_KEYS: &[&str] = &[
        "duration_ms",
        "duration.ms",
        "otel.duration_ms",
        "durationmillis",
    ];
    for key in MS_KEYS {
        if let Some(v) = field_by_path(entry, key)
            && let Some(n) = value_as_f64(&v)
        {
            return Some((n * 1_000_000.0) as u64);
        }
    }

    // Bare `duration`: OTel inspect dumps use microseconds; otherwise heuristic.
    if let Some(v) = field_by_path(entry, "duration")
        && let Some(n) = value_as_f64(&v)
    {
        if entry.format == crate::model::LineFormat::Otel {
            return Some((n * 1_000.0) as u64);
        }
        if n >= 1_000_000_000.0 {
            return Some(n as u64);
        }
        if n >= 1_000_000.0 {
            return Some((n * 1_000.0) as u64);
        }
        if n >= 1_000.0 {
            return Some((n * 1_000_000.0) as u64);
        }
        return Some((n * 1_000_000.0) as u64);
    }

    None
}

fn field_by_path(entry: &LogEntry, path: &str) -> Option<FieldValue> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 1 {
        let key = parts[0];
        for field in &entry.fields {
            if field.key.eq_ignore_ascii_case(key) {
                return Some(field.value.clone());
            }
        }
        // Also search one level into common nesting bags.
        for bag in ["dd", "otel", "attributes", "attr", "span"] {
            if let Some(FieldValue::Nested(Value::Object(map))) = entry
                .fields
                .iter()
                .find(|f| f.key.eq_ignore_ascii_case(bag))
                .map(|f| &f.value)
            {
                for (k, v) in map {
                    if k.eq_ignore_ascii_case(key) {
                        return Some(json_to_field(v));
                    }
                }
            }
        }
        return None;
    }

    let head = parts[0];
    let rest = &parts[1..];
    for field in &entry.fields {
        if !field.key.eq_ignore_ascii_case(head) {
            continue;
        }
        match &field.value {
            FieldValue::Nested(value) => {
                return nested_path(value, rest);
            }
            other if rest.is_empty() => return Some(other.clone()),
            _ => {}
        }
    }
    None
}

fn nested_path(value: &Value, parts: &[&str]) -> Option<FieldValue> {
    if parts.is_empty() {
        return Some(json_to_field(value));
    }
    let obj = value.as_object()?;
    let key = parts[0];
    for (k, v) in obj {
        if k.eq_ignore_ascii_case(key) {
            return nested_path(v, &parts[1..]);
        }
    }
    None
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

fn value_as_string(value: &FieldValue) -> Option<String> {
    match value {
        FieldValue::String(s) => Some(s.clone()),
        FieldValue::Number(n) => Some(n.clone()),
        FieldValue::Bool(b) => Some(b.to_string()),
        FieldValue::Null => None,
        FieldValue::Nested(Value::String(s)) => Some(s.clone()),
        FieldValue::Nested(Value::Number(n)) => Some(n.to_string()),
        FieldValue::Nested(_) => None,
    }
}

fn value_as_u64(value: &FieldValue) -> Option<u64> {
    match value {
        FieldValue::Number(n) => n.parse().ok(),
        FieldValue::String(s) => s.parse().ok(),
        FieldValue::Nested(Value::Number(n)) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
        FieldValue::Nested(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn value_as_f64(value: &FieldValue) -> Option<f64> {
    match value {
        FieldValue::Number(n) => n.parse().ok(),
        FieldValue::String(s) => s.parse().ok(),
        FieldValue::Nested(Value::Number(n)) => n.as_f64(),
        FieldValue::Nested(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, LineFormat, LogLevel};

    fn entry(fields: Vec<(&str, FieldValue)>, msg: &str) -> LogEntry {
        LogEntry {
            line_no: 1,
            raw: String::new(),
            format: LineFormat::Json,
            level: LogLevel::Info,
            timestamp: None,
            timestamp_parsed: None,
            message: Some(msg.into()),
            fields: fields
                .into_iter()
                .map(|(k, v)| Field {
                    key: k.into(),
                    value: v,
                })
                .collect(),
        }
    }

    #[test]
    fn builds_parent_child_tree() {
        let entries = vec![
            entry(
                vec![
                    ("trace_id", FieldValue::String("t1".into())),
                    ("span_id", FieldValue::String("s1".into())),
                    ("name", FieldValue::String("root".into())),
                    ("duration_ms", FieldValue::Number("10".into())),
                ],
                "root",
            ),
            entry(
                vec![
                    ("trace_id", FieldValue::String("t1".into())),
                    ("span_id", FieldValue::String("s2".into())),
                    ("parent_span_id", FieldValue::String("s1".into())),
                    ("name", FieldValue::String("child".into())),
                    ("duration_ms", FieldValue::Number("4".into())),
                ],
                "child",
            ),
        ];
        let forest = build_forest(&entries, &[0, 1]);
        assert_eq!(forest.traces.len(), 1);
        let t = &forest.traces[0];
        assert_eq!(t.roots, vec![0]);
        assert_eq!(t.spans[0].children, vec![1]);
        assert_eq!(t.spans[0].name, "root");
        assert_eq!(t.spans[1].name, "child");
    }

    #[test]
    fn reads_dd_nested_ids() {
        let dd = serde_json::json!({"trace_id": "99", "span_id": "7"});
        let e = entry(
            vec![
                ("dd", FieldValue::Nested(dd)),
                ("msg", FieldValue::String("hi".into())),
            ],
            "hi",
        );
        let forest = build_forest(std::slice::from_ref(&e), &[0]);
        assert_eq!(forest.traces.len(), 1);
        assert_eq!(forest.traces[0].trace_id, "99");
        assert_eq!(forest.traces[0].spans[0].span_id, "7");
    }

    #[test]
    fn format_duration_units() {
        assert_eq!(format_duration(500), "500ns");
        assert_eq!(format_duration(2_500), "2.5µs");
        assert_eq!(format_duration(3_500_000), "3.5ms");
        assert_eq!(format_duration(1_500_000_000), "1.50s");
    }

    #[test]
    fn fold_hides_children() {
        let entries = vec![entry(
            vec![
                ("trace_id", FieldValue::String("t1".into())),
                ("span_id", FieldValue::String("s1".into())),
                ("name", FieldValue::String("root".into())),
            ],
            "root",
        )];
        let forest = build_forest(&entries, &[0]);
        let theme = Theme::resolve("neovim").unwrap();
        let mut folded = HashSet::new();
        folded.insert("trace:t1".into());
        let lines = build_lines(&forest, &folded, &theme, 2);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].foldable);
    }
}
