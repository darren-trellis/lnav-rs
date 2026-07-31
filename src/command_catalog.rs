#[derive(Clone, Copy)]
pub struct CommandInfo {
    pub name: &'static str,
    pub help: &'static str,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "quit",
        help: "quit lnav-rs",
    },
    CommandInfo {
        name: "help",
        help: "show help; on|off|toggle details hints when focused",
    },
    CommandInfo {
        name: "details",
        help: "on|off|toggle details overlay",
    },
    CommandInfo {
        name: "focus",
        help: "on|off|toggle focus across list/details/sidebar",
    },
    CommandInfo {
        name: "sidebar",
        help: "on|off|toggle filters sidebar",
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
        name: "follow",
        help: "on|off|toggle live follow",
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
        help: "theme | list | set [NAME] | cycle",
    },
    CommandInfo {
        name: "filter",
        help: "list | in|out [PATTERN] | on|off|toggle | clear | delete [N]",
    },
    CommandInfo {
        name: "clear-hidden",
        help: "restore lines hidden with hide",
    },
    CommandInfo {
        name: "config",
        help: "path | init | set KEY VAL | get KEY | save",
    },
    CommandInfo {
        name: "noh",
        help: "clear search highlights",
    },
];

/// Accepted by keybindings (and typed for compatibility), but omitted from `:` completions.
const HIDDEN_COMMANDS: &[&str] = &[
    "nav",
    "page",
    "match",
    "up",
    "down",
    "top",
    "bottom",
    "page-up",
    "page-down",
    "next-match",
    "prev-match",
    "cycle-theme",
    "clear-filters",
    "delete-filter",
    "toggle-follow",
    "set",
];

pub fn catalog() -> &'static [CommandInfo] {
    COMMANDS
}

pub fn is_known_command(name: &str) -> bool {
    COMMANDS
        .iter()
        .any(|command| command.name.eq_ignore_ascii_case(name))
        || HIDDEN_COMMANDS
            .iter()
            .any(|command| command.eq_ignore_ascii_case(name))
}
