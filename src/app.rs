use std::collections::HashSet;
use std::io::stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use ratatui::layout::Rect;
use regex::Regex;

use crate::command;
use crate::completion::{self, CompletionState};
use crate::config::Config;
use crate::details;
use crate::filter::{self, Filter};
use crate::keys;
use crate::model::LogEntry;
use crate::object_span;
use crate::session::{self, Session};
use crate::tail::LogSource;
use crate::theme::Theme;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Command,
}

/// Vim-style operator waiting for a motion (`d` / `D`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOp {
    Hide,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldAction {
    On,
    Off,
    Toggle,
}

/// Interactive theme selector opened by `:theme set`.
#[derive(Debug, Clone)]
pub struct ThemePicker {
    pub names: Vec<String>,
    pub selected: usize,
    /// Committed theme name to restore on cancel.
    pub previous_name: String,
    /// Full popup rect from the last draw.
    pub popup_area: Rect,
    /// List inner area from the last draw (for mouse hit-testing).
    pub list_area: Rect,
    /// First visible name index in `list_area`.
    pub list_start: usize,
}

/// Widget rects from the last frame, used for mouse hit-testing.
#[derive(Debug, Default, Clone, Copy)]
pub struct HitAreas {
    pub list_inner: Rect,
    pub overlay: Rect,
    pub suggest_inner: Rect,
    pub suggest_start: usize,
    pub status: Rect,
}

#[derive(Debug, Clone, Copy)]
struct LastClick {
    at: Instant,
    vis_idx: usize,
}

pub struct App {
    pub source: LogSource,
    pub config: Config,
    pub theme: Theme,
    pub theme_index: usize,
    pub filters: Vec<Filter>,
    pub filtering_enabled: bool,
    /// Source indices hidden with `d` (session-only).
    pub hidden: HashSet<usize>,
    /// Source indices currently visible after filters.
    pub visible: Vec<usize>,
    /// Index into `visible`.
    pub selected: usize,
    pub scroll: usize,
    pub follow: bool,
    pub show_overlay: bool,
    /// When true, navigation keys scroll the details overlay instead of the list.
    pub overlay_focused: bool,
    /// Selected row inside the details overlay (when focused).
    pub overlay_cursor: usize,
    pub overlay_scroll: usize,
    pub overlay_content_len: usize,
    pub overlay_inner_height: usize,
    /// Folded JSON tree paths (`details::path_key`).
    pub overlay_folded: HashSet<String>,
    pub input_mode: InputMode,
    pub pending_op: Option<PendingOp>,
    /// Visible-row anchor when the pending operator was started.
    pub op_anchor: usize,
    /// Vim-style count prefix being typed (`5` in `5j`).
    pub count: Option<usize>,
    pub theme_picker: Option<ThemePicker>,
    pub hit: HitAreas,
    last_click: Option<LastClick>,
    pub search_query: String,
    /// Compiled from `search_query` (case-insensitive). `None` if empty/invalid.
    pub search_regex: Option<Regex>,
    pub search_error: Option<String>,
    pub command_buffer: String,
    pub completions: CompletionState,
    pub search_matches: Vec<usize>,
    pub search_cursor: Option<usize>,
    /// When true, `/` search targets the focused details overlay.
    pub search_in_overlay: bool,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(source: LogSource, config: Config) -> Result<Self> {
        let overrides = config.theme.overrides();
        let theme = Theme::resolve_with_overrides(config.theme.name(), &overrides)?;
        let names = Theme::list_names();
        let theme_index = names
            .iter()
            .position(|t| t == config.theme.name() || t == &theme.name)
            .unwrap_or(0);

        let mut status_message = None;
        let (filters, filtering_enabled) = match session::load(&source, &config) {
            Ok(Some(s)) => match s.into_filters(config.case_mode) {
                Ok(pair) => pair,
                Err(err) => {
                    status_message = Some(format!("session: {err:#}"));
                    (Vec::new(), true)
                }
            },
            Ok(None) => (Vec::new(), true),
            Err(err) => {
                status_message = Some(format!("session: {err:#}"));
                (Vec::new(), true)
            }
        };

        let mut app = Self {
            source,
            follow: config.follow,
            config,
            theme,
            theme_index,
            filters,
            filtering_enabled,
            hidden: HashSet::new(),
            visible: Vec::new(),
            selected: 0,
            scroll: 0,
            show_overlay: false,
            overlay_focused: false,
            overlay_cursor: 0,
            overlay_scroll: 0,
            overlay_content_len: 0,
            overlay_inner_height: 0,
            overlay_folded: HashSet::new(),
            input_mode: InputMode::Normal,
            pending_op: None,
            op_anchor: 0,
            count: None,
            theme_picker: None,
            hit: HitAreas::default(),
            last_click: None,
            search_query: String::new(),
            search_regex: None,
            search_error: None,
            command_buffer: String::new(),
            completions: CompletionState::default(),
            search_matches: Vec::new(),
            search_cursor: None,
            search_in_overlay: false,
            status_message,
            should_quit: false,
        };
        app.rebuild_visible(None);
        if app.follow && !app.visible.is_empty() {
            app.selected = app.visible.len() - 1;
        }
        Ok(app)
    }

