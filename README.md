# lnav-rs

A modern log file navigator written in Rust. Inspired by [lnav](https://lnav.org).

## Install

```bash
cargo install --path .
```

## Usage

```bash
lnav-rs examples/sample.jsonl
lnav-rs examples/sample.logfmt --theme nord
lnav-rs --list-themes
lnav-rs --init-config

# Pipe JSON / log lines from a program (keyboard still works via /dev/tty)
myapp | lnav-rs
myapp | lnav-rs -
lnav-rs - < app.jsonl
```

### CLI flags

| Flag | Description |
|------|-------------|
| `[FILE]` | Log file to open, or `-` for stdin (also: `prog \| lnav-rs`) |
| `-t`, `--theme <THEME>` | Theme name (overrides config) |
| `--config <PATH>` | Path to config file |
| `--init-config` | Write a default config file and exit |
| `--list-themes` | List themes (built-in + user) and exit |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

### Keys

Keys are commands, configured under `[keys]` in the config (defaults below).
`:` completion lists **commands** (`quit`, `hide`, `pin`, `delete`, …) — not key aliases like `q`/`d`/`p`/`D`.

| Key | Command |
|-----|---------|
| `j` / `↓` | `nav down` (prefix with a count: `5j`) |
| `k` / `↑` | `nav up` (`5k`) |
| `PgDn` / `Space` | `page down` |
| `PgUp` | `page up` |
| `g` / `Home` | `nav top` |
| `G` / `End` | `nav bottom` |
| `h` / `←` | `scroll left` (list; sidebar when focused) |
| `l` / `→` | `scroll right` (list; sidebar when focused) |
| `Enter` | `view details on` (list); in sidebar `hide reveal` (unhide + jump) |
| `Tab` | `focus toggle` (cycle list → details → sidebar) |
| `Space` | `page down` (list); `fold toggle` in details; `filter set toggle` in sidebar |
| `c` | `copy` (copy focused details value to clipboard) |
| `Esc` | `command clear` (list); `view current off` in details/sidebar |
| `/` | `search` (regex; highlights matched text; Up/Down recall history) |
| `:` | `command` |
| `n` / `N` | `match next` / `match prev` |
| `s` | `view sidebar toggle` (filters + hidden lines) |
| `d` | `hide` operator — `dd` current, `dj`/`dG`/… range; in sidebar `dd` deletes filter / unhides line |
| `Backspace` | `hide line` (same as `dd`; accepts a count); in sidebar `filter delete line` (unhides if on a hidden row) |
| `p` | `pin` — toggle sticky pin at top of list (accepts a count) |
| `D` | `delete` operator — `DD` current, `Dj`/`DG`/… range (in-place; safe with `tee -a`) |
| `?` | `help` (cheatsheet modal; Esc closes; `:help toggle` details border hints when focused) |
| `q` | `quit` |

### Commands (`:`)

| Command | Action | Description |
|---------|--------|-------------|
| `:quit` | | Quit |
| `:help` | `on` \| `off` \| `toggle` | Cheatsheet modal; with details focused, show/hide key hints on the overlay border |
| `:view` | `details` \| `sidebar` \| `current` [`on` \| `off` \| `toggle`] | Open/close/toggle the details overlay, filters sidebar, or focused pane |
| `:fold` | `on` \| `off` \| `toggle` | Fold/unfold the details tree item under the cursor |
| `:copy` | | Copy the focused details value to the clipboard |
| `:hide` | `[line]` \| `clear` \| `unhide [N]` \| `reveal [N]` | Hide current line(s) (`dd`); clear restores; unhide/reveal by sidebar selection or source line N |
| `:pin` | `[clear]` | Pin/unpin sticky line(s) at the top of the list; `clear` unpins all |
| `:delete` | | Delete current line(s) from the file (`DD` / `D`+motion) |
| `:filter` | `list` \| `in [PATTERN]` \| `out [PATTERN]` \| `on` \| `off` \| `toggle` \| `set on` \| `set off` \| `set toggle` `[N]` \| `clear` \| `delete [N]` | List, add, enable/disable, or remove include/exclude filters (omit PATTERN to use list `/` search) |
| `:config` | `path` \| `set KEY [VAL]` \| `get KEY` \| `save` \| `load` | Show path, get/set options (picker/editor when VAL omitted), save, or reload — see [config reference](docs/config.md) |
| `:N` | | Jump to view line `N` |

Keybinding commands (used in `[keys]`; omitted from `:` completions):

| Command | Action | Description |
|---------|--------|-------------|
| `nav` | `up` \| `down` \| `top` \| `bottom` | Move selection / cursor (accepts a count) |
| `page` | `up` \| `down` | Page by viewport height or `page_lines` |
| `scroll` | `left` \| `right` | Horizontal scroll (list, or sidebar when focused) |
| `match` | `next` \| `prev` | Jump between search matches |
| `focus` | `on` \| `off` \| `toggle` | Focus details, list, or cycle list → details → sidebar |
| `search` | `[clear]` | Enter `/` search, or clear highlights |
| `command` | `[clear]` | Enter `:` mode, or clear status / cancel pending input |

## Config

Default path: `~/.config/lnav-rs/config.toml`

```toml
[main]
follow = true
wrap_details = true
details_json_tree = true
details_max_height = 24
details_tab_width = 4
line_numbers = false
relative_line_numbers = false
list_scrollbar_vertical = true
list_scrollbar_horizontal = true
sidebar_scrollbar_vertical = true
sidebar_scrollbar_horizontal = true
details_scrollbar_vertical = true
border = true
autosave = true
autoreload = true
sidebar = false
sidebar_width = 28
scroll_lines = 1
page_lines = 0
scroll_moves_selection = true
timestamp_format = "%H:%M:%S"
case_mode = "smart"   # or "sensitive" | "insensitive"
session_filters = true
session_stdin = true

[theme]
name = "catppuccin"

# [colors]
# background = "#11111b"
# dim = { fg = "#6c7086", bg = "#313244" }

# [levels]
# error = { fg = "#1e1e2e", bg = "#f38ba8" }

# [ui]
# timestamp = { fg = "#89b4fa", bg = "#11111b" }
# border_color = { fg = "#585b70", bg = "#1e1e2e" }
# border_width = 1
# border_padding = 1

[[columns]]
source = "level"
width = 5
align = "center"
padding = 1
# border = true
# border_color = "#585b70"
# border_width = 1
# border_padding = 1

[[columns]]
source = "timestamp"

[[columns]]
source = "message"

[[columns]]
source = "annotations.url"

[keys]
q = "quit"
space = "page down"
h = "scroll left"
l = "scroll right"
# r = "view details on; focus toggle"

[keys.details]
space = "fold toggle"
d = "hide"
D = "delete"

[keys.sidebar]
d = "filter delete"
backspace = "filter delete line"
space = "filter set toggle"
enter = "hide reveal"
h = "scroll left"
l = "scroll right"
```

Create one with `lnav-rs --init-config` or `:config save`. CLI flags override the file (`--theme`, `--config PATH`).

More detail:

- [Config reference](docs/config.md) — options, columns, themes, keybindings
- [Details overlay](docs/details.md)
- [Filters & sidebar](docs/filters.md)
- [Mouse](docs/mouse.md)

## Themes

Set with `:config set theme <name>` or `[theme] name = "..."`:

- `ayu`
- `catppuccin` (default)
- `catppuccin-latte`
- `dayfox`
- `dracula`
- `everforest`
- `github-dark`
- `github-light`
- `gotham`
- `gruvbox`
- `gruvbox-light`
- `horizon`
- `kanagawa`
- `monokai`
- `neovim`
- `night-owl`
- `nightfox`
- `nord`
- `one-dark`
- `oxocarbon`
- `palenight`
- `rose-pine`
- `solarized-dark`
- `solarized-light`
- `synthwave`
- `tokyo-night`
- `zenburn`
