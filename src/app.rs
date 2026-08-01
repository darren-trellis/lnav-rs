use std::collections::HashSet;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::layout::Rect;
use regex::Regex;

use crate::completion::CompletionState;
use crate::config::Config;
use crate::details;
use crate::filter::{self, Filter};
use crate::history::History;
use crate::model::LogEntry;
use crate::session::{self, Session};
use crate::tail::{LogSource, RefreshOutcome};
use crate::theme::Theme;
use crate::ui;

mod input;
pub mod mouse;
mod operators;
mod search;

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
    /// Sidebar delete (`dd`: filter delete or unhide).
    DeleteFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleAction {
    On,
    Off,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Details,
    Sidebar,
}

/// Selectable row in the filters/hidden sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarItem {
    Filter(usize),
    Hidden(usize),
}

/// Modal opened by bare `:config set KEY` (list picker or freeform editor).
#[derive(Debug, Clone)]
pub enum ConfigModal {
    Picker(ConfigPicker),
    Editor(ConfigValueEditor),
}

#[derive(Debug, Clone)]
pub struct ConfigPicker {
    pub option_name: &'static str,
    pub values: Vec<String>,
    pub selected: usize,
    /// Committed value to restore on cancel (theme preview) / mark with `*`.
    pub previous_value: String,
    pub popup_area: Rect,
    pub list_area: Rect,
    pub list_start: usize,
}

#[derive(Debug, Clone)]
pub struct ConfigValueEditor {
    pub option_name: &'static str,
    pub previous_value: String,
    pub buffer: String,
    pub popup_area: Rect,
}

#[derive(Debug, Clone)]
pub struct HelpModal {
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub viewport_h: usize,
    pub viewport_w: usize,
    pub content_w: usize,
    pub popup_area: Rect,
}

/// Widget rects from the last frame, used for mouse hit-testing.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct HitAreas {
    pub(crate) list_inner: Rect,
    pub(crate) list_scrollbar: Rect,
    /// Number of sticky pinned rows currently drawn at the top of the list.
    pub(crate) list_pin_rows: usize,
    pub(crate) overlay: Rect,
    pub(crate) overlay_scrollbar: Rect,
    pub(crate) sidebar_inner: Rect,
    pub(crate) suggest_inner: Rect,
    pub(crate) suggest_start: usize,
    pub(crate) status: Rect,
}