    pub fn persist_session(&self) {
        let session = Session::from_app(&self.filters, self.filtering_enabled);
        let _ = session::save(&self.source, &self.config, &session);
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        let src = *self.visible.get(self.selected)?;
        self.source.entries().get(src)
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn hidden_count(&self) -> usize {
        self.source.len().saturating_sub(self.visible.len())
    }

    pub fn jump_to_public(&mut self, visible_idx: usize) {
        self.jump_to(visible_idx);
    }

    pub fn rebuild_visible(&mut self, prefer_source_idx: Option<usize>) {
        let prefer = prefer_source_idx.or_else(|| {
            self.visible
                .get(self.selected)
                .copied()
        });
        self.visible = filter::build_visible(
            self.source.entries(),
            &self.filters,
            self.filtering_enabled,
            &self.hidden,
        );

        if self.visible.is_empty() {
            self.selected = 0;
            self.scroll = 0;
            return;
        }

        if let Some(src) = prefer {
            if let Some(pos) = self.visible.iter().position(|&i| i == src) {
                self.selected = pos;
                return;
            }
        }
        self.selected = self.selected.min(self.visible.len() - 1);
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        let _ = execute!(stdout(), EnableMouseCapture);
        let result = self.run_loop(terminal);
        let _ = execute!(stdout(), DisableMouseCapture);
        self.close_theme_picker(false);
        result
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        loop {
            let added = self.source.refresh().unwrap_or(0);
            if added > 0 {
                let prefer = self.visible.get(self.selected).copied();
                self.rebuild_visible(prefer);
                if self.follow && !self.visible.is_empty() {
                    self.selected = self.visible.len() - 1;
                }
                if !self.search_query.is_empty() {
                    self.run_search();
                }
            }

            terminal.draw(|frame| ui::draw(frame, self))?;

            if self.should_quit {
                self.persist_session();
                break;
            }

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key(key);
                    }
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.theme_picker.is_some() {
            self.handle_theme_picker_mouse(mouse);
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.cancel_pending_op();
                if self.show_overlay
                    && (self.overlay_focused
                        || contains(self.hit.overlay, mouse.column, mouse.row))
                {
                    self.scroll_overlay(-(self.config.scroll_lines.max(1) as isize));
                } else if self.input_mode == InputMode::Command && !self.completions.items.is_empty()
                {
                    self.completions.select_prev();
                } else {
                    let n = self.config.scroll_lines.max(1) as isize;
                    self.with_motion(|a| a.move_selection(-n));
                }
            }
            MouseEventKind::ScrollDown => {
                self.cancel_pending_op();
                if self.show_overlay
                    && (self.overlay_focused
                        || contains(self.hit.overlay, mouse.column, mouse.row))
                {
                    self.scroll_overlay(self.config.scroll_lines.max(1) as isize);
                } else if self.input_mode == InputMode::Command && !self.completions.items.is_empty()
                {
                    self.completions.select_next();
                } else {
                    let n = self.config.scroll_lines.max(1) as isize;
                    self.with_motion(|a| a.move_selection(n));
                }
            }
            MouseEventKind::Down(MouseButton::Left) => self.handle_left_click(mouse.column, mouse.row),
            MouseEventKind::Moved
                if self.input_mode == InputMode::Command
                    && contains(self.hit.suggest_inner, mouse.column, mouse.row) =>
            {
                if let Some(idx) = self.suggest_index_at(mouse.column, mouse.row) {
                    self.completions.selected = idx;
                }
            }
            _ => {}
        }
    }

