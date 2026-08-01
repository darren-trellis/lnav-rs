use crate::app::{App, Focus, InputMode, PendingOp, SidebarItem, ToggleAction};
use crate::config_options;
use crate::filter::{Filter, FilterKind};

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
    for part in raw.split(';') {
        let line = part.trim();
        if line.is_empty() {
            continue;
        }
        execute_one(app, line, invoke);
        if app.should_quit {
            break;
        }
    }
}

fn execute_one(app: &mut App, line: &str, invoke: Invoke) {
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
        "nav" => nav_command(app, rest),
        "page" => page_command(app, rest),
        "scroll" => scroll_command(app, rest),
        "view" => {
            app.cancel_pending_op();
            view_command(app, rest);
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
        "search" => search_command(app, rest),
        "command" => command_command(app, rest),
        "match" => match_command(app, rest),
        "hide" => hide_command(app, rest, invoke),
        "pin" => pin_command(app, rest),
        "delete" => delete_command(app, rest, invoke),
        "filter" => filter_command(app, rest, invoke),
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

fn page_step(app: &App, viewport: usize) -> isize {
    let lines = if app.config.page_lines == 0 {
        viewport.max(1)
    } else {
        app.config.page_lines
    };
    lines as isize
}

fn navigate(app: &mut App, navigation: Navigation) {
    let focus = app.focus();
    match focus {
        Focus::Sidebar if app.config.sidebar => {
            app.with_sidebar_motion(|app| match navigation {
                Navigation::Lines(delta) => app.move_sidebar_cursor(delta),
                Navigation::Pages(pages) => {
                    let step = page_step(app, app.pointer.hit.sidebar_inner.height as usize);
                    app.move_sidebar_cursor(step * pages);
                }
                Navigation::Top(line) => {
                    app.jump_sidebar_cursor(line.unwrap_or(1).saturating_sub(1));
                }
                Navigation::Bottom(Some(line)) => {
                    app.jump_sidebar_cursor(line.saturating_sub(1));
                }
                Navigation::Bottom(None) => {
                    app.jump_sidebar_cursor(app.sidebar_len().saturating_sub(1));
                }
            });
        }
        Focus::Details if app.details.visible => match navigation {
            Navigation::Lines(delta) => app.move_overlay_cursor(delta),
            Navigation::Pages(pages) => {
                let step = page_step(app, app.details.viewport_height);
                app.move_overlay_cursor(step * pages);
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
    let page_height = page_step(app, app.pointer.hit.list_inner.height as usize);
    app.with_motion(|app| match navigation {
        Navigation::Lines(delta) => app.move_selection(delta),
        Navigation::Pages(pages) => app.move_selection(page_height * pages),
        Navigation::Top(line) => {
            app.view.follow = false;
            if app.display_len() == 0 {
                return;
            }
            // Bare `g` / Home: first scrollable body row, not the sticky pin band.
            let target = match line {
                Some(n) => n.saturating_sub(1),
                None => app.pin_count(),
            };
            app.jump_to(target.min(app.display_len() - 1));
        }
        Navigation::Bottom(Some(line)) => {
            app.view.follow = false;
            app.jump_to(line.saturating_sub(1));
        }
        Navigation::Bottom(None) => {
            app.view.follow = true;
            if app.display_len() > 0 {
                app.jump_to(app.display_len() - 1);
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
            if app.view.follow && app.display_len() > 0 {
                app.view.selected = app.display_len() - 1;
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
            let mark = if f.enabled { "*" } else { " " };
            format!("{mark}{i}:{} /{}/", f.label(), f.pattern)
        })
        .collect();
    let state = if app.filtering_enabled { "on" } else { "off" };
    app.status_message = Some(format!("filters[{state}]: {}", parts.join(" · ")));
}

fn delete_filter(app: &mut App, rest: &str) {
    if rest.is_empty() {
        app.delete_sidebar_selection();
        return;
    }
    let Ok(idx) = rest.parse::<usize>() else {
        app.status_message = Some("usage: :filter delete [INDEX]".into());
        return;
    };
    if idx >= app.filters.len() {
        app.status_message = Some("no such filter".into());
        return;
    }
    app.select_sidebar_item(SidebarItem::Filter(idx));
    app.delete_sidebar_selection();
}

fn set_filter_item(app: &mut App, rest: &str) {
    let (action_str, idx_str) = split_cmd(rest);
    let Some(action) = parse_on_off_toggle(action_str) else {
        app.status_message = Some("usage: :filter set on|off|toggle [INDEX]".into());
        return;
    };
    if idx_str.is_empty() {
        app.set_selected_filter_enabled(action);
        return;
    }
    let Ok(idx) = idx_str.parse::<usize>() else {
        app.status_message = Some("usage: :filter set on|off|toggle [INDEX]".into());
        return;
    };
    app.set_filter_enabled(idx, action);
}

fn nav_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "down" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Lines(n));
        }
        "up" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Lines(-n));
        }
        "top" => {
            let line = app.take_count_opt();
            navigate(app, Navigation::Top(line));
        }
        "bottom" => {
            let line = app.take_count_opt();
            navigate(app, Navigation::Bottom(line));
        }
        other => {
            app.status_message = Some(format!(
                "usage: nav [up|down|top|bottom]  (unknown: {other})"
            ));
        }
    }
}

fn page_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "down" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Pages(n));
        }
        "up" => {
            let n = app.take_count() as isize;
            navigate(app, Navigation::Pages(-n));
        }
        other => {
            app.status_message =
                Some(format!("usage: page [up|down]  (unknown: {other})"));
        }
    }
}

