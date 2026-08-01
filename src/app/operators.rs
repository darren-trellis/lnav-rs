use std::collections::HashSet;

use super::{App, PendingOp, SidebarItem, ToggleAction};
use crate::object_span;

/// Map a source index through a delete that renumbers survivors.
///
/// Returns `None` if `old` itself was deleted; otherwise the new index after
/// removing every deleted index that was below `old`.
fn remap_index_after_delete(old: usize, deleted: &HashSet<usize>) -> Option<usize> {
    if deleted.contains(&old) {
        return None;
    }
    Some(old - deleted.iter().filter(|&&d| d < old).count())
}

fn remap_pinned_after_delete(pinned: &[usize], deleted: &HashSet<usize>) -> Vec<usize> {
    pinned
        .iter()
        .filter_map(|&index| remap_index_after_delete(index, deleted))
        .collect()
}

fn remap_hidden_after_delete(hidden: &HashSet<usize>, deleted: &HashSet<usize>) -> HashSet<usize> {
    hidden
        .iter()
        .filter_map(|&index| remap_index_after_delete(index, deleted))
        .collect()
}

impl App {
    pub fn start_or_repeat_filter_delete(&mut self) {
        if self.sidebar_len() == 0 {
            self.pending_op = None;
            self.count = None;
            self.status_message = Some("sidebar empty".into());
            return;
        }
        if self.pending_op == Some(PendingOp::DeleteFilter) {
            self.pending_op = None;
            self.count = None;
            self.delete_sidebar_selection();
            return;
        }
        self.pending_op = Some(PendingOp::DeleteFilter);
        self.status_message = None;
    }

    pub fn delete_sidebar_selection(&mut self) {
        match self.sidebar_selection() {
            Some(SidebarItem::Filter(index)) => self.delete_filter_at(index),
            Some(SidebarItem::Hidden(source)) => self.unhide_source(source, false),
            None => self.status_message = Some("sidebar empty".into()),
        }
    }

    pub fn delete_selected_filter(&mut self) {
        self.delete_sidebar_selection();
    }

    fn delete_filter_at(&mut self, index: usize) {
        if index >= self.filters.len() {
            self.status_message = Some("no such filter".into());
            return;
        }
        let removed = self.filters.remove(index);
        self.clamp_sidebar_selection();
        self.rebuild_visible(None);
        self.persist_session();
        self.status_message = Some(format!(
            "deleted filter-{} /{}/",
            removed.label(),
            removed.pattern
        ));
    }

    pub fn unhide_source(&mut self, source: usize, reveal: bool) {
        if !self.view.hidden.remove(&source) {
            self.status_message = Some("line not hidden".into());
            return;
        }
        self.rebuild_visible(Some(source));
        self.clamp_sidebar_selection();
        if reveal {
            if let Some(display) = self.display_of_source(source) {
                self.jump_to(display);
            }
            self.focus_list();
            self.status_message = Some(format!("revealed line {}", source + 1));
        } else {
            self.status_message = Some(format!("unhid line {}", source + 1));
        }
    }

    pub fn reveal_sidebar_selection(&mut self) {
        match self.sidebar_selection() {
            Some(SidebarItem::Hidden(source)) => self.unhide_source(source, true),
            Some(SidebarItem::Filter(_)) => {
                self.status_message = Some("select a hidden line".into());
            }
            None => self.status_message = Some("no hidden lines".into()),
        }
    }

    pub fn set_filter_enabled(&mut self, index: usize, action: ToggleAction) {
        if index >= self.filters.len() {
            self.status_message = Some("no such filter".into());
            return;
        }
        self.select_sidebar_item(SidebarItem::Filter(index));
        let enabled = match action {
            ToggleAction::On => true,
            ToggleAction::Off => false,
            ToggleAction::Toggle => !self.filters[index].enabled,
        };
        self.filters[index].enabled = enabled;
        let label = self.filters[index].label();
        let pattern = self.filters[index].pattern.clone();
        self.rebuild_visible(None);
        self.persist_session();
        self.status_message = Some(format!(
            "filter-{label} /{pattern}/: {}",
            if enabled { "on" } else { "off" },
        ));
    }

