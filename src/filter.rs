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

    let mut has_include = false;
    let mut matches_include = false;
    for filter in filters.iter().filter(|filter| filter.enabled) {
        match filter.kind {
            FilterKind::Include => {
                has_include = true;
                matches_include |= filter.matches(entry);
            }
            FilterKind::Exclude if filter.matches(entry) => return false,
            FilterKind::Exclude => {}
        }
    }
    !has_include || matches_include
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
        .filter(|(i, e)| !hidden.contains(i) && entry_passes(filters, filtering_enabled, e))
        .map(|(i, _)| i)
        .collect()
}
