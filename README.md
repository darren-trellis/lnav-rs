# lnav-rs

A modern log file navigator — a focused Rust + [ratatui](https://ratatui.rs) rewrite of [lnav](https://lnav.org) essentials.

Think neovim to vim: same job, cleaner defaults, friendlier UX.

## Status

Early MVP. Supported today:

- Open a **single log file**
- Parse **JSON** (JSONL) and **logfmt** lines
- **Live tail** as the file grows
- Level-aware **coloring** + built-in **themes**
- **Enter** to open a field details overlay
- Basic navigation + search

## Install

```bash
cargo install --path .
```

## Usage

```bash
lnav-rs examples/sample.jsonl
lnav-rs examples/sample.logfmt --theme nord
lnav-rs --list-themes
```

### Keys

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `PgDn` / `Space` | Page down |
| `PgUp` | Page up |
| `g` / `Home` | Jump to start |
| `G` / `End` | Jump to end (follow) |
| `Enter` | Toggle details overlay |
| `Esc` | Close overlay |
| `/` | Search |
| `n` / `N` | Next / previous match |
| `f` | Toggle follow |
| `t` | Cycle theme |
| `?` | Show help in status bar |
| `q` | Quit |

## Themes

Built-in: `catppuccin` (default), `nord`, `tokyo-night`.

Theme files live in `themes/*.toml` and are embedded at build time.

## Project layout

```
src/
  app.rs          # state + keybindings
  model.rs        # log entries / fields
  parse/          # json + logfmt
  tail.rs         # file read + live watch
  theme.rs        # theme loader
  ui/             # ratatui views
themes/           # TOML themes
examples/         # sample logs
```

## Roadmap (not yet)

- Multiple files / time merge
- Filters and bookmarks
- SQL / query surface
- Custom format definitions
- Config file + user themes from disk
