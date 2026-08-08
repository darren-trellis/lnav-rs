# Filters and sidebar

`s` / `:view sidebar toggle` shows a list of filters, then manually hidden lines (`·N` with a preview). File settings live under `[sidebar]`: `position` (`left` or `right`, default `right`), `width` (default 28; clamped to leave room for the list), and `scrollbar_vertical` / `scrollbar_horizontal` (same idea as `[main]` list scrollbars). Set via `:config set sidebar.width`, `sidebar.position`, etc.

When focused, `j`/`k` move the selection, `h`/`l` (or ←/→ / Shift+wheel) scroll horizontally, `Space` toggles the selected filter on/off, `dd` / Backspace deletes a filter or unhides a hidden line, `DD` / Shift+Backspace permanently deletes from the file (selected hidden line, or every line matching the selected filter), and Enter reveals a hidden line and jumps to it. Operator ranges work the same as in the list (`dG`, `D5k`, `dj`, …). (`[keys.sidebar]` defaults: `space = "filter set toggle"`, `d = "filter delete"`, `backspace = "filter delete line"`, `D = "delete"`, `S-backspace = "delete line"`, `enter = "hide reveal"`, `h`/`left = "scroll left"`, `l`/`right = "scroll right"`, `esc = "view current off"`).

Enabled filters are marked with `*`; disabled ones are ignored. With `:filter off`, the title shows `filters (disabled)` and filter rows use the dim style. When both filters and hidden lines are present, the title prefers `filters (N) · hidden (M)` and abbreviates to `FN · HM` only if the full form does not fit. Esc cancels a pending operator (`d`/`D`/…) if one is active, otherwise hides the sidebar; Tab cycles focus without closing it.

## Sessions

Filters persist under `~/.local/share/teleminator/sessions/` (one file per log path hash; stdin uses `stdin.toml`). Controlled by `[persist] filters` (default on; `:config set persist.filters`).
