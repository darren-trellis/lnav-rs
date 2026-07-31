use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::filter::{Filter, FilterKind};
use crate::tail::LogSource;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Session {
    #[serde(default = "default_true")]
    pub filtering_enabled: bool,
    #[serde(default)]
    pub filters: Vec<SessionFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SessionFilter {
    /// `"in"` or `"out"`.
    pub kind: String,
    pub pattern: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Session {
    fn default() -> Self {
        Self {
            filtering_enabled: true,
            filters: Vec::new(),
        }
    }
}

impl Session {
    pub fn from_app(filters: &[Filter], filtering_enabled: bool) -> Self {
        Self {
            filtering_enabled,
            filters: filters
                .iter()
                .map(|f| SessionFilter {
                    kind: f.label().to_string(),
                    pattern: f.pattern.clone(),
                    enabled: f.enabled,
                })
                .collect(),
        }
    }

    pub fn into_filters(self, case_mode: crate::config::CaseMode) -> Result<(Vec<Filter>, bool)> {
        let mut out = Vec::with_capacity(self.filters.len());
        for f in self.filters {
            let kind = match f.kind.as_str() {
                "in" => FilterKind::Include,
                "out" => FilterKind::Exclude,
                other => anyhow::bail!("unknown filter kind '{other}' (expected in|out)"),
            };
            let mut filter = Filter::new(kind, &f.pattern, case_mode)
                .with_context(|| format!("invalid filter regex /{}/", f.pattern))?;
            filter.enabled = f.enabled;
            out.push(filter);
        }
        Ok((out, self.filtering_enabled))
    }
}

fn path_key(path: &Path) -> String {
    let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canon.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Session file for this source, if persistence is enabled in config.
pub fn session_path(source: &LogSource, config: &Config) -> Option<PathBuf> {
    if source.is_file() {
        if !config.session_filters {
            return None;
        }
        let path = source.path()?;
        return Some(Config::sessions_dir().join(format!("{}.toml", path_key(path))));
    }
    // stdin / pipe
    if !config.session_stdin {
        return None;
    }
    Some(Config::sessions_dir().join("stdin.toml"))
}

pub fn load(source: &LogSource, config: &Config) -> Result<Option<Session>> {
    let Some(path) = session_path(source, config) else {
        return Ok(None);
    };
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read session {}", path.display()))?;
    let session: Session =
        toml::from_str(&raw).with_context(|| format!("invalid session {}", path.display()))?;
    Ok(Some(session))
}

pub fn save(source: &LogSource, config: &Config, session: &Session) -> Result<()> {
    let Some(path) = session_path(source, config) else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    // Drop empty sessions so the store stays tidy.
    if session.filters.is_empty() && session.filtering_enabled {
        if path.is_file() {
            let _ = fs::remove_file(&path);
        }
        return Ok(());
    }
    let raw = toml::to_string_pretty(session).context("failed to serialize session")?;
    fs::write(&path, raw).with_context(|| format!("failed to write session {}", path.display()))?;
    Ok(())
}
