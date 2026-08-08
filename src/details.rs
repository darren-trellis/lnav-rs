use std::collections::HashSet;

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use serde_json::Value;

use crate::config::Config;
use crate::model::{FieldValue, LogEntry};
use crate::theme::Theme;

/// Stable path identifying a foldable tree node (e.g. `["annotations","tags"]`).
pub type FoldPath = Vec<String>;

pub fn path_key(path: &[String]) -> String {
    path.join("\0")
}

/// One display row in the details overlay.
#[derive(Debug, Clone)]
pub struct DetailLine {
    pub spans: Vec<Span<'static>>,
    /// Path of this node (empty for the header / non-tree rows).
    pub path: FoldPath,
    /// True when this row can be folded (object/array with children).
    pub foldable: bool,
    /// Value copied by `c` / `:copy` (without tree chrome).
    pub copy_value: Option<String>,
}

impl DetailLine {
    fn plain(text: impl Into<String>, style: Style) -> Self {
        let text = text.into();
        Self {
            copy_value: Some(text.clone()),
            spans: vec![Span::styled(text, style)],
            path: Vec::new(),
            foldable: false,
        }
    }

    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.content.as_ref()).collect()
    }
}

fn field_copy_value(value: &FieldValue) -> String {
    match value {
        FieldValue::String(s) => s.clone(),
        FieldValue::Nested(value) => json_copy_value(value),
        other => other.display(),
    }
}

fn json_copy_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
    }
}

fn json_empty_literal(value: &Value) -> Option<&'static str> {
    match value {
        Value::Object(m) if m.is_empty() => Some("{}"),
        Value::Array(a) if a.is_empty() => Some("[]"),
        _ => None,
    }
}

/// Build the full details content for an entry (header + fields).
pub fn build_lines(
    entry: &LogEntry,
    theme: &Theme,
    config: &Config,
    folded: &HashSet<String>,
) -> Vec<DetailLine> {
    let surface = theme.overlay_bg();
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
        path: Vec::new(),
        foldable: false,
        copy_value: Some(entry.raw.clone()),
    });

    if entry.fields.is_empty() {
        lines.push(DetailLine::plain(
            entry.raw.clone(),
            theme.tone_style(theme.foreground, surface),
        ));
        return lines;
    }

    let tab = config.details_tab_width.max(2);
    let context = TreeRenderContext {
        theme,
        json_tree: config.details_json_tree,
        folded,
        tab,
    };
    let n = entry.fields.len();
    for (i, field) in entry.fields.iter().enumerate() {
        push_field(
            &mut lines,
            &field.key,
            &field.value,
            &[],
            "",
            i + 1 == n,
            &context,
        );
    }
    lines
}

fn fold_marker(folded: bool) -> &'static str {
    if folded { "▸ " } else { "▾ " }
}

/// Branch connector for one tree level, e.g. width 4 → `├── ` / `└── `.
fn branch_connector(is_last: bool, width: usize) -> String {
    let w = width.max(2);
    let mut s = String::with_capacity(w);
    s.push(if is_last { '└' } else { '├' });
    for _ in 0..w.saturating_sub(2) {
        s.push('─');
    }
    s.push(' ');
    s
}

/// Indent guide under a parent, e.g. width 4 → `│   ` or `    `.
fn indent_guide(is_last: bool, width: usize) -> String {
    let w = width.max(2);
    if is_last {
        " ".repeat(w)
    } else {
        let mut s = String::with_capacity(w);
        s.push('│');
        s.push_str(&" ".repeat(w - 1));
        s
    }
}

struct TreeRenderContext<'a> {
    theme: &'a Theme,
    json_tree: bool,
    folded: &'a HashSet<String>,
    tab: usize,
}

