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
    /// `None` until the user Tabs or navigates the list.
    pub selected: Option<usize>,
    /// True after ↑↓ / mouse highlight without applying — Tab/Enter apply current.
    pub browsed: bool,
}

impl CompletionState {
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected = None;
        self.browsed = false;
    }

    pub fn selected(&self) -> Option<&Suggestion> {
        self.selected.and_then(|i| self.items.get(i))
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => (i + 1) % self.items.len(),
        });
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None | Some(0) => self.items.len() - 1,
            Some(i) => i - 1,
        });
    }
}

const SET_KEYS: &[(&str, &str)] = &[
    ("theme", "theme name"),
    ("follow", "on|off|toggle"),
    ("wrap_details", "on|off|toggle"),
    ("details_json_tree", "on|off|toggle"),
    ("details_max_height", "max overlay rows"),
    ("details_tab_width", "tree indent columns"),
    ("line_numbers", "on|off|toggle"),
    ("relative_line_numbers", "on|off|toggle"),
    ("scrollbar", "on|off|toggle"),
    ("autosave", "on|off|toggle"),
    ("scroll_lines", "mouse wheel step"),
    ("timestamp_format", "strftime or raw"),
    ("case_mode", "sensitive|insensitive|smart"),
    ("session_filters", "on|off|toggle"),
    ("session_stdin", "on|off|toggle"),
];

const BOOLS: &[&str] = &["on", "off", "toggle"];

/// Refresh suggestions for the current command buffer. Clears selection.
pub fn refresh(app: &mut App) {
    let buffer = app.command_buffer.clone();
    app.completions.items = suggestions_for(&buffer, app);
    app.completions.selected = None;
    app.completions.browsed = false;
}

/// Insert the selected suggestion into the buffer. Keeps the current suggestion
/// list frozen so Tab can cycle without the other matches disappearing.
pub fn apply_selected(app: &mut App) {
    let Some(sel) = app.completions.selected().cloned() else {
        return;
    };
    if sel.text.is_empty() {
        return;
    }
    let mut new_buf =
        app.command_buffer[..sel.replace_from.min(app.command_buffer.len())].to_string();
    new_buf.push_str(&sel.text);
    app.command_buffer = new_buf;
    app.completions.browsed = false;
}

/// True when the buffer already has the selected suggestion applied.
pub fn selection_applied(app: &App) -> bool {
    let Some(sel) = app.completions.selected() else {
        return false;
    };
    let from = sel.replace_from.min(app.command_buffer.len());
    app.command_buffer[from..] == sel.text
}

/// Tab: apply current after ↑↓/mouse browse; otherwise select first/next and apply.
pub fn tab_complete(app: &mut App) {
    if app.completions.items.is_empty() {
        refresh(app);
        if app.completions.items.is_empty() {
            return;
        }
    }

    if app.completions.browsed {
        apply_selected(app);
        return;
    }

    app.completions.select_next();
    apply_selected(app);
}

/// Shift-Tab / BackTab: apply current after browse; otherwise select previous and apply.
pub fn tab_complete_prev(app: &mut App) {
    if app.completions.items.is_empty() {
        refresh(app);
        if app.completions.items.is_empty() {
            return;
        }
    }

    if app.completions.browsed {
        apply_selected(app);
        return;
    }

    app.completions.select_prev();
    apply_selected(app);
}

fn suggestions_for(buffer: &str, app: &App) -> Vec<Suggestion> {
    let trimmed_start = buffer.len() - buffer.trim_start().len();
    let body = buffer.trim_start();

    let mut items = if !body.contains(char::is_whitespace) {
        // Still typing the command name (no whitespace yet, or only leading ws).
        command_suggestions(body, trimmed_start)
    } else {
        let (cmd, rest_raw) = split_once_ws(body);
        let rest = rest_raw.trim_start();
        let rest_from = buffer.len() - rest.len();

        match cmd.to_ascii_lowercase().as_str() {
            "theme" => theme_suggestions(rest, rest_from),
            "filter" => filter_suggestions(rest, rest_from),
            "fold" => on_off_toggle_suggestions(rest, rest_from, &FOLD_SUBS),
            "focus" => on_off_toggle_suggestions(rest, rest_from, &FOCUS_SUBS),
            "follow" => on_off_toggle_suggestions(rest, rest_from, &FOLLOW_SUBS),
            "details" => on_off_toggle_suggestions(rest, rest_from, &DETAILS_SUBS),
            "help" => on_off_toggle_suggestions(rest, rest_from, &HELP_SUBS),
            "set" => set_key_suggestions(rest, rest_from),
            "config" => config_suggestions(rest, rest_from),
            "delete-filter" | "delete_filter" => {
                let idxs: Vec<String> = (0..app.filters.len()).map(|i| i.to_string()).collect();
                value_suggestions(rest, rest_from, &idxs, "index")
            }
            _ => Vec::new(),
        }
    };
    sort_suggestions(&mut items);
    items
}

fn sort_suggestions(items: &mut [Suggestion]) {
    items.sort_by(|a, b| {
        a.text
            .to_ascii_lowercase()
            .cmp(&b.text.to_ascii_lowercase())
            .then_with(|| a.text.cmp(&b.text))
    });
}

const FILTER_SUBS: &[(&str, &str)] = &[
    ("list", "list active filters"),
    ("in", "keep matching regex (default: search)"),
    ("out", "hide matching regex (default: search)"),
    ("on", "enable filtering"),
    ("off", "disable filtering"),
    ("toggle", "toggle filtering on/off"),
];

const FOLD_SUBS: &[(&str, &str)] = &[
    ("on", "fold tree item under cursor"),
    ("off", "unfold tree item under cursor"),
    ("toggle", "toggle fold under cursor"),
];

