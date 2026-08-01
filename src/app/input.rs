use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::{App, InputMode, ThemePicker};
use crate::command;
use crate::completion;
use crate::keys;
use crate::theme::Theme;

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) {
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
            _ => {
                let command = keys::encode(key)
                    .and_then(|spec| self.config.keys.get(&spec))
                    .map(|raw| raw.trim().to_ascii_lowercase());
                match command.as_deref() {
                    Some("nav up") => self.theme_picker_move(-1),
                    Some("nav down") => self.theme_picker_move(1),
                    Some("page up") => {
                        let page = self
                            .theme_picker
                            .as_ref()
                            .map(|picker| picker.list_area.height.max(1) as isize)
                            .unwrap_or(1);
                        self.theme_picker_move(-page);
                    }
                    Some("page down") => {
                        let page = self
                            .theme_picker
                            .as_ref()
                            .map(|picker| picker.list_area.height.max(1) as isize)
                            .unwrap_or(1);
                        self.theme_picker_move(page);
                    }
                    Some("nav top") => self.theme_picker_select(0),
                    Some("nav bottom") => {
                        if let Some(picker) = &self.theme_picker {
                            let last = picker.names.len().saturating_sub(1);
                            self.theme_picker_select(last);
                        }
                    }
                    Some(
                        "view details"
                        | "view details toggle"
                        | "view details on"
                        | "view current"
                        | "view current toggle"
                        | "view current on",
                    ) => self.close_theme_picker(true),
                    Some("view details off" | "view current off") => {
                        self.close_theme_picker(false)
                    }
                    _ => {}
                }
            }
        }
    }

    pub(super) fn handle_theme_picker_mouse(&mut self, mouse: MouseEvent) {
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
        self.command_line.buffer.clear();
        self.command_line.history.reset_navigation();
        self.search.history.reset_navigation();
        self.command_line.completions.clear();
        self.preview_theme_at_selection();
        let confirm =
            keys::binding_for_command(&self.config.keys, None, "view details").unwrap_or("enter");
        self.status_message = Some(format!(
            "theme picker · click/{confirm} to set · Esc to cancel"
        ));
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
        let overrides = self.config.theme_overrides();
        if let Ok(theme) = Theme::resolve_with_overrides(&name, &overrides) {
            self.theme = theme;
            self.status_message = Some(format!("preview: {name}"));
        }
    }

    pub(crate) fn close_theme_picker(&mut self, confirm: bool) {
        let Some(picker) = self.theme_picker.take() else {
            return;
        };
        if confirm && let Some(name) = picker.names.get(picker.selected) {
            self.commit_theme(name);
            return;
        }
        let overrides = self.config.theme_overrides();
        if let Ok(theme) = Theme::resolve_with_overrides(&picker.previous_name, &overrides) {
            self.theme = theme;
            self.theme_index = Theme::list_names()
                .iter()
                .position(|t| t == &picker.previous_name)
                .unwrap_or(self.theme_index);
        }
        self.status_message = Some(format!("theme: {}", self.config.theme.name()));
    }

    pub(crate) fn commit_theme(&mut self, name: &str) {
        let overrides = self.config.theme_overrides();
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

    pub fn begin_command_mode(&mut self) {
        self.input_mode = InputMode::Command;
        self.command_line.buffer.clear();
        self.command_line.history.reset_navigation();
        self.status_message = None;
        completion::refresh(self);
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.command_line.buffer.clear();
                self.command_line.history.reset_navigation();
                self.command_line.completions.clear();
                self.status_message = None;
            }
            KeyCode::Enter => {
                if self.command_line.completions.browsed
                    && self.command_line.completions.selected().is_some()
                {
                    completion::apply_selected(self);
                    return;
                }
                let cmd = std::mem::take(&mut self.command_line.buffer);
                self.command_line.completions.clear();
                self.command_line.history.reset_navigation();
                self.input_mode = InputMode::Normal;
                if !cmd.trim().is_empty() {
                    self.command_line.history.push(&cmd);
                    let _ = self.command_line.history.save();
                }
                command::execute(self, &cmd);
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    completion::tab_complete_prev(self);
                } else {
                    completion::tab_complete(self);
                }
            }
            KeyCode::BackTab => completion::tab_complete_prev(self),
            KeyCode::Down => {
                if self.command_line.completions.selected.is_some() {
                    self.command_line.completions.select_next();
                    self.command_line.completions.browsed = true;
                } else if let Some(line) = self.command_line.history.down() {
                    self.command_line.buffer = line;
                    completion::refresh(self);
                }
            }
            KeyCode::Up => {
                if self.command_line.completions.selected.is_some() {
                    self.command_line.completions.select_prev();
                    self.command_line.completions.browsed = true;
                } else if let Some(line) = self.command_line.history.up(&self.command_line.buffer) {
                    self.command_line.buffer = line;
                    completion::refresh(self);
                }
            }
            KeyCode::Backspace => {
                if self.command_line.buffer.is_empty() {
                    self.input_mode = InputMode::Normal;
                    self.command_line.history.reset_navigation();
                    self.command_line.completions.clear();
                    self.status_message = None;
                } else {
                    self.command_line.buffer.pop();
                    self.command_line.history.reset_navigation();
                    completion::refresh(self);
                }
            }
            KeyCode::Char(c) => {
                if self.command_line.completions.selected().is_some()
                    && !completion::selection_applied(self)
                {
                    completion::apply_selected(self);
                }
                self.command_line.buffer.push(c);
                self.command_line.history.reset_navigation();
                completion::refresh(self);
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.is_sidebar_focused() && self.config.sidebar && self.handle_sidebar_key(key) {
            return;
        }
        if self.is_details_focused() && self.details.visible && self.handle_overlay_key(key) {
            return;
        }

        if key.modifiers.is_empty()
            && let KeyCode::Char(c) = key.code
            && c.is_ascii_digit()
            && (c != '0' || self.count.is_some())
        {
            let digit = (c as u8 - b'0') as usize;
            self.count = Some(
                self.count
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit),
            );
            return;
        }

        let Some(spec) = keys::encode(key) else {
            return;
        };
        let Some(cmd) = self.resolve_key_command(&spec) else {
            self.count = None;
            return;
        };
        command::execute_from_key(self, &cmd);
    }

    fn resolve_key_command(&self, spec: &str) -> Option<String> {
        if self.is_sidebar_focused()
            && self.config.sidebar
            && let Some(cmd) = self.config.sidebar_keys.get(spec)
        {
            return if cmd.is_empty() {
                None
            } else {
                Some(cmd.clone())
            };
        }
        if self.is_details_focused()
            && self.details.visible
            && let Some(cmd) = self.config.details_keys.get(spec)
        {
            return if cmd.is_empty() {
                None
            } else {
                Some(cmd.clone())
            };
        }
        self.config
            .keys
            .get(spec)
            .filter(|cmd| !cmd.is_empty())
            .cloned()
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(':') | KeyCode::Char('q') => {
                self.focus_list();
                self.details.help = false;
                false
            }
            _ => false,
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(':') | KeyCode::Char('q') => {
                self.focus_list();
                false
            }
            _ => false,
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
}

fn contains(area: Rect, col: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && col >= area.x
        && col < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}
