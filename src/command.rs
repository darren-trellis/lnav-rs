use crate::app::{App, Focus, InputMode, PendingOp, ToggleAction};
use crate::config::Config;
use crate::config_options;
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
enum Navigation {
    Lines(isize),
    Pages(isize),
    Top(Option<usize>),
    Bottom(Option<usize>),
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
            navigate(app, Navigation::Lines(n));
        }
        "up" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Lines(-n));
        }
        "page-down" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Pages(n));
        }
        "page-up" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Pages(-n));
        }
        "top" => {
            let line = app.take_count_opt();
            navigate(app, Navigation::Top(line));
        }
        "bottom" => {
            let line = app.take_count_opt();
            navigate(app, Navigation::Bottom(line));
        }
        "details" => {
            app.cancel_pending_op();
            details_command(app, rest);
        }
        "focus" => {
            app.cancel_pending_op();
            focus_command(app, rest);
        }
        "sidebar" => {
            app.cancel_pending_op();
            sidebar_command(app, rest);
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
            } else if app.details.visible {
                app.close_details();
            }
        }
        "search" => {
            app.cancel_pending_op();
            let in_details = app.is_details_focused() && app.details.visible;
            app.input_mode = InputMode::Search;
            app.clear_search();
            app.search.history.reset_navigation();
            app.search.in_details = in_details;
            if in_details {
                app.focus_details();
            } else {
                app.focus_list();
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
                app.set_follow(!app.view.follow);
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
            let n = app.view.hidden.len();
            app.view.hidden.clear();
            app.rebuild_visible(None);
            app.status_message = Some(format!("unhid {n} line(s)"));
        }
        "delete-filter" => match invoke {
            Invoke::Key if rest.is_empty() => app.start_or_repeat_filter_delete(),
            _ => delete_filter(app, rest),
        },
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

fn navigate(app: &mut App, navigation: Navigation) {
    let focus = app.focus();
    match focus {
        Focus::Sidebar if app.config.sidebar => match navigation {
            Navigation::Lines(delta) => app.move_sidebar_cursor(delta),
            Navigation::Pages(pages) => {
                let height = app.pointer.hit.sidebar_inner.height.max(1) as isize;
                app.move_sidebar_cursor(height * pages);
            }
            Navigation::Top(line) => {
                app.jump_sidebar_cursor(line.unwrap_or(1).saturating_sub(1));
            }
            Navigation::Bottom(Some(line)) => {
                app.jump_sidebar_cursor(line.saturating_sub(1));
            }
            Navigation::Bottom(None) => {
                app.jump_sidebar_cursor(app.filters.len().saturating_sub(1));
            }
        },
        Focus::Details if app.details.visible => match navigation {
            Navigation::Lines(delta) => app.move_overlay_cursor(delta),
            Navigation::Pages(pages) => {
                let height = app.details.viewport_height.max(1) as isize;
                app.move_overlay_cursor(height * pages);
            }
            Navigation::Top(line) => {
                app.jump_overlay_cursor(line.unwrap_or(1).saturating_sub(1));
            }
            Navigation::Bottom(Some(line)) => {
                app.jump_overlay_cursor(line.saturating_sub(1));
            }
            Navigation::Bottom(None) => {
                app.jump_overlay_cursor(app.details.content_len.saturating_sub(1));
            }
        },
        _ => navigate_list(app, navigation),
    }
}

fn navigate_list(app: &mut App, navigation: Navigation) {
    let page_height = app.pointer.hit.list_inner.height.max(1) as isize;
    app.with_motion(|app| match navigation {
        Navigation::Lines(delta) => app.move_selection(delta),
        Navigation::Pages(pages) => app.move_selection(page_height * pages),
        Navigation::Top(line) => {
            app.view.follow = false;
            app.jump_to(line.unwrap_or(1).saturating_sub(1));
        }
        Navigation::Bottom(Some(line)) => {
            app.view.follow = false;
            app.jump_to(line.saturating_sub(1));
        }
        Navigation::Bottom(None) => {
            app.view.follow = true;
            if !app.view.visible.is_empty() {
                app.jump_to(app.view.visible.len() - 1);
            }
        }
    });
}

fn add_filter(app: &mut App, kind: FilterKind, pattern: &str) {
    let pattern = if pattern.is_empty() {
        if app.search.in_details {
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
        if app.search.query.is_empty() {
            app.status_message = Some(match kind {
                FilterKind::Include => "usage: :filter in [PATTERN]  (or /search first)".into(),
                FilterKind::Exclude => "usage: :filter out [PATTERN]  (or /search first)".into(),
            });
            return;
        }
        if app.search.regex.is_none() {
            app.status_message = Some(
                app.search
                    .error
                    .clone()
                    .unwrap_or_else(|| "no valid search pattern".into()),
            );
            return;
        }
        app.search.query.clone()
    } else {
        pattern.to_string()
    };
    match Filter::new(kind, &pattern, app.config.case_mode) {
        Ok(filter) => {
            let label = filter.label();
            app.filters.push(filter);
            app.filtering_enabled = true;
            app.rebuild_visible(None);
            if app.view.follow && !app.view.visible.is_empty() {
                app.view.selected = app.view.visible.len() - 1;
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
    let state = if app.filtering_enabled { "on" } else { "off" };
    app.status_message = Some(format!("filters[{state}]: {}", parts.join(" · ")));
}

fn delete_filter(app: &mut App, rest: &str) {
    if rest.is_empty() {
        app.delete_selected_filter();
        return;
    }
    let Ok(idx) = rest.parse::<usize>() else {
        app.status_message = Some("usage: :delete-filter [INDEX]".into());
        return;
    };
    if idx >= app.filters.len() {
        app.status_message = Some("no such filter".into());
        return;
    }
    app.sidebar_selected = idx;
    app.delete_selected_filter();
}

fn split_cmd(line: &str) -> (&str, &str) {
    let line = line.trim();
    match line.split_once(char::is_whitespace) {
        Some((cmd, rest)) => (cmd, rest.trim()),
        None => (line, ""),
    }
}

fn parse_on_off_toggle(sub: &str) -> Option<ToggleAction> {
    match sub.to_ascii_lowercase().as_str() {
        "" | "toggle" => Some(ToggleAction::Toggle),
        "on" => Some(ToggleAction::On),
        "off" => Some(ToggleAction::Off),
        _ => None,
    }
}

fn help_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    let focused = app.details.visible && app.is_details_focused();
    if focused {
        match parse_on_off_toggle(sub) {
            Some(action) => app.set_overlay_help(action),
            None => {
                app.status_message =
                    Some(format!("usage: :help [on|off|toggle]  (unknown: {sub})"));
            }
        }
        return;
    }
    if sub.is_empty() {
        app.status_message =
            Some(":theme [list|set] · d/D: dd/DD · dj/dG · :hide/:delete · :clear-hidden".into());
    } else {
        app.status_message =
            Some("usage: :help  (or focus details, then :help [on|off|toggle])".into());
    }
}

fn details_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_details(action),
        None => {
            app.status_message = Some(format!("usage: :details [on|off|toggle]  (unknown: {sub})"));
        }
    }
}

fn focus_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_overlay_focus(action),
        None => {
            app.status_message = Some(format!("usage: :focus [on|off|toggle]  (unknown: {sub})"));
        }
    }
}