    fn handle_left_click(&mut self, col: u16, row: u16) {
        if contains(self.hit.suggest_inner, col, row) {
            if let Some(idx) = self.suggest_index_at(col, row) {
                self.completions.selected = idx;
                completion::apply_selected(self);
            }
            return;
        }

        if contains(self.hit.overlay, col, row) {
            self.overlay_focused = true;
            // Click a row inside the overlay to move the cursor there.
            let inner_y = self.hit.overlay.y.saturating_add(1);
            if row >= inner_y {
                let row_off = (row - inner_y) as usize;
                let idx = self.overlay_scroll + row_off;
                if idx < self.overlay_content_len {
                    self.jump_overlay_cursor(idx);
                }
            }
            self.status_message =
                Some("details focused · j/k move · Tab fold · Esc close".into());
            return;
        }

        if contains(self.hit.list_inner, col, row) {
            self.click_list_row(col, row);
            return;
        }

        if contains(self.hit.status, col, row) {
            match self.input_mode {
                InputMode::Search | InputMode::Command => {}
                InputMode::Normal => {
                    self.input_mode = InputMode::Command;
                    self.command_buffer.clear();
                    self.status_message = None;
                    completion::refresh(self);
                }
            }
        }
    }

    fn click_list_row(&mut self, _col: u16, row: u16) {
        let area = self.hit.list_inner;
        if area.height == 0 || self.visible.is_empty() {
            return;
        }
        let row_off = (row - area.y) as usize;
        let vis_idx = self.scroll + row_off;
        if vis_idx >= self.visible.len() {
            return;
        }

        // Leave search/command editing when interacting with the list.
        if self.input_mode != InputMode::Normal {
            self.input_mode = InputMode::Normal;
            self.command_buffer.clear();
            self.completions.clear();
        }
        self.overlay_focused = false;

        let double = self
            .last_click
            .map(|c| {
                c.vis_idx == vis_idx && c.at.elapsed() < Duration::from_millis(400)
            })
            .unwrap_or(false);

        if let Some(op) = self.pending_op.take() {
            let start = self.op_anchor;
            self.follow = false;
            self.selected = vis_idx;
            self.apply_op_visible_range(op, start, vis_idx);
            self.last_click = Some(LastClick {
                at: Instant::now(),
                vis_idx,
            });
            return;
        }

        self.follow = false;
        self.selected = vis_idx;
        if double {
            self.toggle_details();
            self.last_click = None;
        } else {
            self.last_click = Some(LastClick {
                at: Instant::now(),
                vis_idx,
            });
        }
    }

    fn suggest_index_at(&self, col: u16, row: u16) -> Option<usize> {
        let area = self.hit.suggest_inner;
        if !contains(area, col, row) || area.height == 0 {
            return None;
        }
        let idx = self.hit.suggest_start + (row - area.y) as usize;
        if idx < self.completions.items.len() {
            Some(idx)
        } else {
            None
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.theme_picker.is_some() {
            self.handle_theme_picker_key(key);
            return;
        }
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::Command => self.handle_command_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_theme_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.close_theme_picker(false),
            KeyCode::Enter => self.close_theme_picker(true),
            KeyCode::Up | KeyCode::Char('k') => self.theme_picker_move(-1),
            KeyCode::Down | KeyCode::Char('j') => self.theme_picker_move(1),
            KeyCode::Home | KeyCode::Char('g') => self.theme_picker_select(0),
            KeyCode::End | KeyCode::Char('G') => {
                if let Some(picker) = &self.theme_picker {
                    let last = picker.names.len().saturating_sub(1);
                    self.theme_picker_select(last);
                }
            }
            _ => {}
        }
    }

