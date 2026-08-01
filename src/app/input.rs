use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::{App, ConfigModal, ConfigPicker, ConfigValueEditor, HelpModal, InputMode};
use crate::command;
use crate::completion;
use crate::config_options::{self, ConfigOption, ValueKind};
use crate::keys;
use crate::theme::Theme;
use crate::ui::help_modal::HELP_LINES;

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        if self.help_modal.is_some() {
            self.handle_help_modal_key(key);
            return;
        }
        if self.config_modal.is_some() {
            self.handle_config_modal_key(key);
            return;
        }
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::Command => self.handle_command_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_help_modal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => self.close_help_modal(),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_help_modal(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_help_modal(-1),
            KeyCode::PageDown | KeyCode::Char(' ') => {
                let page = self
                    .help_modal
                    .as_ref()
                    .map(|m| m.viewport.max(1) as isize)
                    .unwrap_or(1);
                self.scroll_help_modal(page);
            }
            KeyCode::PageUp => {
                let page = self
                    .help_modal
                    .as_ref()
                    .map(|m| m.viewport.max(1) as isize)
                    .unwrap_or(1);
                self.scroll_help_modal(-page);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                if let Some(modal) = &mut self.help_modal {
                    modal.scroll = 0;
                }
            }
            KeyCode::End | KeyCode::Char('G') => {
                if let Some(modal) = &mut self.help_modal {
                    modal.scroll = HELP_LINES.len().saturating_sub(modal.viewport.max(1));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn open_help_modal(&mut self) {
        self.cancel_pending_op();
        self.close_config_modal(false);
        self.input_mode = InputMode::Normal;
        self.command_line.buffer.clear();
        self.command_line.completions.clear();
        self.help_modal = Some(HelpModal {
            scroll: 0,
            viewport: 0,
            popup_area: Rect::default(),
        });
        self.status_message = None;
    }

    pub(crate) fn close_help_modal(&mut self) {
        self.help_modal = None;
    }

    pub(crate) fn toggle_help_modal(&mut self) {
        if self.help_modal.is_some() {
            self.close_help_modal();
        } else {
            self.open_help_modal();
        }
    }

    fn scroll_help_modal(&mut self, delta: isize) {
        let Some(modal) = &mut self.help_modal else {
            return;
        };
        let max = HELP_LINES.len().saturating_sub(modal.viewport.max(1));
        let next = (modal.scroll as isize + delta).clamp(0, max as isize) as usize;
        modal.scroll = next;
    }

    fn handle_config_modal_key(&mut self, key: KeyEvent) {
        match &self.config_modal {
            Some(ConfigModal::Editor(_)) => self.handle_config_editor_key(key),
            Some(ConfigModal::Picker(_)) => self.handle_config_picker_key(key),
            None => {}
        }
    }

    fn handle_config_editor_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.close_config_modal(false),
            KeyCode::Enter => self.close_config_modal(true),
            KeyCode::Backspace => {
                if let Some(ConfigModal::Editor(editor)) = &mut self.config_modal {
                    editor.buffer.pop();
                }
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(ConfigModal::Editor(editor)) = &mut self.config_modal {
                    editor.buffer.push(c);
                }
            }
            _ => {}
        }
    }

    fn handle_config_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.close_config_modal(false),
            _ => {
                let command = keys::encode(key)
                    .and_then(|spec| self.config.keys.bindings.get(&spec))
                    .map(|raw| raw.trim().to_ascii_lowercase());
                match command.as_deref() {
                    Some("nav up") => self.config_picker_move(-1),
                    Some("nav down") => self.config_picker_move(1),
                    Some("page up") => {
                        let page = self
                            .config_modal
                            .as_ref()
                            .and_then(|m| match m {
                                ConfigModal::Picker(p) => Some(p.list_area.height.max(1) as isize),
                                _ => None,
                            })
                            .unwrap_or(1);
                        self.config_picker_move(-page);
                    }
                    Some("page down") => {
                        let page = self
                            .config_modal
                            .as_ref()
                            .and_then(|m| match m {
                                ConfigModal::Picker(p) => Some(p.list_area.height.max(1) as isize),
                                _ => None,
                            })
                            .unwrap_or(1);
                        self.config_picker_move(page);
                    }
                    Some("nav top") => self.config_picker_select(0),
                    Some("nav bottom") => {
                        if let Some(ConfigModal::Picker(picker)) = &self.config_modal {
                            let last = picker.values.len().saturating_sub(1);
                            self.config_picker_select(last);
                        }
                    }
                    Some(
                        "view details"
                        | "view details toggle"
                        | "view details on"
                        | "view current"
                        | "view current toggle"
                        | "view current on",
                    ) => self.close_config_modal(true),
                    Some("view details off" | "view current off") => self.close_config_modal(false),
                    _ => {}
                }
            }
        }
    }

    pub(super) fn handle_help_modal_mouse(&mut self, mouse: MouseEvent) {
        let Some(modal) = &self.help_modal else {
            return;
        };
        let popup = modal.popup_area;
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_help_modal(-1),
            MouseEventKind::ScrollDown => self.scroll_help_modal(1),
            MouseEventKind::Down(MouseButton::Left) => {
                if !contains(popup, mouse.column, mouse.row) {
                    self.close_help_modal();
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_config_modal_mouse(&mut self, mouse: MouseEvent) {
        let Some(ConfigModal::Picker(picker)) = &self.config_modal else {
            return;
        };
        let popup = picker.popup_area;
        let area = picker.list_area;
        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::ScrollUp => self.config_picker_move(-1),
            MouseEventKind::ScrollDown => self.config_picker_move(1),
            MouseEventKind::Moved => {
                if contains(area, col, row) {
                    let idx = picker.list_start + (row - area.y) as usize;
                    if idx < picker.values.len() {
                        self.config_picker_select(idx);
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if contains(area, col, row) {
                    let idx = picker.list_start + (row - area.y) as usize;
                    if idx < picker.values.len() {
                        self.config_picker_select(idx);
                        self.close_config_modal(true);
                    }
                } else if !contains(popup, col, row) {
                    self.close_config_modal(false);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn open_config_modal(&mut self, option: &'static ConfigOption) {
        self.cancel_pending_op();
        self.close_help_modal();
        self.input_mode = InputMode::Normal;
        self.command_line.buffer.clear();
        self.command_line.history.reset_navigation();
        self.search.history.reset_navigation();
        self.command_line.completions.clear();

        let previous_value = option.get(self);
        match option.value_kind {
            ValueKind::Unsigned => {
                self.config_modal = Some(ConfigModal::Editor(ConfigValueEditor {
                    option_name: option.name,
                    previous_value: previous_value.clone(),
                    buffer: previous_value,
                    popup_area: Rect::default(),
                }));
                self.status_message = Some(format!(
                    "{} · type value · Enter to set · Esc to cancel",
                    option.name
                ));
            }
            ValueKind::Theme | ValueKind::Bool | ValueKind::CaseMode | ValueKind::TimestampFormat => {
                let values = option.value_kind.suggestions();
                if values.is_empty() {
                    self.status_message = Some(format!("no values for {}", option.name));
                    return;
                }
                let selected = values
                    .iter()
                    .position(|v| v == &previous_value)
                    .unwrap_or(0);
                self.config_modal = Some(ConfigModal::Picker(ConfigPicker {
                    option_name: option.name,
                    values,
                    selected,
                    previous_value,
                    popup_area: Rect::default(),
                    list_area: Rect::default(),
                    list_start: 0,
                }));
                if option.name == "theme" {
                    self.preview_theme_at_selection();
                }
                let confirm = keys::binding_for_command(
                    &self.config.keys.bindings,
                    None,
                    "view details on",
                )
                .or_else(|| {
                    keys::binding_for_command(&self.config.keys.bindings, None, "view details")
                })
                .unwrap_or("enter");
                self.status_message = Some(format!(
                    "{} · click/{confirm} to set · Esc to cancel",
                    option.name
                ));
            }
        }
    }

    fn config_picker_move(&mut self, delta: isize) {
        let Some(ConfigModal::Picker(picker)) = &self.config_modal else {
            return;
        };
        let len = picker.values.len() as isize;
        if len == 0 {
            return;
        }
        let next = (picker.selected as isize + delta).rem_euclid(len) as usize;
        self.config_picker_select(next);
    }

    fn config_picker_select(&mut self, idx: usize) {
        let option_name = {
            let Some(ConfigModal::Picker(picker)) = &mut self.config_modal else {
                return;
            };
            if idx >= picker.values.len() || idx == picker.selected {
                return;
            }
            picker.selected = idx;
            picker.option_name
        };
        if option_name == "theme" {
            self.preview_theme_at_selection();
        }
    }

    fn preview_theme_at_selection(&mut self) {
        let Some(name) = self.config_modal.as_ref().and_then(|m| match m {
            ConfigModal::Picker(p) if p.option_name == "theme" => p.values.get(p.selected).cloned(),
            _ => None,
        }) else {
            return;
        };
        let overrides = self.config.theme_overrides();
        if let Ok(theme) = Theme::resolve_with_overrides(&name, &overrides) {
            self.theme = theme;
            self.status_message = Some(format!("preview: {name}"));
        }
    }

    pub(crate) fn close_config_modal(&mut self, confirm: bool) {
        let Some(modal) = self.config_modal.take() else {
            return;
        };
        match modal {
            ConfigModal::Picker(picker) => {
                if confirm && let Some(value) = picker.values.get(picker.selected) {
                    self.commit_config_value(picker.option_name, value);
                    return;
                }
                if picker.option_name == "theme" {
                    let overrides = self.config.theme_overrides();
                    if let Ok(theme) =
                        Theme::resolve_with_overrides(&picker.previous_value, &overrides)
                    {
                        self.theme = theme;
                        self.theme_index = Theme::list_names()
                            .iter()
                            .position(|t| t == &picker.previous_value)
                            .unwrap_or(self.theme_index);
                    }
                    self.status_message = Some(format!("theme: {}", self.config.theme.name()));
                } else {
                    self.status_message = Some(format!(
                        "{}={}",
                        picker.option_name, picker.previous_value
                    ));
                }
            }
            ConfigModal::Editor(editor) => {
                if confirm {
                    self.commit_config_value(editor.option_name, &editor.buffer);
                    return;
                }
                self.status_message = Some(format!(
                    "{}={}",
                    editor.option_name, editor.previous_value
                ));
            }
        }
    }

    fn commit_config_value(&mut self, name: &str, value: &str) {
        let Some(option) = config_options::find(name) else {
            self.status_message = Some(format!("unknown option: {name}"));
            return;
        };
        if option.set(self, value) && option.name != "theme" {
            self.maybe_autosave();
        }
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
                self.maybe_autosave();
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
            && let Some(cmd) = self.config.keys.sidebar.get(spec)
        {
            return if cmd.is_empty() {
                None
            } else {
                Some(cmd.clone())
            };
        }
        if self.is_details_focused()
            && self.details.visible
            && let Some(cmd) = self.config.keys.details.get(spec)
        {
            return if cmd.is_empty() {
                None
            } else {
                Some(cmd.clone())
            };
        }
        self.config
            .keys
            .bindings
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

}

fn contains(area: Rect, col: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && col >= area.x
        && col < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}
