use crate::app::{App, InputMode, PendingOp};
use crate::completion;
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
            help: "show command help",
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
            help: "open/focus/close details overlay",
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
            name: "toggle-follow",
            help: "toggle live follow",
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
            name: "set",
            help: "set option  :set KEY VAL",
        },
        CommandInfo {
            name: "write",
            help: "write config to disk",
        },
        CommandInfo {
            name: "config",
            help: "config path | init",
        },
        CommandInfo {
            name: "noh",
            help: "clear search highlights",
        },
    ]
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
            app.status_message = Some(
                ":theme [list|set] · d/D: dd/DD · dj/dG · :hide/:delete · :clear-hidden"
                    .into(),
            );
        }
        "down" => {
            let n = app.take_count() as isize;
            app.with_motion(|a| a.move_selection(n));
        }
        "up" => {
            let n = app.take_count() as isize;
            app.with_motion(|a| a.move_selection(-n));
        }
        "page-down" => {
            let n = app.take_count() as isize;
            app.with_motion(|a| a.move_selection(20 * n));
        }
        "page-up" => {
            let n = app.take_count() as isize;
            app.with_motion(|a| a.move_selection(-20 * n));
        }
        "top" => {
            let line = app.take_count_opt();
            app.with_motion(|a| {
                a.follow = false;
                if let Some(n) = line {
                    a.jump_to_public(n.saturating_sub(1));
                } else {
                    a.jump_to_public(0);
                }
            });
        }
        "bottom" => {
            let line = app.take_count_opt();
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
        "details" => app.toggle_details(),
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
            app.search_in_overlay = in_details;
            // Keep details focused while searching inside it.
            if in_details {
                app.overlay_focused = true;
            }
            app.status_message = None;
        }
        "command-mode" => {
            app.cancel_pending_op();
            app.input_mode = InputMode::Command;
            app.command_buffer.clear();
            app.status_message = None;
            completion::refresh(app);
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
        "toggle-follow" => {
            app.cancel_pending_op();
            app.follow = !app.follow;
            app.config.follow = app.follow;
            if app.follow && !app.visible.is_empty() {
                app.jump_to_public(app.visible.len() - 1);
            }
            app.status_message = Some(if app.follow {
                "follow: on".into()
            } else {
                "follow: off".into()
            });
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
        "set" => set_option(app, rest),
        "write" => match app.config.write() {
            Ok(path) => app.status_message = Some(format!("wrote {}", path.display())),
            Err(err) => app.status_message = Some(format!("error: {err:#}")),
        },
        "config" => {
            if rest.is_empty() || rest == "path" {
                app.status_message =
                    Some(format!("config: {}", Config::default_path().display()));
            } else if rest == "init" {
                match init_config(app) {
                    Ok(path) => app.status_message = Some(format!("wrote {}", path.display())),
                    Err(err) => app.status_message = Some(format!("error: {err:#}")),
                }
            } else {
                app.status_message = Some("usage: :config [path|init]".into());
            }
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

fn set_option(app: &mut App, rest: &str) {
    let (key, value) = split_cmd(rest);
    if key.is_empty() {
        app.status_message = Some(
            "usage: :set theme|follow|wrap_details|details_json_tree|details_max_height|line_numbers|relative_line_numbers|scroll_lines|timestamp_format|case_mode|session_filters|session_stdin VALUE"
                .into(),
        );
        return;
    }
    match key.to_ascii_lowercase().as_str() {
        "theme" => {
            if value.is_empty() {
                app.status_message = Some(format!("theme={}", app.config.theme.name()));
            } else {
                app.commit_theme(value);
            }
        }
        "follow" => set_bool_option(app, "follow", value, |app, v| {
            app.follow = v;
            app.config.follow = v;
            if v && !app.visible.is_empty() {
                app.jump_to_public(app.visible.len() - 1);
            }
        }),
        "wrap_details" => set_bool_option(app, "wrap_details", value, |app, v| {
            app.config.wrap_details = v;
        }),
        "details_json_tree" => set_bool_option(app, "details_json_tree", value, |app, v| {
            app.config.details_json_tree = v;
            app.overlay_scroll = 0;
        }),
        "details_max_height" => {
            if value.is_empty() {
                app.status_message = Some(format!(
                    "details_max_height={}",
                    app.config.details_max_height.max(4)
                ));
                return;
            }
            match value.parse::<usize>() {
                Ok(n) if n >= 4 => {
                    app.config.details_max_height = n;
                    app.status_message = Some(format!("details_max_height={n}"));
                }
                _ => {
                    app.status_message =
                        Some("usage: :set details_max_height N (N >= 4)".into());
                }
            }
        }
        "line_numbers" => set_bool_option(app, "line_numbers", value, |app, v| {
            app.config.line_numbers = v;
        }),
        "relative_line_numbers" => set_bool_option(app, "relative_line_numbers", value, |app, v| {
            app.config.relative_line_numbers = v;
        }),
        "session_filters" => set_bool_option(app, "session_filters", value, |app, v| {
            app.config.session_filters = v;
        }),
        "session_stdin" => set_bool_option(app, "session_stdin", value, |app, v| {
            app.config.session_stdin = v;
        }),
        "case_mode" => {
            if value.is_empty() {
                app.status_message =
                    Some(format!("case_mode={}", app.config.case_mode.as_str()));
                return;
            }
            match crate::config::CaseMode::parse(value) {
                Some(mode) => {
                    app.config.case_mode = mode;
                    app.status_message = app
                        .apply_case_mode()
                        .or_else(|| Some(format!("case_mode={}", mode.as_str())));
                }
                None => {
                    app.status_message = Some(
                        "usage: :set case_mode sensitive|insensitive|smart".into(),
                    );
                }
            }
        }
        "scroll_lines" => {
            if value.is_empty() {
                app.status_message =
                    Some(format!("scroll_lines={}", app.config.scroll_lines.max(1)));
                return;
            }
            match value.parse::<usize>() {
                Ok(0) => app.status_message = Some("usage: :set scroll_lines N (N >= 1)".into()),
                Ok(n) => {
                    app.config.scroll_lines = n;
                    app.status_message = Some(format!("scroll_lines={n}"));
                }
                Err(_) => {
                    app.status_message = Some("usage: :set scroll_lines N (N >= 1)".into());
                }
            }
        }
        "timestamp_format" => {
            if value.is_empty() {
                app.status_message =
                    Some(format!("timestamp_format={}", app.config.timestamp_format));
            } else {
                app.config.timestamp_format = value.to_string();
                app.status_message =
                    Some(format!("timestamp_format={}", app.config.timestamp_format));
            }
        }
        other => {
            app.status_message = Some(format!("unknown option: {other}"));
        }
    }
}

fn set_bool_option(app: &mut App, name: &str, value: &str, apply: impl FnOnce(&mut App, bool)) {
    if value.is_empty() {
        let current = match name {
            "follow" => app.config.follow,
            "wrap_details" => app.config.wrap_details,
            "details_json_tree" => app.config.details_json_tree,
            "line_numbers" => app.config.line_numbers,
            "relative_line_numbers" => app.config.relative_line_numbers,
            "session_filters" => app.config.session_filters,
            "session_stdin" => app.config.session_stdin,
            _ => false,
        };
        app.status_message = Some(format!("{name}={current}"));
        return;
    }
    match parse_bool(value) {
        Some(v) => {
            apply(app, v);
            app.status_message = Some(format!("{name}={v}"));
        }
        None => app.status_message = Some(format!("usage: :set {name} on|off")),
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
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

fn init_config(app: &mut App) -> anyhow::Result<std::path::PathBuf> {
    app.config.theme.set_name(app.theme.name.clone());
    app.config.follow = app.follow;
    app.config.write()
}
