# Config reference

Default path: `~/.config/teleminator/config.toml`.

`:config set` / `:config get` use flat option names for scalars (`wrap_details`, `sidebar_width`, …) even when the file key lives under a section.

## `[main]`

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `follow` | `true` \| `false` | `true` | Live-tail as the file or pipe grows |
| `line_numbers` | `true` \| `false` | `false` | Absolute view line numbers in the gutter |
| `relative_line_numbers` | `true` \| `false` | `false` | Relative (vim-style) line numbers |
| `list_scrollbar_vertical` | `true` \| `false` | `true` | List vertical scrollbar |
| `list_scrollbar_horizontal` | `true` \| `false` | `true` | List horizontal scrollbar |
| `border` | `true` \| `false` | `true` | Draw vertical rules between list columns (overridable per column) |
| `autosave` | `true` \| `false` | `true` | Write `config.toml` after a successful `:config set` |
| `autoreload` | `true` \| `false` | `true` | Reload when the config file changes on disk |
| `page_lines` | integer `≥ 0` | `0` | Page up/down step (`0` = viewport height) |
| `timestamp_format` | strftime string \| `"raw"` | `"%H:%M:%S"` | Timestamp column format (`"raw"` keeps the log string) |
| `case_mode` | `"sensitive"` \| `"insensitive"` \| `"smart"` | `"smart"` | Case matching for `/` search and `:filter` (`"smartcase"` accepted as an alias for `"smart"`) |
| `session_filters` | `true` \| `false` | `true` | Persist filters per log file |
| `session_stdin` | `true` \| `false` | `true` | Persist filters for stdin |

## `[details]`

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `wrap` | `true` \| `false` | `true` | Wrap text in the details overlay (`:config set wrap_details`) |
| `json_tree` | `true` \| `false` | `true` | Tree-view nested JSON in details (`:config set details_json_tree`) |
| `max_height` | integer `≥ 4` | `24` | Max details overlay height in rows (`:config set details_max_height`; relative `+N`/`-N` are layout/content-capped) |
| `tab_width` | integer `≥ 2` | `4` | Details tree indent width (`:config set details_tab_width`) |
| `scrollbar_vertical` | `true` \| `false` | `true` | Details vertical scrollbar (`:config set details_scrollbar_vertical`) |

## `[sidebar]`

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `enabled` | `true` \| `false` | `false` | Show filters/hidden sidebar (`:config set sidebar`) |
| `width` | integer `≥ 12` | `28` | Preferred sidebar width in columns (`:config set sidebar_width`; relative adjusts show the sidebar) |
| `position` | `"left"` \| `"right"` | `"right"` | Place the filters sidebar on the left or right of the main pane (`:config set sidebar_position`) |
| `scrollbar_vertical` | `true` \| `false` | `true` | Sidebar vertical scrollbar (`:config set sidebar_scrollbar_vertical`) |
| `scrollbar_horizontal` | `true` \| `false` | `true` | Sidebar horizontal scrollbar (`:config set sidebar_scrollbar_horizontal`) |

## `[mouse]`

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `enabled` | `true` \| `false` | `true` | Enable terminal mouse capture (`:config set mouse`) |
| `scroll_lines` | integer `≥ 1` | `1` | Mouse wheel step in lines |
| `scroll_moves_selection` | `true` \| `false` | `true` | Wheel moves selection (`false` scrolls the viewport only) |

## `[theme]`

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `name` | theme name | `"catppuccin"` | Color theme (bundled names or `~/.config/teleminator/themes/<name>.toml`) |

## `[colors]`

Optional patches over the selected theme. Omit a key to keep the theme default.

