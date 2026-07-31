use crate::app::App;
use crate::command;
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Text that replaces `command_buffer[replace_from..]`.
    pub text: String,
    /// Label shown in the popup.
    pub label: String,
    /// Short help shown beside the label.
    pub help: String,
    pub replace_from: usize,
}

#[derive(Debug, Default, Clone)]
pub struct CompletionState {
    pub items: Vec<Suggestion>,
    pub selected: usize,
}

impl CompletionState {
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected = 0;
    }

    pub fn selected(&self) -> Option<&Suggestion> {
        self.items.get(self.selected)
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.items.len();
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.items.len() - 1
        } else {
            self.selected - 1
        };
    }
}

const SET_KEYS: &[(&str, &str)] = &[
    ("theme", "theme name"),
    ("follow", "on|off"),
    ("wrap_details", "on|off"),
    ("line_numbers", "on|off"),
    ("relative_line_numbers", "on|off"),
    ("scroll_lines", "mouse wheel step"),
    ("timestamp_format", "strftime or raw"),
];

const BOOLS: &[&str] = &["on", "off", "true", "false"];

/// Refresh suggestions for the current command buffer.
pub fn refresh(app: &mut App) {
    let buffer = app.command_buffer.clone();
    let items = suggestions_for(&buffer, app);
    let keep = app
        .completions
        .selected()
        .map(|s| s.text.clone())
        .and_then(|prev| items.iter().position(|s| s.text == prev));
    app.completions.items = items;
    app.completions.selected = keep.unwrap_or(0);
}

pub fn apply_selected(app: &mut App) {
    let Some(sel) = app.completions.selected().cloned() else {
        return;
    };
    if sel.text.is_empty() {
        return;
    }
    let mut new_buf = app.command_buffer[..sel.replace_from.min(app.command_buffer.len())].to_string();
    new_buf.push_str(&sel.text);
    // Add a trailing space after a completed command name so args can be typed.
    if !sel.text.contains(' ')
        && !new_buf.ends_with(' ')
        && command::catalog().iter().any(|c| c.name == sel.text)
    {
        new_buf.push(' ');
    }
    app.command_buffer = new_buf;
    refresh(app);
}

pub fn tab_complete(app: &mut App) {
    if app.completions.items.is_empty() {
        refresh(app);
        if app.completions.items.is_empty() {
            return;
        }
    }

    if app.completions.items.len() == 1 {
        apply_selected(app);
        return;
    }

    // First try to extend by the common prefix of all suggestions.
    if let Some(prefix) = common_prefix(
        app.completions
            .items
            .iter()
            .map(|s| s.text.as_str()),
    ) {
        let from = app.completions.items[0].replace_from;
        let current = &app.command_buffer[from..];
        if prefix.len() > current.len() && prefix.starts_with(current) {
            app.command_buffer.truncate(from);
            app.command_buffer.push_str(&prefix);
            refresh(app);
            return;
        }
    }

    // Otherwise cycle to the next match and apply it.
    app.completions.select_next();
    apply_selected(app);
}

fn suggestions_for(buffer: &str, app: &App) -> Vec<Suggestion> {
    let trimmed_start = buffer.len() - buffer.trim_start().len();
    let body = buffer.trim_start();

    // Still typing the command name (no whitespace yet, or only leading ws).
    if !body.contains(char::is_whitespace) {
        return command_suggestions(body, trimmed_start);
    }

    let (cmd, rest_raw) = split_once_ws(body);
    let rest = rest_raw.trim_start();
    let rest_from = buffer.len() - rest.len();

    match cmd.to_ascii_lowercase().as_str() {
        "theme" => theme_suggestions(rest, rest_from),
        "filter" => filter_suggestions(rest, rest_from),
        "set" => set_suggestions(rest, rest_from, app),
        "config" => {
            let opts = vec!["path".to_string(), "init".to_string()];
            value_suggestions(rest, rest_from, &opts, "config")
        }
        "delete-filter" | "delete_filter" => {
            let idxs: Vec<String> = (0..app.filters.len()).map(|i| i.to_string()).collect();
            value_suggestions(rest, rest_from, &idxs, "index")
        }
        _ => Vec::new(),
    }
}

