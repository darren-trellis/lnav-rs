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
`:` completion lists **commands** (`quit`, `hide`, `pin`, `delete`, …) — not key aliases like `q`/`d`/`p`/`D`.

| Key | Command |
|-----|---------|
| `j` / `↓` | `nav down` (prefix with a count: `5j`) |
| `k` / `↑` | `nav up` (`5k`) |
| `PgDn` / `Space` | `page down` |
| `PgUp` | `page up` |
| `g` / `Home` | `nav top` |
| `G` / `End` | `nav bottom` |
| `Enter` | `view details on` (open and focus details) |
| `Tab` | `focus toggle` (cycle list → details → sidebar) |
| `Space` | `page down` (list); `fold toggle` in details; `filter set toggle` in sidebar |
| `c` | `copy` (copy focused details value to clipboard) |
| `Esc` | `command-mode clear` (list); `view current off` in details/sidebar |
| `/` | `search` (regex; highlights matched text; Up/Down recall history) |
| `:` | `command-mode` |
| `n` / `N` | `match next` / `match prev` |
| `s` | `view sidebar toggle` (filters sidebar) |
| `d` | `hide` operator — `dd` current, `dj`/`dG`/… range; in sidebar `dd` deletes filter |
| `Backspace` | `hide line` (same as `dd`; accepts a count); in sidebar `filter delete line` |
| `p` | `pin line` — toggle sticky pin at top of list (accepts a count) |
| `D` | `delete` operator — `DD` current, `Dj`/`DG`/… range (in-place; safe with `tee -a`) |
| `?` | `help` (cheatsheet modal; Esc closes; `:help toggle` details border hints when focused) |
| `q` | `quit` |

### Mouse

| Action | Effect |
|--------|--------|
| Click a log line | Select it (completes a pending `d`/`D` operator) |
| Double-click a log line | Toggle details overlay |
| Scroll wheel | Move selection (or cycle completions / config picker) |
| Click / drag scrollbar | Scroll the list or details |
| Click a completion | Insert it |
| Click details overlay | Focus it / move details cursor |
| Click filters sidebar | Focus it / select a filter |
| Click status bar | Enter `:` command mode |
| Config picker hover / click | Preview (theme) / set value |

### Commands (`:`)

In `:` mode, **Up/Down** recall command history when no completion is selected (Tab/↑↓ browse completions once one is selected). History is stored in `~/.local/share/lnav-rs/command_history`.

In `/` mode, **Up/Down** recall search history (shared for list and details search). Stored in `~/.local/share/lnav-rs/search_history`. Enter commits the query to history.

