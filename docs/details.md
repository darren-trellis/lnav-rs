# Details overlay

`Enter` (`view details on`) opens and focuses the overlay — the selection highlight moves into details (`j`/`k` move the cursor; Esc runs `view current off`, canceling a pending operator first, otherwise closing the overlay).

`Tab` / `:focus toggle` cycles focus across the list, details (if open), and the filters sidebar (if open). `Space` folds/unfolds the tree item under the cursor when details is focused (`:fold on|off|toggle`). `:help toggle` (when details focused) toggles keybinding hints on the overlay border. `c` / `:copy` copies the focused item’s value (strings without quotes; objects/arrays as pretty JSON).

With details focused, `/` searches inside the overlay (`n`/`N` cycle matches). Nested JSON fields render as a tree when `[details] json_tree` is on (`tab_width` sets indent per level). Overlay height grows with content up to `max_height` (and screen space). If opening or resizing details would clip the selected log, the list scrolls just enough to keep that line visible (on the last list row, above details). Set via `:config set details.json_tree`, `details.tab_width`, `details.max_height`, etc.
