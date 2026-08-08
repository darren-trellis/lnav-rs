use crate::app::App;
use crate::config::{CaseMode, SidebarPosition};
use crate::theme::Theme;

#[derive(Clone, Copy)]
pub enum ValueKind {
    Theme,
    Bool,
    Unsigned,
    CaseMode,
    SidebarPosition,
    TimestampFormat,
}

impl ValueKind {
    pub fn suggestions(self) -> Vec<String> {
        match self {
            Self::Theme => Theme::list_names(),
            Self::Bool => ["on", "off", "toggle"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            Self::CaseMode => ["sensitive", "insensitive", "smart", "smartcase"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            Self::SidebarPosition => ["left", "right"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            Self::TimestampFormat => [
                "%H:%M:%S",
                "%H:%M:%S%.3f",
                "%Y-%m-%d %H:%M:%S",
                "%Y-%m-%dT%H:%M:%SZ",
                "raw",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            Self::Unsigned => Vec::new(),
        }
    }

    pub fn suggestion_help(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Bool => "bool",
            Self::Unsigned => "number",
            Self::CaseMode => "case",
            Self::SidebarPosition => "side",
            Self::TimestampFormat => "strftime",
        }
    }
}

pub struct ConfigOption {
    pub name: &'static str,
    pub help: &'static str,
    pub value_kind: ValueKind,
    getter: fn(&App) -> String,
    setter: fn(&mut App, &str) -> bool,
}

impl ConfigOption {
    pub fn get(&self, app: &App) -> String {
        (self.getter)(app)
    }

    pub fn set(&self, app: &mut App, value: &str) -> bool {
        (self.setter)(app, value)
    }
}

const OPTIONS: &[ConfigOption] = &[
    ConfigOption {
        name: "theme",
        help: "theme name",
        value_kind: ValueKind::Theme,
        getter: get_theme,
        setter: set_theme,
    },
    ConfigOption {
        name: "follow",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_follow,
        setter: set_follow,
    },
    ConfigOption {
        name: "wrap_details",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_wrap_details,
        setter: set_wrap_details,
    },
    ConfigOption {
        name: "details_json_tree",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_details_json_tree,
        setter: set_details_json_tree,
    },
    ConfigOption {
        name: "details_max_height",
        help: "max overlay rows",
        value_kind: ValueKind::Unsigned,
        getter: get_details_max_height,
        setter: set_details_max_height,
    },
    ConfigOption {
        name: "details_tab_width",
        help: "tree indent columns",
        value_kind: ValueKind::Unsigned,
        getter: get_details_tab_width,
        setter: set_details_tab_width,
    },
    ConfigOption {
        name: "line_numbers",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_line_numbers,
        setter: set_line_numbers,
    },
    ConfigOption {
        name: "relative_line_numbers",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_relative_line_numbers,
        setter: set_relative_line_numbers,
    },
    ConfigOption {
        name: "list_scrollbar_vertical",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_list_scrollbar_vertical,
        setter: set_list_scrollbar_vertical,
    },
    ConfigOption {
        name: "list_scrollbar_horizontal",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_list_scrollbar_horizontal,
        setter: set_list_scrollbar_horizontal,
    },
    ConfigOption {
        name: "sidebar_scrollbar_vertical",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_sidebar_scrollbar_vertical,
        setter: set_sidebar_scrollbar_vertical,
    },
    ConfigOption {
        name: "sidebar_scrollbar_horizontal",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_sidebar_scrollbar_horizontal,
        setter: set_sidebar_scrollbar_horizontal,
    },
    ConfigOption {
        name: "details_scrollbar_vertical",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_details_scrollbar_vertical,
        setter: set_details_scrollbar_vertical,
    },
    ConfigOption {
        name: "border",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_border,
        setter: set_border,
    },
    ConfigOption {
        name: "autosave",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_autosave,
        setter: set_autosave,
    },
    ConfigOption {
        name: "autoreload",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_autoreload,
        setter: set_autoreload,
    },
    ConfigOption {
        name: "sidebar",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_sidebar,
        setter: set_sidebar,
    },
    ConfigOption {
        name: "sidebar_width",
        help: "sidebar columns",
        value_kind: ValueKind::Unsigned,
        getter: get_sidebar_width,
        setter: set_sidebar_width,
    },
    ConfigOption {
        name: "sidebar_position",
        help: "left|right",
        value_kind: ValueKind::SidebarPosition,
        getter: get_sidebar_position,
        setter: set_sidebar_position,
    },
    ConfigOption {
        name: "mouse",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_mouse,
        setter: set_mouse,
    },
    ConfigOption {
        name: "scroll_lines",
        help: "mouse wheel step",
        value_kind: ValueKind::Unsigned,
        getter: get_scroll_lines,
        setter: set_scroll_lines,
    },
    ConfigOption {
        name: "page_lines",
        help: "page up/down step (0 = viewport)",
        value_kind: ValueKind::Unsigned,
        getter: get_page_lines,
        setter: set_page_lines,
    },
    ConfigOption {
        name: "scroll_moves_selection",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_scroll_moves_selection,
        setter: set_scroll_moves_selection,
    },
    ConfigOption {
        name: "timestamp_format",
        help: "strftime or raw",
        value_kind: ValueKind::TimestampFormat,
        getter: get_timestamp_format,
        setter: set_timestamp_format,
    },
    ConfigOption {
        name: "case_mode",
        help: "sensitive|insensitive|smart",
        value_kind: ValueKind::CaseMode,
        getter: get_case_mode,
        setter: set_case_mode,
    },
    ConfigOption {
        name: "session_filters",
        help: "on|off|toggle",
        value_kind: ValueKind::Bool,
        getter: get_session_filters,
        setter: set_session_filters,
    },
];

pub fn catalog() -> &'static [ConfigOption] {
    OPTIONS
}

pub fn find(name: &str) -> Option<&'static ConfigOption> {
    OPTIONS
        .iter()
        .find(|option| option.name.eq_ignore_ascii_case(name))
}

/// Minimum accepted value for unsigned config options (for spinner adjust).
pub fn unsigned_min(name: &str) -> usize {
    match name {
        "details_max_height" => 4,
        "details_tab_width" => 2,
        "sidebar_width" => crate::config::default_sidebar_width_min(),
        "scroll_lines" => 1,
        "page_lines" => 0,
        _ => 0,
    }
}

pub fn usage() -> String {
    OPTIONS
        .iter()
        .map(|option| option.name)
        .collect::<Vec<_>>()
        .join("|")
}

fn on_off(value: bool) -> String {
    if value { "on" } else { "off" }.into()
}

fn set_bool(
    app: &mut App,
    name: &str,
    value: &str,
    current: fn(&App) -> bool,
    apply: fn(&mut App, bool),
    reports_status: bool,
) -> bool {
    let resolved = match value.to_ascii_lowercase().as_str() {
        "toggle" => Some(!current(app)),
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    };
    match resolved {
        Some(enabled) => {
            apply(app, enabled);
            if !reports_status {
                app.status_message = Some(format!("{name}={}", on_off(enabled)));
            }
            true
        }
        None => {
            app.status_message = Some(format!("usage: :config set {name} on|off|toggle"));
            false
        }
    }
}

fn get_theme(app: &App) -> String {
    app.config.theme.name().to_string()
}

fn set_theme(app: &mut App, value: &str) -> bool {
    app.commit_theme(value);
    !app.status_message
        .as_deref()
        .is_some_and(|message| message.starts_with("error:"))
}

fn get_follow(app: &App) -> String {
    on_off(app.view.follow)
}

fn set_follow(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "follow",
        value,
        |app| app.view.follow,
        App::set_follow,
        true,
    )
}