#[derive(Debug, Clone, Copy)]
struct LastClick {
    at: Instant,
    vis_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollbarDrag {
    List,
    Overlay,
}

pub(crate) struct ViewState {
    pub(crate) hidden: HashSet<usize>,
    /// Source indices kept sticky at the top of the list (pin order).
    pub(crate) pinned: Vec<usize>,
    /// Scrollable body source indices (excludes pinned).
    pub(crate) visible: Vec<usize>,
    /// Cursor into `pinned ++ visible` (display index).
    pub(crate) selected: usize,
    /// Scroll offset into `visible` (body only).
    pub(crate) scroll: usize,
    pub(crate) follow: bool,
}

pub(crate) struct DetailsState {
    pub(crate) visible: bool,
    pub(crate) cursor: usize,
    pub(crate) scroll: usize,
    pub(crate) content_len: usize,
    pub(crate) viewport_height: usize,
    pub(crate) folded: HashSet<String>,
    pub(crate) help: bool,
}

pub(crate) struct SearchState {
    pub(crate) query: String,
    pub(crate) regex: Option<Regex>,
    pub(crate) error: Option<String>,
    pub(crate) matches: Vec<usize>,
    pub(crate) cursor: Option<usize>,
    pub(crate) in_details: bool,
    pub(crate) history: History,
}

pub(crate) struct CommandLineState {
    pub(crate) buffer: String,
    pub(crate) history: History,
    pub(crate) completions: CompletionState,
}

pub(crate) struct PointerState {
    pub(crate) hit: HitAreas,
    last_click: Option<LastClick>,
    scrollbar_drag: Option<ScrollbarDrag>,
}

pub struct App {
    pub source: LogSource,
    pub config: Config,
    pub config_path: PathBuf,
    pub theme: Theme,
    pub theme_index: usize,
    pub filters: Vec<Filter>,
    pub filtering_enabled: bool,
    pub(crate) view: ViewState,
    pub(crate) details: DetailsState,
    focus: Focus,
    /// Selected row index in the sidebar (filters + hidden lines).
    pub sidebar_selected: usize,
    pub sidebar_scroll: usize,
    pub sidebar_scroll_x: usize,
    pub list_scroll_x: usize,
    /// Full rendered list-row width from the last draw (for horizontal scroll).
    pub(crate) list_content_width: usize,
    pub input_mode: InputMode,
    pub pending_op: Option<PendingOp>,
    /// Visible-row anchor when the pending operator was started.
    pub op_anchor: usize,
    /// Vim-style count prefix being typed (`5` in `5j`).
    pub count: Option<usize>,
    pub config_modal: Option<ConfigModal>,
    pub help_modal: Option<HelpModal>,
    /// When true, config setters apply for picker preview without autosave.
    pub(crate) config_preview: bool,
    pub(crate) pointer: PointerState,
    pub(crate) search: SearchState,
    pub(crate) command_line: CommandLineState,
    pub status_message: Option<String>,
    pub should_quit: bool,
    _config_watcher: Option<RecommendedWatcher>,
    config_notify_rx: Receiver<()>,
    suppress_config_reload_until: Option<Instant>,
}

impl App {
    pub fn new(source: LogSource, config: Config, config_path: PathBuf) -> Result<Self> {
        let overrides = config.theme_overrides();
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

        let (config_watcher, config_notify_rx) = match watch_config_file(&config_path) {
            Ok((watcher, rx)) => (Some(watcher), rx),
            Err(err) => {
                status_message = Some(match status_message {
                    Some(prev) => format!("{prev}; config watch: {err:#}"),
                    None => format!("config watch: {err:#}"),
                });
                let (_tx, rx) = mpsc::channel();
                (None, rx)
            }
        };

        let mut app = Self {
            source,
            view: ViewState {
                hidden: HashSet::new(),
                pinned: Vec::new(),
                visible: Vec::new(),
                selected: 0,
                scroll: 0,
                follow: config.follow,
            },
            details: DetailsState {
                visible: false,
                cursor: 0,
                scroll: 0,
                content_len: 0,
                viewport_height: 0,
                folded: HashSet::new(),
                help: false,
            },
            config,
            config_path,
            theme,
            theme_index,
            filters,
            filtering_enabled,
            focus: Focus::List,
            sidebar_selected: 0,
            sidebar_scroll: 0,
            sidebar_scroll_x: 0,
            list_scroll_x: 0,
            list_content_width: 0,
            input_mode: InputMode::Normal,
            pending_op: None,
            op_anchor: 0,
            count: None,
            config_modal: None,
            help_modal: None,
            config_preview: false,
            pointer: PointerState {
                hit: HitAreas::default(),
                last_click: None,
                scrollbar_drag: None,
            },
            search: SearchState {
                query: String::new(),
                regex: None,
                error: None,
                matches: Vec::new(),
                cursor: None,
                in_details: false,
                history: History::load_searches(),
            },
            command_line: CommandLineState {
                buffer: String::new(),
                history: History::load_commands(),
                completions: CompletionState::default(),
            },
            status_message,
            should_quit: false,
            _config_watcher: config_watcher,
            config_notify_rx,
            suppress_config_reload_until: None,
        };
        app.rebuild_visible(None);
        if app.view.follow && app.display_len() > 0 {
            app.view.selected = app.display_len() - 1;
        }
        Ok(app)
    }

    pub fn persist_session(&self) {
        let session = Session::from_app(&self.filters, self.filtering_enabled);
        let _ = session::save(&self.source, &self.config, &session);
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        let src = self.source_at_display(self.view.selected)?;
        self.source.entries().get(src)
    }

    pub fn pin_count(&self) -> usize {
        self.view.pinned.len()
    }

    pub fn display_len(&self) -> usize {
        self.view.pinned.len() + self.view.visible.len()
    }

    pub fn visible_len(&self) -> usize {
        self.display_len()
    }

    pub fn hidden_count(&self) -> usize {
        self.source
            .len()
            .saturating_sub(self.view.visible.len() + self.view.pinned.len())
    }

    pub fn source_at_display(&self, display: usize) -> Option<usize> {
        let pin_count = self.view.pinned.len();
        if display < pin_count {
            self.view.pinned.get(display).copied()
        } else {
            self.view.visible.get(display - pin_count).copied()
        }
    }

