# Config reference

Default path: `~/.config/lnav-rs/config.toml`. Create one with `lnav-rs --init-config` or `:config save`. CLI flags (`--theme`, `--config PATH`) override the file.

Scalars live under `[main]` in TOML. `:config set KEY` / `:config get KEY` use the flat names below (`theme` is under `[theme]`).

## Options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | name | `catppuccin` | Color theme (`lnav-rs --list-themes`) |
| `follow` | `on` \| `off` \| `toggle` | `on` | Live-tail as the file or pipe grows |
| `wrap_details` | `on` \| `off` \| `toggle` | `on` | Wrap text in the details overlay |
| `details_json_tree` | `on` \| `off` \| `toggle` | `on` | Tree-view nested JSON in details |
| `details_max_height` | number | `24` | Max details overlay height in rows |
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
| `sidebar_width` | number | `28` | Preferred sidebar width in columns (min `12`) |
| `scroll_lines` | number | `1` | Mouse wheel step |
| `page_lines` | number | `0` | Page up/down step (`0` = viewport height) |
| `scroll_moves_selection` | `on` \| `off` \| `toggle` | `on` | Wheel moves selection (`off` scrolls the viewport only) |
| `timestamp_format` | strftime \| `raw` | `%H:%M:%S` | Timestamp column format (`raw` keeps the log string) |
| `case_mode` | `sensitive` \| `insensitive` \| `smart` | `smart` | Case matching for `/` search and `:filter` |
| `session_filters` | `on` \| `off` \| `toggle` | `on` | Persist filters per log file |
| `session_stdin` | `on` \| `off` \| `toggle` | `on` | Persist filters for stdin |

Boolean `:config set` values use `on` / `off` / `toggle` only. Bare `:config set KEY` opens a modal: a list for theme/bool/case/timestamp options, or a value editor for numbers. List pickers and the number editor apply values live (Esc restores the previous value; Enter / `view details` commits). In the number editor, `↑`/`↓` (or `k`/`j`) increment/decrement.

With `autosave` on, successful `:config set` (including modal confirms) and sidebar visibility writes `config.toml`; use `:config save` to write manually. With `autoreload` on, edits to the config file on disk are applied automatically; use `:config load` to reload manually.

## Line numbers

`line_numbers` / `relative_line_numbers` show a gutter for the visible list (not file line numbers). With both on, the current line is absolute and others are relative. Counts work like vim (`5j`, `3dd`, `10G`). `:N` jumps to view line `N`.

## Columns

`[[columns]]` define the list layout. `source` is a builtin (`level`, `timestamp`, `message`, `raw`, `line`, `format`) or a field path (`annotations.url`, `items.0.id`). Columns without `width` auto-size to the widest value in the current viewport so fields share an X position. Set `width` to fix/truncate; `align` is `"left"` (default), `"center"`, or `"right"`. `padding` is spaces around the cell (`1` for both sides, or `{ left = 1, right = 2 }`). Optional `border` / `border_color` / `border_width` / `border_padding` control the leading rule before that column (`border` overrides `[main].border`; omit to inherit). The first column’s border is also used between line numbers and the list. When the full row is wider than the list pane, `h`/`l` (or ←/→) scroll all columns together horizontally.

## Timestamps and case matching

`timestamp_format` uses [chrono strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html). Set to `raw` to keep the original log string.

`case_mode`: `smart` / `smartcase` (insensitive unless the pattern has an uppercase letter), `insensitive`, or `sensitive`.

## Themes and colors

`[theme]` selects the theme name. Optional `[colors]` / `[levels]` / `[ui]` patches at the config root use the same keys as `themes/*.toml` (not nested under `[theme]`).

Text colors (`foreground`, `border`, `window_focus_border`, `search_match`, `dim`, levels, and `[ui]` color keys) accept a hex string (fg only) or `{ fg = "...", bg = "..." }`. Surface keys (`background`, `overlay_bg`, `selection_*`, `status_*`) stay plain color strings.

Focused chrome: `window_focus_border` is the border of the focused pane (list or details); unfocused panes use `[colors].border`. List column separators: `[ui].border_color`, `[ui].border_width` (`0` = space between columns; `N` draws `N`× `│`), and `[ui].border_padding` (`1` or `{ left, right }`, like column `padding`), gated by `[main].border` unless a column sets `border = true|false`. The same rule is drawn between line numbers and the first column when line numbers are on.

Drop custom TOML themes in `~/.config/lnav-rs/themes/<name>.toml` (same shape as files in `themes/`), then `:config set theme <name>` or `:config set theme` for the picker.

Unknown keys, invalid colors, unknown theme names, and unknown keybinding commands are rejected.

## Keybindings

`[keys]` overrides defaults (merged). Use `key = ""` to unbind. Chain commands with `;` (e.g. `r = "view details on; focus toggle"`). Special key names: `enter`, `esc`, `up`, `down`, `left`, `right`, `home`, `end`, `pagedown`, `pageup`, `space`, `backspace`, `C-c`.

- `[keys.details]` overrides `[keys]` while the details overlay is focused.
- `[keys.sidebar]` overrides `[keys]` while the filters sidebar is focused.
- An empty binding in either contextual section blocks fallback to the same key in `[keys]`.

Keybinding-only commands (`nav`, `page`, `scroll`, `match`, `focus`, `search`, `command`) are listed in the README Commands section; they are omitted from `:` completions.

`?` / `:help` opens a cheatsheet modal (Esc / `q` / `?` closes; `j`/`k` and `h`/`l` scroll).