fn get_wrap_details(app: &App) -> String {
    on_off(app.config.wrap_details)
}

fn set_wrap_details(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "wrap_details",
        value,
        |app| app.config.wrap_details,
        |app, enabled| app.config.wrap_details = enabled,
        false,
    )
}

fn get_details_json_tree(app: &App) -> String {
    on_off(app.config.details_json_tree)
}

fn set_details_json_tree(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "details_json_tree",
        value,
        |app| app.config.details_json_tree,
        |app, enabled| {
            app.config.details_json_tree = enabled;
            app.details.scroll = 0;
        },
        false,
    )
}

fn get_details_max_height(app: &App) -> String {
    app.config.details_max_height.max(4).to_string()
}

fn set_details_max_height(app: &mut App, value: &str) -> bool {
    const MIN: usize = 4;
    if let Some(delta) = parse_relative_delta(value) {
        let delta = delta.saturating_mul(app.take_count() as isize);
        let max = app.details_max_height_cap();
        // Prefer the on-screen overlay height so a config value above the cap
        // still shrinks/grows from what the user sees.
        let current = if app.details.visible && app.pointer.hit.overlay.height > 0 {
            (app.pointer.hit.overlay.height as usize).min(max).max(MIN)
        } else {
            app.config.details_max_height.min(max).max(MIN)
        };
        let next = (current as isize + delta).clamp(MIN as isize, max as isize) as usize;
        app.config.details_max_height = next;
        if !app.details.visible {
            app.open_details();
        }
        app.status_message = Some(format!("details_max_height={next}"));
        return true;
    }
    match value.parse::<usize>() {
        Ok(height) if height >= MIN => {
            app.config.details_max_height = height;
            app.status_message = Some(format!("details_max_height={height}"));
            true
        }
        _ => {
            app.status_message = Some(
                "usage: :config set details_max_height N|+N|-N (N >= 4)".into(),
            );
            false
        }
    }
}

