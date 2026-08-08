# Details overlay

`Enter` (`view details on`) opens and focuses the overlay — the selection highlight moves into details (`j`/`k` move the cursor; Esc runs `view current off`, canceling a pending operator first, otherwise closing the overlay).

`Tab` / `:focus toggle` cycles focus across the list, details (if open), and the filters sidebar (if open). `Space` folds/unfolds the tree item under the cursor when details is focused (`:fold on|off|toggle`). `:help toggle` (when details focused) toggles keybinding hints on the overlay border. `c` / `:copy` copies the focused item’s value (strings without quotes; objects/arrays as pretty JSON).

With details focused, `/` searches inside the overlay (`n`/`N` cycle matches). Nested JSON fields render as a tree when `[view] details.json_tree` is on (`details.tab_width` sets indent per level). With `details.wrap` off, `h`/`l` (or ←/→) scroll horizontally and `details.scrollbar.horizontal` shows a bottom bar. Overlay height grows with content up to `details.max_height` (and screen space). If opening or resizing details would clip the selected log, the list scrolls just enough to keep that line visible (on the last list row, above details). Set via `:config set view.details.json_tree`, `view.details.tab_width`, `view.details.max_height`, etc.
