# Filters and sidebar

`s` / `:view sidebar toggle` shows a right-hand list of filters, then manually hidden lines (`·N` with a preview). Width is `sidebar_width` (default 28; clamped to leave room for the list). Vertical/horizontal scrollbars are controlled by `sidebar_scrollbar_*` (same idea as `list_scrollbar_*`).

When focused, `j`/`k` move the selection, `h`/`l` (or ←/→ / Shift+wheel) scroll horizontally, `Space` toggles the selected filter on/off, `dd` / Backspace deletes a filter or unhides a hidden line, and Enter reveals a hidden line and jumps to it (`[keys.sidebar]` defaults: `space = "filter set toggle"`, `d = "filter delete"`, `backspace = "filter delete line"`, `enter = "hide reveal"`, `h`/`left = "scroll left"`, `l`/`right = "scroll right"`, `esc = "view current off"`).

Enabled filters are marked with `*`; disabled ones are ignored. With `:filter off`, the title shows `filters (disabled)` and filter rows use the dim style. When both filters and hidden lines are present, the title prefers `filters (N) · hidden (M)` and abbreviates to `FN · HM` only if the full form does not fit. Esc hides the sidebar; Tab cycles focus without closing it.

## Sessions

Filters persist under `~/.local/share/lnav-rs/sessions/` (one file per log path hash; stdin uses `stdin.toml`). `session_filters` / `session_stdin` control that (both default on). Turn `session_stdin` off if you don’t want every pipe to share one filter set.