fn push_field(
    lines: &mut Vec<DetailLine>,
    key: &str,
    value: &FieldValue,
    parent: &[String],
    prefix: &str,
    is_last: bool,
    context: &TreeRenderContext<'_>,
) {
    let theme = context.theme;
    let tab = context.tab;
    let surface = theme.overlay_bg();
    let branch = if prefix.is_empty() {
        String::new()
    } else {
        branch_connector(is_last, tab)
    };
    let key_style = theme
        .tone_style(theme.key, surface)
        .add_modifier(Modifier::BOLD);
    let mut path = parent.to_vec();
    path.push(key.to_string());

    if context.json_tree
        && let FieldValue::Nested(value) = value
        && (value.is_object() || value.is_array())
        && json_empty_literal(value).is_none()
    {
        let key_str = path_key(&path);
        let is_folded = context.folded.contains(&key_str);
        let mut spans = vec![
            Span::styled(
                format!("{prefix}{branch}"),
                theme.tone_style(theme.dim, surface),
            ),
            Span::styled(
                fold_marker(is_folded).to_string(),
                theme.tone_style(theme.dim, surface),
            ),
            Span::styled(key.to_string(), key_style),
        ];
        if value.is_array() {
            spans.push(Span::styled(
                format!(" [{}]", value.as_array().map(|a| a.len()).unwrap_or(0)),
                theme.tone_style(theme.dim, surface),
            ));
        }
        if is_folded {
            spans.push(Span::styled(
                " …".to_string(),
                theme.tone_style(theme.dim, surface),
            ));
        }
        lines.push(DetailLine {
            spans,
            path: path.clone(),
            foldable: true,
            copy_value: Some(json_copy_value(value)),
        });
        if !is_folded {
            let child_prefix = if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}{}", indent_guide(is_last, tab))
            };
            push_json_value(lines, value, &path, &child_prefix, context);
        }
        return;
    }

    let value_style = theme.field_value_style(value, surface);
    let copy_value = field_copy_value(value);
    let rendered = match value {
        FieldValue::String(s) => format!("\"{s}\""),
        FieldValue::Nested(value) => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
        other => other.display(),
    };

    if prefix.is_empty() {
        let mut spans = vec![
            Span::styled(format!("{key:<16}"), key_style),
            Span::styled(rendered, value_style),
        ];
        if spans[1].content.contains('\n') {
            let text = spans[1].content.to_string();
            let style = spans[1].style;
            let mut parts = text.split('\n');
            if let Some(first) = parts.next() {
                spans[1] = Span::styled(first.to_string(), style);
                lines.push(DetailLine {
                    spans,
                    path: path.clone(),
                    foldable: false,
                    copy_value: Some(copy_value),
                });
            }
            for part in parts {
                lines.push(DetailLine {
                    spans: vec![Span::styled(part.to_string(), style)],
                    path: Vec::new(),
                    foldable: false,
                    copy_value: None,
                });
            }
        } else {
            lines.push(DetailLine {
                spans,
                path,
                foldable: false,
                copy_value: Some(copy_value),
            });
        }
    } else {
        lines.push(DetailLine {
            spans: vec![
                Span::styled(
                    format!("{prefix}{branch}"),
                    theme.tone_style(theme.dim, surface),
                ),
                Span::styled(format!("{key}: "), key_style),
                Span::styled(rendered, value_style),
            ],
            path,
            foldable: false,
            copy_value: Some(copy_value),
        });
    }
}

fn push_json_value(
    lines: &mut Vec<DetailLine>,
    value: &Value,
    parent: &[String],
    prefix: &str,
    context: &TreeRenderContext<'_>,
) {
    let theme = context.theme;
    match value {
        Value::Object(map) => {
            let keys: Vec<&String> = map.keys().collect();
            for (i, key) in keys.iter().enumerate() {
                let is_last = i + 1 == keys.len();
                push_json_entry(lines, key, &map[*key], parent, prefix, is_last, context);
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let is_last = i + 1 == arr.len();
                push_json_entry(
                    lines,
                    &i.to_string(),
                    item,
                    parent,
                    prefix,
                    is_last,
                    context,
                );
            }
        }
        other => {
            let fv = json_value_to_field(other);
            let style = theme.field_value_style(&fv, theme.overlay_bg());
            let text = match &fv {
                FieldValue::String(s) => format!("\"{s}\""),
                _ => fv.display(),
            };
            lines.push(DetailLine {
                spans: vec![Span::styled(text, style)],
                path: parent.to_vec(),
                foldable: false,
                copy_value: Some(json_copy_value(other)),
            });
        }
    }
}