    fn handle_theme_picker_mouse(&mut self, mouse: MouseEvent) {
        let Some(picker) = &self.theme_picker else {
            return;
        };
        let popup = picker.popup_area;
        let area = picker.list_area;
        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::ScrollUp => self.theme_picker_move(-1),
            MouseEventKind::ScrollDown => self.theme_picker_move(1),
            MouseEventKind::Moved => {
                if contains(area, col, row) {
                    let idx = picker.list_start + (row - area.y) as usize;
                    if idx < picker.names.len() {
                        self.theme_picker_select(idx);
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if contains(area, col, row) {
                    let idx = picker.list_start + (row - area.y) as usize;
                    if idx < picker.names.len() {
                        self.theme_picker_select(idx);
                        self.close_theme_picker(true);
                    }
                } else if !contains(popup, col, row) {
                    self.close_theme_picker(false);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn open_theme_picker(&mut self) {
        self.cancel_pending_op();
        let names = Theme::list_names();
        if names.is_empty() {
            self.status_message = Some("no themes available".into());
            return;
        }
        let previous_name = self.config.theme.name().to_string();
        let selected = names
            .iter()
            .position(|n| n == &previous_name || n == &self.theme.name)
            .unwrap_or(0);
        self.theme_picker = Some(ThemePicker {
            names,
            selected,
            previous_name,
            popup_area: Rect::default(),
            list_area: Rect::default(),
            list_start: 0,
        });
        self.input_mode = InputMode::Normal;
        self.command_buffer.clear();
        self.completions.clear();
        self.preview_theme_at_selection();
        self.status_message = Some("theme picker · click/Enter to set · Esc to cancel".into());
    }

    fn theme_picker_move(&mut self, delta: isize) {
        let Some(picker) = &self.theme_picker else {
            return;
        };
        let len = picker.names.len() as isize;
        if len == 0 {
            return;
        }
        let next = (picker.selected as isize + delta).rem_euclid(len) as usize;
        self.theme_picker_select(next);
    }

    fn theme_picker_select(&mut self, idx: usize) {
        {
            let Some(picker) = &mut self.theme_picker else {
                return;
            };
            if idx >= picker.names.len() || idx == picker.selected {
                return;
            }
            picker.selected = idx;
        }
        self.preview_theme_at_selection();
    }

    fn preview_theme_at_selection(&mut self) {
        let Some(name) = self
            .theme_picker
            .as_ref()
            .and_then(|p| p.names.get(p.selected).cloned())
        else {
            return;
        };
        let overrides = self.config.theme.overrides();
        if let Ok(theme) = Theme::resolve_with_overrides(&name, &overrides) {
            self.theme = theme;
            self.status_message = Some(format!("preview: {name}"));
        }
    }

    /// Confirm (`true`) keeps the previewed theme; cancel restores the previous one.
    pub(crate) fn close_theme_picker(&mut self, confirm: bool) {
        let Some(picker) = self.theme_picker.take() else {
            return;
        };
        if confirm {
            if let Some(name) = picker.names.get(picker.selected) {
                self.commit_theme(name);
                return;
            }
        }
        let overrides = self.config.theme.overrides();
        if let Ok(theme) =
            Theme::resolve_with_overrides(&picker.previous_name, &overrides)
        {
            self.theme = theme;
            self.theme_index = Theme::list_names()
                .iter()
                .position(|t| t == &picker.previous_name)
                .unwrap_or(self.theme_index);
        }
        self.status_message = Some(format!("theme: {}", self.config.theme.name()));
    }

    pub(crate) fn commit_theme(&mut self, name: &str) {
        let overrides = self.config.theme.overrides();
        match Theme::resolve_with_overrides(name, &overrides) {
            Ok(theme) => {
                self.config.theme.set_name(name);
                self.theme_index = Theme::list_names()
                    .iter()
                    .position(|t| t == name || t == &theme.name)
                    .unwrap_or(self.theme_index);
                self.status_message = Some(format!("theme: {}", theme.name));
                self.theme = theme;
            }
            Err(err) => self.status_message = Some(format!("error: {err:#}")),
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status_message = None;
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                if let Some(err) = self.search_error.clone() {
                    self.status_message = Some(err);
                } else if self.search_matches.is_empty() {
                    self.status_message = if self.search_query.is_empty() {
                        None
                    } else {
                        Some("no matches".into())
                    };
                } else {
                    let n = self.search_matches.len();
                    let cur = self.search_cursor.unwrap_or(0).min(n - 1) + 1;
                    self.status_message = Some(if self.search_in_overlay {
                        format!("{cur}/{n} in details")
                    } else {
                        format!("{cur}/{n} matches")
                    });
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.refresh_search_live();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.refresh_search_live();
            }
            _ => {}
        }
    }

    /// Update match highlights as the query changes (incremental search).
    fn refresh_search_live(&mut self) {
        self.run_search();
        if self.search_matches.is_empty() {
            self.search_cursor = None;
            return;
        }
        let cursor = if self.search_in_overlay {
            self.search_matches
                .iter()
                .position(|&m| m >= self.overlay_scroll)
                .unwrap_or(0)
        } else {
            self.search_matches
                .iter()
                .position(|&m| m >= self.selected)
                .unwrap_or(0)
        };
        self.search_cursor = Some(cursor);
        self.follow = false;
        if self.search_in_overlay {
            self.overlay_cursor = self.search_matches[cursor];
            self.ensure_overlay_cursor_visible();
        } else {
            self.jump_to(self.search_matches[cursor]);
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.command_buffer.clear();
                self.completions.clear();
                self.status_message = None;
            }
            KeyCode::Enter => {
                let cmd = std::mem::take(&mut self.command_buffer);
                self.completions.clear();
                self.input_mode = InputMode::Normal;
                command::execute(self, &cmd);
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.completions.select_prev();
                    completion::apply_selected(self);
                } else {
                    completion::tab_complete(self);
                }
            }
            KeyCode::BackTab => {
                self.completions.select_prev();
                completion::apply_selected(self);
            }
            KeyCode::Down => {
                self.completions.select_next();
            }
            KeyCode::Up => {
                self.completions.select_prev();
            }
            KeyCode::Backspace => {
                if self.command_buffer.is_empty() {
                    self.input_mode = InputMode::Normal;
                    self.completions.clear();
                    self.status_message = None;
                } else {
                    self.command_buffer.pop();
                    completion::refresh(self);
                }
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                completion::refresh(self);
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.overlay_focused && self.show_overlay && self.handle_overlay_key(key) {
            return;
        }

        // Vim-style count prefix: `5j`, `3dd`, `10G`, …
        if key.modifiers.is_empty()
            && let KeyCode::Char(c) = key.code
            && c.is_ascii_digit()
            && (c != '0' || self.count.is_some())
        {
            let digit = (c as u8 - b'0') as usize;
            self.count = Some(self.count.unwrap_or(0).saturating_mul(10).saturating_add(digit));
            return;
        }

        let Some(spec) = keys::encode(key) else {
            return;
        };
        let Some(cmd) = self.config.keys.get(&spec).cloned() else {
            // Unbound key clears a half-typed count.
            self.count = None;
            return;
        };
        if cmd.is_empty() {
            self.count = None;
            return;
        }
        command::execute_from_key(self, &cmd);
    }

    /// Handle keys while the details overlay is focused. Returns true if consumed.
    /// Navigation is handled via normal keybindings (`down`/`up`/…) so only
    /// Esc needs a hard intercept here; other keys fall through.
    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.close_details();
                true
            }
            // Leave focus so : / q use the list context, except / which searches details.
            KeyCode::Char(':') | KeyCode::Char('q') => {
                self.overlay_focused = false;
                false
            }
            _ => false,
        }
    }

    pub fn toggle_details(&mut self) {
        self.cancel_pending_op();
        if !self.show_overlay {
            if self.selected_entry().is_none() {
                return;
            }
            self.show_overlay = true;
            self.overlay_focused = true;
            self.overlay_cursor = 0;
            self.overlay_scroll = 0;
            self.status_message =
                Some("details focused · j/k move · Tab fold · Esc close".into());
        } else if !self.overlay_focused {
            self.overlay_focused = true;
            self.status_message =
                Some("details focused · j/k move · Tab fold · Esc close".into());
        } else {
            self.close_details();
        }
    }

    pub fn close_details(&mut self) {
        self.show_overlay = false;
        self.overlay_focused = false;
        self.overlay_cursor = 0;
        self.overlay_scroll = 0;
        if self.search_in_overlay {
            self.search_in_overlay = false;
            if !self.search_query.is_empty() {
                self.run_search();
            }
        }
    }

    pub fn move_overlay_cursor(&mut self, delta: isize) {
        if !self.show_overlay || self.overlay_content_len == 0 {
            return;
        }
        let max = self.overlay_content_len - 1;
        let next = (self.overlay_cursor as isize + delta).clamp(0, max as isize) as usize;
        self.overlay_cursor = next;
        self.ensure_overlay_cursor_visible();
    }

    pub fn jump_overlay_cursor(&mut self, idx: usize) {
        if self.overlay_content_len == 0 {
            self.overlay_cursor = 0;
            return;
        }
        self.overlay_cursor = idx.min(self.overlay_content_len - 1);
        self.ensure_overlay_cursor_visible();
    }

    pub fn ensure_overlay_cursor_visible(&mut self) {
        let view = self.overlay_inner_height.max(1);
        let idx = self.overlay_cursor;
        if idx < self.overlay_scroll {
            self.overlay_scroll = idx;
        } else if idx >= self.overlay_scroll + view {
            self.overlay_scroll = idx + 1 - view;
        }
    }

    pub fn scroll_overlay(&mut self, delta: isize) {
        // Mouse wheel: move the details cursor (same as j/k).
        self.move_overlay_cursor(delta);
    }

    pub fn copy_overlay_value(&mut self) {
        if !self.show_overlay || !self.overlay_focused {
            self.status_message = Some("focus details first (Enter)".into());
            return;
        }
        let cursor = self.overlay_cursor;
        let value = {
            let Some(entry) = self.selected_entry() else {
                return;
            };
            let lines =
                details::build_lines(entry, &self.theme, &self.config, &self.overlay_folded);
            lines.get(cursor).and_then(|l| l.copy_value.clone())
        };
        let Some(value) = value else {
            self.status_message = Some("nothing to copy".into());
            return;
        };
        match copy_to_clipboard(&value) {
            Ok(()) => {
                let preview = if value.len() > 60 {
                    format!("{}…", &value[..57])
                } else {
                    value
                };
                self.status_message = Some(format!("copied {preview}"));
            }
            Err(err) => {
                self.status_message = Some(format!("copy failed: {err:#}"));
            }
        }
    }

    pub fn set_overlay_fold(&mut self, action: FoldAction) {
        if !self.show_overlay || !self.overlay_focused {
            self.status_message = Some("focus details first (Enter)".into());
            return;
        }
        let cursor = self.overlay_cursor;
        let (foldable, path) = {
            let Some(entry) = self.selected_entry() else {
                return;
            };
            let lines =
                details::build_lines(entry, &self.theme, &self.config, &self.overlay_folded);
            let Some(line) = lines.get(cursor) else {
                return;
            };
            (line.foldable, line.path.clone())
        };
        if !foldable || path.is_empty() {
            self.status_message = Some("not a foldable tree item".into());
            return;
        }
        let key = details::path_key(&path);
        let label = path.join(".");
        let currently_folded = self.overlay_folded.contains(&key);
        let fold = match action {
            FoldAction::On => true,
            FoldAction::Off => false,
            FoldAction::Toggle => !currently_folded,
        };
        if fold {
            self.overlay_folded.insert(key);
            self.status_message = Some(format!("folded {label}"));
        } else {
            self.overlay_folded.remove(&key);
            self.status_message = Some(format!("unfolded {label}"));
        }
        let new_len = self
            .selected_entry()
            .map(|entry| {
                details::build_lines(entry, &self.theme, &self.config, &self.overlay_folded).len()
            })
            .unwrap_or(0);
        self.overlay_content_len = new_len;
        if new_len == 0 {
            self.overlay_cursor = 0;
        } else {
            self.overlay_cursor = self.overlay_cursor.min(new_len - 1);
        }
        self.ensure_overlay_cursor_visible();
        if self.search_in_overlay && !self.search_query.is_empty() {
            self.run_search();
        }
    }

    fn reset_overlay_for_selection_change(&mut self) {
        self.overlay_cursor = 0;
        self.overlay_scroll = 0;
        self.overlay_folded.clear();
    }

    /// Consume the typed count prefix, or `1` if none.
    pub(crate) fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1).max(1)
    }

