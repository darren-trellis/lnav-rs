# Teleminator

A modern log and trace navigator written in Rust. Inspired by [lnav](https://lnav.org).

## Install

```bash
cargo install --path .
```

If you previously used `lnav-rs`, move config and sessions:

```bash
mv ~/.config/lnav-rs ~/.config/teleminator
mv ~/.local/share/lnav-rs ~/.local/share/teleminator
```

## Usage

```bash
teleminator examples/sample.jsonl
teleminator examples/sample-spans.jsonl
teleminator examples/sample-otel.txt
teleminator examples/sample.logfmt
teleminator --init-config

# Pipe JSON / log lines from a program (keyboard still works via /dev/tty)
myapp | teleminator
myapp | teleminator -
teleminator - < app.jsonl
```

### CLI flags

| Flag | Description |
|------|-------------|
| `[FILE]` | Log file to open, or `-` for stdin (also: `prog \| teleminator`) |
| `--config <PATH>` | Path to config file |
| `--init-config` | Write a default config file and exit |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

### Keys

Keys are commands, configured under `[keys]` in the config (defaults below).
`:` completion lists **commands** (`quit`, `hide`, `pin`, `delete`, …) — not key aliases like `q`/`d`/`p`/`D`.

| Key | Command |
|-----|---------|
| `j` / `↓` | `nav down` (prefix with a count: `5j`) |
| `k` / `↑` | `nav up` (`5k`) |
| `PgDn` | `page down` |
| `PgUp` | `page up` |
| `g` / `Home` | `nav top` |
| `G` / `End` | `nav bottom` |
| `h` / `←` | `scroll left` (list; sidebar when focused) |
| `l` / `→` | `scroll right` (list; sidebar when focused) |
| `Enter` | `view details on` (list); in Spans opens the span's log; in sidebar `hide reveal` (unhide + jump) |
| `Tab` | `focus toggle` (cycle list → details → sidebar) |
| `[` / `]` | `view tab prev` / `view tab next` (Logs ↔ Spans) |
| `Space` | `fold toggle` in details or Spans tree; `filter set toggle` in sidebar |
| `c` | `copy` (copy focused details value to clipboard) |
| `Esc` | `command clear` (list); in details/sidebar `view current off` (cancels pending `d`/`D` first, else closes the pane) |
| `/` | `search` (regex; highlights matched text; Up/Down recall history) |
| `:` | `command` |
| `n` / `N` | `match next` / `match prev` |
| `s` | `view sidebar toggle` (filters + hidden lines) |
| `d` | `hide` operator — `dd` current, `dj`/`dG`/… range; in sidebar `dd` deletes filter / unhides line (`dj`/`dG`/… range over sidebar rows) |
| `Backspace` | `hide line` (same as `dd`; accepts a count); in sidebar `filter delete line` (unhides if on a hidden row) |
| `Shift+Backspace` | `delete line` (same as `DD`; accepts a count); in sidebar deletes the selected hidden line |
| `Alt+Shift+←/→` / `h`/`l` | `config set sidebar_width +1` / `-1` (← grows, → shrinks; accepts a count) |
| `Alt+Shift+↑/↓` / `k`/`j` | `config set details_max_height +1` / `-1` (capped by content/layout; accepts a count) |
| `p` | `pin` — toggle sticky pin at top of list (accepts a count) |
| `D` | `delete` operator — `DD` current, `Dj`/`DG`/… range (in-place; safe with `tee -a`); in sidebar `DD`/`Dj`/`DG`/… deletes hidden lines or lines matching selected filter(s) |
| `Ctrl+L` | `delete all` — clear every line (rewrites the file, or drops the in-memory stdin buffer) |
| `?` | `help` (cheatsheet modal; Esc closes; `:help toggle` details border hints when focused) |
| `q` | `quit` |

### Commands

| Command | Action | Description |
|---------|--------|-------------|
| `:quit` | | Quit |
| `:help` | `on` \| `off` \| `toggle` | Cheatsheet modal; with details focused, show/hide key hints on the overlay border |
| `:view` | `details` \| `sidebar` \| `current` [`on` \| `off` \| `toggle`] \| `tab` [`logs` \| `spans` \| `toggle`] \| `logs` \| `spans` | Open/close panes, or switch the Logs / Spans tab |
| `:fold` | `on` \| `off` \| `toggle` | Fold/unfold the details tree item under the cursor |
| `:copy` | | Copy the focused details value to the clipboard |
| `:hide` | `[line]` \| `clear` \| `unhide [N]` \| `reveal [N]` | Hide current line(s) (`dd`); clear restores; unhide/reveal by sidebar selection or source line N |
| `:pin` | `[clear]` | Pin/unpin sticky line(s) at the top of the list; `clear` unpins all |
| `:delete` | `[line]` \| `all` | Delete current line(s) from the file (`DD` / `D`+motion); with sidebar focused, deletes the selected hidden line or all lines matching the selected filter; `line` is immediate; `all` clears the file or the stdin buffer (`Ctrl+L`) |
| `:filter` | `list` \| `in [PATTERN]` \| `out [PATTERN]` \| `on` \| `off` \| `toggle` \| `set on` \| `set off` \| `set toggle` `[N]` \| `clear` \| `delete [N]` | List, add, enable/disable, or remove include/exclude filters (omit PATTERN to use list `/` search) |
| `:config` | `path` \| `set KEY [VAL]` \| `get KEY` \| `save` \| `load` | Show path, get/set options (picker/editor when VAL omitted), save, or reload — see [config reference](docs/config.md) |
| `:N` | | Jump to view line `N` |

Hidden commands (these are mainly used internally for keybindings):

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

Default path: `~/.config/teleminator/config.toml`

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
details_scrollbar_vertical = true
border = true
autosave = true
autoreload = true
scroll_lines = 1
page_lines = 0
scroll_moves_selection = true
timestamp_format = "%H:%M:%S"
case_mode = "smart"   # or "sensitive" | "insensitive"
session_filters = true
session_stdin = true

[sidebar]
enabled = false
width = 28
position = "right"   # or "left"
scrollbar_vertical = true
scrollbar_horizontal = true

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
h = "scroll left"
l = "scroll right"
# r = "view details on; focus toggle"

[keys.details]
space = "fold toggle"
d = "hide"
D = "delete"
C-l = "delete all"

[keys.sidebar]
d = "filter delete"
backspace = "filter delete line"
D = "delete"
S-backspace = "delete line"
space = "filter set toggle"
enter = "hide reveal"
h = "scroll left"
l = "scroll right"

[keys.spans]
space = "fold toggle"
enter = "view details on"
```

### Spans

Log lines that carry both a `trace_id` and `span_id` (OpenTelemetry / Datadog-style fields, including nested `dd.*`) appear under the **Spans** tab as a tree per trace. Use `[` / `]` (or `:view tab spans`) to switch tabs, `Space` to fold, and `Enter` on a span to jump to its log line. See [docs/spans.md](docs/spans.md).

## Options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | name | `catppuccin` | Color theme (see [Themes](#themes)) |
| `follow` | `on` \| `off` \| `toggle` | `on` | Live-tail as the file or pipe grows |
| `wrap_details` | `on` \| `off` \| `toggle` | `on` | Wrap text in the details overlay |
| `details_json_tree` | `on` \| `off` \| `toggle` | `on` | Tree-view nested JSON in details |
| `details_max_height` | number \| `+N`/`-N` | `24` | Max details overlay height in rows (relative adjusts are layout/content-capped) |
| `details_tab_width` | number | `4` | Details tree indent width (min `2`) |
| `line_numbers` | `on` \| `off` \| `toggle` | `off` | Absolute view line numbers in the gutter |
| `relative_line_numbers` | `on` \| `off` \| `toggle` | `off` | Relative (vim-style) line numbers |
| `list_scrollbar_vertical` | `on` \| `off` \| `toggle` | `on` | List vertical scrollbar |
| `list_scrollbar_horizontal` | `on` \| `off` \| `toggle` | `on` | List horizontal scrollbar |
| `sidebar_scrollbar_vertical` | `on` \| `off` \| `toggle` | `on` | Sidebar vertical scrollbar |
| `sidebar_scrollbar_horizontal` | `on` \| `off` \| `toggle` | `on` | Sidebar horizontal scrollbar |
| `details_scrollbar_vertical` | `on` \| `off` \| `toggle` | `on` | Details vertical scrollbar |
| `border` | `on` \| `off` \| `toggle` | `on` | Draw vertical rules between list columns |
| `autosave` | `on` \| `off` \| `toggle` | `on` | Write `config.toml` after successful `:config set` |
| `autoreload` | `on` \| `off` \| `toggle` | `on` | Reload when the config file changes on disk |
| `sidebar` | `on` \| `off` \| `toggle` | `off` | Show filters/hidden sidebar (same as `:view sidebar`) |
| `sidebar_width` | number \| `+N`/`-N` | `28` | Preferred sidebar width in columns (min `12`; relative adjusts show the sidebar) |
| `sidebar_position` | `left` \| `right` | `right` | Place the filters sidebar on the left or right of the main pane |
| `scroll_lines` | number | `1` | Mouse wheel step |
| `page_lines` | number | `0` | Page up/down step (`0` = viewport height) |
| `scroll_moves_selection` | `on` \| `off` \| `toggle` | `on` | Wheel moves selection (`off` scrolls the viewport only) |
| `timestamp_format` | strftime \| `raw` | `%H:%M:%S` | Timestamp column format (`raw` keeps the log string) |
| `case_mode` | `sensitive` \| `insensitive` \| `smart` | `smart` | Case matching for `/` search and `:filter` |
| `session_filters` | `on` \| `off` \| `toggle` | `on` | Persist filters per log file |
| `session_stdin` | `on` \| `off` \| `toggle` | `on` | Persist filters for stdin |

## Themes

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