fn get_details_tab_width(app: &App) -> String {
    app.config.details_tab_width.max(2).to_string()
}

fn set_details_tab_width(app: &mut App, value: &str) -> bool {
    match value.parse::<usize>() {
        Ok(width) if width >= 2 => {
            app.config.details_tab_width = width;
            app.status_message = Some(format!("details_tab_width={width}"));
            true
        }
        _ => {
            app.status_message = Some("usage: :config set details_tab_width N (N >= 2)".into());
            false
        }
    }
}

fn get_line_numbers(app: &App) -> String {
    on_off(app.config.line_numbers)
}

fn set_line_numbers(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "line_numbers",
        value,
        |app| app.config.line_numbers,
        |app, enabled| app.config.line_numbers = enabled,
        false,
    )
}

fn get_relative_line_numbers(app: &App) -> String {
    on_off(app.config.relative_line_numbers)
}

fn set_relative_line_numbers(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "relative_line_numbers",
        value,
        |app| app.config.relative_line_numbers,
        |app, enabled| app.config.relative_line_numbers = enabled,
        false,
    )
}

fn get_list_scrollbar_vertical(app: &App) -> String {
    on_off(app.config.list_scrollbar_vertical)
}

fn set_list_scrollbar_vertical(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "list_scrollbar_vertical",
        value,
        |app| app.config.list_scrollbar_vertical,
        |app, enabled| app.config.list_scrollbar_vertical = enabled,
        false,
    )
}

fn get_list_scrollbar_horizontal(app: &App) -> String {
    on_off(app.config.list_scrollbar_horizontal)
}

fn set_list_scrollbar_horizontal(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "list_scrollbar_horizontal",
        value,
        |app| app.config.list_scrollbar_horizontal,
        |app, enabled| app.config.list_scrollbar_horizontal = enabled,
        false,
    )
}

fn get_sidebar_scrollbar_vertical(app: &App) -> String {
    on_off(app.config.sidebar_scrollbar_vertical)
}

fn set_sidebar_scrollbar_vertical(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "sidebar_scrollbar_vertical",
        value,
        |app| app.config.sidebar_scrollbar_vertical,
        |app, enabled| app.config.sidebar_scrollbar_vertical = enabled,
        false,
    )
}

fn get_sidebar_scrollbar_horizontal(app: &App) -> String {
    on_off(app.config.sidebar_scrollbar_horizontal)
}

fn set_sidebar_scrollbar_horizontal(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "sidebar_scrollbar_horizontal",
        value,
        |app| app.config.sidebar_scrollbar_horizontal,
        |app, enabled| app.config.sidebar_scrollbar_horizontal = enabled,
        false,
    )
}

fn get_details_scrollbar_vertical(app: &App) -> String {
    on_off(app.config.details_scrollbar_vertical)
}

fn set_details_scrollbar_vertical(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "details_scrollbar_vertical",
        value,
        |app| app.config.details_scrollbar_vertical,
        |app, enabled| app.config.details_scrollbar_vertical = enabled,
        false,
    )
}

fn get_border(app: &App) -> String {
    on_off(app.config.border)
}

fn set_border(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "border",
        value,
        |app| app.config.border,
        |app, enabled| app.config.border = enabled,
        false,
    )
}

fn get_autosave(app: &App) -> String {
    on_off(app.config.autosave)
}

fn set_autosave(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "autosave",
        value,
        |app| app.config.autosave,
        |app, enabled| app.config.autosave = enabled,
        false,
    )
}

fn get_autoreload(app: &App) -> String {
    on_off(app.config.autoreload)
}

fn set_autoreload(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "autoreload",
        value,
        |app| app.config.autoreload,
        |app, enabled| app.config.autoreload = enabled,
        false,
    )
}

fn get_sidebar(app: &App) -> String {
    on_off(app.config.sidebar)
}

fn set_sidebar(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "sidebar",
        value,
        |app| app.config.sidebar,
        |app, enabled| {
            app.config.sidebar = enabled;
            if app.config_preview {
                return;
            }
            if enabled {
                app.focus_sidebar();
            } else if app.is_sidebar_focused() {
                app.focus_list();
            }
        },
        false,
    )
}

fn get_sidebar_width(app: &App) -> String {
    app.config
        .sidebar_width
        .max(crate::config::default_sidebar_width_min())
        .to_string()
}

