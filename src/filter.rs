use std::collections::HashSet;

use regex::{Regex, RegexBuilder};

use crate::config::CaseMode;
use crate::model::LogEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    /// Keep lines that match (`:filter in`).
    Include,
    /// Hide lines that match (`:filter out`).
    Exclude,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub kind: FilterKind,
    pub pattern: String,
    pub regex: Regex,
    pub enabled: bool,
}

impl Filter {
    pub fn new(kind: FilterKind, pattern: &str, case_mode: CaseMode) -> Result<Self, regex::Error> {
        let regex = compile_regex(pattern, case_mode)?;
        Ok(Self {
            kind,
            pattern: pattern.to_string(),
            regex,
            enabled: true,
        })
    }

    pub fn recompile(&mut self, case_mode: CaseMode) -> Result<(), regex::Error> {
        self.regex = compile_regex(&self.pattern, case_mode)?;
        Ok(())
    }

    pub fn matches(&self, entry: &LogEntry) -> bool {
        self.regex.is_match(&entry.raw)
    }

    pub fn label(&self) -> &'static str {
        match self.kind {
            FilterKind::Include => "in",
            FilterKind::Exclude => "out",
        }
    }
}

pub fn compile_regex(pattern: &str, case_mode: CaseMode) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(case_mode.ignore_case(pattern))
        .build()
}

/// lnav semantics:
/// - if any enabled include filters exist, a line must match at least one
/// - a line must not match any enabled exclude filter
pub fn entry_passes(filters: &[Filter], filtering_enabled: bool, entry: &LogEntry) -> bool {
    if !filtering_enabled || filters.is_empty() {
        return true;
    }

    let includes: Vec<&Filter> = filters
        .iter()
        .filter(|f| f.enabled && f.kind == FilterKind::Include)
        .collect();
    let excludes: Vec<&Filter> = filters
        .iter()
        .filter(|f| f.enabled && f.kind == FilterKind::Exclude)
        .collect();

    if !includes.is_empty() && !includes.iter().any(|f| f.matches(entry)) {
        return false;
    }
    if excludes.iter().any(|f| f.matches(entry)) {
        return false;
    }
    true
}

pub fn build_visible(
    entries: &[LogEntry],
    filters: &[Filter],
    filtering_enabled: bool,
    hidden: &HashSet<usize>,
) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            !hidden.contains(i) && entry_passes(filters, filtering_enabled, e)
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LineFormat, LogLevel};

    fn entry(raw: &str) -> LogEntry {
        LogEntry {
            line_no: 1,
            raw: raw.into(),
            format: LineFormat::Plain,
            level: LogLevel::Info,
            timestamp: None,
            timestamp_parsed: None,
            message: None,
            fields: Vec::new(),
        }
    }

    #[test]
    fn filter_in_requires_match_when_present() {
        let filters = vec![Filter::new(FilterKind::Include, "error", CaseMode::Insensitive).unwrap()];
        assert!(!entry_passes(&filters, true, &entry("info ok")));
        assert!(entry_passes(&filters, true, &entry("got error here")));
        assert!(entry_passes(&filters, true, &entry("got ERROR here")));
    }

    #[test]
    fn filter_out_hides_matches() {
        let filters = vec![Filter::new(FilterKind::Exclude, "spam", CaseMode::Insensitive).unwrap()];
        assert!(!entry_passes(&filters, true, &entry("spam message")));
        assert!(entry_passes(&filters, true, &entry("real message")));
    }

    #[test]
    fn include_and_exclude_combine() {
        let filters = vec![
            Filter::new(FilterKind::Include, "http", CaseMode::Insensitive).unwrap(),
            Filter::new(FilterKind::Exclude, "health", CaseMode::Insensitive).unwrap(),
        ];
        assert!(entry_passes(&filters, true, &entry("http request /api")));
        assert!(!entry_passes(&filters, true, &entry("http health")));
        assert!(!entry_passes(&filters, true, &entry("db query")));
    }

    #[test]
    fn smartcase_sensitive_when_uppercase() {
        let f = Filter::new(FilterKind::Include, "ERROR", CaseMode::Smart).unwrap();
        assert!(f.matches(&entry("got ERROR here")));
        assert!(!f.matches(&entry("got error here")));
    }

    #[test]
    fn smartcase_insensitive_when_lowercase() {
        let f = Filter::new(FilterKind::Include, "error", CaseMode::Smart).unwrap();
        assert!(f.matches(&entry("got ERROR here")));
        assert!(f.matches(&entry("got error here")));
    }
}