    pub fn display_of_source(&self, source: usize) -> Option<usize> {
        if let Some(index) = self.view.pinned.iter().position(|&pinned| pinned == source) {
            return Some(index);
        }
        self.view
            .visible
            .iter()
            .position(|&visible| visible == source)
            .map(|index| self.view.pinned.len() + index)
    }

    pub fn is_display_pinned(&self, display: usize) -> bool {
        display < self.view.pinned.len()
    }

    /// Layout of the sticky pin band + scrollable body for a viewport height.
    /// Returns `(pin_rows, separator_rows, body_height)`.
    pub fn list_band_layout(&self, viewport: usize) -> (usize, usize, usize) {
        if viewport == 0 {
            return (0, 0, 0);
        }
        let pins = self.pin_count();
        if pins == 0 {
            return (0, 0, viewport);
        }
        if self.view.visible.is_empty() {
            return (pins.min(viewport), 0, 0);
        }
        let mut pin_rows = pins.min(viewport.saturating_sub(1));
        let rem = viewport.saturating_sub(pin_rows);
        if pin_rows > 0 && rem >= 2 {
            (pin_rows, 1, rem - 1)
        } else if pin_rows > 0 && rem == 1 {
            // Prefer a body row over a separator when the viewport is tight.
            (pin_rows, 0, 1)
        } else {
            pin_rows = pins.min(viewport);
            (pin_rows, 0, viewport.saturating_sub(pin_rows))
        }
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn is_list_focused(&self) -> bool {
        self.focus() == Focus::List
    }

    pub fn is_details_focused(&self) -> bool {
        self.focus() == Focus::Details
    }

    pub fn is_sidebar_focused(&self) -> bool {
        self.focus() == Focus::Sidebar
    }

    pub fn focus_list(&mut self) {
        self.set_focus(Focus::List);
    }

    pub(crate) fn focus_details(&mut self) {
        if self.details.visible {
            self.set_focus(Focus::Details);
        }
    }

    fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn rebuild_visible(&mut self, prefer_source_idx: Option<usize>) {
        let prefer = prefer_source_idx.or_else(|| self.source_at_display(self.view.selected));
        let entry_count = self.source.len();
        self.view.pinned.retain(|&index| index < entry_count);
        let pinned: HashSet<usize> = self.view.pinned.iter().copied().collect();
        self.view.visible = filter::build_visible(
            self.source.entries(),
            &self.filters,
            self.filtering_enabled,
            &self.view.hidden,
        )
        .into_iter()
        .filter(|index| !pinned.contains(index))
        .collect();

        if self.display_len() == 0 {
            self.view.selected = 0;
            self.view.scroll = 0;
            return;
        }

        if let Some(src) = prefer
            && let Some(pos) = self.display_of_source(src)
        {
            self.view.selected = pos;
            return;
        }
        self.view.selected = self.view.selected.min(self.display_len() - 1);
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        let _ = execute!(stdout(), EnableMouseCapture);
        let result = self.run_loop(terminal);
        let _ = execute!(stdout(), DisableMouseCapture);
        self.close_config_modal(false);
        self.close_help_modal();
        result
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        loop {
            match self.source.refresh() {
                Ok(outcome) if outcome.changed() => {
                    let reset = outcome.reset();
                    let prefer = if reset {
                        self.view.hidden.clear();
                        self.view.pinned.clear();
                        self.view.selected = 0;
                        self.view.scroll = 0;
                        self.reset_overlay_for_selection_change();
                        None
                    } else {
                        self.source_at_display(self.view.selected)
                    };
                    self.rebuild_visible(prefer);
                    if self.view.follow && self.display_len() > 0 {
                        self.view.selected = self.display_len() - 1;
                    }
                    if self.display_len() == 0 {
                        self.close_details();
                    }
                    if !self.search.query.is_empty() {
                        self.run_search();
                    }
                }
                Ok(RefreshOutcome::Unchanged) => {}
                Ok(_) => {}
                Err(err) => self.status_message = Some(format!("refresh failed: {err:#}")),
            }

            if self.poll_config_file_changed() {
                match self.reload_config() {
                    Ok(()) => self.status_message = Some("config reloaded".into()),
                    Err(err) => self.status_message = Some(format!("config reload: {err:#}")),
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

    pub fn set_details(&mut self, action: ToggleAction) {
        match action {
            ToggleAction::On => self.open_details(),
            ToggleAction::Off => {
                self.cancel_pending_op();
                self.close_details();
            }
            ToggleAction::Toggle => self.toggle_details(),
        }
    }

    /// Apply a view action to the focused pane (details or sidebar).
    /// Returns true when the sidebar visibility may have changed (for autosave).
    pub fn set_current_view(&mut self, action: ToggleAction) -> bool {
        if self.is_sidebar_focused() && self.config.sidebar {
            self.set_sidebar(action);
            return true;
        }
        if self.is_details_focused() && self.details.visible {
            self.set_details(action);
            return false;
        }
        if !matches!(action, ToggleAction::Off) {
            self.status_message = Some("no focused view".into());
        }
        false
    }

    pub fn set_overlay_focus(&mut self, action: ToggleAction) {
        self.cancel_pending_op();
        match action {
            ToggleAction::On => {
                if !self.details.visible {
                    self.open_details();
                } else {
                    self.focus_details();
                }
            }
            ToggleAction::Off => {
                self.focus_list();
                self.details.help = false;
            }
            ToggleAction::Toggle => self.cycle_focus(),
        }
    }

    /// Cycle focus across list → details (if open) → sidebar (if open) → list.
    pub fn cycle_focus(&mut self) {
        self.cancel_pending_op();
        if self.is_sidebar_focused() {
            self.focus_list();
            self.details.help = false;
            return;
        }
        if self.is_details_focused() && self.details.visible {
            self.focus_list();
            self.details.help = false;
            if self.config.sidebar {
                self.focus_sidebar();
            }
            return;
        }
        // List focused.
        if self.details.visible {
            self.focus_details();
        } else if self.config.sidebar {
            self.focus_sidebar();
        }
    }

    pub fn set_sidebar(&mut self, action: ToggleAction) {
        self.cancel_pending_op();
        let on = match action {
            ToggleAction::On => true,
            ToggleAction::Off => false,
            ToggleAction::Toggle => !self.config.sidebar,
        };
        self.config.sidebar = on;
        if on {
            self.focus_sidebar();
            self.status_message = Some("sidebar: on".into());
        } else {
            if self.is_sidebar_focused() {
                self.focus_list();
            }
            self.status_message = Some("sidebar: off".into());
        }
    }

    pub fn focus_sidebar(&mut self) {
        if !self.config.sidebar {
            return;
        }
        self.details.help = false;
        self.set_focus(Focus::Sidebar);
        let len = self.sidebar_len();
        if len == 0 {
            self.sidebar_selected = 0;
        } else {
            self.sidebar_selected = self.sidebar_selected.min(len - 1);
        }
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items: Vec<SidebarItem> = (0..self.filters.len()).map(SidebarItem::Filter).collect();
        let mut hidden: Vec<usize> = self.view.hidden.iter().copied().collect();
        hidden.sort_unstable();
        items.extend(hidden.into_iter().map(SidebarItem::Hidden));
        items
    }

    pub fn sidebar_len(&self) -> usize {
        self.filters.len() + self.view.hidden.len()
    }

    pub fn sidebar_selection(&self) -> Option<SidebarItem> {
        self.sidebar_items().get(self.sidebar_selected).copied()
    }

    pub fn select_sidebar_item(&mut self, item: SidebarItem) {
        if let Some(idx) = self.sidebar_items().iter().position(|row| *row == item) {
            self.sidebar_selected = idx;
        }
    }

    pub fn clamp_sidebar_selection(&mut self) {
        let len = self.sidebar_len();
        if len == 0 {
            self.sidebar_selected = 0;
            self.sidebar_scroll = 0;
            self.sidebar_scroll_x = 0;
        } else if self.sidebar_selected >= len {
            self.sidebar_selected = len - 1;
        }
    }

    pub fn sidebar_item_text(&self, item: SidebarItem) -> String {
        match item {
            SidebarItem::Filter(fi) => {
                let filter = &self.filters[fi];
                let mark = if filter.enabled { "*" } else { " " };
                format!("{mark}{fi}:{} /{}/", filter.label(), filter.pattern)
            }
            SidebarItem::Hidden(src) => {
                let preview = self
                    .source
                    .entries()
                    .get(src)
                    .map(|e| e.raw.replace('\n', " "))
                    .unwrap_or_default();
                format!("·{} {preview}", src + 1)
            }
        }
    }

    pub fn sidebar_content_width(&self) -> usize {
        use unicode_width::UnicodeWidthStr;
        self.sidebar_items()
            .into_iter()
            .map(|item| UnicodeWidthStr::width(self.sidebar_item_text(item).as_str()))
            .max()
            .unwrap_or(0)
    }

    pub fn clamp_sidebar_scroll_x(&mut self, viewport_width: usize, content_width: usize) {
        let max = content_width.saturating_sub(viewport_width.max(1));
        if self.sidebar_scroll_x > max {
            self.sidebar_scroll_x = max;
        }
    }

    pub fn scroll_sidebar_x(&mut self, delta: isize) {
        let viewport = self.pointer.hit.sidebar_inner.width.max(1) as usize;
        let content_w = self.sidebar_content_width();
        let max = content_w.saturating_sub(viewport);
        self.sidebar_scroll_x =
            (self.sidebar_scroll_x as isize + delta).clamp(0, max as isize) as usize;
    }

    pub fn clamp_list_scroll_x(&mut self, viewport_width: usize, content_width: usize) {
        let max = content_width.saturating_sub(viewport_width.max(1));
        if self.list_scroll_x > max {
            self.list_scroll_x = max;
        }
    }

    pub fn scroll_list_x(&mut self, delta: isize) {
        let viewport = self.pointer.hit.list_inner.width.max(1) as usize;
        let max = self.list_content_width.saturating_sub(viewport);
        self.list_scroll_x =
            (self.list_scroll_x as isize + delta).clamp(0, max as isize) as usize;
    }

    pub fn toggle_details(&mut self) {
        self.cancel_pending_op();
        if !self.details.visible {
            self.open_details();
        } else if !self.is_details_focused() {
            self.focus_details();
        } else {
            self.close_details();
        }
    }

    pub fn open_details(&mut self) {
        self.cancel_pending_op();
        if self.selected_entry().is_none() {
            return;
        }
        if !self.details.visible {
            self.details.visible = true;
            self.details.cursor = 0;
            self.details.scroll = 0;
        }
        self.focus_details();
    }

    pub fn close_details(&mut self) {
        self.details.visible = false;
        if self.is_details_focused() {
            self.focus_list();
        }
        self.details.help = false;
        self.details.cursor = 0;
        self.details.scroll = 0;
        if self.search.in_details {
            self.search.in_details = false;
            if !self.search.query.is_empty() {
                self.run_search();
            }
        }
    }

    pub fn set_overlay_help(&mut self, action: ToggleAction) {
        if !(self.details.visible && self.is_details_focused()) {
            return;
        }
        self.details.help = match action {
            ToggleAction::On => true,
            ToggleAction::Off => false,
            ToggleAction::Toggle => !self.details.help,
        };
    }

    pub fn copy_overlay_value(&mut self) {
        if !self.details.visible || !self.is_details_focused() {
            self.status_message = Some("focus details first (Enter)".into());
            return;
        }
        let cursor = self.details.cursor;
        let value = {
            let Some(entry) = self.selected_entry() else {
                return;
            };
            let lines =
                details::build_lines(entry, &self.theme, &self.config, &self.details.folded);
            lines.get(cursor).and_then(|l| l.copy_value.clone())
        };
        let Some(value) = value else {
            self.status_message = Some("nothing to copy".into());
            return;
        };
        match copy_to_clipboard(&value) {
            Ok(()) => {
                let preview = crate::text::truncate_width(&value.replace('\n', " "), 60);
                self.status_message = Some(format!("copied {preview}"));
            }
            Err(err) => {
                self.status_message = Some(format!("copy failed: {err:#}"));
            }
        }
    }

    pub fn set_overlay_fold(&mut self, action: ToggleAction) {
        if !self.details.visible || !self.is_details_focused() {
            self.status_message = Some("focus details first (Enter)".into());
            return;
        }
        let cursor = self.details.cursor;
        let (foldable, path) = {
            let Some(entry) = self.selected_entry() else {
                return;
            };
            let lines =
                details::build_lines(entry, &self.theme, &self.config, &self.details.folded);
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
        let currently_folded = self.details.folded.contains(&key);
        let fold = match action {
            ToggleAction::On => true,
            ToggleAction::Off => false,
            ToggleAction::Toggle => !currently_folded,
        };
        if fold {
            self.details.folded.insert(key);
            self.status_message = Some(format!("folded {label}"));
        } else {
            self.details.folded.remove(&key);
            self.status_message = Some(format!("unfolded {label}"));
        }
        let new_len = self
            .selected_entry()
            .map(|entry| {
                details::build_lines(entry, &self.theme, &self.config, &self.details.folded).len()
            })
            .unwrap_or(0);
        self.details.content_len = new_len;
        if new_len == 0 {
            self.details.cursor = 0;
        } else {
            self.details.cursor = self.details.cursor.min(new_len - 1);
        }
        self.ensure_overlay_cursor_visible(true);
        if self.search.in_details && !self.search.query.is_empty() {
            self.run_search();
        }
    }

    fn reset_overlay_for_selection_change(&mut self) {
        self.details.cursor = 0;
        self.details.scroll = 0;
        self.details.folded.clear();
    }

    pub(crate) fn maybe_autosave(&mut self) {
        if self.config_preview || !self.config.autosave {
            return;
        }
        let msg = self.status_message.clone();
        if let Err(err) = self.save_config() {
            self.status_message = Some(format!("error: {err:#}"));
        } else {
            self.status_message = msg;
        }
    }

    pub(crate) fn save_config(&mut self) -> anyhow::Result<PathBuf> {
        self.config.theme.set_name(self.theme.name.clone());
        self.config.follow = self.view.follow;
        let path = self.config.write_to(&self.config_path)?;
        self.rebind_config_watcher();
        self.suppress_config_reload(Duration::from_millis(750));
        Ok(path)
    }

    fn rebind_config_watcher(&mut self) {
        match watch_config_file(&self.config_path) {
            Ok((watcher, rx)) => {
                self._config_watcher = Some(watcher);
                self.config_notify_rx = rx;
            }
            Err(_) => {}
        }
    }

    /// Reload settings from `config_path` and apply them to the running session.
    pub fn reload_config(&mut self) -> anyhow::Result<()> {
        if !self.config_path.is_file() {
            anyhow::bail!("no config file at {}", self.config_path.display());
        }
        let (config, _) = Config::load_from(&self.config_path)?;
        let overrides = config.theme_overrides();
        let theme = Theme::resolve_with_overrides(config.theme.name(), &overrides)
            .with_context(|| format!("theme '{}'", config.theme.name()))?;
        let theme_index = Theme::list_names()
            .iter()
            .position(|t| t == config.theme.name() || t == &theme.name)
            .unwrap_or(0);

        self.close_config_modal(false);
        self.close_help_modal();
        self.config = config;
        self.theme = theme;
        self.theme_index = theme_index;
        self.view.follow = self.config.follow;
        if !self.config.sidebar && self.is_sidebar_focused() {
            self.focus_list();
        }
        if let Some(err) = self.apply_case_mode() {
            anyhow::bail!("{err}");
        }
        Ok(())
    }

    fn poll_config_file_changed(&mut self) -> bool {
        let mut changed = false;
        while self.config_notify_rx.try_recv().is_ok() {
            changed = true;
        }
        if !changed {
            return false;
        }
        if !self.config.autoreload {
            return false;
        }
        if let Some(until) = self.suppress_config_reload_until {
            if Instant::now() < until {
                return false;
            }
            self.suppress_config_reload_until = None;
        }
        true
    }

    fn suppress_config_reload(&mut self, for_duration: Duration) {
        self.suppress_config_reload_until = Some(Instant::now() + for_duration);
        while self.config_notify_rx.try_recv().is_ok() {}
    }
}

fn watch_config_file(path: &Path) -> Result<(RecommendedWatcher, Receiver<()>)> {
    let watch_name = path
        .file_name()
        .map(|n| n.to_os_string())
        .context("config path has no file name")?;
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) | EventKind::Any
            )
            && event
                .paths
                .iter()
                .any(|p| p.file_name().is_some_and(|n| n == watch_name))
        {
            let _ = tx.send(());
        }
    })
    .context("failed to create config file watcher")?;

    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        if parent.is_dir() {
            watcher
                .watch(parent, RecursiveMode::NonRecursive)
                .with_context(|| format!("failed to watch {}", parent.display()))?;
        } else if path.is_file() {
            watcher
                .watch(path, RecursiveMode::NonRecursive)
                .with_context(|| format!("failed to watch {}", path.display()))?;
        }
        // Parent may not exist yet; `:config save` creates it.
    } else if path.is_file() {
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .with_context(|| format!("failed to watch {}", path.display()))?;
    }

    Ok((watcher, rx))
}

fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
