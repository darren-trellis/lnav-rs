use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::Value;

use crate::config::Config;
use crate::model::{FieldValue, LogEntry};
use crate::theme::Theme;

/// One display row in the details overlay.
#[derive(Debug, Clone)]
pub struct DetailLine {
    pub spans: Vec<Span<'static>>,
}

impl DetailLine {
    fn plain(text: impl Into<String>, style: Style) -> Self {
        Self {
            spans: vec![Span::styled(text.into(), style)],
        }
    }

    pub fn to_line(&self) -> Line<'static> {
        Line::from(self.spans.clone())
    }

    pub fn plain_text(&self) -> String {
        self.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect()
    }
}

/// Build the full details content for an entry (header + fields).
pub fn build_lines(entry: &LogEntry, theme: &Theme, config: &Config) -> Vec<DetailLine> {
    let surface = theme.overlay_bg;
    let mut lines = Vec::new();

    lines.push(DetailLine {
        spans: vec![
            Span::styled("file ", theme.tone_style(theme.dim, surface)),
            Span::styled(
                entry.line_no.to_string(),
                theme
                    .tone_style(theme.number, surface)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  level ", theme.tone_style(theme.dim, surface)),
            Span::styled(
                entry.level.as_str().to_string(),
                theme.level_style(entry.level).add_modifier(Modifier::BOLD),
            ),
        ],
    });

    if entry.fields.is_empty() {
        lines.push(DetailLine::plain(
            entry.raw.clone(),
            theme.tone_style(theme.foreground, surface),
        ));
        return lines;
    }

    for field in &entry.fields {
        push_field(
            &mut lines,
            &field.key,
            &field.value,
            "",
            true,
            theme,
            config.details_json_tree,
        );
    }
    lines
}

fn push_field(
    lines: &mut Vec<DetailLine>,
    key: &str,
    value: &FieldValue,
    prefix: &str,
    is_last: bool,
    theme: &Theme,
    json_tree: bool,
) {
    let surface = theme.overlay_bg;
    let branch = if prefix.is_empty() {
        String::new()
    } else if is_last {
        "└── ".into()
    } else {
        "├── ".into()
    };
    let key_style = theme
        .tone_style(theme.key, surface)
        .add_modifier(Modifier::BOLD);

    if json_tree {
        if let FieldValue::Nested(raw) = value {
            if let Ok(v) = serde_json::from_str::<Value>(raw) {
                if v.is_object() || v.is_array() {
                    lines.push(DetailLine {
                        spans: vec![
                            Span::styled(format!("{prefix}{branch}"), theme.tone_style(theme.dim, surface)),
                            Span::styled(key.to_string(), key_style),
                            Span::styled(
                                if v.is_array() {
                                    format!(" [{}]", v.as_array().map(|a| a.len()).unwrap_or(0))
                                } else {
                                    String::new()
                                },
                                theme.tone_style(theme.dim, surface),
                            ),
                        ],
                    });
                    let child_prefix = if prefix.is_empty() {
                        String::new()
                    } else if is_last {
                        format!("{prefix}    ")
                    } else {
                        format!("{prefix}│   ")
                    };
                    push_json_value(lines, &v, &child_prefix, theme);
                    return;
                }
            }
        }
    }

    let value_style = theme.field_value_style(value, surface);
    let rendered = match value {
        FieldValue::String(s) => format!("\"{s}\""),
        FieldValue::Nested(s) => s.clone(),
        other => other.display(),
    };

    if prefix.is_empty() {
        // Top-level flat row (legacy layout when not expanding).
        let mut spans = vec![
            Span::styled(format!("{key:<16}"), key_style),
            Span::styled(rendered, value_style),
        ];
        // Nested pretty-print may contain newlines — split into rows.
        if spans[1].content.contains('\n') {
            let text = spans[1].content.to_string();
            let style = spans[1].style;
            let mut parts = text.split('\n');
            if let Some(first) = parts.next() {
                spans[1] = Span::styled(first.to_string(), style);
                lines.push(DetailLine { spans });
            }
            for part in parts {
                lines.push(DetailLine::plain(part.to_string(), style));
            }
        } else {
            lines.push(DetailLine { spans });
        }
    } else {
        lines.push(DetailLine {
            spans: vec![
                Span::styled(format!("{prefix}{branch}"), theme.tone_style(theme.dim, surface)),
                Span::styled(format!("{key}: "), key_style),
                Span::styled(rendered, value_style),
            ],
        });
    }
}

fn push_json_value(lines: &mut Vec<DetailLine>, value: &Value, prefix: &str, theme: &Theme) {
    match value {
        Value::Object(map) => {
            let keys: Vec<&String> = map.keys().collect();
            for (i, key) in keys.iter().enumerate() {
                let is_last = i + 1 == keys.len();
                push_json_entry(lines, key, &map[*key], prefix, is_last, theme);
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let is_last = i + 1 == arr.len();
                push_json_entry(lines, &i.to_string(), item, prefix, is_last, theme);
            }
        }
        other => {
            let fv = json_value_to_field(other);
            let style = theme.field_value_style(&fv, theme.overlay_bg);
            lines.push(DetailLine::plain(
                match &fv {
                    FieldValue::String(s) => format!("\"{s}\""),
                    _ => fv.display(),
                },
                style,
            ));
        }
    }
}

fn push_json_entry(
    lines: &mut Vec<DetailLine>,
    key: &str,
    value: &Value,
    prefix: &str,
    is_last: bool,
    theme: &Theme,
) {
    let surface = theme.overlay_bg;
    let branch = if is_last { "└── " } else { "├── " };
    let key_style = theme
        .tone_style(theme.key, surface)
        .add_modifier(Modifier::BOLD);
    let child_prefix = if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };

    match value {
        Value::Object(map) => {
            lines.push(DetailLine {
                spans: vec![
                    Span::styled(format!("{prefix}{branch}"), theme.tone_style(theme.dim, surface)),
                    Span::styled(key.to_string(), key_style),
                ],
            });
            let keys: Vec<&String> = map.keys().collect();
            for (i, k) in keys.iter().enumerate() {
                push_json_entry(lines, k, &map[*k], &child_prefix, i + 1 == keys.len(), theme);
            }
        }
        Value::Array(arr) => {
            lines.push(DetailLine {
                spans: vec![
                    Span::styled(format!("{prefix}{branch}"), theme.tone_style(theme.dim, surface)),
                    Span::styled(key.to_string(), key_style),
                    Span::styled(
                        format!(" [{}]", arr.len()),
                        theme.tone_style(theme.dim, surface),
                    ),
                ],
            });
            for (i, item) in arr.iter().enumerate() {
                push_json_entry(
                    lines,
                    &i.to_string(),
                    item,
                    &child_prefix,
                    i + 1 == arr.len(),
                    theme,
                );
            }
        }
        other => {
            let fv = json_value_to_field(other);
            let value_style = theme.field_value_style(&fv, surface);
            let rendered = match &fv {
                FieldValue::String(s) => format!("\"{s}\""),
                _ => fv.display(),
            };
            lines.push(DetailLine {
                spans: vec![
                    Span::styled(format!("{prefix}{branch}"), theme.tone_style(theme.dim, surface)),
                    Span::styled(format!("{key}: "), key_style),
                    Span::styled(rendered, value_style),
                ],
            });
        }
    }
}

