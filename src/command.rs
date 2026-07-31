use crate::app::{App, InputMode, PendingOp};
use crate::config::Config;
use crate::filter::{Filter, FilterKind};
use crate::theme::Theme;

#[derive(Clone, Copy)]
enum Invoke {
    /// Typed on the `:` command line — apply immediately to the current line.
    Line,
    /// Triggered by a keybinding — vim-style operators wait for a motion.
    Key,
}

#[derive(Clone, Copy)]
pub struct CommandInfo {
    pub name: &'static str,
    pub help: &'static str,
}

/// Commands shown in `:` completion. Keybinding-only shortcuts (q/d/D/…) are not listed.
pub fn catalog() -> &'static [CommandInfo] {
    &[
        CommandInfo {
            name: "quit",
            help: "quit lnav-rs",
        },
        CommandInfo {
            name: "help",
            help: "show help; on|off|toggle details hints when focused",
        },
        CommandInfo {
            name: "down",
            help: "move selection down",
        },
        CommandInfo {
            name: "up",
            help: "move selection up",
        },
        CommandInfo {
            name: "page-down",
            help: "page down",
        },
        CommandInfo {
            name: "page-up",
            help: "page up",
        },
        CommandInfo {
            name: "top",
            help: "jump to first line",
        },
        CommandInfo {
            name: "bottom",
            help: "jump to last line (follow)",
        },
        CommandInfo {
            name: "details",
            help: "on|off|toggle details overlay",
        },
        CommandInfo {
            name: "focus",
            help: "on|off|toggle details vs list focus",
        },
        CommandInfo {
            name: "fold",
            help: "on|off|toggle details tree item",
        },
        CommandInfo {
            name: "copy",
            help: "copy focused details value to clipboard",
        },
        CommandInfo {
            name: "close",
            help: "close details overlay",
        },
        CommandInfo {
            name: "search",
            help: "start search",
        },
        CommandInfo {
            name: "command-mode",
            help: "open command line",
        },
        CommandInfo {
            name: "next-match",
            help: "next search match",
        },
        CommandInfo {
            name: "prev-match",
            help: "previous search match",
        },
        CommandInfo {
            name: "follow",
            help: "on|off|toggle live follow",
        },
        CommandInfo {
            name: "cycle-theme",
            help: "cycle color theme",
        },
        CommandInfo {
            name: "hide",
            help: "hide line(s): dd or d{{motion}}",
        },
        CommandInfo {
            name: "delete",
            help: "delete line(s): DD or D{{motion}}",
        },
        CommandInfo {
            name: "theme",
            help: "theme | list | set [NAME]",
        },
        CommandInfo {
            name: "filter",
            help: "list | in|out [PATTERN] | on|off|toggle",
        },
        CommandInfo {
            name: "clear-filters",
            help: "remove all filters",
        },
        CommandInfo {
            name: "clear-hidden",
            help: "restore lines hidden with hide",
        },
        CommandInfo {
            name: "delete-filter",
            help: "delete filter by index",
        },
        CommandInfo {
            name: "config",
            help: "path | init | set KEY VAL | get KEY | save",
        },
        CommandInfo {
            name: "noh",
            help: "clear search highlights",
        },
    ]
}

/// Command names accepted in keybindings (catalog + compatibility aliases).
pub fn is_known_command(name: &str) -> bool {
    catalog()
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case(name))
        || name.eq_ignore_ascii_case("toggle-follow")
        || name.eq_ignore_ascii_case("set")
}

pub fn execute(app: &mut App, raw: &str) {
    execute_inner(app, raw, Invoke::Line);
}

pub fn execute_from_key(app: &mut App, raw: &str) {
    execute_inner(app, raw, Invoke::Key);
}

