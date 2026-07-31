# lnav-rs

A modern log file navigator — a focused Rust + [ratatui](https://ratatui.rs) rewrite of [lnav](https://lnav.org) essentials.

Think neovim to vim: same job, cleaner defaults, friendlier UX.

## Status

Early MVP. Supported today:

- Open a **single log file** or read from **stdin** (`prog | lnav-rs`)
- Parse **JSON** (JSONL) and **logfmt** lines
- **Live tail** as the file / pipe grows
- Level-aware **coloring** + built-in / user **themes**
- **Enter** to open a field details overlay
- Basic navigation + **regex search** (`/`) with per-match highlighting
- **Config file** (`~/.config/lnav-rs/config.toml`) including list `[[columns]]`
- **Command mode** (`:`) with `:filter in` / `:filter out`

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

### Keys

Keys are commands, configured under `[keys]` in the config (defaults below).
`:` completion lists **commands** (`quit`, `hide`, `delete`, …) — not key aliases like `q`/`d`/`D`.

| Key | Command |
|-----|---------|
| `j` / `↓` | `down` (prefix with a count: `5j`) |
| `k` / `↑` | `up` (`5k`) |
| `PgDn` / `Space` | `page-down` |
| `PgUp` | `page-up` |
| `g` / `Home` | `top` |
| `G` / `End` | `bottom` |
| `Enter` | `details` / `details toggle` (open/focus; again to close when focused) |
| `Tab` | `focus toggle` (switch focus between details and list) |
| `Space` | `page-down` (list); `fold toggle` via `[details_keys]` when details focused |
| `c` | `copy` (copy focused details value to clipboard) |
| `Esc` | `close` |
| `/` | `search` (case-insensitive regex; highlights matched text) |
| `:` | `command-mode` |
| `n` / `N` | `next-match` / `prev-match` |
| `f` | `follow toggle` |
| `t` | `cycle-theme` |
| `d` | `hide` operator — `dd` current, `dj`/`dG`/… range |
| `D` | `delete` operator — `DD` current, `Dj`/`DG`/… range (in-place; safe with `tee -a`) |
| `?` | `help` (status cheat sheet; `help toggle` details key hints when focused) |
| `q` | `quit` |

### Mouse

| Action | Effect |
|--------|--------|
| Click a log line | Select it (completes a pending `d`/`D` operator) |
| Double-click a log line | Toggle details overlay |
| Scroll wheel | Move selection (or cycle completions / theme picker) |
| Click a completion | Insert it |
| Click details overlay | Close it |
| Click status bar | Enter `:` command mode |
| Theme picker hover / click | Preview / set theme |

### Commands (`:`)

In `:` mode, **Up/Down** recall command history when no completion is selected (Tab/↑↓ browse completions once one is selected). History is stored in `~/.local/share/lnav-rs/command_history`.

| Command | Action |
|---------|--------|
| `:q` | Quit |
| `:help` | Status cheat sheet |
| `:help on\|off\|toggle` | Show/hide/toggle details key hints (when focused) |
| `:filter` / `:filter list` | List active filters |
| `:filter in [PATTERN]` | Keep only lines matching regex (omit PATTERN to use list `/` search, not details search) |
| `:filter out [PATTERN]` | Hide lines matching regex (omit PATTERN to use list `/` search, not details search) |
| `:filter on\|off\|toggle` | Enable/disable/toggle filtering |
| `:fold on\|off\|toggle` | Fold/unfold details tree item under cursor |
| `:follow on\|off\|toggle` | Enable/disable/toggle live follow |
| `:details on\|off\|toggle` | Open/close/toggle details overlay |
| `:focus on\|off\|toggle` | Focus details / list / switch between them |
| `:copy` | Copy focused details value to the clipboard |
| `:delete-filter N` | Remove filter by index |
| `:clear-filters` | Remove all filters |
| `:clear-hidden` | Restore lines hidden with `d` |
| `:theme` | Show current theme |
| `:theme list` | List available themes |
| `:theme set` | Open theme picker (preview on hover / ↑↓) |
| `:theme set NAME` | Switch theme |
| `:config set theme NAME` | Set theme |
| `:config set follow on\|off\|toggle` | Enable/disable/toggle live follow |
| `:config set wrap_details on\|off\|toggle` | Wrap overlay text |
| `:config set details_json_tree on\|off\|toggle` | Tree-view nested JSON in details (default on) |
| `:config set details_max_height N` | Max details overlay height in rows (default 24) |
| `:config set details_tab_width N` | Details tree indent width (default 4, min 2) |
| `:config set line_numbers on\|off\|toggle` | Show absolute view line numbers |
| `:config set relative_line_numbers on\|off\|toggle` | Show relative line numbers (vim-style) |
| `:config set scrollbar on\|off\|toggle` | Show a right-side scrollbar (list + details) |
| `:config set scroll_lines N` | Mouse wheel step (default 1) |
| `:config set timestamp_format …` | strftime for timestamp columns (`raw` = original) |
| `:config set case_mode …` | `sensitive` / `insensitive` / `smart` (search + filters) |
| `:config set session_filters on\|off\|toggle` | Persist filters per log file (default on) |
| `:config set session_stdin on\|off\|toggle` | Persist filters for stdin (default on) |
| `:config get KEY` | Show current value of a config option |
| `:w` / `:write` | Write current settings to config |
| `:config` / `:config path` | Show config path |
| `:config init` | Create config from current settings |
| `:hide` | Hide current line / JSON object (immediate) |
| `:delete` | Delete current line / JSON object from file (immediate) |
| `:noh` | Clear search highlights |
| `:N` | Jump to view line `N` |

## Config

Default path: `~/.config/lnav-rs/config.toml`

```toml
[theme]
name = "catppuccin"
# [theme.colors]
# background = "#11111b"
# dim = { fg = "#6c7086", bg = "#313244" }
# [theme.levels]
# error = { fg = "#1e1e2e", bg = "#f38ba8" }
# [theme.ui]
# timestamp = { fg = "#89b4fa", bg = "#11111b" }
# column_border = { fg = "#585b70", bg = "#1e1e2e" }
# column_border_width = 1
# column_border_padding = 1
# column_border_padding = { left = 1, right = 2 }

follow = true
wrap_details = true
details_json_tree = true
details_max_height = 24
details_tab_width = 4
line_numbers = false
relative_line_numbers = false
scrollbar = false
scroll_lines = 1
timestamp_format = "%H:%M:%S"
case_mode = "smart"   # or "sensitive" | "insensitive"
session_filters = true
session_stdin = true

[[columns]]
source = "level"
width = 5
align = "center"
padding = 1

[[columns]]
source = "timestamp"

[[columns]]
source = "message"

[[columns]]
source = "annotations.url"

[keys]
q = "quit"
space = "page-down"

[details_keys]
space = "fold toggle"
d = "hide"
D = "delete"
```

`line_numbers` / `relative_line_numbers` show a gutter for the visible list (not file line numbers). With both on, the current line is absolute and others are relative. Counts work like vim (`5j`, `3dd`, `10G`). `:N` jumps to view line `N`.

`[[columns]]` define the list layout. `source` is a builtin (`level`, `timestamp`, `message`, `raw`, `line`, `format`) or a field path (`annotations.url`, `items.0.id`). Columns without `width` auto-size to the widest value in the current viewport so fields share an X position. Set `width` to fix/truncate; `align` is `"left"` (default), `"center"`, or `"right"`. `padding` is spaces around the cell (`1` for both sides, or `{ left = 1, right = 2 }`).

`timestamp_format` uses [chrono strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) (default `%H:%M:%S`). Set to `raw` to keep the original log string.

`case_mode` controls `/` search and `:filter` matching: `smart` / `smartcase` (default; insensitive unless the pattern has an uppercase letter), `insensitive`, or `sensitive`.

Details: `Enter` opens and focuses the overlay — the selection highlight moves into details (`j`/`k` move the cursor; Esc closes). `Tab` / `:focus toggle` switches focus between details and the list. `Space` folds/unfolds the tree item under the cursor when details is focused (`:fold on|off|toggle`). `?` toggles keybinding hints on the overlay border. `c` / `:copy` copies the focused item’s value (strings without quotes; objects/arrays as pretty JSON). With details focused, `/` searches inside the overlay (`n`/`N` cycle matches). Nested JSON fields render as a tree when `details_json_tree` is on (`details_tab_width` sets indent per level). Overlay height grows with content up to `details_max_height` (and screen space).

Filters persist under `~/.local/share/lnav-rs/sessions/` (one file per log path hash; stdin uses `stdin.toml`). `session_filters` / `session_stdin` control that (both default on). Turn `session_stdin` off if you don’t want every pipe to share one filter set.

`[theme]` selects the theme (`name`) and optional `[theme.colors]` / `[theme.levels]` / `[theme.ui]` patches (same keys as `themes/*.toml`). Text colors (`foreground`, `border`, `window_focus_border`, `search_match`, `dim`, levels, and `[ui]` color keys) accept a hex string (fg only) or `{ fg = "...", bg = "..." }`. Surface keys (`background`, `overlay_bg`, `selection_*`, `status_*`) stay plain color strings. Focused chrome: `window_focus_border` is the border of the focused pane (list or details); unfocused panes use `border`. List column separators: `ui.column_border` (color), `ui.column_border_width` (`0` = space between columns; `N` draws `N`× `│`), and `ui.column_border_padding` (`1` or `{ left, right }`, like column `padding`). Unknown keys, invalid colors, unknown theme names, and unknown keybinding commands are rejected.

`[keys]` overrides defaults (merged). Use `key = ""` to unbind. Special key names: `enter`, `esc`, `up`, `down`, `home`, `end`, `pagedown`, `pageup`, `space`, `C-c`. `[details_keys]` overrides `[keys]` while the details overlay is focused (default: `space = "fold toggle"`).

Create one with:

```bash
lnav-rs --init-config
# or inside the TUI:
:config init
```

CLI flags override the config file (`--theme`, `--config PATH`).

### User themes

Drop TOML themes in `~/.config/lnav-rs/themes/<name>.toml` (same shape as files in `themes/`), then:

```text
:theme set <name>
:theme set          # interactive picker
```

## Themes

Built-in: `catppuccin` (default), `dracula`, `github-dark`, `github-light`, `gotham`, `nord`, `solarized-dark`, `solarized-light`, `tokyo-night`. Patch individual colors in `config.toml` under `[theme]`.

## Project layout

```
src/
  app.rs          # state + keybindings
  columns.rs      # list column rendering
  command.rs      # : command mode
  config.rs       # config.toml load/save
  details.rs      # details overlay content (JSON tree)
  history.rs      # : command history
  model.rs        # log entries / fields
  parse/          # json + logfmt
  session.rs      # per-source filter persistence
  tail.rs         # file read + live watch
  theme.rs        # theme loader
  ui/             # ratatui views
themes/           # built-in TOML themes
examples/         # sample logs + example config
```

## Roadmap (not yet)

- Multiple files / time merge
- Bookmarks
- SQL / query surface
- Custom format definitions
