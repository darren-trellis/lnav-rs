use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::model::{FieldValue, LogLevel};

/// Parsed color with optional background (badge / highlight).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tone {
    pub fg: Color,
    pub bg: Option<Color>,
}

impl Tone {
    pub fn fg_only(fg: Color) -> Self {
        Self { fg, bg: None }
    }
}

/// Color in theme/config TOML: `"#hex"` or `{ fg = "...", bg = "..." }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ColorSpec {
    Fg(String),
    FgBg(ColorSpecFgBg),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ColorSpecFgBg {
    pub fg: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
}

impl ColorSpec {
    pub fn parse(&self) -> Result<Tone> {
        match self {
            Self::Fg(fg) => Ok(Tone::fg_only(parse_color(fg)?)),
            Self::FgBg(ColorSpecFgBg { fg, bg }) => Ok(Tone {
                fg: parse_color(fg)?,
                bg: bg.as_deref().map(parse_color).transpose()?,
            }),
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.parse().map(|_| ())
    }
}

fn apply_optional<T, P: ?Sized>(
    target: &mut T,
    patch: Option<&P>,
    parse: impl FnOnce(&P) -> Result<T>,
) -> Result<()> {
    if let Some(value) = patch {
        *target = parse(value)?;
    }
    Ok(())
}

fn apply_level(
    levels: &mut HashMap<LogLevel, Tone>,
    level: LogLevel,
    patch: Option<&ColorSpec>,
) -> Result<()> {
    if let Some(value) = patch {
        levels.insert(level, value.parse()?);
    }
    Ok(())
}

fn parse_levels(levels: &ThemeLevels) -> Result<HashMap<LogLevel, Tone>> {
    [
        (LogLevel::Trace, &levels.trace),
        (LogLevel::Debug, &levels.debug),
        (LogLevel::Info, &levels.info),
        (LogLevel::Warn, &levels.warn),
        (LogLevel::Error, &levels.error),
        (LogLevel::Fatal, &levels.fatal),
        (LogLevel::Unknown, &levels.unknown),
    ]
    .into_iter()
    .map(|(level, spec)| Ok((level, spec.parse()?)))
    .collect()
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub background: Color,
    pub foreground: Tone,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub overlay_bg: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub border: Tone,
    /// Border for the focused pane (list or details).
    pub window_focus_border: Tone,
    pub search_match: Tone,
    pub dim: Tone,
    pub timestamp: Tone,
    pub key: Tone,
    pub string: Tone,
    pub number: Tone,
    pub bool_color: Tone,
    pub null: Tone,
    /// Vertical rules between list columns (`│` × `column_border_width`).
    pub column_border: Tone,
    /// `0` = plain space between columns (default); `N` = N× `│`.
    pub column_border_width: usize,
    /// Spaces around the column border rule (`1` or `{ left, right }`).
    pub column_border_padding: crate::config::Padding,
    pub levels: HashMap<LogLevel, Tone>,
}

impl Theme {
    pub fn tone_style(&self, tone: Tone, fallback_bg: Color) -> Style {
        Style::default()
            .fg(tone.fg)
            .bg(tone.bg.unwrap_or(fallback_bg))
    }

    /// Style for a tone; omits bg when unset (inherits from parent widget).
    pub fn tone_fg_style(&self, tone: Tone) -> Style {
        let mut style = Style::default().fg(tone.fg);
        if let Some(bg) = tone.bg {
            style = style.bg(bg);
        }
        style
    }

    pub fn level_color(&self, level: LogLevel) -> Tone {
        self.levels
            .get(&level)
            .copied()
            .unwrap_or_else(|| Tone::fg_only(self.foreground.fg))
    }

    pub fn level_style(&self, level: LogLevel) -> Style {
        self.tone_fg_style(self.level_color(level))
    }

    pub fn field_value_tone(&self, value: &FieldValue) -> Tone {
        match value {
            FieldValue::String(_) | FieldValue::Nested(_) => self.string,
            FieldValue::Number(_) => self.number,
            FieldValue::Bool(_) => self.bool_color,
            FieldValue::Null => self.null,
        }
    }

    pub fn field_value_style(&self, value: &FieldValue, fallback_bg: Color) -> Style {
        self.tone_style(self.field_value_tone(value), fallback_bg)
    }