    pub fn set_selected_filter_enabled(&mut self, action: ToggleAction) {
        match self.sidebar_selection() {
            Some(SidebarItem::Filter(index)) => self.set_filter_enabled(index, action),
            Some(SidebarItem::Hidden(_)) => {
                self.status_message = Some("select a filter".into());
            }
            None => self.status_message = Some("no filters".into()),
        }
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
        if matches!(op, PendingOp::Delete)
            && self.is_sidebar_focused()
            && self.config.sidebar
        {
            self.start_or_repeat_sidebar_delete();
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

    /// `DD` in the sidebar: delete the selected hidden line, or all lines
    /// matching the selected filter.
    pub(crate) fn start_or_repeat_sidebar_delete(&mut self) {
        match self.sidebar_selection() {
            Some(SidebarItem::Hidden(_)) | Some(SidebarItem::Filter(_)) => {
                if self.pending_op == Some(PendingOp::Delete) {
                    self.pending_op = None;
                    self.delete_sidebar_selection_from_file();
                    return;
                }
                self.pending_op = Some(PendingOp::Delete);
                self.status_message = None;
            }
            None => {
                self.pending_op = None;
                self.count = None;
                self.status_message = Some("sidebar empty".into());
            }
        }
    }

    /// Permanently delete from the file based on the sidebar selection:
    /// hidden row → that line; filter → every line matching the filter.
    pub fn delete_sidebar_selection_from_file(&mut self) {
        match self.sidebar_selection() {
            Some(SidebarItem::Hidden(_)) => self.delete_sidebar_hidden_selection(),
            Some(SidebarItem::Filter(index)) => {
                let _ = self.take_count();
                self.delete_lines_matching_filter(index);
            }
            None => self.status_message = Some("sidebar empty".into()),
        }
    }

    /// Permanently delete the sidebar's selected hidden line(s) from the file.
    fn delete_sidebar_hidden_selection(&mut self) {
        let count = self.take_count();
        let items = self.sidebar_items();
        let start = self.sidebar_selected.min(items.len().saturating_sub(1));
        let mut indices = HashSet::new();
        let mut taken = 0usize;
        for item in items.into_iter().skip(start) {
            let SidebarItem::Hidden(source) = item else {
                continue;
            };
            for index in object_span::object_span(self.source.entries(), source) {
                indices.insert(index);
            }
            taken += 1;
            if taken >= count {
                break;
            }
        }
        if indices.is_empty() {
            self.status_message = Some("select a hidden line".into());
            return;
        }
        let mut indices: Vec<usize> = indices.into_iter().collect();
        indices.sort_unstable();
        self.delete_source_indices(&indices, None, None);
    }

    /// Delete every source record that matches the filter at `index`.
    fn delete_lines_matching_filter(&mut self, index: usize) {
        let (label, pattern, regex) = match self.filters.get(index) {
            Some(filter) => (
                filter.label(),
                filter.pattern.clone(),
                filter.regex.clone(),
            ),
            None => {
                self.status_message = Some("no such filter".into());
                return;
            }
        };
        let matched: HashSet<usize> = self
            .source
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| regex.is_match(&entry.raw))
            .map(|(i, _)| i)
            .collect();
        if matched.is_empty() {
            self.status_message = Some(format!("no lines match filter-{label} /{pattern}/"));
            return;
        }

        // Expand matches to whole JSON objects so pretty-printed records stay intact.
        let entries = self.source.entries();
        let mut indices = HashSet::new();
        let mut i = 0usize;
        while i < entries.len() {
            let span = object_span::object_span(entries, i);
            let end = *span.end();
            if span.clone().any(|idx| matched.contains(&idx)) {
                indices.extend(span);
            }
            i = end + 1;
        }
        let mut indices: Vec<usize> = indices.into_iter().collect();
        indices.sort_unstable();
        self.delete_source_indices(
            &indices,
            None,
            Some(format!("matching filter-{label} /{pattern}/")),
        );
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
                self.delete_source_indices(&indices, Some(low), None);
            }
        }
    }

    /// Delete source indices from the file and remap pin/hide state.
    ///
    /// `prefer_display` selects a list row after rebuild (list `DD`); `None`
    /// keeps the current selection clamped (sidebar delete).
    /// `detail` is an optional phrase inserted before `from <path>`.
    fn delete_source_indices(
        &mut self,
        indices: &[usize],
        prefer_display: Option<usize>,
        detail: Option<String>,
    ) {
        if indices.is_empty() {
            return;
        }
        if !self.source.is_file() {
            self.status_message = Some("cannot delete from stdin".into());
            return;
        }
        match self.source.delete_entries(indices) {
            Ok(removed) => {
                let deleted: HashSet<usize> = indices.iter().copied().collect();
                self.view.pinned = remap_pinned_after_delete(&self.view.pinned, &deleted);
                self.view.hidden = remap_hidden_after_delete(&self.view.hidden, &deleted);
                self.clamp_sidebar_selection();
                self.rebuild_visible(None);
                if self.display_len() == 0 {
                    self.view.selected = 0;
                    self.close_details();
                } else if let Some(low) = prefer_display {
                    self.view.selected = low.min(self.display_len() - 1);
                    self.reset_overlay_for_selection_change();
                } else {
                    self.view.selected = self.view.selected.min(self.display_len() - 1);
                    self.reset_overlay_for_selection_change();
                }
                if !self.search.query.is_empty() {
                    self.run_search();
                }
                let detail = detail.map(|d| format!(" {d}")).unwrap_or_default();
                self.status_message = Some(format!(
                    "deleted {removed} line{}{detail} from {}",
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
        if self.is_sidebar_focused() && self.config.sidebar {
            self.pending_op = None;
            self.delete_sidebar_selection_from_file();
            return;
        }
        if self.display_len() == 0 {
            self.pending_op = None;
            self.count = None;
            return;
        }
        let count = self.take_count();
        let at = self.view.selected;
        let end = (at + count - 1).min(self.display_len().saturating_sub(1));
        self.pending_op = None;
        self.apply_op_display_range(PendingOp::Delete, at, end);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_skips_deleted_and_shifts_survivors() {
        let deleted: HashSet<usize> = [0, 2].into_iter().collect();
        assert_eq!(remap_index_after_delete(0, &deleted), None);
        assert_eq!(remap_index_after_delete(1, &deleted), Some(0));
        assert_eq!(remap_index_after_delete(2, &deleted), None);
        assert_eq!(remap_index_after_delete(3, &deleted), Some(1));
        assert_eq!(remap_index_after_delete(4, &deleted), Some(2));
    }

    #[test]
    fn remap_preserves_pin_order() {
        let deleted: HashSet<usize> = [1].into_iter().collect();
        assert_eq!(
            remap_pinned_after_delete(&[3, 0, 1], &deleted),
            vec![2, 0]
        );
    }

    #[test]
    fn remap_keeps_unrelated_hides() {
        let deleted: HashSet<usize> = [0].into_iter().collect();
        let hidden: HashSet<usize> = [0, 2, 4].into_iter().collect();
        let remapped = remap_hidden_after_delete(&hidden, &deleted);
        assert!(!remapped.contains(&0));
        assert!(remapped.contains(&1)); // was 2
        assert!(remapped.contains(&3)); // was 4
        assert_eq!(remapped.len(), 2);
    }
}