| Command | Action |
|---------|--------|
| `:q` | Quit |
| `:help` | Open/close the help cheatsheet modal |
| `:help on\|off\|toggle` | Show/hide/toggle details key hints (when details focused) |
| `:filter` / `:filter list` | List active filters |
| `:filter in [PATTERN]` | Keep only lines matching regex (omit PATTERN to use list `/` search, not details search) |
| `:filter out [PATTERN]` | Hide lines matching regex (omit PATTERN to use list `/` search, not details search) |
| `:filter on\|off\|toggle` | Enable/disable/toggle filtering (all filters) |
| `:filter set on\|off\|toggle [N]` | Enable/disable/toggle one filter (selected sidebar filter if N omitted) |
| `:filter clear` | Remove all filters |
| `:filter delete [N]` | Remove filter by index (or selected sidebar filter) |
| `:fold on\|off\|toggle` | Fold/unfold details tree item under cursor |
| `:view details [on\|off\|toggle]` | Open/close/toggle details overlay |
| `:view sidebar [on\|off\|toggle]` | Show/hide/toggle filters sidebar |
| `:view current [on\|off\|toggle]` | Same for the focused details or sidebar pane |
| `:copy` | Copy focused details value to the clipboard |
| `:hide line` | Hide current line(s) immediately (same as `dd`) |
| `:hide clear` | Restore lines hidden with `d` |
| `:pin` / `:pin line` | Pin/unpin current line(s) sticky at the top of the list |
| `:pin clear` | Unpin all sticky lines |
| `:config set KEY` | Open picker/editor for a setting (theme, bools, formats, numbers) |
| `:config set theme [NAME]` | Set theme, or open theme picker with live preview |
| `:config set follow on\|off\|toggle` | Enable/disable/toggle live follow |
| `:config set wrap_details on\|off\|toggle` | Wrap overlay text |
| `:config set details_json_tree on\|off\|toggle` | Tree-view nested JSON in details (default on) |
| `:config set details_max_height N` | Max details overlay height in rows (default 24) |
| `:config set details_tab_width N` | Details tree indent width (default 4, min 2) |
| `:config set line_numbers on\|off\|toggle` | Show absolute view line numbers |
| `:config set relative_line_numbers on\|off\|toggle` | Show relative line numbers (vim-style) |
| `:config set scrollbar on\|off\|toggle` | Show a right-side scrollbar (default on) |
| `:config set border on\|off\|toggle` | Draw vertical rules between list columns (default on) |
| `:config set autosave on\|off\|toggle` | Auto-save after `:config set` (default on) |
| `:config set autoreload on\|off\|toggle` | Reload when the config file changes on disk (default on) |
| `:config set sidebar on\|off\|toggle` | Show filters sidebar (default off; same as `:view sidebar`) |
| `:config set scroll_lines N` | Mouse wheel step (default 1) |
| `:config set scroll_moves_selection on\|off\|toggle` | Mouse wheel moves selection in list/details/sidebar (default on; off scrolls the viewport only) |
| `:config set timestamp_format …` | strftime for timestamp columns (`raw` = original) |
| `:config set case_mode …` | `sensitive` / `insensitive` / `smart` (search + filters) |
| `:config set session_filters on\|off\|toggle` | Persist filters per log file (default on) |
| `:config set session_stdin on\|off\|toggle` | Persist filters for stdin (default on) |
| `:config get KEY` | Show current value of a config option |
| `:config save` | Save current settings to config |
| `:config load` | Reload settings from the config file |
| `:config` / `:config path` | Show config path |
| `:config init` | Create config from current settings |
| `:hide` | Hide current line / JSON object (immediate) |
| `:pin` | Pin/unpin current line / JSON object (sticky at top) |
| `:delete` | Delete current line / JSON object from file (immediate) |
| `:search clear` | Clear search highlights |
| `:N` | Jump to view line `N` |

## Config

Default path: `~/.config/lnav-rs/config.toml`

```toml
follow = true
wrap_details = true
details_json_tree = true
details_max_height = 24
details_tab_width = 4
line_numbers = false
relative_line_numbers = false
scrollbar = true
border = true
autosave = true
autoreload = true
sidebar = false
scroll_lines = 1
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
# r = "view details on; focus toggle"

[keys.details]
space = "fold toggle"
d = "hide"
D = "delete"

[keys.sidebar]
d = "filter delete"
backspace = "filter delete line"
space = "filter set toggle"
```

`line_numbers` / `relative_line_numbers` show a gutter for the visible list (not file line numbers). With both on, the current line is absolute and others are relative. Counts work like vim (`5j`, `3dd`, `10G`). `:N` jumps to view line `N`.

`[[columns]]` define the list layout. `source` is a builtin (`level`, `timestamp`, `message`, `raw`, `line`, `format`) or a field path (`annotations.url`, `items.0.id`). Columns without `width` auto-size to the widest value in the current viewport so fields share an X position. Set `width` to fix/truncate; `align` is `"left"` (default), `"center"`, or `"right"`. `padding` is spaces around the cell (`1` for both sides, or `{ left = 1, right = 2 }`). Optional `border` / `border_color` / `border_width` / `border_padding` control the leading rule before that column (`border` overrides the top-level `border` setting; omit to inherit). The first column’s border is also used between line numbers and the list.

`timestamp_format` uses [chrono strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) (default `%H:%M:%S`). Set to `raw` to keep the original log string.

`case_mode` controls `/` search and `:filter` matching: `smart` / `smartcase` (default; insensitive unless the pattern has an uppercase letter), `insensitive`, or `sensitive`.

Boolean `:config set` values use `on` / `off` / `toggle` only. Bare `:config set KEY` opens a modal: a list for theme/bool/case/timestamp options, or a value editor for numbers. With `autosave` on (default), successful `:config set` (including modal confirms) and sidebar visibility writes `config.toml`; use `:config save` to write manually. With `autoreload` on (default), edits to the config file on disk are applied automatically; use `:config load` to reload manually.

