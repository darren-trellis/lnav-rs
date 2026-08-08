# Mouse

Controlled by `[mouse]` in config (`enabled`, `scroll_lines`, `scroll_moves_selection`). When `enabled` is off, terminal mouse capture is disabled.

| Action | Effect |
|--------|--------|
| Click a log line | Select it (completes a pending `d`/`D` operator) |
| Double-click a log line | Toggle details overlay |
| Scroll wheel | Move selection (or cycle completions / config picker) |
| Shift + scroll wheel | Scroll list / sidebar horizontally |
| Horizontal scroll | Scroll list / sidebar sideways (when the terminal sends it) |
| Click / drag scrollbar | Scroll the list, sidebar, or details |
| Click a completion | Highlight it |
| Double-click a completion | Insert it into the command line |
| Click details overlay | Focus it / move details cursor |
| Click filters sidebar | Focus it / select a filter or hidden line |
| Click status bar | Enter `:` command mode |
| Click a config picker row | Highlight / preview that value |
| Double-click a config picker row | Commit that value |