    /// Consume the typed count prefix if present.
    pub(crate) fn take_count_opt(&mut self) -> Option<usize> {
        self.count.take().filter(|&n| n > 0)
    }

    pub(crate) fn cancel_pending_op(&mut self) {
        self.count = None;
        if self.pending_op.take().is_some() {
            self.status_message = None;
        }
    }

    /// Start a vim-style operator, or complete `dd` / `DD` if the same op is pending.
    pub(crate) fn start_or_repeat_op(&mut self, op: PendingOp) {
        if self.visible.is_empty() {
            self.pending_op = None;
            self.count = None;
            return;
        }
        if self.pending_op == Some(op) {
            // dd / DD — optional count: `5dd` affects 5 lines.
            let n = self.take_count();
            let at = self.selected;
            let end = (at + n - 1).min(self.visible.len().saturating_sub(1));
            self.pending_op = None;
            self.apply_op_visible_range(op, at, end);
            return;
        }
        // Keep count for the following motion (`5dj`) or second `d` (`5dd`).
        self.pending_op = Some(op);
        self.op_anchor = self.selected;
        self.status_message = None;
    }

    /// Run a motion; if an operator is pending, apply it to the anchor…cursor range.
    pub(crate) fn with_motion<F>(&mut self, motion: F)
    where
        F: FnOnce(&mut Self),
    {
        if let Some(op) = self.pending_op {
            let start = self.op_anchor;
            motion(self);
            let end = self.selected;
            self.pending_op = None;
            self.count = None;
            self.apply_op_visible_range(op, start, end);
        } else {
            motion(self);
        }
    }

