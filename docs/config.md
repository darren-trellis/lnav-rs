
## Columns

`[[columns]]` define the list layout. `source` is a builtin (`level`, `timestamp`, `message`, `raw`, `line`, `format`) or a field path (`annotations.url`, `items.0.id`). Columns without `width` auto-size to the widest value in the current viewport so fields share an X position. Set `width` to fix/truncate; `align` is `"left"` (default), `"center"`, or `"right"`. `padding` is spaces around the cell (`1` for both sides, or `{ left = 1, right = 2 }`). Optional `border` / `border_color` / `border_width` / `border_padding` control the leading rule before that column (`border` overrides `[main].border`; omit to inherit). The first column’s border is also used between line numbers and the list. When the full row is wider than the list pane, `h`/`l` (or ←/→) scroll all columns together horizontally.

## Themes and colors

`[theme]` selects the theme name. Optional `[colors]` / `[levels]` / `[ui]` patches at the config root use the same keys as `themes/*.toml` (not nested under `[theme]`).

Text colors (`foreground`, `border`, `window_focus_border`, `search_match`, `dim`, levels, and `[ui]` color keys) accept a hex string (fg only) or `{ fg = "...", bg = "..." }`. Surface keys (`background`, `overlay_bg`, `selection_*`, `status_*`) stay plain color strings.

Focused chrome: `window_focus_border` is the border of the focused pane (list or details); unfocused panes use `[colors].border`. List column separators: `[ui].border_color`, `[ui].border_width` (`0` = space between columns; `N` draws `N`× `│`), and `[ui].border_padding` (`1` or `{ left, right }`, like column `padding`), gated by `[main].border` unless a column sets `border = true|false`. The same rule is drawn between line numbers and the first column when line numbers are on.

Drop custom TOML themes in `~/.config/lnav-rs/themes/<name>.toml` (same shape as files in `themes/`), then `:config set theme <name>` or `:config set theme` for the picker.

Unknown keys, invalid colors, unknown theme names, and unknown keybinding commands are rejected.

## Keybindings

`[keys]` overrides defaults (merged). Use `key = ""` to unbind. Chain commands with `;` (e.g. `r = "view details on; focus toggle"`). Special key names: `enter`, `esc`, `up`, `down`, `left`, `right`, `home`, `end`, `pagedown`, `pageup`, `space`, `backspace`, `S-backspace`, `C-c`, `D-` (Super/Command), combinable as `A-S-left`.

- `[keys.details]` overrides `[keys]` while the details overlay is focused.
- `[keys.sidebar]` overrides `[keys]` while the filters sidebar is focused.
- An empty binding in either contextual section blocks fallback to the same key in `[keys]`.

Keybinding-only commands (`nav`, `page`, `scroll`, `match`, `focus`, `search`, `command`) are listed in the README Commands section; they are omitted from `:` completions.

Default resize chords (Alt/Option+Shift, encoded `A-S-…`) call `:config set` with relative values: `A-S-left`/`A-S-h` → `sidebar_width +1`, `A-S-right`/`A-S-l` → `sidebar_width -1`, `A-S-up`/`A-S-k` → `details_max_height +1`, `A-S-down`/`A-S-j` → `details_max_height -1`. Relative `details_max_height` adjusts are capped by content height and main viewport minus pinned rows minus 5; relative `sidebar_width` is capped to leave room for the list.

`?` / `:help` opens a cheatsheet modal (Esc / `q` / `?` closes; `j`/`k` and `h`/`l` scroll).
