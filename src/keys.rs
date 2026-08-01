use std::collections::BTreeMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

/// Keybindings: base map plus contextual overlays under `[keys.details]` / `[keys.sidebar]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysConfig {
    #[serde(default, flatten)]
    pub bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub details: BTreeMap<String, String>,
    #[serde(default)]
    pub sidebar: BTreeMap<String, String>,
}

impl KeysConfig {
    pub fn with_defaults() -> Self {
        Self {
            bindings: defaults(),
            details: details_defaults(),
            sidebar: sidebar_defaults(),
        }
    }

    pub fn merge_user(user: Self) -> Self {
        Self {
            bindings: merge(defaults(), user.bindings),
            details: merge_overlay(details_defaults(), user.details),
            sidebar: merge_overlay(sidebar_defaults(), user.sidebar),
        }
    }
}

/// Default key → command mappings.
///
/// Key names:
/// - single chars keep case: `q`, `d`, `D`
/// - specials: `enter`, `esc`, `up`, `down`, `left`, `right`,
///   `home`, `end`, `pagedown`, `pageup`, `tab`, `backtab`, `space`,
///   `backspace`
/// - modifiers: `C-c` (control), `A-` (alt), `S-` (shift on specials),
///   `D-` (super / Command), combinable as `S-D-left`
pub fn defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("q".into(), "quit".into()),
        ("C-c".into(), "quit".into()),
        ("j".into(), "nav down".into()),
        ("down".into(), "nav down".into()),
        ("k".into(), "nav up".into()),
        ("up".into(), "nav up".into()),
        ("pagedown".into(), "page down".into()),
        ("pageup".into(), "page up".into()),
        ("g".into(), "nav top".into()),
        ("home".into(), "nav top".into()),
        ("G".into(), "nav bottom".into()),
        ("end".into(), "nav bottom".into()),
        ("h".into(), "scroll left".into()),
        ("left".into(), "scroll left".into()),
        ("l".into(), "scroll right".into()),
        ("right".into(), "scroll right".into()),
        ("enter".into(), "view details on".into()),
        ("tab".into(), "focus toggle".into()),
        ("c".into(), "copy".into()),
        ("esc".into(), "command clear".into()),
        ("/".into(), "search".into()),
        (":".into(), "command".into()),
        ("n".into(), "match next".into()),
        ("N".into(), "match prev".into()),
        ("d".into(), "hide".into()),
        ("backspace".into(), "hide line".into()),
        ("D".into(), "delete".into()),
        ("S-backspace".into(), "delete line".into()),
        ("p".into(), "pin".into()),
        ("?".into(), "help".into()),
        ("s".into(), "view sidebar toggle".into()),
        // Left grows the sidebar (divider moves left); right shrinks it.
        ("S-D-left".into(), "config set sidebar_width +1".into()),
        ("S-D-h".into(), "config set sidebar_width +1".into()),
        ("S-D-right".into(), "config set sidebar_width -1".into()),
        ("S-D-l".into(), "config set sidebar_width -1".into()),
        ("S-D-up".into(), "config set details_max_height +1".into()),
        ("S-D-k".into(), "config set details_max_height +1".into()),
        ("S-D-down".into(), "config set details_max_height -1".into()),
        ("S-D-j".into(), "config set details_max_height -1".into()),
    ])
}

/// Defaults applied when the details overlay is focused (override `[keys]`).
pub fn details_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("space".into(), "fold toggle".into()),
        ("esc".into(), "view current off".into()),
        // Re-declare resize chords so Cmd+Shift+↑/↓/j/k still win over nav while focused.
        ("S-D-up".into(), "config set details_max_height +1".into()),
        ("S-D-k".into(), "config set details_max_height +1".into()),
        ("S-D-down".into(), "config set details_max_height -1".into()),
        ("S-D-j".into(), "config set details_max_height -1".into()),
        ("S-D-left".into(), "config set sidebar_width +1".into()),
        ("S-D-h".into(), "config set sidebar_width +1".into()),
        ("S-D-right".into(), "config set sidebar_width -1".into()),
        ("S-D-l".into(), "config set sidebar_width -1".into()),
    ])
}

/// Defaults applied when the filters sidebar is focused (override `[keys]`).
pub fn sidebar_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("d".into(), "filter delete".into()),
        ("backspace".into(), "filter delete line".into()),
        ("space".into(), "filter set toggle".into()),
        ("enter".into(), "hide reveal".into()),
        ("h".into(), "scroll left".into()),
        ("left".into(), "scroll left".into()),
        ("l".into(), "scroll right".into()),
        ("right".into(), "scroll right".into()),
        ("esc".into(), "view current off".into()),
        ("S-D-left".into(), "config set sidebar_width +1".into()),
        ("S-D-h".into(), "config set sidebar_width +1".into()),
        ("S-D-right".into(), "config set sidebar_width -1".into()),
        ("S-D-l".into(), "config set sidebar_width -1".into()),
        ("S-D-up".into(), "config set details_max_height +1".into()),
        ("S-D-k".into(), "config set details_max_height +1".into()),
        ("S-D-down".into(), "config set details_max_height -1".into()),
        ("S-D-j".into(), "config set details_max_height -1".into()),
    ])
}