fn execute_inner(app: &mut App, raw: &str, invoke: Invoke) {
    let line = raw.trim();
    if line.is_empty() {
        return;
    }

    let (cmd, rest) = split_cmd(line);
    let cmd_l = cmd.to_ascii_lowercase();

    match cmd_l.as_str() {
        "quit" => {
            app.cancel_pending_op();
            app.should_quit = true;
        }
        "help" => {
            app.cancel_pending_op();
            help_command(app, rest);
        }
        "down" => {
            let n = app.take_count() as isize;
            if app.overlay_focused && app.show_overlay {
                app.move_overlay_cursor(n);
            } else {
                app.with_motion(|a| a.move_selection(n));
            }
        }
        "up" => {
            let n = app.take_count() as isize;
            if app.overlay_focused && app.show_overlay {
                app.move_overlay_cursor(-n);
            } else {
                app.with_motion(|a| a.move_selection(-n));
            }
        }
        "page-down" => {
            let n = app.take_count() as isize;
            if app.overlay_focused && app.show_overlay {
                let page = app.overlay_inner_height.max(1) as isize;
                app.move_overlay_cursor(page * n);
            } else {
                app.with_motion(|a| a.move_selection(20 * n));
            }
        }
        "page-up" => {
            let n = app.take_count() as isize;
            if app.overlay_focused && app.show_overlay {
                let page = app.overlay_inner_height.max(1) as isize;
                app.move_overlay_cursor(-page * n);
            } else {
                app.with_motion(|a| a.move_selection(-20 * n));
            }
        }
        "top" => {
            let line = app.take_count_opt();
            if app.overlay_focused && app.show_overlay {
                if let Some(n) = line {
                    app.jump_overlay_cursor(n.saturating_sub(1));
                } else {
                    app.jump_overlay_cursor(0);
                }
            } else {
                app.with_motion(|a| {
                    a.follow = false;
                    if let Some(n) = line {
                        a.jump_to_public(n.saturating_sub(1));
                    } else {
                        a.jump_to_public(0);
                    }
                });
            }
        }
        "bottom" => {
            let line = app.take_count_opt();
            if app.overlay_focused && app.show_overlay {
                if let Some(n) = line {
                    app.jump_overlay_cursor(n.saturating_sub(1));
                } else {
                    let last = app.overlay_content_len.saturating_sub(1);
                    app.jump_overlay_cursor(last);
                }
            } else {
                app.with_motion(|a| {
                    if let Some(n) = line {
                        a.follow = false;
                        a.jump_to_public(n.saturating_sub(1));
                    } else {
                        a.follow = true;
                        if !a.visible.is_empty() {
                            a.jump_to_public(a.visible.len() - 1);
                        }
                    }
                });
            }
        }
        "details" => {
            app.cancel_pending_op();
            details_command(app, rest);
        }
        "focus" => {
            app.cancel_pending_op();
            focus_command(app, rest);
        }
        "fold" => {
            app.cancel_pending_op();
            fold_command(app, rest);
        }
        "copy" => {
            app.cancel_pending_op();
            app.copy_overlay_value();
        }
        "close" => {
            if app.pending_op.is_some() || app.count.is_some() {
                app.cancel_pending_op();
            } else if app.show_overlay {
                app.close_details();
            }
        }
        "search" => {
            app.cancel_pending_op();
            let in_details = app.overlay_focused && app.show_overlay;
            app.input_mode = InputMode::Search;
            app.clear_search();
            app.search_history.reset_navigation();
            app.search_in_overlay = in_details;
            // Keep details focused while searching inside it.
            if in_details {
                app.overlay_focused = true;
            }
            app.status_message = None;
        }
        "command-mode" => {
            app.cancel_pending_op();
            app.begin_command_mode();
        }
        "next-match" => {
            let n = app.take_count();
            app.with_motion(|a| {
                for _ in 0..n {
                    a.next_match(1);
                }
            });
        }
        "prev-match" => {
            let n = app.take_count();
            app.with_motion(|a| {
                for _ in 0..n {
                    a.next_match(-1);
                }
            });
        }
        "follow" | "toggle-follow" => {
            app.cancel_pending_op();
            // `toggle-follow` is a compatibility alias for `follow toggle`.
            if cmd_l == "toggle-follow" {
                app.set_follow(!app.follow);
            } else {
                follow_command(app, rest);
            }
        }
        "cycle-theme" => {
            app.cancel_pending_op();
            app.cycle_theme();
        }
        "hide" => match invoke {
            Invoke::Key => app.start_or_repeat_op(PendingOp::Hide),
            Invoke::Line => app.hide_current(),
        },
        "delete" => match invoke {
            Invoke::Key => app.start_or_repeat_op(PendingOp::Delete),
            Invoke::Line => app.delete_current(),
        },
        "theme" => {
            app.cancel_pending_op();
            theme_command(app, rest);
        }
        "filter" => {
            app.cancel_pending_op();
            filter_command(app, rest);
        }
        "clear-filters" => {
            app.cancel_pending_op();
            let n = app.filters.len();
            app.filters.clear();
            app.rebuild_visible(None);
            app.persist_session();
            app.status_message = Some(format!("cleared {n} filters"));
        }
        "clear-hidden" => {
            app.cancel_pending_op();
            let n = app.hidden.len();
            app.hidden.clear();
            app.rebuild_visible(None);
            app.status_message = Some(format!("unhid {n} line(s)"));
        }
        "delete-filter" => delete_filter(app, rest),
        "noh" => {
            app.clear_search();
            app.status_message = Some("cleared search".into());
        }
        // Compatibility alias for `config set`.
        "set" => {
            app.cancel_pending_op();
            set_option(app, rest);
        }
        "config" => {
            app.cancel_pending_op();
            config_command(app, rest);
        }
        other if other.chars().all(|c| c.is_ascii_digit()) => {
            if let Ok(n) = other.parse::<usize>() {
                goto_line(app, n);
            }
        }
        other => {
            app.status_message = Some(format!("unknown command: :{other}  (try :help)"));
        }
    }
}

