use std::collections::HashSet;

use super::{App, PendingOp};
use crate::object_span;

impl App {
    pub fn start_or_repeat_filter_delete(&mut self) {
        if self.filters.is_empty() {
            self.pending_op = None;
            self.count = None;
            self.status_message = Some("no filters".into());
            return;
        }
        if self.pending_op == Some(PendingOp::DeleteFilter) {
            self.pending_op = None;
            self.count = None;
            self.delete_selected_filter();
            return;
        }
        self.pending_op = Some(PendingOp::DeleteFilter);
        self.status_message = None;
    }

    pub fn delete_selected_filter(&mut self) {
        if self.filters.is_empty() {
            self.status_message = Some("no filters".into());
            return;
        }
        let index = self.sidebar_selected.min(self.filters.len() - 1);
        let removed = self.filters.remove(index);
        if self.filters.is_empty() {
            self.sidebar_selected = 0;
        } else if self.sidebar_selected >= self.filters.len() {
            self.sidebar_selected = self.filters.len() - 1;
        }
        self.rebuild_visible(None);
        self.persist_session();
        self.status_message = Some(format!(
            "deleted filter-{} /{}/",
            removed.label(),
            removed.pattern
        ));
    }

    pub(crate) fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1).max(1)
    }

    pub(crate) fn take_count_opt(&mut self) -> Option<usize> {
        self.count.take().filter(|&count| count > 0)
    }

    pub(crate) fn cancel_pending_op(&mut self) {
        self.count = None;
        if self.pending_op.take().is_some() {
            self.status_message = None;
        }
    }

    pub(crate) fn start_or_repeat_op(&mut self, op: PendingOp) {
        if matches!(op, PendingOp::DeleteFilter) {
            self.start_or_repeat_filter_delete();
            return;
        }
        if self.view.visible.is_empty() {
            self.pending_op = None;
            self.count = None;
            return;
        }
        if self.pending_op == Some(op) {
            let count = self.take_count();
            let at = self.view.selected;
            let end = (at + count - 1).min(self.view.visible.len().saturating_sub(1));
            self.pending_op = None;
            self.apply_op_visible_range(op, at, end);
            return;
        }
        self.pending_op = Some(op);
        self.op_anchor = self.view.selected;
        self.status_message = None;
    }

    pub(crate) fn with_motion<F>(&mut self, motion: F)
    where
        F: FnOnce(&mut Self),
    {
        if let Some(op @ (PendingOp::Hide | PendingOp::Delete)) = self.pending_op {
            let start = self.op_anchor;
            motion(self);
            let end = self.view.selected;
            self.pending_op = None;
            self.count = None;
            self.apply_op_visible_range(op, start, end);
        } else {
            if self.pending_op == Some(PendingOp::DeleteFilter) {
                self.pending_op = None;
            }
            motion(self);
        }
    }

    pub(crate) fn apply_op_visible_range(&mut self, op: PendingOp, from: usize, to: usize) {
        if self.view.visible.is_empty() {
            return;
        }
        let low = from.min(to).min(self.view.visible.len() - 1);
        let high = from.max(to).min(self.view.visible.len() - 1);
        let mut indices = HashSet::new();
        for visible in low..=high {
            let Some(&source) = self.view.visible.get(visible) else {
                continue;
            };
            for index in object_span::object_span(self.source.entries(), source) {
                indices.insert(index);
            }
        }
        let mut indices: Vec<usize> = indices.into_iter().collect();
        indices.sort_unstable();
        if indices.is_empty() {
            return;
        }

        self.view.follow = false;
        match op {
            PendingOp::DeleteFilter => self.delete_selected_filter(),
            PendingOp::Hide => {
                let count = indices.len();
                for index in indices {
                    self.view.hidden.insert(index);
                }
                self.rebuild_visible(None);
                self.view.selected = if self.view.visible.is_empty() {
                    0
                } else {
                    low.min(self.view.visible.len() - 1)
                };
                if self.view.visible.is_empty() {
                    self.close_details();
                } else {
                    self.reset_overlay_for_selection_change();
                }
                if !self.search.query.is_empty() {
                    self.run_search();
                }
                self.status_message = Some(format!(
                    "hidden {count} line{}  (:clear-hidden to restore)",
                    if count == 1 { "" } else { "s" }
                ));
            }
            PendingOp::Delete => {
                if !self.source.is_file() {
                    self.status_message = Some("cannot delete from stdin".into());
                    return;
                }
                match self.source.delete_entries(&indices) {
                    Ok(removed) => {
                        self.view.hidden.clear();
                        self.rebuild_visible(None);
                        self.view.selected = if self.view.visible.is_empty() {
                            0
                        } else {
                            low.min(self.view.visible.len() - 1)
                        };
                        if self.view.visible.is_empty() {
                            self.close_details();
                        } else {
                            self.reset_overlay_for_selection_change();
                        }
                        if !self.search.query.is_empty() {
                            self.run_search();
                        }
                        self.status_message = Some(format!(
                            "deleted {removed} line{} from {}",
                            if removed == 1 { "" } else { "s" },
                            self.source
                                .path()
                                .map(|path| path.display().to_string())
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
        let at = self.view.selected;
        self.apply_op_visible_range(PendingOp::Hide, at, at);
    }

    pub(crate) fn delete_current(&mut self) {
        let at = self.view.selected;
        self.apply_op_visible_range(PendingOp::Delete, at, at);
    }
}
