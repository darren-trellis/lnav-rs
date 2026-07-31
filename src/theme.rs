use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

use crate::model::{FieldValue, LogLevel};

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub overlay_bg: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub border: Color,
    pub search_match: Color,
    pub dim: Color,
    pub timestamp: Color,
    pub key: Color,
    pub string: Color,
    pub number: Color,
    pub bool_color: Color,
    pub null: Color,
    pub levels: HashMap<LogLevel, Color>,
}

impl Theme {
    pub fn level_style(&self, level: LogLevel) -> Style {
        let fg = self
            .levels
            .get(&level)
            .copied()
            .unwrap_or(self.foreground);
        Style::default().fg(fg)
    }

    pub fn field_value_style(&self, value: &FieldValue) -> Style {
        match value {
            FieldValue::String(_) | FieldValue::Nested(_) => Style::default().fg(self.string),
            FieldValue::Number(_) => Style::default().fg(self.number),
            FieldValue::Bool(_) => Style::default().fg(self.bool_color),
            FieldValue::Null => Style::default().fg(self.null),
        }
    }

    pub fn selection_style(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.selection_fg)
            .add_modifier(Modifier::BOLD)
    }
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    name: String,
    colors: ThemeColors,
    levels: ThemeLevels,
    ui: ThemeUi,
}

#[derive(Debug, Deserialize)]
struct ThemeColors {
    background: String,
    foreground: String,
    selection_bg: String,
    selection_fg: String,
    overlay_bg: String,
    status_bg: String,
    status_fg: String,
    border: String,
    search_match: String,
    dim: String,
}

#[derive(Debug, Deserialize)]
struct ThemeLevels {
    trace: String,
    debug: String,
    info: String,
    warn: String,
    error: String,
    fatal: String,
    unknown: String,
}

#[derive(Debug, Deserialize)]
struct ThemeUi {
    timestamp: String,
    key: String,
    string: String,
    number: String,
    #[serde(rename = "bool")]
    bool_color: String,
    null: String,
}

impl Theme {
    pub fn builtin(name: &str) -> Result<Self> {
        let raw = match name {
            "default" | "catppuccin" => include_str!("../themes/catppuccin.toml"),
            "nord" => include_str!("../themes/nord.toml"),
            "tokyo-night" => include_str!("../themes/tokyo-night.toml"),
            other => bail!("unknown theme '{other}' (try: catppuccin, nord, tokyo-night)"),
        };
        Self::parse(raw)
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let file: ThemeFile = toml::from_str(raw).context("invalid theme toml")?;
        let mut levels = HashMap::new();
        levels.insert(LogLevel::Trace, parse_color(&file.levels.trace)?);
        levels.insert(LogLevel::Debug, parse_color(&file.levels.debug)?);
        levels.insert(LogLevel::Info, parse_color(&file.levels.info)?);
        levels.insert(LogLevel::Warn, parse_color(&file.levels.warn)?);
        levels.insert(LogLevel::Error, parse_color(&file.levels.error)?);
        levels.insert(LogLevel::Fatal, parse_color(&file.levels.fatal)?);
        levels.insert(LogLevel::Unknown, parse_color(&file.levels.unknown)?);

        Ok(Self {
            name: file.name,
            background: parse_color(&file.colors.background)?,
            foreground: parse_color(&file.colors.foreground)?,
            selection_bg: parse_color(&file.colors.selection_bg)?,
            selection_fg: parse_color(&file.colors.selection_fg)?,
            overlay_bg: parse_color(&file.colors.overlay_bg)?,
            status_bg: parse_color(&file.colors.status_bg)?,
            status_fg: parse_color(&file.colors.status_fg)?,
            border: parse_color(&file.colors.border)?,
            search_match: parse_color(&file.colors.search_match)?,
            dim: parse_color(&file.colors.dim)?,
            timestamp: parse_color(&file.ui.timestamp)?,
            key: parse_color(&file.ui.key)?,
            string: parse_color(&file.ui.string)?,
            number: parse_color(&file.ui.number)?,
            bool_color: parse_color(&file.ui.bool_color)?,
            null: parse_color(&file.ui.null)?,
            levels,
        })
    }

    pub fn available() -> &'static [&'static str] {
        &["catppuccin", "nord", "tokyo-night"]
    }
}

fn parse_color(s: &str) -> Result<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            return Ok(Color::Rgb(r, g, b));
        }
    }
    Color::from_str(s).map_err(|_| anyhow::anyhow!("invalid color '{s}'"))
}
