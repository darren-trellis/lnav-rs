use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::{App, InputMode, LastClick, ScrollbarDrag};
use crate::completion;

impl App {
    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.config_modal.is_some() {
            self.handle_config_modal_mouse(mouse);
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.cancel_pending_op();
                let n = self.config.scroll_lines.max(1) as isize;
                if self.config.sidebar
                    && (self.is_sidebar_focused()
                        || contains(self.pointer.hit.sidebar_inner, mouse.column, mouse.row))
                {
                    if self.config.scroll_moves_selection {
                        self.move_sidebar_cursor(-n);
                    } else {
                        self.scroll_sidebar(-n);
                    }
                } else if self.details.visible
                    && (self.is_details_focused()
                        || contains(self.pointer.hit.overlay, mouse.column, mouse.row))
                {
                    if self.config.scroll_moves_selection {
                        self.move_overlay_cursor(-n);
                    } else {
                        self.scroll_overlay(-n);
                    }
                } else if self.input_mode == InputMode::Command
                    && !self.command_line.completions.items.is_empty()
                {
                    self.command_line.completions.select_prev();
                    self.command_line.completions.browsed = true;
                } else if self.config.scroll_moves_selection {
                    self.with_motion(|a| a.move_selection(-n));
                } else {
                    self.scroll_list(-n);
                }
            }
            MouseEventKind::ScrollDown => {
                self.cancel_pending_op();
                let n = self.config.scroll_lines.max(1) as isize;
                if self.config.sidebar
                    && (self.is_sidebar_focused()
                        || contains(self.pointer.hit.sidebar_inner, mouse.column, mouse.row))
                {
                    if self.config.scroll_moves_selection {
                        self.move_sidebar_cursor(n);
                    } else {
                        self.scroll_sidebar(n);
                    }
                } else if self.details.visible
                    && (self.is_details_focused()
                        || contains(self.pointer.hit.overlay, mouse.column, mouse.row))
                {
                    if self.config.scroll_moves_selection {
                        self.move_overlay_cursor(n);
                    } else {
                        self.scroll_overlay(n);
                    }
                } else if self.input_mode == InputMode::Command
                    && !self.command_line.completions.items.is_empty()
                {
                    self.command_line.completions.select_next();
                    self.command_line.completions.browsed = true;
                } else if self.config.scroll_moves_selection {
                    self.with_motion(|a| a.move_selection(n));
                } else {
                    self.scroll_list(n);
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.handle_scrollbar_pointer(mouse.column, mouse.row, true) {
                    return;
                }
                self.handle_left_click(mouse.column, mouse.row);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let _ = self.handle_scrollbar_pointer(mouse.column, mouse.row, false);
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.pointer.scrollbar_drag = None;
            }
            MouseEventKind::Moved
                if self.input_mode == InputMode::Command
                    && contains(self.pointer.hit.suggest_inner, mouse.column, mouse.row) =>
            {
                if let Some(idx) = self.suggest_index_at(mouse.column, mouse.row) {
                    self.command_line.completions.selected = Some(idx);
                    self.command_line.completions.browsed = true;
                }
            }
            _ => {}
        }
    }

    fn handle_scrollbar_pointer(&mut self, col: u16, row: u16, is_down: bool) -> bool {
        if is_down {
            if contains(self.pointer.hit.overlay_scrollbar, col, row) {
                self.pointer.scrollbar_drag = Some(ScrollbarDrag::Overlay);
                self.apply_overlay_scrollbar(row);
                return true;
            }
            if contains(self.pointer.hit.list_scrollbar, col, row) {
                self.leave_editing_modes();
                self.pointer.scrollbar_drag = Some(ScrollbarDrag::List);
                self.apply_list_scrollbar(row);
                return true;
            }
            self.pointer.scrollbar_drag = None;
            return false;
        }
        match self.pointer.scrollbar_drag {
            Some(ScrollbarDrag::List) => {
                self.apply_list_scrollbar(row);
                true
            }
            Some(ScrollbarDrag::Overlay) => {
                self.apply_overlay_scrollbar(row);
                true
            }
            None => false,
        }
    }

    fn leave_editing_modes(&mut self) {
        if self.input_mode != InputMode::Normal {
            self.input_mode = InputMode::Normal;
            self.command_line.buffer.clear();
            self.command_line.history.reset_navigation();
            self.search.history.reset_navigation();
            self.command_line.completions.clear();
        }
    }

    fn apply_list_scrollbar(&mut self, row: u16) {
        let bar = self.pointer.hit.list_scrollbar;
        let viewport = self.pointer.hit.list_inner.height as usize;
        if viewport == 0 || bar.height == 0 || self.view.visible.is_empty() {
            return;
        }
        let (_, _, body_h) = self.list_band_layout(viewport);
        let body_h = body_h.max(1);
        let new_scroll = scroll_index_at(bar, row, self.view.visible.len(), body_h);
        let max_scroll = self.view.visible.len().saturating_sub(body_h);
        let new_scroll = new_scroll.min(max_scroll);
        self.view.follow = false;
        self.focus_list();
        if self.config.scroll_moves_selection && self.view.selected >= self.pin_count() {
            let body_sel = self.view.selected - self.pin_count();
            let offset = body_sel
                .saturating_sub(self.view.scroll)
                .min(body_h.saturating_sub(1));
            self.view.scroll = new_scroll;
            let new_selected =
                self.pin_count() + (new_scroll + offset).min(self.view.visible.len() - 1);
            if new_selected != self.view.selected {
                self.reset_overlay_for_selection_change();
                self.view.selected = new_selected;
            }
        } else {
            self.view.scroll = new_scroll;
        }
    }

    fn apply_overlay_scrollbar(&mut self, row: u16) {
        let bar = self.pointer.hit.overlay_scrollbar;
        let viewport = self.details.viewport_height;
        if viewport == 0 || bar.height == 0 || self.details.content_len == 0 {
            return;
        }
        let new_scroll = scroll_index_at(bar, row, self.details.content_len, viewport);
        let max_scroll = self.details.content_len.saturating_sub(viewport);
        self.details.scroll = new_scroll.min(max_scroll);
        self.focus_details();
        if self.config.scroll_moves_selection {
            if self.details.cursor < self.details.scroll {
                self.details.cursor = self.details.scroll;
            } else if self.details.cursor >= self.details.scroll + viewport {
                self.details.cursor = self.details.scroll + viewport - 1;
            }
        }
    }

    fn handle_left_click(&mut self, col: u16, row: u16) {
        if contains(self.pointer.hit.suggest_inner, col, row) {
            if let Some(idx) = self.suggest_index_at(col, row) {
                self.command_line.completions.selected = Some(idx);
                self.command_line.completions.browsed = false;
                completion::apply_selected(self);
            }
            return;
        }

        if contains(self.pointer.hit.sidebar_inner, col, row) {
            self.leave_editing_modes();
            self.focus_sidebar();
            let area = self.pointer.hit.sidebar_inner;
            if area.height > 0 {
                let row_off = (row - area.y) as usize;
                let idx = self.sidebar_scroll + row_off;
                if idx < self.filters.len() {
                    self.sidebar_selected = idx;
                }
            }
            return;
        }

        if contains(self.pointer.hit.overlay, col, row) {
            if contains(self.pointer.hit.overlay_scrollbar, col, row) {
                return;
            }
            self.focus_details();
            let inner_y = self.pointer.hit.overlay.y.saturating_add(1);
            if row >= inner_y {
                let row_off = (row - inner_y) as usize;
                let idx = self.details.scroll + row_off;
                if idx < self.details.content_len {
                    self.jump_overlay_cursor(idx);
                }
            }
            return;
        }

        if contains(self.pointer.hit.list_inner, col, row) {
            self.click_list_row(row);
            return;
        }

        if contains(self.pointer.hit.status, col, row) {
            match self.input_mode {
                InputMode::Search | InputMode::Command => {}
                InputMode::Normal => self.begin_command_mode(),
            }
        }
    }

    fn click_list_row(&mut self, row: u16) {
        let area = self.pointer.hit.list_inner;
        if area.height == 0 || self.display_len() == 0 {
            return;
        }
        let row_off = (row - area.y) as usize;
        let pin_rows = self.pointer.hit.list_pin_rows;
        let (_, sep_rows, _) = self.list_band_layout(area.height as usize);
        let display = if row_off < pin_rows {
            row_off
        } else if row_off < pin_rows + sep_rows {
            return;
        } else {
            let body_row = row_off - pin_rows - sep_rows;
            let body_idx = self.view.scroll + body_row;
            if body_idx >= self.view.visible.len() {
                return;
            }
            self.pin_count() + body_idx
        };
        if display >= self.display_len() {
            return;
        }

        self.leave_editing_modes();
        self.focus_list();

        let double = self
            .pointer
            .last_click
            .map(|c| c.vis_idx == display && c.at.elapsed() < Duration::from_millis(400))
            .unwrap_or(false);

        if let Some(op) = self.pending_op.take() {
            let start = self.op_anchor;
            self.view.follow = false;
            self.view.selected = display;
            self.apply_op_display_range(op, start, display);
            self.pointer.last_click = Some(LastClick {
                at: Instant::now(),
                vis_idx: display,
            });
            return;
        }

        self.view.follow = false;
        self.view.selected = display;
        if double {
            self.toggle_details();
            self.pointer.last_click = None;
        } else {
            self.pointer.last_click = Some(LastClick {
                at: Instant::now(),
                vis_idx: display,
            });
        }
    }

    fn suggest_index_at(&self, col: u16, row: u16) -> Option<usize> {
        let area = self.pointer.hit.suggest_inner;
        if !contains(area, col, row) || area.height == 0 {
            return None;
        }
        let idx = self.pointer.hit.suggest_start + (row - area.y) as usize;
        if idx < self.command_line.completions.items.len() {
            Some(idx)
        } else {
            None
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

pub fn scroll_index_at(bar: Rect, row: u16, content_len: usize, viewport: usize) -> usize {
    let max_scroll = content_len.saturating_sub(viewport);
    if max_scroll == 0 || bar.height <= 1 {
        return 0;
    }
    let y = row.saturating_sub(bar.y) as usize;
    let y = y.min(bar.height as usize - 1);
    (y * max_scroll) / (bar.height as usize - 1)
}
