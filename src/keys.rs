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
/// - modifiers: `C-c`, `C-d` (control + key)
pub fn defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("q".into(), "quit".into()),
        ("C-c".into(), "quit".into()),
        ("j".into(), "nav down".into()),
        ("down".into(), "nav down".into()),
        ("k".into(), "nav up".into()),
        ("up".into(), "nav up".into()),
        ("pagedown".into(), "page down".into()),
        ("space".into(), "page down".into()),
        ("pageup".into(), "page up".into()),
        ("g".into(), "nav top".into()),
        ("home".into(), "nav top".into()),
        ("G".into(), "nav bottom".into()),
        ("end".into(), "nav bottom".into()),
        ("enter".into(), "view details on".into()),
        ("tab".into(), "focus toggle".into()),
        ("c".into(), "copy".into()),
        ("esc".into(), "command-mode clear".into()),
        ("/".into(), "search".into()),
        (":".into(), "command-mode".into()),
        ("n".into(), "match next".into()),
        ("N".into(), "match prev".into()),
        ("d".into(), "hide".into()),
        ("backspace".into(), "hide line".into()),
        ("D".into(), "delete".into()),
        ("p".into(), "pin".into()),
        ("?".into(), "help".into()),
        ("s".into(), "view sidebar toggle".into()),
    ])
}

/// Defaults applied when the details overlay is focused (override `[keys]`).
pub fn details_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("space".into(), "fold toggle".into()),
        ("esc".into(), "view current off".into()),
    ])
}

/// Defaults applied when the filters sidebar is focused (override `[keys]`).
pub fn sidebar_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("d".into(), "filter delete".into()),
        ("backspace".into(), "filter delete line".into()),
        ("space".into(), "filter set toggle".into()),
        ("enter".into(), "hide reveal".into()),
        ("esc".into(), "view current off".into()),
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

/// Encode a key event as a config key name.
pub fn encode(key: KeyEvent) -> Option<String> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let base = match key.code {
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

    if ctrl && alt {
        Some(format!("C-A-{base}"))
    } else if ctrl {
        // Normalize C-C / C-c → C-c for letters.
        let base = if base.len() == 1 {
            base.to_ascii_lowercase()
        } else {
            base
        };
        Some(format!("C-{base}"))
    } else if alt {
        Some(format!("A-{base}"))
    } else {
        Some(base)
    }
}