fn get_sidebar_position(app: &App) -> String {
    app.config.sidebar_position.as_str().to_string()
}

fn set_sidebar_position(app: &mut App, value: &str) -> bool {
    let Some(position) = SidebarPosition::parse(value) else {
        app.status_message = Some("usage: :config set sidebar_position left|right".into());
        return false;
    };
    app.config.sidebar_position = position;
    app.status_message = Some(format!("sidebar_position={}", position.as_str()));
    true
}

fn set_sidebar_width(app: &mut App, value: &str) -> bool {
    let min = crate::config::default_sidebar_width_min();
    if let Some(delta) = parse_relative_delta(value) {
        let delta = delta.saturating_mul(app.take_count() as isize);
        let max = app.sidebar_width_max();
        let current = app.config.sidebar_width.max(min).min(max);
        let next = (current as isize + delta).clamp(min as isize, max as isize) as usize;
        app.config.sidebar_width = next;
        app.config.sidebar = true;
        app.status_message = Some(format!("sidebar_width={next}"));
        return true;
    }
    match value.parse::<usize>() {
        Ok(width) if width >= min => {
            app.config.sidebar_width = width;
            app.status_message = Some(format!("sidebar_width={width}"));
            true
        }
        _ => {
            app.status_message = Some(format!(
                "usage: :config set sidebar_width N|+N|-N (N >= {min})"
            ));
            false
        }
    }
}

/// `+N` / `-N` relative adjust (not a bare integer).
fn parse_relative_delta(value: &str) -> Option<isize> {
    let value = value.trim();
    if value.len() < 2 || !(value.starts_with('+') || value.starts_with('-')) {
        return None;
    }
    if !value[1..].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn get_mouse(app: &App) -> String {
    on_off(app.config.mouse)
}

fn set_mouse(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "mouse",
        value,
        |app| app.config.mouse,
        |app, enabled| {
            app.config.mouse = enabled;
            if !app.config_preview {
                app.sync_mouse_capture();
            }
        },
        false,
    )
}

fn get_scroll_lines(app: &App) -> String {
    app.config.scroll_lines.max(1).to_string()
}

fn set_scroll_lines(app: &mut App, value: &str) -> bool {
    match value.parse::<usize>() {
        Ok(0) | Err(_) => {
            app.status_message = Some("usage: :config set scroll_lines N (N >= 1)".into());
            false
        }
        Ok(lines) => {
            app.config.scroll_lines = lines;
            app.status_message = Some(format!("scroll_lines={lines}"));
            true
        }
    }
}

fn get_page_lines(app: &App) -> String {
    app.config.page_lines.to_string()
}

fn set_page_lines(app: &mut App, value: &str) -> bool {
    match value.parse::<usize>() {
        Err(_) => {
            app.status_message =
                Some("usage: :config set page_lines N (0 = viewport height)".into());
            false
        }
        Ok(lines) => {
            app.config.page_lines = lines;
            app.status_message = Some(if lines == 0 {
                "page_lines=0 (viewport)".into()
            } else {
                format!("page_lines={lines}")
            });
            true
        }
    }
}

fn get_scroll_moves_selection(app: &App) -> String {
    on_off(app.config.scroll_moves_selection)
}

fn set_scroll_moves_selection(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "scroll_moves_selection",
        value,
        |app| app.config.scroll_moves_selection,
        |app, enabled| app.config.scroll_moves_selection = enabled,
        false,
    )
}

fn get_timestamp_format(app: &App) -> String {
    app.config.timestamp_format.clone()
}

fn set_timestamp_format(app: &mut App, value: &str) -> bool {
    app.config.timestamp_format = value.to_string();
    app.status_message = Some(format!("timestamp_format={}", app.config.timestamp_format));
    true
}

fn get_case_mode(app: &App) -> String {
    app.config.case_mode.as_str().to_string()
}

fn set_case_mode(app: &mut App, value: &str) -> bool {
    let Some(mode) = CaseMode::parse(value) else {
        app.status_message =
            Some("usage: :config set case_mode sensitive|insensitive|smart".into());
        return false;
    };
    app.config.case_mode = mode;
    if let Some(error) = app.apply_case_mode() {
        app.status_message = Some(error);
        false
    } else {
        app.status_message = Some(format!("case_mode={}", mode.as_str()));
        true
    }
}

fn get_session_filters(app: &App) -> String {
    on_off(app.config.session_filters)
}

fn set_session_filters(app: &mut App, value: &str) -> bool {
    set_bool(
        app,
        "session_filters",
        value,
        |app| app.config.session_filters,
        |app, enabled| app.config.session_filters = enabled,
        false,
    )
}
