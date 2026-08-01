use crate::app::App;
use crate::command_catalog;
use crate::config_options;
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

/// Refresh suggestions for the current command buffer. Clears selection.
pub fn refresh(app: &mut App) {
    let buffer = app.command_line.buffer.clone();
    app.command_line.completions.items = suggestions_for(&buffer, app);
    app.command_line.completions.selected = None;
    app.command_line.completions.browsed = false;
}

/// Insert the selected suggestion into the buffer. Keeps the current suggestion
/// list frozen so Tab can cycle without the other matches disappearing.
pub fn apply_selected(app: &mut App) {
    let Some(sel) = app.command_line.completions.selected().cloned() else {
        return;
    };
    if sel.text.is_empty() {
        return;
    }
    let mut new_buf =
        app.command_line.buffer[..sel.replace_from.min(app.command_line.buffer.len())].to_string();
    new_buf.push_str(&sel.text);
    app.command_line.buffer = new_buf;
    app.command_line.completions.browsed = false;
}

/// True when the buffer already has the selected suggestion applied.
pub fn selection_applied(app: &App) -> bool {
    let Some(sel) = app.command_line.completions.selected() else {
        return false;
    };
    let from = sel.replace_from.min(app.command_line.buffer.len());
    app.command_line.buffer[from..] == sel.text
}

/// Tab: apply current after ↑↓/mouse browse; otherwise select first/next and apply.
pub fn tab_complete(app: &mut App) {
    if app.command_line.completions.items.is_empty() {
        refresh(app);
        if app.command_line.completions.items.is_empty() {
            return;
        }
    }

    if app.command_line.completions.browsed {
        apply_selected(app);
        return;
    }

    app.command_line.completions.select_next();
    apply_selected(app);
}

/// Shift-Tab / BackTab: apply current after browse; otherwise select previous and apply.
pub fn tab_complete_prev(app: &mut App) {
    if app.command_line.completions.items.is_empty() {
        refresh(app);
        if app.command_line.completions.items.is_empty() {
            return;
        }
    }

    if app.command_line.completions.browsed {
        apply_selected(app);
        return;
    }

    app.command_line.completions.select_prev();
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
            "filter" => filter_suggestions(rest, rest_from, app.filters.len()),
            "fold" => on_off_toggle_suggestions(rest, rest_from, FOLD_SUBS),
            "focus" => on_off_toggle_suggestions(rest, rest_from, FOCUS_SUBS),
            "follow" => on_off_toggle_suggestions(rest, rest_from, FOLLOW_SUBS),
            "view" => view_suggestions(rest, rest_from),
            "search" => on_off_toggle_suggestions(rest, rest_from, SEARCH_SUBS),
            "hide" => on_off_toggle_suggestions(rest, rest_from, HIDE_SUBS),
            "pin" => on_off_toggle_suggestions(rest, rest_from, PIN_SUBS),
            "help" => on_off_toggle_suggestions(rest, rest_from, HELP_SUBS),
            "config" => config_suggestions(rest, rest_from),
            _ => Vec::new(),
        }
    };
    sort_suggestions(&mut items);
    items
}

pub fn sort_suggestions(items: &mut [Suggestion]) {
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
    ("clear", "remove all filters"),
    ("delete", "delete filter by index"),
];

const FOLD_SUBS: &[(&str, &str)] = &[
    ("on", "fold tree item under cursor"),
    ("off", "unfold tree item under cursor"),
    ("toggle", "toggle fold under cursor"),
];

const FOCUS_SUBS: &[(&str, &str)] = &[
    ("on", "focus details overlay"),
    ("off", "focus log list"),
    ("toggle", "cycle list/details/sidebar focus"),
];

const FOLLOW_SUBS: &[(&str, &str)] = &[
    ("on", "enable live follow"),
    ("off", "pause live follow"),
    ("toggle", "toggle live follow"),
];

const VIEW_SUBS: &[(&str, &str)] = &[
    ("details", "open/close/toggle details overlay"),
    ("sidebar", "show/hide/toggle filters sidebar"),
    ("current", "control the focused details/sidebar pane"),
];

const DETAILS_SUBS: &[(&str, &str)] = &[
    ("on", "open and focus details"),
    ("off", "close details overlay"),
    ("toggle", "open/focus/close details"),
];

const SIDEBAR_SUBS: &[(&str, &str)] = &[
    ("on", "show filters sidebar"),
    ("off", "hide filters sidebar"),
    ("toggle", "toggle filters sidebar"),
];

const CURRENT_SUBS: &[(&str, &str)] = &[
    ("on", "show/focus the focused pane"),
    ("off", "close the focused details or sidebar pane"),
    ("toggle", "toggle the focused details or sidebar pane"),
];

const HELP_SUBS: &[(&str, &str)] = &[
    ("on", "show details key hints"),
    ("off", "hide details key hints"),
    ("toggle", "toggle details key hints"),
];

const SEARCH_SUBS: &[(&str, &str)] = &[("clear", "clear search highlights")];

const HIDE_SUBS: &[(&str, &str)] = &[
    ("line", "hide current line(s) immediately"),
    ("clear", "restore lines hidden with hide"),
];

const PIN_SUBS: &[(&str, &str)] = &[
    ("line", "pin/unpin current line(s)"),
    ("clear", "unpin all sticky lines"),
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

fn view_suggestions(rest: &str, rest_from: usize) -> Vec<Suggestion> {
    if !rest.contains(char::is_whitespace) {
        return on_off_toggle_suggestions(rest, rest_from, VIEW_SUBS);
    }
    let (target, arg_raw) = split_once_ws(rest);
    let arg = arg_raw.trim_start();
    let arg_from = rest_from + (rest.len() - arg.len());
    match target.to_ascii_lowercase().as_str() {
        "details" => on_off_toggle_suggestions(arg, arg_from, DETAILS_SUBS),
        "sidebar" => on_off_toggle_suggestions(arg, arg_from, SIDEBAR_SUBS),
        "current" => on_off_toggle_suggestions(arg, arg_from, CURRENT_SUBS),
        _ => Vec::new(),
    }
}

pub fn filter_suggestions(rest: &str, rest_from: usize, filter_count: usize) -> Vec<Suggestion> {
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
        "delete" => {
            let mut items = value_suggestions(
                value,
                value_from,
                &["line".to_string()],
                "selected filter",
            );
            let idxs: Vec<String> = (0..filter_count).map(|i| i.to_string()).collect();
            items.extend(value_suggestions(value, value_from, &idxs, "index"));
            items
        }
        _ => Vec::new(),
    }
}

pub fn command_suggestions(prefix: &str, replace_from: usize) -> Vec<Suggestion> {
    let prefix_l = prefix.to_ascii_lowercase();
    let mut items: Vec<Suggestion> = command_catalog::catalog()
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
        items = command_catalog::catalog()
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
    ("cycle", "cycle to the next theme"),
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

pub fn config_suggestions(rest: &str, rest_from: usize) -> Vec<Suggestion> {
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
    config_options::catalog()
        .iter()
        .filter(|option| option.name.starts_with(&prefix_l))
        .map(|option| Suggestion {
            text: option.name.to_string(),
            label: option.name.to_string(),
            help: option.help.to_string(),
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

    let Some(option) = config_options::find(key) else {
        return Vec::new();
    };
    value_suggestions(
        value,
        value_from,
        &option.value_kind.suggestions(),
        option.value_kind.suggestion_help(),
    )
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

pub fn common_prefix<'a>(mut iter: impl Iterator<Item = &'a str>) -> Option<String> {
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
