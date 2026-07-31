use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::model::LogEntry;
use crate::tail::LogSource;
use crate::theme::Theme;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

pub struct App {
    pub source: LogSource,
    pub theme: Theme,
    pub theme_index: usize,
    pub selected: usize,
    pub scroll: usize,
    pub follow: bool,
    pub show_overlay: bool,
    pub input_mode: InputMode,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub search_cursor: Option<usize>,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(path: PathBuf, theme_name: &str) -> Result<Self> {
        let source = LogSource::open(&path)?;
        let themes = Theme::available();
        let theme_index = themes
            .iter()
            .position(|t| *t == theme_name || (*t == "catppuccin" && theme_name == "default"))
            .unwrap_or(0);
        let theme = Theme::builtin(themes[theme_index])?;

        let mut app = Self {
            source,
            theme,
            theme_index,
            selected: 0,
            scroll: 0,
            follow: true,
            show_overlay: false,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_cursor: None,
            status_message: None,
            should_quit: false,
        };
        if app.source.len() > 0 {
            app.selected = app.source.len() - 1;
        }
        Ok(app)
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        self.source.entries().get(self.selected)
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        loop {
            let added = self.source.refresh().unwrap_or(0);
            if added > 0 && self.follow {
                self.selected = self.source.len().saturating_sub(1);
            }

            terminal.draw(|frame| ui::draw(frame, self))?;

            if self.should_quit {
                break;
            }

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key(key);
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Search => self.handle_search_key(key),
            InputMode::Normal => self.handle_normal_key(key),
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
                self.run_search();
                if let Some(idx) = self.search_matches.first().copied() {
                    self.jump_to(idx);
                    self.search_cursor = Some(0);
                    self.status_message = Some(format!(
                        "{}/{} matches",
                        1,
                        self.search_matches.len()
                    ));
                } else {
                    self.status_message = Some("no matches".into());
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if key.code == KeyCode::Char('c') {
                self.should_quit = true;
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::PageDown | KeyCode::Char(' ') => self.move_selection(20),
            KeyCode::PageUp => self.move_selection(-20),
            KeyCode::Home | KeyCode::Char('g') => {
                self.follow = false;
                self.jump_to(0);
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.follow = true;
                if self.source.len() > 0 {
                    self.jump_to(self.source.len() - 1);
                }
            }
            KeyCode::Enter => {
                if self.selected_entry().is_some() {
                    self.show_overlay = !self.show_overlay;
                }
            }
            KeyCode::Esc => {
                if self.show_overlay {
                    self.show_overlay = false;
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
                self.status_message = Some("search: ".into());
            }
            KeyCode::Char('n') => self.next_match(1),
            KeyCode::Char('N') => self.next_match(-1),
            KeyCode::Char('f') => {
                self.follow = !self.follow;
                if self.follow && self.source.len() > 0 {
                    self.jump_to(self.source.len() - 1);
                }
                self.status_message = Some(if self.follow {
                    "follow: on".into()
                } else {
                    "follow: off".into()
                });
            }
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('?') => {
                self.status_message = Some(
                    "j/k move · Enter details · / search · n/N next · f follow · t theme · q quit"
                        .into(),
                );
            }
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.source.len() == 0 {
            return;
        }
        self.follow = false;
        let next = (self.selected as isize + delta)
            .clamp(0, self.source.len() as isize - 1) as usize;
        self.selected = next;
    }

    fn jump_to(&mut self, idx: usize) {
        if self.source.len() == 0 {
            return;
        }
        self.selected = idx.min(self.source.len() - 1);
    }

    fn run_search(&mut self) {
        let q = self.search_query.to_ascii_lowercase();
        self.search_matches = self
            .source
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| e.raw.to_ascii_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
    }

    fn next_match(&mut self, dir: isize) {
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
        self.jump_to(self.search_matches[next]);
        self.status_message = Some(format!("{}/{} matches", next + 1, len));
    }

    fn cycle_theme(&mut self) {
        let themes = Theme::available();
        self.theme_index = (self.theme_index + 1) % themes.len();
        if let Ok(theme) = Theme::builtin(themes[self.theme_index]) {
            self.status_message = Some(format!("theme: {}", theme.name));
            self.theme = theme;
        }
    }

    pub fn ensure_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 || self.source.len() == 0 {
            return;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + viewport_height {
            self.scroll = self.selected + 1 - viewport_height;
        }
    }
}