    pub(crate) fn apply_op_visible_range(&mut self, op: PendingOp, from: usize, to: usize) {
        if self.visible.is_empty() {
            return;
        }
        let lo = from.min(to).min(self.visible.len() - 1);
        let hi = from.max(to).min(self.visible.len() - 1);
        let mut indices = HashSet::new();
        for vis in lo..=hi {
            let Some(&src) = self.visible.get(vis) else {
                continue;
            };
            for i in object_span::object_span(self.source.entries(), src) {
                indices.insert(i);
            }
        }
        let mut indices: Vec<usize> = indices.into_iter().collect();
        indices.sort_unstable();
        if indices.is_empty() {
            return;
        }

        self.follow = false;
        match op {
            PendingOp::Hide => {
                let n = indices.len();
                for i in indices {
                    self.hidden.insert(i);
                }
                self.rebuild_visible(None);
                self.selected = if self.visible.is_empty() {
                    0
                } else {
                    lo.min(self.visible.len() - 1)
                };
                if self.visible.is_empty() {
                    self.close_details();
                } else {
                    self.reset_overlay_for_selection_change();
                }
                if !self.search_query.is_empty() {
                    self.run_search();
                }
                self.status_message = Some(format!(
                    "hidden {n} line{}  (:clear-hidden to restore)",
                    if n == 1 { "" } else { "s" }
                ));
            }
            PendingOp::Delete => {
                if !self.source.is_file() {
                    self.status_message = Some("cannot delete from stdin".into());
                    return;
                }
                match self.source.delete_entries(&indices) {
                    Ok(removed) => {
                        self.hidden.clear();
                        self.rebuild_visible(None);
                        self.selected = if self.visible.is_empty() {
                            0
                        } else {
                            lo.min(self.visible.len() - 1)
                        };
                        if self.visible.is_empty() {
                            self.close_details();
                        } else {
                            self.reset_overlay_for_selection_change();
                        }
                        if !self.search_query.is_empty() {
                            self.run_search();
                        }
                        self.status_message = Some(format!(
                            "deleted {removed} line{} from {}",
                            if removed == 1 { "" } else { "s" },
                            self.source
                                .path()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default()
                        ));
                    }
                    Err(err) => {
                        self.status_message = Some(format!("delete failed: {err:#}"));
                    }
                }
            }
        }
    }