fn json_value_to_field(value: &Value) -> FieldValue {
    match value {
        Value::Null => FieldValue::Null,
        Value::Bool(b) => FieldValue::Bool(*b),
        Value::Number(n) => FieldValue::Number(n.to_string()),
        Value::String(s) => FieldValue::String(s.clone()),
        Value::Array(_) | Value::Object(_) => FieldValue::Nested(
            serde_json::to_string(value).unwrap_or_else(|_| value.to_string()),
        ),
    }
}

/// Overlay height in rows (including border), capped by config and available space.
pub fn desired_height(content_lines: usize, available: u16, max_height: usize) -> u16 {
    // +2 for the border.
    let needed = content_lines.saturating_add(2).max(4);
    let cap_cfg = max_height.max(4);
    let cap_screen = (available as usize).saturating_sub(3).max(4);
    needed.min(cap_cfg).min(cap_screen) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, LineFormat, LogLevel};

    fn theme() -> Theme {
        Theme::resolve("catppuccin").unwrap()
    }

    #[test]
    fn tree_expands_nested_object() {
        let entry = LogEntry {
            line_no: 1,
            raw: "{}".into(),
            format: LineFormat::Json,
            level: LogLevel::Info,
            timestamp: None,
            timestamp_parsed: None,
            message: None,
            fields: vec![Field {
                key: "annotations".into(),
                value: FieldValue::Nested(r#"{"url":"http://x","n":1}"#.into()),
            }],
        };
        let mut cfg = Config::default();
        cfg.details_json_tree = true;
        let lines = build_lines(&entry, &theme(), &cfg);
        let text: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect();
        assert!(text.iter().any(|t| t.contains("annotations")));
        assert!(text.iter().any(|t| t.contains("url")));
        assert!(text.iter().any(|t| t.contains("http://x")));
        assert!(text.len() >= 3);
    }

    #[test]
    fn flat_keeps_nested_as_blob() {
        let entry = LogEntry {
            line_no: 1,
            raw: "{}".into(),
            format: LineFormat::Json,
            level: LogLevel::Info,
            timestamp: None,
            timestamp_parsed: None,
            message: None,
            fields: vec![Field {
                key: "annotations".into(),
                value: FieldValue::Nested("{\n  \"url\": \"http://x\"\n}".into()),
            }],
        };
        let mut cfg = Config::default();
        cfg.details_json_tree = false;
        let lines = build_lines(&entry, &theme(), &cfg);
        assert!(lines.len() >= 2);
    }

    #[test]
    fn height_respects_max() {
        assert_eq!(desired_height(100, 50, 10), 10);
        assert_eq!(desired_height(2, 50, 24), 4);
        assert!(desired_height(20, 10, 24) <= 7);
    }
}
