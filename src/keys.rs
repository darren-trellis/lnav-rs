use std::collections::BTreeMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Default key → command mappings.
///
/// Key names:
/// - single chars keep case: `q`, `d`, `D`
/// - specials: `enter`, `esc`, `up`, `down`, `left`, `right`,
///   `home`, `end`, `pagedown`, `pageup`, `tab`, `backtab`, `space`
/// - modifiers: `C-c`, `C-d` (control + key)
pub fn defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("q".into(), "quit".into()),
        ("C-c".into(), "quit".into()),
        ("j".into(), "down".into()),
        ("down".into(), "down".into()),
        ("k".into(), "up".into()),
        ("up".into(), "up".into()),
        ("pagedown".into(), "page-down".into()),
        ("space".into(), "page-down".into()),
        ("pageup".into(), "page-up".into()),
        ("g".into(), "top".into()),
        ("home".into(), "top".into()),
        ("G".into(), "bottom".into()),
        ("end".into(), "bottom".into()),
        ("enter".into(), "details".into()),
        ("tab".into(), "focus toggle".into()),
        ("c".into(), "copy".into()),
        ("esc".into(), "close".into()),
        ("/".into(), "search".into()),
        (":".into(), "command-mode".into()),
        ("n".into(), "next-match".into()),
        ("N".into(), "prev-match".into()),
        ("f".into(), "follow toggle".into()),
        ("t".into(), "cycle-theme".into()),
        ("d".into(), "hide".into()),
        ("D".into(), "delete".into()),
        ("?".into(), "help".into()),
        ("s".into(), "sidebar toggle".into()),
    ])
}

/// Defaults applied when the details overlay is focused (override `[keys]`).
pub fn details_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([("space".into(), "fold toggle".into())])
}

/// Defaults applied when the filters sidebar is focused (override `[keys]`).
pub fn sidebar_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([("d".into(), "delete-filter".into())])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn encodes_case_sensitive_chars() {
        let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let big = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
        assert_eq!(encode(d).as_deref(), Some("d"));
        assert_eq!(encode(big).as_deref(), Some("D"));
    }

    #[test]
    fn default_d_maps_to_hide() {
        assert_eq!(defaults().get("d").map(String::as_str), Some("hide"));
        assert_eq!(defaults().get("D").map(String::as_str), Some("delete"));
        assert_eq!(defaults().get("q").map(String::as_str), Some("quit"));
        assert_eq!(
            defaults().get("s").map(String::as_str),
            Some("sidebar toggle")
        );
        assert_eq!(
            sidebar_defaults().get("d").map(String::as_str),
            Some("delete-filter")
        );
    }
}
