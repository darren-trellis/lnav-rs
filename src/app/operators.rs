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
        if self.display_len() == 0 {
            self.pending_op = None;
            self.count = None;
            return;
        }
        if self.pending_op == Some(op) {
            let count = self.take_count();
            let at = self.view.selected;
            let end = (at + count - 1).min(self.display_len().saturating_sub(1));
            self.pending_op = None;
            self.apply_op_display_range(op, at, end);
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
            self.apply_op_display_range(op, start, end);
        } else {
            if self.pending_op == Some(PendingOp::DeleteFilter) {
                self.pending_op = None;
            }
            motion(self);
        }
    }

    pub(crate) fn apply_op_display_range(&mut self, op: PendingOp, from: usize, to: usize) {
        if self.display_len() == 0 {
            return;
        }
        let low = from.min(to).min(self.display_len() - 1);
        let high = from.max(to).min(self.display_len() - 1);
        let mut indices = HashSet::new();
        for display in low..=high {
            let Some(source) = self.source_at_display(display) else {
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
                let prefer = self.source_at_display(low);
                for &index in &indices {
                    self.view.hidden.insert(index);
                    self.view.pinned.retain(|&pinned| pinned != index);
                }
                self.rebuild_visible(prefer);
                if self.display_len() == 0 {
                    self.view.selected = 0;
                    self.close_details();
                } else {
                    self.view.selected = self.view.selected.min(self.display_len() - 1);
                    self.reset_overlay_for_selection_change();
                }
                if !self.search.query.is_empty() {
                    self.run_search();
                }
                self.status_message = Some(format!(
                    "hidden {count} line{}  (:hide clear to restore)",
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
                        let deleted: HashSet<usize> = indices.iter().copied().collect();
                        self.view.hidden.clear();
                        self.view.pinned.retain(|index| !deleted.contains(index));
                        self.rebuild_visible(None);
                        if self.display_len() == 0 {
                            self.view.selected = 0;
                            self.close_details();
                        } else {
                            self.view.selected = low.min(self.display_len() - 1);
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
        if self.display_len() == 0 {
            self.pending_op = None;
            self.count = None;
            return;
        }
        let count = self.take_count();
        let at = self.view.selected;
        let end = (at + count - 1).min(self.display_len().saturating_sub(1));
        self.pending_op = None;
        self.apply_op_display_range(PendingOp::Hide, at, end);
    }

    pub(crate) fn delete_current(&mut self) {
        let at = self.view.selected;
        self.apply_op_display_range(PendingOp::Delete, at, at);
    }

    pub(crate) fn pin_current(&mut self) {
        if self.display_len() == 0 {
            self.pending_op = None;
            self.count = None;
            return;
        }
        let count = self.take_count();
        let at = self.view.selected;
        let end = (at + count - 1).min(self.display_len().saturating_sub(1));
        self.pending_op = None;
        self.toggle_pin_display_range(at, end);
    }

    pub(crate) fn clear_pins(&mut self) {
        let n = self.view.pinned.len();
        self.view.pinned.clear();
        self.rebuild_visible(None);
        if !self.search.query.is_empty() {
            self.run_search();
        }
        self.status_message = Some(format!("unpinned {n} line(s)"));
    }

    fn toggle_pin_display_range(&mut self, from: usize, to: usize) {
        if self.display_len() == 0 {
            return;
        }
        let low = from.min(to).min(self.display_len() - 1);
        let high = from.max(to).min(self.display_len() - 1);
        let mut indices = HashSet::new();
        for display in low..=high {
            let Some(source) = self.source_at_display(display) else {
                continue;
            };
            for index in object_span::object_span(self.source.entries(), source) {
                indices.insert(index);
            }
        }
        if indices.is_empty() {
            return;
        }

        let pinned_set: HashSet<usize> = self.view.pinned.iter().copied().collect();
        let all_pinned = indices.iter().all(|index| pinned_set.contains(index));
        let prefer = self.source_at_display(low);
        self.view.follow = false;

        if all_pinned {
            let count = indices.len();
            self.view.pinned.retain(|index| !indices.contains(index));
            self.rebuild_visible(prefer);
            if !self.search.query.is_empty() {
                self.run_search();
            }
            self.status_message = Some(format!(
                "unpinned {count} line{}",
                if count == 1 { "" } else { "s" }
            ));
            return;
        }

        let mut ordered: Vec<usize> = indices.iter().copied().collect();
        ordered.sort_unstable();
        let count = ordered.len();
        for index in ordered {
            if !self.view.pinned.contains(&index) {
                self.view.pinned.push(index);
            }
            self.view.hidden.remove(&index);
        }
        self.rebuild_visible(prefer);
        if !self.search.query.is_empty() {
            self.run_search();
        }
        self.status_message = Some(format!(
            "pinned {count} line{}  (:pin clear to restore)",
            if count == 1 { "" } else { "s" }
        ));
    }
}