const FILTER_SUBS: &[(&str, &str)] = &[
    ("list", "list active filters"),
    ("in", "keep lines matching regex"),
    ("out", "hide lines matching regex"),
    ("on", "enable filtering"),
    ("off", "disable filtering"),
    ("toggle", "toggle filtering on/off"),
];

fn filter_suggestions(rest: &str, rest_from: usize) -> Vec<Suggestion> {
    if !rest.contains(char::is_whitespace) {
        let prefix = rest.to_ascii_lowercase();
        return FILTER_SUBS
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, help)| Suggestion {
                text: (*k).to_string(),
                label: (*k).to_string(),
                help: (*help).to_string(),
                replace_from: rest_from,
            })
            .collect();
    }

    let (sub, value_raw) = split_once_ws(rest);
    let value = value_raw.trim_start();
    let value_from = rest_from + (rest.len() - value.len());
    match sub.to_ascii_lowercase().as_str() {
        "in" | "out" if value.is_empty() => vec![Suggestion {
            text: String::new(),
            label: "<regex>".into(),
            help: "pattern matched against each raw line".into(),
            replace_from: value_from,
        }],
        _ => Vec::new(),
    }
}

fn command_suggestions(prefix: &str, replace_from: usize) -> Vec<Suggestion> {
    let prefix_l = prefix.to_ascii_lowercase();
    let mut items: Vec<Suggestion> = command::catalog()
        .iter()
        .filter(|c| c.name.starts_with(&prefix_l))
        .map(|c| Suggestion {
            text: c.name.to_string(),
            label: c.name.to_string(),
            help: c.help.to_string(),
            replace_from,
        })
        .collect();

    if items.is_empty() && !prefix_l.is_empty() {
        items = command::catalog()
            .iter()
            .filter(|c| c.name.contains(&prefix_l))
            .map(|c| Suggestion {
                text: c.name.to_string(),
                label: c.name.to_string(),
                help: c.help.to_string(),
                replace_from,
            })
            .collect();
    }

    // Digits → goto line hint.
    if prefix_l.chars().all(|c| c.is_ascii_digit()) && !prefix_l.is_empty() {
        items.insert(
            0,
            Suggestion {
                text: prefix.to_string(),
                label: prefix.to_string(),
                help: "jump to line".into(),
                replace_from,
            },
        );
    }

    items
}

const THEME_SUBS: &[(&str, &str)] = &[
    ("list", "list available themes"),
    ("set", "set theme or open picker"),
];

fn theme_suggestions(rest: &str, rest_from: usize) -> Vec<Suggestion> {
    if !rest.contains(char::is_whitespace) {
        let prefix = rest.to_ascii_lowercase();
        return THEME_SUBS
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, help)| Suggestion {
                text: (*k).to_string(),
                label: (*k).to_string(),
                help: (*help).to_string(),
                replace_from: rest_from,
            })
            .collect();
    }

    let (sub, value_raw) = split_once_ws(rest);
    let value = value_raw.trim_start();
    let value_from = rest_from + (rest.len() - value.len());
    match sub.to_ascii_lowercase().as_str() {
        "set" => value_suggestions(value, value_from, &Theme::list_names(), "theme"),
        _ => Vec::new(),
    }
}