    pub(crate) fn hide_current(&mut self) {
        let at = self.selected;
        self.apply_op_visible_range(PendingOp::Hide, at, at);
    }

    pub(crate) fn delete_current(&mut self) {
        let at = self.selected;
        self.apply_op_visible_range(PendingOp::Delete, at, at);
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        self.follow = false;
        let next = (self.selected as isize + delta)
            .clamp(0, self.visible.len() as isize - 1) as usize;
        if next != self.selected {
            self.reset_overlay_for_selection_change();
        }
        self.selected = next;
    }

    fn jump_to(&mut self, idx: usize) {
        if self.visible.is_empty() {
            return;
        }
        let next = idx.min(self.visible.len() - 1);
        if next != self.selected {
            self.reset_overlay_for_selection_change();
        }
        self.selected = next;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_regex = None;
        self.search_error = None;
        self.search_matches.clear();
        self.search_cursor = None;
        self.search_in_overlay = false;
    }

    /// Recompile filters/search after `case_mode` changes. Returns an error message if a filter fails.
    pub fn apply_case_mode(&mut self) -> Option<String> {
        let mode = self.config.case_mode;
        for f in &mut self.filters {
            if let Err(err) = f.recompile(mode) {
                return Some(format!("filter /{}/: {err}", f.pattern));
            }
        }
        self.rebuild_visible(None);
        if !self.search_query.is_empty() {
            self.run_search();
        }
        None
    }