Details: `Enter` (`view details on`) opens and focuses the overlay — the selection highlight moves into details (`j`/`k` move the cursor; Esc runs `view current off` and closes it). `Tab` / `:focus toggle` cycles focus across the list, details (if open), and the filters sidebar (if open). `Space` folds/unfolds the tree item under the cursor when details is focused (`:fold on|off|toggle`). `:help toggle` (when details focused) toggles keybinding hints on the overlay border. `c` / `:copy` copies the focused item’s value (strings without quotes; objects/arrays as pretty JSON). With details focused, `/` searches inside the overlay (`n`/`N` cycle matches). Nested JSON fields render as a tree when `details_json_tree` is on (`details_tab_width` sets indent per level). Overlay height grows with content up to `details_max_height` (and screen space).

Filters sidebar: `s` / `:view sidebar toggle` shows a right-hand list of current filters. When focused, `j`/`k` move the selection, `Space` toggles the selected filter on/off, and `dd` / Backspace deletes it (`[keys.sidebar]` defaults: `space = "filter set toggle"`, `d = "filter delete"`, `backspace = "filter delete line"`, `esc = "view current off"`). Enabled filters are marked with `*`; disabled ones are ignored. Esc hides the sidebar; Tab cycles focus without closing it.

Filters persist under `~/.local/share/lnav-rs/sessions/` (one file per log path hash; stdin uses `stdin.toml`). `session_filters` / `session_stdin` control that (both default on). Turn `session_stdin` off if you don’t want every pipe to share one filter set.

`[theme]` selects the theme name. Optional `[colors]` / `[levels]` / `[ui]` patches at the config root use the same keys as `themes/*.toml` (not nested under `[theme]`). Text colors (`foreground`, `border`, `window_focus_border`, `search_match`, `dim`, levels, and `[ui]` color keys) accept a hex string (fg only) or `{ fg = "...", bg = "..." }`. Surface keys (`background`, `overlay_bg`, `selection_*`, `status_*`) stay plain color strings. Focused chrome: `window_focus_border` is the border of the focused pane (list or details); unfocused panes use `[colors].border`. List column separators: `[ui].border_color`, `[ui].border_width` (`0` = space between columns; `N` draws `N`× `│`), and `[ui].border_padding` (`1` or `{ left, right }`, like column `padding`), gated by top-level `border` unless a column sets `border = true|false`. The same rule is drawn between line numbers and the first column when line numbers are on. Unknown keys, invalid colors, unknown theme names, and unknown keybinding commands are rejected.

`[keys]` overrides defaults (merged). Use `key = ""` to unbind. Chain commands with `;` (e.g. `r = "view details on; focus toggle"`). Special key names: `enter`, `esc`, `up`, `down`, `home`, `end`, `pagedown`, `pageup`, `space`, `backspace`, `C-c`. `[keys.details]` overrides `[keys]` while the details overlay is focused (defaults: `space = "fold toggle"`, `esc = "view current off"`). `[keys.sidebar]` overrides `[keys]` while the filters sidebar is focused (defaults: `space = "filter set toggle"`, `d = "filter delete"`, `backspace = "filter delete line"`, `esc = "view current off"`). An empty binding in either contextual section blocks fallback to the same key in `[keys]`. Keybinding-only commands omitted from `:` completions: `nav`, `page`, `match`, `focus`, `search`, `command-mode`.

`?` / `:help` opens a cheatsheet modal (Esc / `q` / `?` closes; `j`/`k` and `h`/`l` scroll).

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
:config set theme <name>
:config set theme          # interactive picker
```

## Themes

Built-in: `ayu`, `catppuccin` (default), `catppuccin-latte`, `dayfox`, `dracula`, `everforest`, `github-dark`, `github-light`, `gotham`, `gruvbox`, `gruvbox-light`, `horizon`, `kanagawa`, `monokai`, `neovim`, `night-owl`, `nightfox`, `nord`, `one-dark`, `oxocarbon`, `palenight`, `rose-pine`, `solarized-dark`, `solarized-light`, `synthwave`, `tokyo-night`, `zenburn`. Patch individual colors in `config.toml` under `[colors]` / `[levels]` / `[ui]`.

## Project layout

```
src/
  app.rs, app/    # state coordinator + focused input/mouse/operator/search modules
  columns.rs      # list column rendering
  command.rs      # : command mode
  command_catalog.rs
                  # command metadata shared by config and completion
  config.rs       # config.toml load/save
  config_options.rs
                  # :config option metadata and behavior
  details.rs      # details overlay content (JSON tree)
  history.rs      # : / history (commands + searches)
  model.rs        # log entries / fields
  parse/          # json + logfmt
  session.rs      # per-source filter persistence
  tail.rs         # file read + live watch
  text.rs         # shared display-width text helpers
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
