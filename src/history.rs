use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::Config;

const MAX_ENTRIES: usize = 1000;

/// Persistent `:` command history with readline-style up/down navigation.
#[derive(Debug, Clone, Default)]
pub struct CommandHistory {
    /// Oldest → newest.
    entries: Vec<String>,
    /// Index into `entries` while browsing; `None` means the live buffer.
    cursor: Option<usize>,
    /// Buffer contents saved when leaving the live line for history.
    staging: Option<String>,
}

impl CommandHistory {
    pub fn load() -> Self {
        Self::load_from(&Self::default_path()).unwrap_or_default()
    }

    pub fn default_path() -> PathBuf {
        Config::data_dir().join("command_history")
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Ok(Self::default());
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
        })
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::default_path())
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

    /// Push a committed command. Skips empty and consecutive duplicates.
    /// Caller should `save` when persistence is desired.
    pub fn push(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return;
        }
        if self.entries.last().is_some_and(|last| last == cmd) {
            self.reset_navigation();
            return;
        }
        self.entries.push(cmd.to_string());
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
        let mut h = CommandHistory::default();
        h.push("  ");
        h.push("filter list");
        h.push("filter list");
        h.push("theme list");
        assert_eq!(h.entries(), &["filter list", "theme list"]);
    }

    #[test]
    fn up_down_restores_staging() {
        let mut h = CommandHistory::default();
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
        let path = dir.join("command_history");
        let mut h = CommandHistory::default();
        h.push("a");
        h.push("b");
        h.save_to(&path).unwrap();
        let loaded = CommandHistory::load_from(&path).unwrap();
        assert_eq!(loaded.entries(), &["a", "b"]);
        let _ = fs::remove_dir_all(&dir);
    }
}
