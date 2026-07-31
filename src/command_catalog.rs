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
        name: "view",
        help: "details|sidebar [on|off|toggle]",
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
        name: "follow",
        help: "on|off|toggle live follow",
    },
    CommandInfo {
        name: "hide",
        help: "hide line(s): dd or d{{motion}} | line | clear",
    },
    CommandInfo {
        name: "pin",
        help: "pin line(s) sticky at top | line | clear",
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
        name: "config",
        help: "path | init | set KEY VAL | get KEY | save",
    },
];

/// Keybinding-only commands omitted from `:` completions.
const HIDDEN_COMMANDS: &[&str] = &["nav", "page", "match", "focus", "search", "command-mode"];

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