    pub fn selection_style(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.selection_fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for regex search matches.
    ///
    /// With `{ fg, bg }`, uses those colors. With a bare accent color, treats it as
    /// the highlight background and uses `contrast_fg` for the text (classic reverse).
    pub fn search_highlight_style(&self, contrast_fg: Color) -> Style {
        let tone = self.search_match;
        match tone.bg {
            Some(bg) => Style::default()
                .fg(tone.fg)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
            None => Style::default()
                .fg(contrast_fg)
                .bg(tone.fg)
                .add_modifier(Modifier::BOLD),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    name: String,
    colors: ThemeColors,
    levels: ThemeLevels,
    ui: ThemeUi,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeColors {
    background: String,
    foreground: ColorSpec,
    selection_bg: String,
    selection_fg: String,
    overlay_bg: String,
    status_bg: String,
    status_fg: String,
    border: ColorSpec,
    window_focus_border: ColorSpec,
    search_match: ColorSpec,
    dim: ColorSpec,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeLevels {
    trace: ColorSpec,
    debug: ColorSpec,
    info: ColorSpec,
    warn: ColorSpec,
    error: ColorSpec,
    fatal: ColorSpec,
    unknown: ColorSpec,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeUi {
    timestamp: ColorSpec,
    key: ColorSpec,
    string: ColorSpec,
    number: ColorSpec,
    #[serde(rename = "bool")]
    bool_color: ColorSpec,
    null: ColorSpec,
    /// Optional; defaults to `dim` when omitted.
    #[serde(default)]
    column_border: Option<ColorSpec>,
    /// Optional; `0` when omitted (space separator, no vertical rule).
    #[serde(default)]
    column_border_width: usize,
    /// Optional; `1` or `{ left, right }` (default zero).
    #[serde(default)]
    column_border_padding: crate::config::Padding,
}

impl Theme {
    pub fn builtin(name: &str) -> Result<Self> {
        let raw = match name {
            "default" | "catppuccin" => include_str!("../themes/catppuccin.toml"),
            "dracula" => include_str!("../themes/dracula.toml"),
            "github-dark" => include_str!("../themes/github-dark.toml"),
            "github-light" => include_str!("../themes/github-light.toml"),
            "gotham" => include_str!("../themes/gotham.toml"),
            "nord" => include_str!("../themes/nord.toml"),
            "solarized-dark" => include_str!("../themes/solarized-dark.toml"),
            "solarized-light" => include_str!("../themes/solarized-light.toml"),
            "tokyo-night" => include_str!("../themes/tokyo-night.toml"),
            other => bail!("unknown built-in theme '{other}'"),
        };
        Self::parse(raw)
    }

    pub fn load_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read theme {}", path.display()))?;
        Self::parse(&raw)
    }

    /// Resolve a theme by name: user themes dir first, then built-ins.
    pub fn resolve(name: &str) -> Result<Self> {
        let user_path = Config::themes_dir().join(format!("{name}.toml"));
        if user_path.is_file() {
            return Self::load_file(user_path);
        }
        match Self::builtin(name) {
            Ok(t) => Ok(t),
            Err(_) => bail!(
                "unknown theme '{name}' (try: {})",
                Self::list_names().join(", ")
            ),
        }
    }

    pub fn resolve_with_overrides(name: &str, overrides: &ThemeOverrides) -> Result<Self> {
        let mut theme = Self::resolve(name)?;
        theme.apply_overrides(overrides)?;
        Ok(theme)
    }

    pub fn apply_overrides(&mut self, overrides: &ThemeOverrides) -> Result<()> {
        let c = &overrides.colors;
        apply_optional(&mut self.background, c.background.as_deref(), parse_color)?;
        apply_optional(
            &mut self.foreground,
            c.foreground.as_ref(),
            ColorSpec::parse,
        )?;
        apply_optional(
            &mut self.selection_bg,
            c.selection_bg.as_deref(),
            parse_color,
        )?;
        apply_optional(
            &mut self.selection_fg,
            c.selection_fg.as_deref(),
            parse_color,
        )?;
        apply_optional(&mut self.overlay_bg, c.overlay_bg.as_deref(), parse_color)?;
        apply_optional(&mut self.status_bg, c.status_bg.as_deref(), parse_color)?;
        apply_optional(&mut self.status_fg, c.status_fg.as_deref(), parse_color)?;
        apply_optional(&mut self.border, c.border.as_ref(), ColorSpec::parse)?;
        apply_optional(
            &mut self.window_focus_border,
            c.window_focus_border.as_ref(),
            ColorSpec::parse,
        )?;
        apply_optional(
            &mut self.search_match,
            c.search_match.as_ref(),
            ColorSpec::parse,
        )?;
        apply_optional(&mut self.dim, c.dim.as_ref(), ColorSpec::parse)?;

        let l = &overrides.levels;
        apply_level(&mut self.levels, LogLevel::Trace, l.trace.as_ref())?;
        apply_level(&mut self.levels, LogLevel::Debug, l.debug.as_ref())?;
        apply_level(&mut self.levels, LogLevel::Info, l.info.as_ref())?;
        apply_level(&mut self.levels, LogLevel::Warn, l.warn.as_ref())?;
        apply_level(&mut self.levels, LogLevel::Error, l.error.as_ref())?;
        apply_level(&mut self.levels, LogLevel::Fatal, l.fatal.as_ref())?;
        apply_level(&mut self.levels, LogLevel::Unknown, l.unknown.as_ref())?;

        let u = &overrides.ui;
        apply_optional(&mut self.timestamp, u.timestamp.as_ref(), ColorSpec::parse)?;
        apply_optional(&mut self.key, u.key.as_ref(), ColorSpec::parse)?;
        apply_optional(&mut self.string, u.string.as_ref(), ColorSpec::parse)?;
        apply_optional(&mut self.number, u.number.as_ref(), ColorSpec::parse)?;
        apply_optional(
            &mut self.bool_color,
            u.bool_color.as_ref(),
            ColorSpec::parse,
        )?;
        apply_optional(&mut self.null, u.null.as_ref(), ColorSpec::parse)?;
        apply_optional(
            &mut self.column_border,
            u.column_border.as_ref(),
            ColorSpec::parse,
        )?;
        apply_optional(
            &mut self.column_border_width,
            u.column_border_width.as_ref(),
            |value| Ok(*value),
        )?;
        apply_optional(
            &mut self.column_border_padding,
            u.column_border_padding.as_ref(),
            |value| Ok(*value),
        )?;
        Ok(())
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let file: ThemeFile = toml::from_str(raw).context("invalid theme toml")?;
        let levels = parse_levels(&file.levels)?;

        let dim = file.colors.dim.parse()?;
        let column_border = match file.ui.column_border {
            Some(spec) => spec.parse()?,
            None => dim,
        };

        Ok(Self {
            name: file.name,
            background: parse_color(&file.colors.background)?,
            foreground: file.colors.foreground.parse()?,
            selection_bg: parse_color(&file.colors.selection_bg)?,
            selection_fg: parse_color(&file.colors.selection_fg)?,
            overlay_bg: parse_color(&file.colors.overlay_bg)?,
            status_bg: parse_color(&file.colors.status_bg)?,
            status_fg: parse_color(&file.colors.status_fg)?,
            border: file.colors.border.parse()?,
            window_focus_border: file.colors.window_focus_border.parse()?,
            search_match: file.colors.search_match.parse()?,
            dim,
            timestamp: file.ui.timestamp.parse()?,
            key: file.ui.key.parse()?,
            string: file.ui.string.parse()?,
            number: file.ui.number.parse()?,
            bool_color: file.ui.bool_color.parse()?,
            null: file.ui.null.parse()?,
            column_border,
            column_border_width: file.ui.column_border_width,
            column_border_padding: file.ui.column_border_padding,
            levels,
        })
    }

    pub fn available() -> &'static [&'static str] {
        &[
            "catppuccin",
            "dracula",
            "github-dark",
            "github-light",
            "gotham",
            "nord",
            "solarized-dark",
            "solarized-light",
            "tokyo-night",
        ]
    }

    pub fn list_names() -> Vec<String> {
        static NAMES: OnceLock<Vec<String>> = OnceLock::new();
        NAMES
            .get_or_init(|| {
                let mut names: Vec<String> = Self::available()
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect();
                if let Ok(entries) = std::fs::read_dir(Config::themes_dir()) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|extension| extension.to_str()) == Some("toml")
                            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
                            && !names.iter().any(|name| name == stem)
                        {
                            names.push(stem.to_string());
                        }
                    }
                }
                names.sort();
                names
            })
            .clone()
    }
}

/// Optional color patches from `[theme.*]` in `config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThemeOverrides {
    #[serde(default, skip_serializing_if = "ColorOverrides::is_empty")]
    pub colors: ColorOverrides,
    #[serde(default, skip_serializing_if = "LevelOverrides::is_empty")]
    pub levels: LevelOverrides,
    #[serde(default, skip_serializing_if = "UiOverrides::is_empty")]
    pub ui: UiOverrides,
}

impl ThemeOverrides {
    pub fn validate(&self) -> Result<()> {
        self.colors.validate()?;
        self.levels.validate()?;
        self.ui.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ColorOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_focus_border: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_match: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dim: Option<ColorSpec>,
}

impl ColorOverrides {
    pub fn is_empty(&self) -> bool {
        self.background.is_none()
            && self.foreground.is_none()
            && self.selection_bg.is_none()
            && self.selection_fg.is_none()
            && self.overlay_bg.is_none()
            && self.status_bg.is_none()
            && self.status_fg.is_none()
            && self.border.is_none()
            && self.window_focus_border.is_none()
            && self.search_match.is_none()
            && self.dim.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        validate_color_strings([
            ("colors.background", self.background.as_deref()),
            ("colors.selection_bg", self.selection_bg.as_deref()),
            ("colors.selection_fg", self.selection_fg.as_deref()),
            ("colors.overlay_bg", self.overlay_bg.as_deref()),
            ("colors.status_bg", self.status_bg.as_deref()),
            ("colors.status_fg", self.status_fg.as_deref()),
        ])?;
        validate_specs([
            ("colors.foreground", self.foreground.as_ref()),
            ("colors.border", self.border.as_ref()),
            (
                "colors.window_focus_border",
                self.window_focus_border.as_ref(),
            ),
            ("colors.search_match", self.search_match.as_ref()),
            ("colors.dim", self.dim.as_ref()),
        ])?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LevelOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warn: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fatal: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown: Option<ColorSpec>,
}

impl LevelOverrides {
    pub fn is_empty(&self) -> bool {
        self.trace.is_none()
            && self.debug.is_none()
            && self.info.is_none()
            && self.warn.is_none()
            && self.error.is_none()
            && self.fatal.is_none()
            && self.unknown.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        validate_specs([
            ("levels.trace", self.trace.as_ref()),
            ("levels.debug", self.debug.as_ref()),
            ("levels.info", self.info.as_ref()),
            ("levels.warn", self.warn.as_ref()),
            ("levels.error", self.error.as_ref()),
            ("levels.fatal", self.fatal.as_ref()),
            ("levels.unknown", self.unknown.as_ref()),
        ])?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UiOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<ColorSpec>,
    #[serde(rename = "bool", skip_serializing_if = "Option::is_none")]
    pub bool_color: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_border: Option<ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_border_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_border_padding: Option<crate::config::Padding>,
}

impl UiOverrides {
    pub fn is_empty(&self) -> bool {
        self.timestamp.is_none()
            && self.key.is_none()
            && self.string.is_none()
            && self.number.is_none()
            && self.bool_color.is_none()
            && self.null.is_none()
            && self.column_border.is_none()
            && self.column_border_width.is_none()
            && self.column_border_padding.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        validate_specs([
            ("ui.timestamp", self.timestamp.as_ref()),
            ("ui.key", self.key.as_ref()),
            ("ui.string", self.string.as_ref()),
            ("ui.number", self.number.as_ref()),
            ("ui.bool", self.bool_color.as_ref()),
            ("ui.null", self.null.as_ref()),
            ("ui.column_border", self.column_border.as_ref()),
        ])?;
        Ok(())
    }
}

fn validate_color_strings<'a>(
    values: impl IntoIterator<Item = (&'static str, Option<&'a str>)>,
) -> Result<()> {
    for (path, value) in values {
        if let Some(value) = value {
            parse_color(value).with_context(|| format!("invalid {path}"))?;
        }
    }
    Ok(())
}

fn validate_specs<'a>(
    values: impl IntoIterator<Item = (&'static str, Option<&'a ColorSpec>)>,
) -> Result<()> {
    for (path, value) in values {
        if let Some(value) = value {
            value
                .validate()
                .with_context(|| format!("invalid {path}"))?;
        }
    }
    Ok(())
}

pub(crate) fn parse_color(s: &str) -> Result<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#')
        && hex.len() == 6
    {
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        return Ok(Color::Rgb(r, g, b));
    }
    Color::from_str(s).map_err(|_| anyhow::anyhow!("invalid color '{s}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_themes_parse() {
        for name in Theme::available() {
            let theme = Theme::builtin(name).unwrap();
            assert_eq!(theme.name, *name);
        }
    }

    #[test]
    fn search_highlight_uses_bg_or_inverts_accent() {
        let theme = Theme::builtin("catppuccin").unwrap();
        let style = theme.search_highlight_style(Color::Rgb(0, 0, 0));
        assert_eq!(style.bg, Some(Color::Rgb(0xf9, 0xe2, 0xaf)));
        assert_eq!(style.fg, Some(Color::Rgb(0x1e, 0x1e, 0x2e)));

        let mut bare = theme.clone();
        bare.search_match = Tone::fg_only(Color::Rgb(0xff, 0xff, 0x00));
        let inverted = bare.search_highlight_style(Color::Rgb(1, 2, 3));
        assert_eq!(inverted.bg, Some(Color::Rgb(0xff, 0xff, 0x00)));
        assert_eq!(inverted.fg, Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn overrides_patch_selected_colors() {
        let mut theme = Theme::builtin("catppuccin").unwrap();
        let overrides = ThemeOverrides {
            colors: ColorOverrides {
                background: Some("#000000".into()),
                ..Default::default()
            },
            levels: LevelOverrides {
                error: Some(ColorSpec::Fg("#ff0000".into())),
                ..Default::default()
            },
            ui: UiOverrides {
                timestamp: Some(ColorSpec::Fg("#00ff00".into())),
                ..Default::default()
            },
        };
        theme.apply_overrides(&overrides).unwrap();
        assert_eq!(theme.background, Color::Rgb(0, 0, 0));
        assert_eq!(
            theme.levels[&LogLevel::Error],
            Tone {
                fg: Color::Rgb(255, 0, 0),
                bg: None,
            }
        );
        assert_eq!(theme.timestamp, Tone::fg_only(Color::Rgb(0, 255, 0)));
        assert_ne!(theme.foreground.fg, Color::Rgb(0, 0, 0));
    }

    #[test]
    fn column_border_defaults_and_overrides() {
        let theme = Theme::builtin("catppuccin").unwrap();
        assert_eq!(theme.column_border_width, 0);
        assert!(theme.column_border_padding.is_zero());
        assert_eq!(theme.column_border, theme.dim);

        let mut theme = theme;
        theme
            .apply_overrides(&ThemeOverrides {
                ui: UiOverrides {
                    column_border: Some(ColorSpec::FgBg(ColorSpecFgBg {
                        fg: "#ff0000".into(),
                        bg: Some("#00ff00".into()),
                    })),
                    column_border_width: Some(2),
                    column_border_padding: Some(crate::config::Padding { left: 2, right: 1 }),
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap();
        assert_eq!(theme.column_border_width, 2);
        assert_eq!(
            theme.column_border_padding,
            crate::config::Padding { left: 2, right: 1 }
        );
        assert_eq!(
            theme.column_border,
            Tone {
                fg: Color::Rgb(255, 0, 0),
                bg: Some(Color::Rgb(0, 255, 0)),
            }
        );
    }

    #[test]
    fn color_spec_accepts_fg_bg_table() {
        let spec: ColorSpec = toml::from_str(
            r##"fg = "#111111"
bg = "#ff0000"
"##,
        )
        .unwrap();
        let c = spec.parse().unwrap();
        assert_eq!(c.fg, Color::Rgb(0x11, 0x11, 0x11));
        assert_eq!(c.bg, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn theme_file_tones_with_bg() {
        let raw = r##"
name = "test"
[colors]
background = "#000000"
foreground = "#ffffff"
selection_bg = "#333333"
selection_fg = "#ffffff"
overlay_bg = "#111111"
status_bg = "#222222"
status_fg = "#aaaaaa"
border = "#444444"
window_focus_border = "#ffff00"
search_match = "#ffff00"
dim = { fg = "#666666", bg = "#222222" }
[levels]
trace = "#111111"
debug = "#222222"
info = "#333333"
warn = "#444444"
error = { fg = "#000000", bg = "#ff0000" }
fatal = "#666666"
unknown = "#777777"
[ui]
timestamp = { fg = "#888888", bg = "#010101" }
key = "#999999"
string = "#aaaaaa"
number = "#bbbbbb"
bool = "#cccccc"
null = "#dddddd"
"##;
        let theme = Theme::parse(raw).unwrap();
        let err = theme.level_color(LogLevel::Error);
        assert_eq!(err.fg, Color::Rgb(0, 0, 0));
        assert_eq!(err.bg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(theme.timestamp.bg, Some(Color::Rgb(1, 1, 1)));
        assert_eq!(theme.dim.bg, Some(Color::Rgb(0x22, 0x22, 0x22)));
        assert!(theme.level_color(LogLevel::Info).bg.is_none());
    }
}