/// Merge user keybindings over defaults. Empty command string unbinds a key.
pub fn merge(
    mut base: BTreeMap<String, String>,
    overrides: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    for (key, cmd) in overrides {
        if cmd.trim().is_empty() {
            base.remove(&key);
        } else {
            base.insert(key, cmd);
        }
    }
    base
}

pub fn merge_overlay(
    mut base: BTreeMap<String, String>,
    overrides: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    base.extend(overrides);
    base
}

pub fn binding_for_command<'a>(
    base: &'a BTreeMap<String, String>,
    overlay: Option<&'a BTreeMap<String, String>>,
    command: &str,
) -> Option<&'a str> {
    let mut best = None;
    if let Some(overlay) = overlay {
        for (key, bound) in overlay {
            if bound == command && best.is_none_or(|current: &str| key.len() < current.len()) {
                best = Some(key.as_str());
            }
        }
    }
    for (key, bound) in base {
        if overlay.is_some_and(|overlay| overlay.contains_key(key)) {
            continue;
        }
        if bound == command && best.is_none_or(|current: &str| key.len() < current.len()) {
            best = Some(key.as_str());
        }
    }
    best
}

/// All keys bound to `command` in `overlay` (if any) then `base`, excluding base keys
/// shadowed by the overlay. Shorter keys are listed first.
pub fn bindings_for_command(
    base: &BTreeMap<String, String>,
    overlay: Option<&BTreeMap<String, String>>,
    command: &str,
) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(overlay) = overlay {
        for (key, bound) in overlay {
            if bound == command {
                keys.push(key.clone());
            }
        }
    }
    for (key, bound) in base {
        if overlay.is_some_and(|overlay| overlay.contains_key(key)) {
            continue;
        }
        if bound == command {
            keys.push(key.clone());
        }
    }
    keys.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    keys
}

/// Pretty-print a config key name for the cheatsheet / UI hints.
pub fn display_key(spec: &str) -> String {
    let mut rest = spec;
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut super_key = false;
    loop {
        if let Some(after) = rest.strip_prefix("C-") {
            ctrl = true;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("A-") {
            alt = true;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("S-") {
            shift = true;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("D-") {
            super_key = true;
            rest = after;
        } else {
            break;
        }
    }
    let mut prefixes = Vec::new();
    if ctrl {
        prefixes.push("C");
    }
    if alt {
        prefixes.push("A");
    }
    if shift {
        prefixes.push("S");
    }
    if super_key {
        prefixes.push("D");
    }
    let pretty = match rest {
        "up" => "↑",
        "down" => "↓",
        "left" => "←",
        "right" => "→",
        "pagedown" => "PgDn",
        "pageup" => "PgUp",
        "enter" => "Enter",
        "esc" => "Esc",
        "space" => "Space",
        "backspace" => "Backspace",
        "tab" => "Tab",
        "backtab" => "S-Tab",
        "home" => "Home",
        "end" => "End",
        "delete-key" => "Delete",
        other => other,
    };
    if prefixes.is_empty() {
        pretty.to_string()
    } else {
        format!("{}-{pretty}", prefixes.join("-"))
    }
}

/// Encode a key event as a config key name.
pub fn encode(key: KeyEvent) -> Option<String> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let super_key = key.modifiers.contains(KeyModifiers::SUPER);

    let mut base = match key.code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Esc => "esc".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::BackTab => "backtab".into(),
        KeyCode::Backspace => "backspace".into(),
        KeyCode::Delete => "delete-key".into(),
        _ => return None,
    };

    let is_char = matches!(key.code, KeyCode::Char(_));
    // With modifiers, normalize letters and keep Shift as an explicit `S-` prefix
    // so Shift+Cmd+h encodes as `S-D-h` (not `D-H`).
    if (ctrl || alt || super_key) && base.len() == 1 {
        base = base.to_ascii_lowercase();
    }
    let shift_prefix = shift && (!is_char || ctrl || alt || super_key);

    let mut out = String::new();
    if ctrl {
        out.push_str("C-");
    }
    if alt {
        out.push_str("A-");
    }
    if shift_prefix {
        out.push_str("S-");
    }
    if super_key {
        out.push_str("D-");
    }
    out.push_str(&base);
    Some(out)
}
