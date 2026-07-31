use crossterm::event::{KeyCode, KeyEvent};

use super::{App, InputMode};
use crate::details;
use crate::filter;

impl App {
    pub(super) fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search.history.reset_navigation();
                self.status_message = None;
            }
            KeyCode::Enter => {
                if !self.search.query.trim().is_empty() {
                    self.search.history.push(&self.search.query);
                    let _ = self.search.history.save();
                } else {
                    self.search.history.reset_navigation();
                }
                self.input_mode = InputMode::Normal;
                if let Some(err) = self.search.error.clone() {
                    self.status_message = Some(err);
                } else if self.search.matches.is_empty() {
                    self.status_message = if self.search.query.is_empty() {
                        None
                    } else {
                        Some("no matches".into())
                    };
                } else {
                    let n = self.search.matches.len();
                    let cur = self.search.cursor.unwrap_or(0).min(n - 1) + 1;
                    self.status_message = Some(if self.search.in_details {
                        format!("{cur}/{n} in details")
                    } else {
                        format!("{cur}/{n} matches")
                    });
                }
            }
            KeyCode::Up => {
                if let Some(line) = self.search.history.up(&self.search.query) {
                    self.search.query = line;
                    self.refresh_search_live();
                }
            }
            KeyCode::Down => {
                if let Some(line) = self.search.history.down() {
                    self.search.query = line;
                    self.refresh_search_live();
                }
            }
            KeyCode::Backspace => {
                self.search.query.pop();
                self.search.history.reset_navigation();
                self.refresh_search_live();
            }
            KeyCode::Char(c) => {
                self.search.query.push(c);
                self.search.history.reset_navigation();
                self.refresh_search_live();
            }
            _ => {}
        }
    }

    fn refresh_search_live(&mut self) {
        self.run_search();
        if self.search.matches.is_empty() {
            self.search.cursor = None;
            return;
        }
        let cursor = if self.search.in_details {
            self.search
                .matches
                .iter()
                .position(|&m| m >= self.details.scroll)
                .unwrap_or(0)
        } else {
            self.search
                .matches
                .iter()
                .position(|&m| m >= self.view.selected)
                .unwrap_or(0)
        };
        self.search.cursor = Some(cursor);
        self.view.follow = false;
        if self.search.in_details {
            self.details.cursor = self.search.matches[cursor];
            self.ensure_overlay_cursor_visible();
        } else {
            self.jump_to(self.search.matches[cursor]);
        }
    }

    pub fn move_sidebar_cursor(&mut self, delta: isize) {
        if self.filters.is_empty() {
            self.sidebar_selected = 0;
            return;
        }
        self.cancel_pending_op();
        let next = (self.sidebar_selected as isize + delta)
            .clamp(0, self.filters.len() as isize - 1) as usize;
        self.sidebar_selected = next;
    }

    pub fn jump_sidebar_cursor(&mut self, idx: usize) {
        if self.filters.is_empty() {
            self.sidebar_selected = 0;
            return;
        }
        self.cancel_pending_op();
        self.sidebar_selected = idx.min(self.filters.len() - 1);
    }

    pub fn ensure_sidebar_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 || self.filters.is_empty() {
            self.sidebar_scroll = 0;
            return;
        }
        if self.sidebar_selected >= self.filters.len() {
            self.sidebar_selected = self.filters.len() - 1;
        }
        if self.sidebar_selected < self.sidebar_scroll {
            self.sidebar_scroll = self.sidebar_selected;
        } else if self.sidebar_selected >= self.sidebar_scroll + viewport_height {
            self.sidebar_scroll = self.sidebar_selected + 1 - viewport_height;
        }
        let max_scroll = self.filters.len().saturating_sub(viewport_height);
        if self.sidebar_scroll > max_scroll {
            self.sidebar_scroll = max_scroll;
        }
    }

    pub fn set_follow(&mut self, enabled: bool) {
        self.view.follow = enabled;
        self.config.follow = enabled;
        if enabled && !self.view.visible.is_empty() {
            self.jump_to(self.view.visible.len() - 1);
        }
        self.status_message = Some(if enabled {
            "follow: on".into()
        } else {
            "follow: off".into()
        });
    }

    pub fn move_overlay_cursor(&mut self, delta: isize) {
        if !self.details.visible || self.details.content_len == 0 {
            return;
        }
        let max = self.details.content_len - 1;
        let next = (self.details.cursor as isize + delta).clamp(0, max as isize) as usize;
        self.details.cursor = next;
        self.ensure_overlay_cursor_visible();
    }

    pub fn jump_overlay_cursor(&mut self, idx: usize) {
        if self.details.content_len == 0 {
            self.details.cursor = 0;
            return;
        }
        self.details.cursor = idx.min(self.details.content_len - 1);
        self.ensure_overlay_cursor_visible();
    }

    pub fn ensure_overlay_cursor_visible(&mut self) {
        let view = self.details.viewport_height.max(1);
        let idx = self.details.cursor;
        if idx < self.details.scroll {
            self.details.scroll = idx;
        } else if idx >= self.details.scroll + view {
            self.details.scroll = idx + 1 - view;
        }
    }

    pub(crate) fn move_selection(&mut self, delta: isize) {
        if self.view.visible.is_empty() {
            return;
        }
        self.view.follow = false;
        let next = (self.view.selected as isize + delta)
            .clamp(0, self.view.visible.len() as isize - 1) as usize;
        if next != self.view.selected {
            self.reset_overlay_for_selection_change();
        }
        self.view.selected = next;
        self.ensure_selection_visible();
    }

    pub(crate) fn jump_to(&mut self, idx: usize) {
        if self.view.visible.is_empty() {
            return;
        }
        let next = idx.min(self.view.visible.len() - 1);
        if next != self.view.selected {
            self.reset_overlay_for_selection_change();
        }
        self.view.selected = next;
        self.ensure_selection_visible();
    }

    pub(crate) fn scroll_list(&mut self, delta: isize) {
        if self.view.visible.is_empty() {
            return;
        }
        self.view.follow = false;
        let viewport = self.pointer.hit.list_inner.height.max(1) as usize;
        let max_scroll = self.view.visible.len().saturating_sub(viewport);
        let next = (self.view.scroll as isize + delta).clamp(0, max_scroll as isize) as usize;
        self.view.scroll = next;
    }

    fn ensure_selection_visible(&mut self) {
        let viewport = self.pointer.hit.list_inner.height as usize;
        if viewport > 0 {
            self.ensure_visible(viewport, true);
        }
    }

    pub fn clear_search(&mut self) {
        self.search.query.clear();
        self.search.regex = None;
        self.search.error = None;
        self.search.matches.clear();
        self.search.cursor = None;
        self.search.in_details = false;
    }

    pub fn apply_case_mode(&mut self) -> Option<String> {
        let mode = self.config.case_mode;
        for filter in &mut self.filters {
            if let Err(err) = filter.recompile(mode) {
                return Some(format!("filter /{}/: {err}", filter.pattern));
            }
        }
        self.rebuild_visible(None);
        if !self.search.query.is_empty() {
            self.run_search();
        }
        None
    }

    pub fn run_search(&mut self) {
        if self.search.query.is_empty() {
            self.search.regex = None;
            self.search.error = None;
            self.search.matches.clear();
            self.search.cursor = None;
            return;
        }
        let regex = match filter::compile_regex(&self.search.query, self.config.case_mode) {
            Ok(regex) => regex,
            Err(err) => {
                self.search.regex = None;
                self.search.error = Some(format!("invalid regex: {err}"));
                self.search.matches.clear();
                self.search.cursor = None;
                return;
            }
        };
        self.search.error = None;
        if self.search.in_details {
            self.search.matches = if let Some(entry) = self.selected_entry() {
                details::build_lines(entry, &self.theme, &self.config, &self.details.folded)
                    .into_iter()
                    .enumerate()
                    .filter(|(_, line)| regex.is_match(&line.plain_text()))
                    .map(|(index, _)| index)
                    .collect()
            } else {
                Vec::new()
            };
        } else {
            self.search.matches = self
                .view
                .visible
                .iter()
                .enumerate()
                .filter(|&(_, source)| regex.is_match(&self.source.entries()[*source].raw))
                .map(|(visible, _)| visible)
                .collect();
        }
        self.search.regex = Some(regex);
    }

    pub(crate) fn next_match(&mut self, direction: isize) {
        if self.search.matches.is_empty() {
            if !self.search.query.is_empty() {
                self.run_search();
            }
            if self.search.matches.is_empty() {
                self.status_message = Some("no matches".into());
                return;
            }
        }

        let len = self.search.matches.len() as isize;
        let current = self.search.cursor.unwrap_or(0) as isize;
        let next = (current + direction).rem_euclid(len) as usize;
        self.search.cursor = Some(next);
        self.view.follow = false;
        if self.search.in_details {
            self.jump_overlay_cursor(self.search.matches[next]);
            self.status_message = Some(format!("{}/{} in details", next + 1, len));
        } else {
            self.jump_to(self.search.matches[next]);
            self.status_message = Some(format!("{}/{} matches", next + 1, len));
        }
    }

    pub fn ensure_visible(&mut self, viewport_height: usize, follow_selection: bool) {
        if viewport_height == 0 || self.view.visible.is_empty() {
            return;
        }
        if follow_selection {
            if self.view.selected < self.view.scroll {
                self.view.scroll = self.view.selected;
            } else if self.view.selected >= self.view.scroll + viewport_height {
                self.view.scroll = self.view.selected + 1 - viewport_height;
            }
        }
        let max_scroll = self.view.visible.len().saturating_sub(viewport_height);
        if self.view.scroll > max_scroll {
            self.view.scroll = max_scroll;
        }
    }
}