fn set_suggestions(rest: &str, rest_from: usize, _app: &App) -> Vec<Suggestion> {
    if !rest.contains(char::is_whitespace) {
        let prefix = rest.to_ascii_lowercase();
        return SET_KEYS
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, help)| Suggestion {
                text: (*k).to_string(),
                label: (*k).to_string(),
                help: (*help).to_string(),
                replace_from: rest_from,
            })
            .collect();
    }

    let (key, value_raw) = split_once_ws(rest);
    let value = value_raw.trim_start();
    let value_from = rest_from + (rest.len() - value.len());

    match key.to_ascii_lowercase().as_str() {
        "theme" => value_suggestions(value, value_from, &Theme::list_names(), "theme"),
        "follow" | "wrap_details" | "line_numbers" | "relative_line_numbers" => {
            value_suggestions(value, value_from, &bool_opts(), "bool")
        }
        "timestamp_format" => {
            let presets = [
                "%H:%M:%S".to_string(),
                "%H:%M:%S%.3f".to_string(),
                "%Y-%m-%d %H:%M:%S".to_string(),
                "%Y-%m-%dT%H:%M:%SZ".to_string(),
                "raw".to_string(),
            ];
            value_suggestions(value, value_from, &presets, "strftime")
        }
        _ => Vec::new(),
    }
}

fn bool_opts() -> Vec<String> {
    BOOLS.iter().map(|s| (*s).to_string()).collect()
}

fn value_suggestions(
    prefix: &str,
    replace_from: usize,
    values: &[String],
    kind: &str,
) -> Vec<Suggestion> {
    let prefix_l = prefix.to_ascii_lowercase();
    let mut items: Vec<Suggestion> = values
        .iter()
        .filter(|v| v.to_ascii_lowercase().starts_with(&prefix_l))
        .map(|v| Suggestion {
            text: v.clone(),
            label: v.clone(),
            help: kind.to_string(),
            replace_from,
        })
        .collect();
    if items.is_empty() && !prefix_l.is_empty() {
        items = values
            .iter()
            .filter(|v| v.to_ascii_lowercase().contains(&prefix_l))
            .map(|v| Suggestion {
                text: v.clone(),
                label: v.clone(),
                help: kind.to_string(),
                replace_from,
            })
            .collect();
    }
    items
}

fn split_once_ws(s: &str) -> (&str, &str) {
    match s.split_once(char::is_whitespace) {
        Some((a, b)) => (a, b),
        None => (s, ""),
    }
}

fn common_prefix<'a>(mut iter: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut prefix = iter.next()?.to_string();
    for s in iter {
        while !s.starts_with(&prefix) {
            if prefix.is_empty() {
                return None;
            }
            prefix.pop();
        }
    }
    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_filter_prefix() {
        let items = command_suggestions("fil", 0);
        assert!(items.iter().any(|s| s.text == "filter"));
        assert!(items.iter().all(|s| {
            s.text != "filter-in" && s.text != "filter-out" && s.text != "filters"
        }));
    }

    #[test]
    fn completes_filter_subcommands() {
        let items = filter_suggestions("", 0);
        assert!(items.iter().any(|s| s.text == "list"));
        assert!(items.iter().any(|s| s.text == "in"));
        assert!(items.iter().any(|s| s.text == "out"));
        assert!(items.iter().any(|s| s.text == "toggle"));
    }

    #[test]
    fn does_not_suggest_keybinding_aliases() {
        let items = command_suggestions("", 0);
        assert!(items.iter().any(|s| s.text == "quit"));
        assert!(items.iter().any(|s| s.text == "hide"));
        assert!(items.iter().any(|s| s.text == "delete"));
        assert!(items.iter().all(|s| s.text != "q" && s.text != "d" && s.text != "D"));
    }

    #[test]
    fn common_prefix_works() {
        assert_eq!(
            common_prefix(["filter", "clear-filters", "delete-filter"].into_iter()).as_deref(),
            None
        );
        assert_eq!(
            common_prefix(["list", "in", "out"].into_iter()).as_deref(),
            None
        );
        assert_eq!(
            common_prefix(["filter-in", "filter-out"].into_iter()).as_deref(),
            Some("filter-")
        );
    }
}