Text colors accept a hex string (`"#cdd6f4"`, fg only) or `{ fg = "...", bg = "..." }`. Surface keys are plain color strings.

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `background` | color string | theme | Main background |
| `foreground` | color / `{ fg, bg }` | theme | Default text |
| `selection_bg` | color string | theme | Selection background |
| `selection_fg` | color string | theme | Selection foreground |
| `overlay_bg` | color string | theme | Details / modal overlay background |
| `status_bg` | color string | theme | Status line background |
| `status_fg` | color string | theme | Status line foreground |
| `border` | color / `{ fg, bg }` | theme | Unfocused pane border |
| `window_focus_border` | color / `{ fg, bg }` | theme | Focused pane border |
| `search_match` | color / `{ fg, bg }` | theme | Search match highlight |
| `dim` | color / `{ fg, bg }` | theme | Dimmed text |

## `[levels]`

Optional per-level colors. Each accepts a hex string or `{ fg = "...", bg = "..." }`. Omit a key to keep the theme default.

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `trace` | color / `{ fg, bg }` | theme | Trace level |
| `debug` | color / `{ fg, bg }` | theme | Debug level |
| `info` | color / `{ fg, bg }` | theme | Info level |
| `warn` | color / `{ fg, bg }` | theme | Warn level |
| `error` | color / `{ fg, bg }` | theme | Error level |
| `fatal` | color / `{ fg, bg }` | theme | Fatal level |
| `unknown` | color / `{ fg, bg }` | theme | Unknown / missing level |

## `[ui]`

Optional UI / syntax colors and list column-separator chrome. Color keys accept a hex string or `{ fg, bg }`. Omit a key to keep the theme default.

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `timestamp` | color / `{ fg, bg }` | theme | Timestamp column / field color |
| `key` | color / `{ fg, bg }` | theme | JSON / field key color |
| `string` | color / `{ fg, bg }` | theme | String value color |
| `number` | color / `{ fg, bg }` | theme | Number value color |
| `bool` | color / `{ fg, bg }` | theme | Boolean value color |
| `null` | color / `{ fg, bg }` | theme | Null value color |
| `border_color` | color / `{ fg, bg }` | theme | List column separator color |
| `border_width` | integer | theme (`1`) | Separator width (`0` = space; `N` draws `N`× `│`) |
| `border_padding` | integer \| `{ left, right }` | theme (`1`) | Spaces around the separator |

## `[[columns]]`

Array of list columns. Defaults when omitted: `level` (width `5`, center, padding `1`), `timestamp`, `message`.

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `source` | builtin or field path | (required) | Builtin: `level`, `timestamp`, `message`, `raw`, `line`, `format`; or a field path like `annotations.url` |
| `width` | integer | unset (auto) | Fixed width / truncate; omit to size to the widest value in the viewport |
| `align` | `"left"` \| `"center"` \| `"right"` | `"left"` | Cell alignment |
| `padding` | integer \| `{ left, right }` | `0` | Spaces around the cell (`1` = both sides) |
| `border` | `true` \| `false` | inherit `[main].border` | Leading separator before this column |
| `border_color` | color / `{ fg, bg }` | inherit `[ui].border_color` | Leading separator color |
| `border_width` | integer | inherit `[ui].border_width` | Leading separator width |
| `border_padding` | integer \| `{ left, right }` | inherit `[ui].border_padding` | Leading separator padding |

## `[keys]`

Key → command map, merged over built-in defaults. Use `key = ""` to unbind. Chain commands with `;`.

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| *(key chord)* | command string \| `""` | built-in bindings | Override or unbind a key. Chords: `enter`, `esc`, `up`/`down`/`left`/`right`, `home`, `end`, `pagedown`, `pageup`, `space`, `backspace`, `S-backspace`, `C-c`, `D-…` (Super/Command), combinable as `A-S-left` |

### `[keys.details]`

Same shape as `[keys]`. Overrides `[keys]` while the details overlay is focused. An empty binding blocks fallback to `[keys]` for that chord.

### `[keys.sidebar]`

Same shape as `[keys]`. Overrides `[keys]` while the filters sidebar is focused. An empty binding blocks fallback to `[keys]` for that chord.

### `[keys.spans]`

Same shape as `[keys]`. Overrides `[keys]` while the Spans tab tree is focused. An empty binding blocks fallback to `[keys]` for that chord.