fn sidebar_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => {
            app.set_sidebar(action);
            maybe_autosave(app);
        }
        None => {
            app.status_message = Some(format!("usage: :sidebar [on|off|toggle]  (unknown: {sub})"));
        }
    }
}

fn follow_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(ToggleAction::On) => app.set_follow(true),
        Some(ToggleAction::Off) => app.set_follow(false),
        Some(ToggleAction::Toggle) => app.set_follow(!app.view.follow),
        None => {
            app.status_message = Some(format!("usage: :follow [on|off|toggle]  (unknown: {sub})"));
        }
    }
}

fn fold_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_overlay_fold(action),
        None => {
            app.status_message = Some(format!("usage: :fold [on|off|toggle]  (unknown: {sub})"));
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
        app.status_message = Some(format!("usage: :config get {}", config_options::usage()));
        return;
    }
    match config_options::find(key) {
        Some(option) => app.status_message = Some(format!("{key}={}", option.get(app))),
        None => app.status_message = Some(format!("unknown option: {key}")),
    }
}

fn set_option(app: &mut App, rest: &str) {
    let (key, value) = split_cmd(rest);
    if key.is_empty() {
        app.status_message = Some(format!(
            "usage: :config set {} VALUE",
            config_options::usage()
        ));
        return;
    }
    if value.is_empty() {
        app.status_message = Some(format!(
            "usage: :config set {key} VALUE  (or :config get {key})"
        ));
        return;
    }
    let Some(option) = config_options::find(key) else {
        app.status_message = Some(format!("unknown option: {}", key.to_ascii_lowercase()));
        return;
    };
    if option.set(app, value) {
        maybe_autosave(app);
    }
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
    app.config.follow = app.view.follow;
    app.config.write()
}

fn goto_line(app: &mut App, n: usize) {
    if n == 0 || app.view.visible.is_empty() {
        app.status_message = Some("no such line".into());
        return;
    }
    let vis = n - 1;
    if vis >= app.view.visible.len() {
        app.status_message = Some(format!("no such line (1–{})", app.view.visible.len()));
        return;
    }
    app.view.follow = false;
    app.jump_to(vis);
    app.status_message = Some(format!("line {n}"));
}