fn add_filter(app: &mut App, kind: FilterKind, pattern: &str) {
    let pattern = if pattern.is_empty() {
        if app.search_in_overlay {
            app.status_message = Some(match kind {
                FilterKind::Include => {
                    "usage: :filter in PATTERN  (details search is not a list filter)".into()
                }
                FilterKind::Exclude => {
                    "usage: :filter out PATTERN  (details search is not a list filter)".into()
                }
            });
            return;
        }
        if app.search_query.is_empty() {
            app.status_message = Some(match kind {
                FilterKind::Include => {
                    "usage: :filter in [PATTERN]  (or /search first)".into()
                }
                FilterKind::Exclude => {
                    "usage: :filter out [PATTERN]  (or /search first)".into()
                }
            });
            return;
        }
        if app.search_regex.is_none() {
            app.status_message = Some(
                app.search_error
                    .clone()
                    .unwrap_or_else(|| "no valid search pattern".into()),
            );
            return;
        }
        app.search_query.clone()
    } else {
        pattern.to_string()
    };
    match Filter::new(kind, &pattern, app.config.case_mode) {
        Ok(filter) => {
            let label = filter.label();
            app.filters.push(filter);
            app.filtering_enabled = true;
            app.rebuild_visible(None);
            if app.follow && !app.visible.is_empty() {
                app.selected = app.visible.len() - 1;
            }
            app.persist_session();
            app.status_message = Some(format!(
                "filter-{label}: /{pattern}/  ({} visible, {} hidden)",
                app.visible_len(),
                app.hidden_count()
            ));
        }
        Err(err) => {
            app.status_message = Some(format!("invalid regex: {err}"));
        }
    }
}

fn list_filters(app: &mut App) {
    if app.filters.is_empty() {
        app.status_message = Some("no filters".into());
        return;
    }
    let parts: Vec<String> = app
        .filters
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let on = if f.enabled { "" } else { " off" };
            format!("{i}:{}{on} /{}/", f.label(), f.pattern)
        })
        .collect();
    let state = if app.filtering_enabled {
        "on"
    } else {
        "off"
    };
    app.status_message = Some(format!("filters[{state}]: {}", parts.join(" · ")));
}

fn delete_filter(app: &mut App, rest: &str) {
    let Ok(idx) = rest.parse::<usize>() else {
        app.status_message = Some("usage: :delete-filter INDEX".into());
        return;
    };
    if idx >= app.filters.len() {
        app.status_message = Some("no such filter".into());
        return;
    }
    let removed = app.filters.remove(idx);
    app.rebuild_visible(None);
    app.persist_session();
    app.status_message = Some(format!(
        "deleted filter-{} /{}/",
        removed.label(),
        removed.pattern
    ));
}

fn split_cmd(line: &str) -> (&str, &str) {
    let line = line.trim();
    match line.split_once(char::is_whitespace) {
        Some((cmd, rest)) => (cmd, rest.trim()),
        None => (line, ""),
    }
}