fn scroll_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    let n = app.take_count() as isize;
    match sub.to_ascii_lowercase().as_str() {
        "left" => {
            if app.is_sidebar_focused() && app.config.sidebar {
                app.scroll_sidebar_x(-n);
            } else {
                app.scroll_list_x(-n);
            }
        }
        "right" => {
            if app.is_sidebar_focused() && app.config.sidebar {
                app.scroll_sidebar_x(n);
            } else {
                app.scroll_list_x(n);
            }
        }
        other => {
            app.status_message =
                Some(format!("usage: scroll [left|right]  (unknown: {other})"));
        }
    }
}

fn match_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    let direction = match sub.to_ascii_lowercase().as_str() {
        "next" => 1,
        "prev" => -1,
        other => {
            app.status_message =
                Some(format!("usage: match [next|prev]  (unknown: {other})"));
            return;
        }
    };
    let n = app.take_count();
    app.with_motion(|a| {
        for _ in 0..n {
            a.next_match(direction);
        }
    });
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

fn command_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" => {
            app.cancel_pending_op();
            app.begin_command_mode();
        }
        "clear" => {
            app.cancel_pending_op();
            app.status_message = None;
        }
        other => {
            app.status_message =
                Some(format!("usage: :command [clear]  (unknown: {other})"));
        }
    }
}

fn help_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    if sub.is_empty() {
        app.toggle_help_modal();
        return;
    }
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
    app.status_message =
        Some("usage: :help  (or focus details, then :help [on|off|toggle])".into());
}

fn search_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" => {
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
        "clear" => {
            app.cancel_pending_op();
            app.clear_search();
            app.status_message = Some("cleared search".into());
        }
        other => {
            app.status_message =
                Some(format!("usage: :search | :search clear  (unknown: {other})"));
        }
    }
}

fn hide_command(app: &mut App, rest: &str, invoke: Invoke) {
    let (sub, arg) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" => match invoke {
            Invoke::Key => app.start_or_repeat_op(PendingOp::Hide),
            Invoke::Line => app.hide_current(),
        },
        "line" => app.hide_current(),
        "clear" => {
            app.cancel_pending_op();
            let n = app.view.hidden.len();
            app.view.hidden.clear();
            app.rebuild_visible(None);
            app.clamp_sidebar_selection();
            app.status_message = Some(format!("unhid {n} line(s)"));
        }
        "unhide" => {
            app.cancel_pending_op();
            if arg.is_empty() {
                match app.sidebar_selection() {
                    Some(SidebarItem::Hidden(source)) => app.unhide_source(source, false),
                    _ => app.status_message = Some("usage: :hide unhide [LINE]".into()),
                }
                return;
            }
            let Ok(line) = arg.parse::<usize>() else {
                app.status_message = Some("usage: :hide unhide [LINE]".into());
                return;
            };
            if line == 0 {
                app.status_message = Some("usage: :hide unhide [LINE]".into());
                return;
            }
            app.unhide_source(line - 1, false);
        }
        "reveal" => {
            app.cancel_pending_op();
            if arg.is_empty() {
                app.reveal_sidebar_selection();
                return;
            }
            let Ok(line) = arg.parse::<usize>() else {
                app.status_message = Some("usage: :hide reveal [LINE]".into());
                return;
            };
            if line == 0 {
                app.status_message = Some("usage: :hide reveal [LINE]".into());
                return;
            }
            app.unhide_source(line - 1, true);
        }
        other => {
            app.status_message = Some(format!(
                "usage: :hide | :hide line | :hide clear|unhide|reveal  (unknown: {other})"
            ));
        }
    }
}

fn pin_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" => app.pin_current(),
        "clear" => {
            app.cancel_pending_op();
            app.clear_pins();
        }
        other => {
            app.status_message =
                Some(format!("usage: :pin | :pin clear  (unknown: {other})"));
        }
    }
}

fn delete_command(app: &mut App, rest: &str, invoke: Invoke) {
    let (sub, _) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" => match invoke {
            Invoke::Key => app.start_or_repeat_op(PendingOp::Delete),
            Invoke::Line => app.delete_current(),
        },
        "line" => app.delete_current(),
        "all" => {
            app.cancel_pending_op();
            app.delete_all_lines();
        }
        other => {
            app.status_message =
                Some(format!("usage: :delete | :delete line | :delete all  (unknown: {other})"));
        }
    }
}