fn push_json_entry(
    lines: &mut Vec<DetailLine>,
    key: &str,
    value: &Value,
    parent: &[String],
    prefix: &str,
    is_last: bool,
    context: &TreeRenderContext<'_>,
) {
    let theme = context.theme;
    let tab = context.tab;
    let surface = theme.overlay_bg();
    let branch = branch_connector(is_last, tab);
    let key_style = theme
        .tone_style(theme.key, surface)
        .add_modifier(Modifier::BOLD);
    let mut path = parent.to_vec();
    path.push(key.to_string());

    if let Some(literal) = json_empty_literal(value) {
        let fv = json_value_to_field(value);
        let value_style = theme.field_value_style(&fv, surface);
        lines.push(DetailLine {
            spans: vec![
                Span::styled(
                    format!("{prefix}{branch}"),
                    theme.tone_style(theme.dim, surface),
                ),
                Span::styled(format!("{key}: "), key_style),
                Span::styled(literal.to_string(), value_style),
            ],
            path,
            foldable: false,
            copy_value: Some(literal.to_string()),
        });
        return;
    }

    let child_prefix = format!("{prefix}{}", indent_guide(is_last, tab));
    match value {
        Value::Object(map) => {
            let key_str = path_key(&path);
            let is_folded = context.folded.contains(&key_str);
            let mut spans = vec![
                Span::styled(
                    format!("{prefix}{branch}"),
                    theme.tone_style(theme.dim, surface),
                ),
                Span::styled(
                    fold_marker(is_folded).to_string(),
                    theme.tone_style(theme.dim, surface),
                ),
                Span::styled(key.to_string(), key_style),
            ];
            if is_folded {
                spans.push(Span::styled(
                    " …".to_string(),
                    theme.tone_style(theme.dim, surface),
                ));
            }
            lines.push(DetailLine {
                spans,
                path: path.clone(),
                foldable: true,
                copy_value: Some(json_copy_value(value)),
            });
            if !is_folded {
                let keys: Vec<&String> = map.keys().collect();
                for (i, k) in keys.iter().enumerate() {
                    push_json_entry(
                        lines,
                        k,
                        &map[*k],
                        &path,
                        &child_prefix,
                        i + 1 == keys.len(),
                        context,
                    );
                }
            }
        }
        Value::Array(arr) => {
            let key_str = path_key(&path);
            let is_folded = context.folded.contains(&key_str);
            let mut spans = vec![
                Span::styled(
                    format!("{prefix}{branch}"),
                    theme.tone_style(theme.dim, surface),
                ),
                Span::styled(
                    fold_marker(is_folded).to_string(),
                    theme.tone_style(theme.dim, surface),
                ),
                Span::styled(key.to_string(), key_style),
                Span::styled(
                    format!(" [{}]", arr.len()),
                    theme.tone_style(theme.dim, surface),
                ),
            ];
            if is_folded {
                spans.push(Span::styled(
                    " …".to_string(),
                    theme.tone_style(theme.dim, surface),
                ));
            }
            lines.push(DetailLine {
                spans,
                path: path.clone(),
                foldable: true,
                copy_value: Some(json_copy_value(value)),
            });
            if !is_folded {
                for (i, item) in arr.iter().enumerate() {
                    push_json_entry(
                        lines,
                        &i.to_string(),
                        item,
                        &path,
                        &child_prefix,
                        i + 1 == arr.len(),
                        context,
                    );
                }
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
                    Span::styled(
                        format!("{prefix}{branch}"),
                        theme.tone_style(theme.dim, surface),
                    ),
                    Span::styled(format!("{key}: "), key_style),
                    Span::styled(rendered, value_style),
                ],
                path,
                foldable: false,
                copy_value: Some(json_copy_value(other)),
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
        Value::Array(_) | Value::Object(_) => FieldValue::Nested(value.clone()),
    }
}

/// Overlay height in rows (including border), capped by config and available space.
pub fn desired_height(content_lines: usize, available: u16, max_height: usize) -> u16 {
    let needed = content_lines.saturating_add(2).max(4);
    let cap_cfg = max_height.max(4);
    let cap_screen = (available as usize).saturating_sub(3).max(4);
    needed.min(cap_cfg).min(cap_screen) as u16
}