fn parse_on_off_toggle(sub: &str) -> Option<crate::app::FoldAction> {
    match sub.to_ascii_lowercase().as_str() {
        "" | "toggle" => Some(crate::app::FoldAction::Toggle),
        "on" => Some(crate::app::FoldAction::On),
        "off" => Some(crate::app::FoldAction::Off),
        _ => None,
    }
}

fn help_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    let focused = app.show_overlay && app.overlay_focused;
    if focused {
        match parse_on_off_toggle(sub) {
            Some(action) => app.set_overlay_help(action),
            None => {
                app.status_message = Some(format!(
                    "usage: :help [on|off|toggle]  (unknown: {sub})"
                ));
            }
        }
        return;
    }
    if sub.is_empty() {
        app.status_message = Some(
            ":theme [list|set] · d/D: dd/DD · dj/dG · :hide/:delete · :clear-hidden"
                .into(),
        );
    } else {
        app.status_message = Some(
            "usage: :help  (or focus details, then :help [on|off|toggle])".into(),
        );
    }
}

fn details_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_details(action),
        None => {
            app.status_message = Some(format!(
                "usage: :details [on|off|toggle]  (unknown: {sub})"
            ));
        }
    }
}

fn focus_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_overlay_focus(action),
        None => {
            app.status_message = Some(format!(
                "usage: :focus [on|off|toggle]  (unknown: {sub})"
            ));
        }
    }
}

fn follow_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(crate::app::FoldAction::On) => app.set_follow(true),
        Some(crate::app::FoldAction::Off) => app.set_follow(false),
        Some(crate::app::FoldAction::Toggle) => app.set_follow(!app.follow),
        None => {
            app.status_message = Some(format!(
                "usage: :follow [on|off|toggle]  (unknown: {sub})"
            ));
        }
    }
}

fn fold_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_overlay_fold(action),
        None => {
            app.status_message = Some(format!(
                "usage: :fold [on|off|toggle]  (unknown: {sub})"
            ));
        }
    }
}

fn filter_command(app: &mut App, rest: &str) {
    let (sub, arg) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" | "list" => list_filters(app),
        "in" => add_filter(app, FilterKind::Include, arg),
        "out" => add_filter(app, FilterKind::Exclude, arg),
        "on" => set_filtering(app, true),
        "off" => set_filtering(app, false),
        "toggle" => set_filtering(app, !app.filtering_enabled),
        other => {
            app.status_message = Some(format!(
                "usage: :filter [list|in|out|on|off|toggle]  (unknown: {other})"
            ));
        }
    }
}

fn set_filtering(app: &mut App, enabled: bool) {
    app.filtering_enabled = enabled;
    app.rebuild_visible(None);
    app.persist_session();
    app.status_message = Some(if enabled {
        "filtering: on".into()
    } else {
        "filtering: off".into()
    });
}

fn theme_command(app: &mut App, rest: &str) {
    let (sub, arg) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" => {
            app.status_message = Some(format!("theme: {}", app.theme.name));
        }
        "list" => {
            let list = Theme::list_names().join(", ");
            app.status_message = Some(format!("themes: {list}"));
        }
        "set" => {
            if arg.is_empty() {
                app.open_theme_picker();
            } else {
                app.commit_theme(arg);
            }
        }
        other => {
            app.status_message = Some(format!(
                "usage: :theme | :theme list | :theme set [NAME]  (unknown: {other})"
            ));
        }
    }
}

const CONFIG_KEYS_USAGE: &str = "theme|follow|wrap_details|details_json_tree|details_max_height|details_tab_width|line_numbers|relative_line_numbers|scrollbar|autosave|scroll_lines|timestamp_format|case_mode|session_filters|session_stdin";

fn config_command(app: &mut App, rest: &str) {
    let (sub, arg) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" | "path" => {
            app.status_message = Some(format!("config: {}", Config::default_path().display()));
        }
        "init" => match save_config(app) {
            Ok(path) => app.status_message = Some(format!("wrote {}", path.display())),
            Err(err) => app.status_message = Some(format!("error: {err:#}")),
        },
        "set" => set_option(app, arg),
        "get" => get_option(app, arg),
        "save" => match save_config(app) {
            Ok(path) => app.status_message = Some(format!("saved {}", path.display())),
            Err(err) => app.status_message = Some(format!("error: {err:#}")),
        },
        other => {
            app.status_message = Some(format!(
                "usage: :config [path|init|set|get|save]  (unknown: {other})"
            ));
        }
    }
}