fn view_command(app: &mut App, rest: &str) {
    let (target, arg) = split_cmd(rest);
    match target.to_ascii_lowercase().as_str() {
        "details" => match parse_on_off_toggle(arg) {
            Some(action) => app.set_details(action),
            None => {
                app.status_message = Some(format!(
                    "usage: :view details [on|off|toggle]  (unknown: {arg})"
                ));
            }
        },
        "sidebar" => match parse_on_off_toggle(arg) {
            Some(action) => {
                app.set_sidebar(action);
                app.maybe_autosave();
            }
            None => {
                app.status_message = Some(format!(
                    "usage: :view sidebar [on|off|toggle]  (unknown: {arg})"
                ));
            }
        },
        "current" => match parse_on_off_toggle(arg) {
            Some(action) => {
                let closed_sidebar = app.set_current_view(action);
                if closed_sidebar {
                    app.maybe_autosave();
                }
            }
            None => {
                app.status_message = Some(format!(
                    "usage: :view current [on|off|toggle]  (unknown: {arg})"
                ));
            }
        },
        other => {
            app.status_message = Some(format!(
                "usage: :view [details|sidebar|current] [on|off|toggle]  (unknown: {other})"
            ));
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

fn fold_command(app: &mut App, rest: &str) {
    let (sub, _) = split_cmd(rest);
    match parse_on_off_toggle(sub) {
        Some(action) => app.set_overlay_fold(action),
        None => {
            app.status_message = Some(format!("usage: :fold [on|off|toggle]  (unknown: {sub})"));
        }
    }
}

fn filter_command(app: &mut App, rest: &str, invoke: Invoke) {
    let (sub, arg) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "delete" => match invoke {
            Invoke::Key if arg.is_empty() => app.start_or_repeat_filter_delete(),
            _ if arg.eq_ignore_ascii_case("line") => {
                app.cancel_pending_op();
                delete_filter(app, "");
            }
            _ => {
                app.cancel_pending_op();
                delete_filter(app, arg);
            }
        },
        other => {
            app.cancel_pending_op();
            match other {
                "" | "list" => list_filters(app),
                "in" => add_filter(app, FilterKind::Include, arg),
                "out" => add_filter(app, FilterKind::Exclude, arg),
                "on" => set_filtering(app, true),
                "off" => set_filtering(app, false),
                "toggle" => set_filtering(app, !app.filtering_enabled),
                "set" => set_filter_item(app, arg),
                "clear" => {
                    let n = app.filters.len();
                    app.filters.clear();
                    app.rebuild_visible(None);
                    app.persist_session();
                    app.status_message = Some(format!("cleared {n} filters"));
                }
                unknown => {
                    app.status_message = Some(format!(
                        "usage: :filter [list|in|out|on|off|toggle|set|clear|delete]  (unknown: {unknown})"
                    ));
                }
            }
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

fn config_command(app: &mut App, rest: &str) {
    let (sub, arg) = split_cmd(rest);
    match sub.to_ascii_lowercase().as_str() {
        "" | "path" => {
            app.status_message = Some(format!("config: {}", app.config_path.display()));
        }
        "set" => set_option(app, arg),
        "get" => get_option(app, arg),
        "save" => match app.save_config() {
            Ok(path) => app.status_message = Some(format!("saved {}", path.display())),
            Err(err) => app.status_message = Some(format!("error: {err:#}")),
        },
        "load" => match app.reload_config() {
            Ok(()) => app.status_message = Some(format!("loaded {}", app.config_path.display())),
            Err(err) => app.status_message = Some(format!("error: {err:#}")),
        },
        other => {
            app.status_message = Some(format!(
                "usage: :config [path|set|get|save|load]  (unknown: {other})"
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
            "usage: :config set {} [VALUE]",
            config_options::usage()
        ));
        return;
    }
    let Some(option) = config_options::find(key) else {
        app.status_message = Some(format!("unknown option: {}", key.to_ascii_lowercase()));
        return;
    };
    if value.is_empty() {
        app.open_config_modal(option);
        return;
    }
    if option.set(app, value) {
        // Theme changes autosave inside `commit_theme`; other options save here.
        if option.name != "theme" {
            app.maybe_autosave();
        }
    }
}

fn goto_line(app: &mut App, n: usize) {
    if n == 0 || app.display_len() == 0 {
        app.status_message = Some("no such line".into());
        return;
    }
    let vis = n - 1;
    if vis >= app.display_len() {
        app.status_message = Some(format!("no such line (1–{})", app.display_len()));
        return;
    }
    app.view.follow = false;
    app.jump_to(vis);
    app.status_message = Some(format!("line {n}"));
}