const FOCUS_SUBS: &[(&str, &str)] = &[
    ("on", "focus details overlay"),
    ("off", "focus log list"),
    ("toggle", "switch details/list focus"),
];

const FOLLOW_SUBS: &[(&str, &str)] = &[
    ("on", "enable live follow"),
    ("off", "pause live follow"),
    ("toggle", "toggle live follow"),
];

const DETAILS_SUBS: &[(&str, &str)] = &[
    ("on", "open and focus details"),
    ("off", "close details overlay"),
    ("toggle", "open/focus/close details"),
];

const HELP_SUBS: &[(&str, &str)] = &[
    ("on", "show details key hints"),
    ("off", "hide details key hints"),
    ("toggle", "toggle details key hints"),
];

fn on_off_toggle_suggestions(
    rest: &str,
    rest_from: usize,
    subs: &[(&str, &str)],
) -> Vec<Suggestion> {
    if rest.contains(char::is_whitespace) {
        return Vec::new();
    }
    let prefix = rest.to_ascii_lowercase();
    subs.iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .map(|(k, help)| Suggestion {
            text: (*k).to_string(),
            label: (*k).to_string(),
            help: (*help).to_string(),
            replace_from: rest_from,
        })
        .collect()
}

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

const CONFIG_SUBS: &[(&str, &str)] = &[
    ("path", "show config file path"),
    ("init", "write config from current settings"),
    ("set", "set config option"),
    ("get", "show config option value"),
    ("save", "save config to disk"),
];

fn config_suggestions(rest: &str, rest_from: usize) -> Vec<Suggestion> {
    if !rest.contains(char::is_whitespace) {
        let prefix = rest.to_ascii_lowercase();
        return CONFIG_SUBS
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

    let (sub, arg_raw) = split_once_ws(rest);
    let arg = arg_raw.trim_start();
    let arg_from = rest_from + (rest.len() - arg.len());
    match sub.to_ascii_lowercase().as_str() {
        "set" => set_key_suggestions(arg, arg_from),
        "get" => {
            if arg.contains(char::is_whitespace) {
                return Vec::new();
            }
            config_key_suggestions(arg, arg_from)
        }
        _ => Vec::new(),
    }
}

fn config_key_suggestions(prefix: &str, replace_from: usize) -> Vec<Suggestion> {
    let prefix_l = prefix.to_ascii_lowercase();
    SET_KEYS
        .iter()
        .filter(|(k, _)| k.starts_with(&prefix_l))
        .map(|(k, help)| Suggestion {
            text: (*k).to_string(),
            label: (*k).to_string(),
            help: (*help).to_string(),
            replace_from,
        })
        .collect()
}

fn set_key_suggestions(rest: &str, rest_from: usize) -> Vec<Suggestion> {
    if !rest.contains(char::is_whitespace) {
        return config_key_suggestions(rest, rest_from);
    }

    let (key, value_raw) = split_once_ws(rest);
    let value = value_raw.trim_start();
    let value_from = rest_from + (rest.len() - value.len());

    match key.to_ascii_lowercase().as_str() {
        "theme" => value_suggestions(value, value_from, &Theme::list_names(), "theme"),
        "follow"
        | "wrap_details"
        | "details_json_tree"
        | "line_numbers"
        | "relative_line_numbers"
        | "scrollbar"
        | "autosave"
        | "session_filters"
        | "session_stdin" => value_suggestions(value, value_from, &bool_opts(), "bool"),
        "case_mode" => {
            let opts = ["sensitive", "insensitive", "smart", "smartcase"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            value_suggestions(value, value_from, &opts, "case")
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

#[cfg(test)]
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
    fn suggestions_sorted_alphanumerically() {
        let mut items = command_suggestions("c", 0);
        sort_suggestions(&mut items);
        let texts: Vec<&str> = items.iter().map(|s| s.text.as_str()).collect();
        assert!(texts.windows(2).all(|w| {
            w[0].to_ascii_lowercase() <= w[1].to_ascii_lowercase()
        }));
        assert!(texts.iter().any(|t| *t == "copy"));
        assert!(texts.iter().any(|t| *t == "config"));
        let copy = texts.iter().position(|t| *t == "copy").unwrap();
        let config = texts.iter().position(|t| *t == "config").unwrap();
        let command_mode = texts.iter().position(|t| *t == "command-mode").unwrap();
        assert!(command_mode < config && config < copy);
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
        assert!(items.iter().any(|s| s.text == "config"));
        assert!(items.iter().all(|s| {
            s.text != "q" && s.text != "d" && s.text != "D" && s.text != "set"
        }));
    }

    #[test]
    fn completes_config_set_get() {
        let items = config_suggestions("", 0);
        assert!(items.iter().any(|s| s.text == "set"));
        assert!(items.iter().any(|s| s.text == "get"));
        let keys = config_suggestions("get ", 4);
        assert!(keys.iter().any(|s| s.text == "follow"));
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

    #[test]
    fn selection_starts_unselected_and_cycles() {
        let mut state = CompletionState::default();
        assert!(state.selected.is_none());
        state.items = vec![
            Suggestion {
                text: "a".into(),
                label: "a".into(),
                help: String::new(),
                replace_from: 0,
            },
            Suggestion {
                text: "b".into(),
                label: "b".into(),
                help: String::new(),
                replace_from: 0,
            },
        ];
        state.select_next();
        assert_eq!(state.selected, Some(0));
        state.select_next();
        assert_eq!(state.selected, Some(1));
        state.select_next();
        assert_eq!(state.selected, Some(0));
    }

}