fn get_option(app: &mut App, rest: &str) {
    let (key, _) = split_cmd(rest);
    if key.is_empty() {
        app.status_message = Some(format!("usage: :config get {CONFIG_KEYS_USAGE}"));
        return;
    }
    match option_value(app, key) {
        Some(value) => app.status_message = Some(format!("{key}={value}")),
        None => app.status_message = Some(format!("unknown option: {key}")),
    }
}

fn option_value(app: &App, key: &str) -> Option<String> {
    match key.to_ascii_lowercase().as_str() {
        "theme" => Some(app.config.theme.name().to_string()),
        "follow" => Some(format_on_off(current_bool_option(app, "follow")).into()),
        "wrap_details" => Some(format_on_off(current_bool_option(app, "wrap_details")).into()),
        "details_json_tree" => {
            Some(format_on_off(current_bool_option(app, "details_json_tree")).into())
        }
        "details_max_height" => Some(app.config.details_max_height.max(4).to_string()),
        "details_tab_width" => Some(app.config.details_tab_width.max(2).to_string()),
        "line_numbers" => Some(format_on_off(current_bool_option(app, "line_numbers")).into()),
        "relative_line_numbers" => {
            Some(format_on_off(current_bool_option(app, "relative_line_numbers")).into())
        }
        "scrollbar" => Some(format_on_off(current_bool_option(app, "scrollbar")).into()),
        "autosave" => Some(format_on_off(current_bool_option(app, "autosave")).into()),
        "session_filters" => {
            Some(format_on_off(current_bool_option(app, "session_filters")).into())
        }
        "session_stdin" => Some(format_on_off(current_bool_option(app, "session_stdin")).into()),
        "case_mode" => Some(app.config.case_mode.as_str().to_string()),
        "scroll_lines" => Some(app.config.scroll_lines.max(1).to_string()),
        "timestamp_format" => Some(app.config.timestamp_format.clone()),
        _ => None,
    }
}

fn set_option(app: &mut App, rest: &str) {
    let (key, value) = split_cmd(rest);
    if key.is_empty() {
        app.status_message = Some(format!("usage: :config set {CONFIG_KEYS_USAGE} VALUE"));
        return;
    }
    if value.is_empty() {
        app.status_message = Some(format!(
            "usage: :config set {key} VALUE  (or :config get {key})"
        ));
        return;
    }
    if apply_set_option(app, key, value) {
        maybe_autosave(app);
    }
}