    pub fn run_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_regex = None;
            self.search_error = None;
            self.search_matches.clear();
            self.search_cursor = None;
            return;
        }
        let regex = match filter::compile_regex(&self.search_query, self.config.case_mode) {
            Ok(re) => re,
            Err(err) => {
                self.search_regex = None;
                self.search_error = Some(format!("invalid regex: {err}"));
                self.search_matches.clear();
                self.search_cursor = None;
                return;
            }
        };
        self.search_error = None;
        if self.search_in_overlay {
            self.search_matches = if let Some(entry) = self.selected_entry() {
                details::build_lines(entry, &self.theme, &self.config, &self.overlay_folded)
                    .into_iter()
                    .enumerate()
                    .filter(|(_, line)| regex.is_match(&line.plain_text()))
                    .map(|(i, _)| i)
                    .collect()
            } else {
                Vec::new()
            };
        } else {
            self.search_matches = self
                .visible
                .iter()
                .enumerate()
                .filter(|&(_, src)| regex.is_match(&self.source.entries()[*src].raw))
                .map(|(vis, _)| vis)
                .collect();
        }
        self.search_regex = Some(regex);
    }

    pub(crate) fn next_match(&mut self, dir: isize) {
        if self.search_matches.is_empty() {
            if !self.search_query.is_empty() {
                self.run_search();
            }
            if self.search_matches.is_empty() {
                self.status_message = Some("no matches".into());
                return;
            }
        }

        let len = self.search_matches.len() as isize;
        let cur = self.search_cursor.unwrap_or(0) as isize;
        let next = (cur + dir).rem_euclid(len) as usize;
        self.search_cursor = Some(next);
        self.follow = false;
        if self.search_in_overlay {
            self.jump_overlay_cursor(self.search_matches[next]);
            self.status_message = Some(format!("{}/{} in details", next + 1, len));
        } else {
            self.jump_to(self.search_matches[next]);
            self.status_message = Some(format!("{}/{} matches", next + 1, len));
        }
    }

    pub(crate) fn cycle_theme(&mut self) {
        if self.theme_picker.is_some() {
            return;
        }
        let themes = Theme::list_names();
        if themes.is_empty() {
            return;
        }
        self.theme_index = (self.theme_index + 1) % themes.len();
        let name = themes[self.theme_index].clone();
        self.commit_theme(&name);
    }

    pub fn ensure_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 || self.visible.is_empty() {
            return;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + viewport_height {
            self.scroll = self.selected + 1 - viewport_height;
        }
    }
}

fn contains(area: Rect, col: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && col >= area.x
        && col < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
