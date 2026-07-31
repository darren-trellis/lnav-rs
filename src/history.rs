use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;

const MAX_ENTRIES: usize = 1000;

/// Persistent line history with readline-style up/down navigation.
///
/// Used for `:` commands (`command_history`) and `/` searches (`search_history`).
#[derive(Debug, Clone, Default)]
pub struct History {
    /// Oldest → newest.
    entries: Vec<String>,
    /// Index into `entries` while browsing; `None` means the live buffer.
    cursor: Option<usize>,
    /// Buffer contents saved when leaving the live line for history.
    staging: Option<String>,
    /// Path used by [`Self::save`]. Empty for in-memory / test instances.
    path: PathBuf,
}

impl History {
    pub fn load_commands() -> Self {
        Self::load(Self::command_path())
    }

    pub fn load_searches() -> Self {
        Self::load(Self::search_path())
    }

    pub fn command_path() -> PathBuf {
        Config::data_dir().join("command_history")
    }

    pub fn search_path() -> PathBuf {
        Config::data_dir().join("search_history")
    }

    pub fn load(path: PathBuf) -> Self {
        match Self::load_from(&path) {
            Ok(mut h) => {
                h.path = path;
                h
            }
            Err(_) => Self {
                path,
                ..Self::default()
            },
        }
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self {
                path: path.to_path_buf(),
                ..Self::default()
            });
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut entries: Vec<String> = raw
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        if entries.len() > MAX_ENTRIES {
            let skip = entries.len() - MAX_ENTRIES;
            entries = entries.split_off(skip);
        }
        Ok(Self {
            entries,
            cursor: None,
            staging: None,
            path: path.to_path_buf(),
        })
    }

    pub fn save(&self) -> Result<()> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        self.save_to(&self.path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let body = self.entries.join("\n");
        let body = if body.is_empty() {
            body
        } else {
            format!("{body}\n")
        };
        fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = None;
        self.staging = None;
    }

    /// Push a committed line. Skips empty and consecutive duplicates.
    /// Caller should `save` when persistence is desired.
    pub fn push(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        if self.entries.last().is_some_and(|last| last == line) {
            self.reset_navigation();
            return;
        }
        self.entries.push(line.to_string());
        if self.entries.len() > MAX_ENTRIES {
            let skip = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(0..skip);
        }
        self.reset_navigation();
    }

    /// Move to an older entry. Returns the buffer to show, if history moved.
    pub fn up(&mut self, current: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        match self.cursor {
            None => {
                self.staging = Some(current.to_string());
                let idx = self.entries.len() - 1;
                self.cursor = Some(idx);
                Some(self.entries[idx].clone())
            }
            Some(0) => Some(self.entries[0].clone()),
            Some(i) => {
                let idx = i - 1;
                self.cursor = Some(idx);
                Some(self.entries[idx].clone())
            }
        }
    }

    /// Move toward newer entries / restore the staged live buffer.
    pub fn down(&mut self) -> Option<String> {
        let Some(i) = self.cursor else {
            return None;
        };
        if i + 1 < self.entries.len() {
            let idx = i + 1;
            self.cursor = Some(idx);
            Some(self.entries[idx].clone())
        } else {
            self.cursor = None;
            Some(self.staging.take().unwrap_or_default())
        }
    }

    #[cfg(test)]
    fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn push_skips_empty_and_consecutive_dupes() {
        let mut h = History::default();
        h.push("  ");
        h.push("filter list");
        h.push("filter list");
        h.push("theme list");
        assert_eq!(h.entries(), &["filter list", "theme list"]);
    }

    #[test]
    fn up_down_restores_staging() {
        let mut h = History::default();
        h.push("one");
        h.push("two");
        assert_eq!(h.up("draft").as_deref(), Some("two"));
        assert_eq!(h.up("draft").as_deref(), Some("one"));
        assert_eq!(h.up("draft").as_deref(), Some("one"));
        assert_eq!(h.down().as_deref(), Some("two"));
        assert_eq!(h.down().as_deref(), Some("draft"));
        assert_eq!(h.down(), None);
    }

    #[test]
    fn roundtrip_file() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-hist-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("search_history");
        let mut h = History::default();
        h.push("a");
        h.push("b");
        h.save_to(&path).unwrap();
        let loaded = History::load_from(&path).unwrap();
        assert_eq!(loaded.entries(), &["a", "b"]);
        let _ = fs::remove_dir_all(&dir);
    }
}