fn apply_set_option(app: &mut App, key: &str, value: &str) -> bool {
    match key.to_ascii_lowercase().as_str() {
        "theme" => {
            app.commit_theme(value);
            !status_is_error(app)
        }
        "follow" => set_bool_option(app, "follow", value, |app, v| {
            app.set_follow(v);
        }),
        "wrap_details" => set_bool_option(app, "wrap_details", value, |app, v| {
            app.config.wrap_details = v;
        }),
        "details_json_tree" => set_bool_option(app, "details_json_tree", value, |app, v| {
            app.config.details_json_tree = v;
            app.overlay_scroll = 0;
        }),
        "details_max_height" => match value.parse::<usize>() {
            Ok(n) if n >= 4 => {
                app.config.details_max_height = n;
                app.status_message = Some(format!("details_max_height={n}"));
                true
            }
            _ => {
                app.status_message =
                    Some("usage: :config set details_max_height N (N >= 4)".into());
                false
            }
        },
        "details_tab_width" => match value.parse::<usize>() {
            Ok(n) if n >= 2 => {
                app.config.details_tab_width = n;
                app.status_message = Some(format!("details_tab_width={n}"));
                true
            }
            _ => {
                app.status_message =
                    Some("usage: :config set details_tab_width N (N >= 2)".into());
                false
            }
        },
        "line_numbers" => set_bool_option(app, "line_numbers", value, |app, v| {
            app.config.line_numbers = v;
        }),
        "relative_line_numbers" => set_bool_option(app, "relative_line_numbers", value, |app, v| {
            app.config.relative_line_numbers = v;
        }),
        "scrollbar" => set_bool_option(app, "scrollbar", value, |app, v| {
            app.config.scrollbar = v;
        }),
        "autosave" => set_bool_option(app, "autosave", value, |app, v| {
            app.config.autosave = v;
        }),
        "session_filters" => set_bool_option(app, "session_filters", value, |app, v| {
            app.config.session_filters = v;
        }),
        "session_stdin" => set_bool_option(app, "session_stdin", value, |app, v| {
            app.config.session_stdin = v;
        }),
        "case_mode" => match crate::config::CaseMode::parse(value) {
            Some(mode) => {
                app.config.case_mode = mode;
                if let Some(err) = app.apply_case_mode() {
                    app.status_message = Some(err);
                    false
                } else {
                    app.status_message = Some(format!("case_mode={}", mode.as_str()));
                    true
                }
            }
            None => {
                app.status_message = Some(
                    "usage: :config set case_mode sensitive|insensitive|smart".into(),
                );
                false
            }
        },
        "scroll_lines" => match value.parse::<usize>() {
            Ok(0) => {
                app.status_message = Some("usage: :config set scroll_lines N (N >= 1)".into());
                false
            }
            Ok(n) => {
                app.config.scroll_lines = n;
                app.status_message = Some(format!("scroll_lines={n}"));
                true
            }
            Err(_) => {
                app.status_message = Some("usage: :config set scroll_lines N (N >= 1)".into());
                false
            }
        },
        "timestamp_format" => {
            app.config.timestamp_format = value.to_string();
            app.status_message =
                Some(format!("timestamp_format={}", app.config.timestamp_format));
            true
        }
        other => {
            app.status_message = Some(format!("unknown option: {other}"));
            false
        }
    }
}

fn current_bool_option(app: &App, name: &str) -> bool {
    match name {
        "follow" => app.follow,
        "wrap_details" => app.config.wrap_details,
        "details_json_tree" => app.config.details_json_tree,
        "line_numbers" => app.config.line_numbers,
        "relative_line_numbers" => app.config.relative_line_numbers,
        "scrollbar" => app.config.scrollbar,
        "autosave" => app.config.autosave,
        "session_filters" => app.config.session_filters,
        "session_stdin" => app.config.session_stdin,
        _ => false,
    }
}

fn set_bool_option(
    app: &mut App,
    name: &str,
    value: &str,
    apply: impl FnOnce(&mut App, bool),
) -> bool {
    let resolved = match value.to_ascii_lowercase().as_str() {
        "toggle" => Some(!current_bool_option(app, name)),
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    };
    match resolved {
        Some(v) => {
            apply(app, v);
            // `set_follow` already sets the status message.
            if name != "follow" {
                app.status_message = Some(format!("{name}={}", format_on_off(v)));
            }
            true
        }
        None => {
            app.status_message = Some(format!("usage: :config set {name} on|off|toggle"));
            false
        }
    }
}

fn format_on_off(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}

fn status_is_error(app: &App) -> bool {
    app.status_message
        .as_deref()
        .is_some_and(|m| m.starts_with("error:"))
}

fn maybe_autosave(app: &mut App) {
    if !app.config.autosave {
        return;
    }
    let msg = app.status_message.clone();
    if let Err(err) = save_config(app) {
        app.status_message = Some(format!("error: {err:#}"));
    } else {
        app.status_message = msg;
    }
}

fn save_config(app: &mut App) -> anyhow::Result<std::path::PathBuf> {
    app.config.theme.set_name(app.theme.name.clone());
    app.config.follow = app.follow;
    app.config.write()
}

fn goto_line(app: &mut App, n: usize) {
    if n == 0 || app.visible.is_empty() {
        app.status_message = Some("no such line".into());
        return;
    }
    let vis = n - 1;
    if vis >= app.visible.len() {
        app.status_message = Some(format!("no such line (1–{})", app.visible.len()));
        return;
    }
    app.follow = false;
    app.jump_to_public(vis);
    app.status_message = Some(format!("line {n}"));
}

