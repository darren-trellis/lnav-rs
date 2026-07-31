use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::command;
use crate::keys;
use crate::theme::Theme;
use crate::timestamp;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub source: String,
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub align: Align,
}

/// Theme selection and optional color patches.
///
/// ```toml
/// [theme]
/// name = "catppuccin"
/// [theme.colors]
/// background = "#11111b"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub name: String,
    #[serde(default)]
    pub colors: crate::theme::ColorOverrides,
    #[serde(default)]
    pub levels: crate::theme::LevelOverrides,
    #[serde(default)]
    pub ui: crate::theme::UiOverrides,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme(),
            colors: crate::theme::ColorOverrides::default(),
            levels: crate::theme::LevelOverrides::default(),
            ui: crate::theme::UiOverrides::default(),
        }
    }
}

impl ThemeConfig {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn overrides(&self) -> crate::theme::ThemeOverrides {
        crate::theme::ThemeOverrides {
            colors: self.colors.clone(),
            levels: self.levels.clone(),
            ui: self.ui.clone(),
        }
    }

    pub fn has_overrides(&self) -> bool {
        !self.overrides().is_empty()
    }

    fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("theme.name must not be empty");
        }
        Theme::resolve(self.name()).with_context(|| {
            format!(
                "unknown theme '{}' (try: {})",
                self.name(),
                Theme::list_names().join(", ")
            )
        })?;
        self.overrides()
            .validate()
            .context("invalid theme color override")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub theme: ThemeConfig,

    #[serde(default = "default_true")]
    pub follow: bool,

    /// When true, wrap the details overlay content.
    #[serde(default = "default_true")]
    pub wrap_details: bool,

    /// Show 1-based view line numbers in the list (not file line numbers).
    #[serde(default)]
    pub line_numbers: bool,

    /// Show distances from the cursor instead of absolute numbers (vim `relativenumber`).
    /// With `line_numbers`, the current line stays absolute.
    #[serde(default)]
    pub relative_line_numbers: bool,

    /// Lines to move per mouse-wheel notch in the log list.
    #[serde(default = "default_scroll_lines")]
    pub scroll_lines: usize,

    /// strftime format for timestamp columns (chrono syntax), or `"raw"`.
    #[serde(default = "default_timestamp_format")]
    pub timestamp_format: String,

    /// List-view columns. Empty / missing → default level / timestamp / message.
    #[serde(default)]
    pub columns: Vec<Column>,

    /// Key → command map. User values override defaults; `""` unbinds.
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
}

fn default_theme() -> String {
    "catppuccin".into()
}

fn default_true() -> bool {
    true
}

fn default_scroll_lines() -> usize {
    1
}

fn default_timestamp_format() -> String {
    timestamp::DEFAULT_FORMAT.into()
}

pub fn default_columns() -> Vec<Column> {
    vec![
        Column {
            source: "level".into(),
            width: Some(5),
            align: Align::Left,
        },
        Column {
            source: "timestamp".into(),
            width: None,
            align: Align::Left,
        },
        Column {
            source: "message".into(),
            width: None,
            align: Align::Left,
        },
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            follow: true,
            wrap_details: true,
            line_numbers: false,
            relative_line_numbers: false,
            scroll_lines: default_scroll_lines(),
            timestamp_format: default_timestamp_format(),
            columns: default_columns(),
            keys: keys::defaults(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("lnav-rs");
            }
        }
        dirs_home()
            .map(|h| h.join(".config").join("lnav-rs"))
            .unwrap_or_else(|| PathBuf::from(".lnav-rs"))
    }

    pub fn themes_dir() -> PathBuf {
        Self::config_dir().join("themes")
    }

    pub fn default_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Result<(Self, Option<PathBuf>)> {
        Self::load_from(&Self::default_path())
    }

    pub fn load_from(path: &Path) -> Result<(Self, Option<PathBuf>)> {
        if !path.is_file() {
            return Ok((Self::default(), None));
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let mut cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("invalid config {}", path.display()))?;
        cfg.validate()
            .with_context(|| format!("invalid config {}", path.display()))?;
        cfg.keys = keys::merge(keys::defaults(), std::mem::take(&mut cfg.keys));
        if cfg.columns.is_empty() {
            cfg.columns = default_columns();
        }
        Ok((cfg, Some(path.to_path_buf())))
    }

    fn validate(&self) -> Result<()> {
        if self.scroll_lines == 0 {
            bail!("scroll_lines must be >= 1");
        }
        if self.timestamp_format.trim().is_empty() {
            bail!("timestamp_format must not be empty");
        }
        self.theme.validate()?;
        for (i, col) in self.columns.iter().enumerate() {
            if col.source.trim().is_empty() {
                bail!("columns[{i}].source must not be empty");
            }
        }
        let known: Vec<&str> = command::catalog().iter().map(|c| c.name).collect();
        for (key, cmd) in &self.keys {
            let cmd = cmd.trim();
            if cmd.is_empty() {
                continue;
            }
            if !known.iter().any(|n| n.eq_ignore_ascii_case(cmd)) {
                bail!(
                    "unknown command {cmd:?} for key {key:?} (try: {})",
                    known.join(", ")
                );
            }
        }
        Ok(())
    }

    pub fn write(&self) -> Result<PathBuf> {
        self.write_to(&Self::default_path())
    }

    pub fn write_to(&self, path: &Path) -> Result<PathBuf> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let _ = fs::create_dir_all(Self::themes_dir());

        let mut body = format!(
            "# lnav-rs config\n\
             # Path: {}\n\
             # User themes: {}/<name>.toml\n\
             #\n\
             # columns: list-view layout\n\
             #   source = builtin (level|timestamp|message|raw|line|format)\n\
             #            or field path (annotations.url, items.0.id)\n\
             #   width  = optional fixed width; omit to auto-align with other rows\n\
             #   align  = \"left\" | \"right\"\n\
             #\n\
             # timestamp_format: chrono/strftime, or \"raw\"\n\n",
            path.display(),
            Self::themes_dir().display(),
        );

        write_theme_config(&mut body, &self.theme);

        body.push_str(&format!(
            "follow = {}\n\
             wrap_details = {}\n\
             line_numbers = {}\n\
             relative_line_numbers = {}\n\
             scroll_lines = {}\n\
             timestamp_format = {:?}\n\n",
            self.follow,
            self.wrap_details,
            self.line_numbers,
            self.relative_line_numbers,
            self.scroll_lines.max(1),
            self.timestamp_format,
        ));

        for col in &self.columns {
            body.push_str("[[columns]]\n");
            body.push_str(&format!("source = {:?}\n", col.source));
            if let Some(w) = col.width {
                body.push_str(&format!("width = {w}\n"));
            }
            if col.align != Align::Left {
                body.push_str("align = \"right\"\n");
            }
            body.push('\n');
        }

        body.push_str(
            "# Keybindings: key = \"command\"  (empty string unbinds)\n\
             # Special keys: enter esc up down home end pagedown pageup space tab\n\
             # Modifiers: C-c  A-x\n\
             [keys]\n",
        );

        for (key, cmd) in &self.keys {
            body.push_str(&format!("{} = {:?}\n", toml_key(key), cmd));
        }

        fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path.to_path_buf())
    }
}

/// Quote key names that aren't bare TOML keys.
fn toml_key(key: &str) -> String {
    if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        key.to_string()
    } else {
        format!("{key:?}")
    }
}

fn write_opt(body: &mut String, key: &str, val: &Option<String>) {
    if let Some(v) = val {
        body.push_str(&format!("{key} = {v:?}\n"));
    }
}

fn write_color_spec_opt(body: &mut String, key: &str, val: &Option<crate::theme::ColorSpec>) {
    use crate::theme::{ColorSpec, ColorSpecFgBg};
    match val {
        None => {}
        Some(ColorSpec::Fg(fg)) => {
            body.push_str(&format!("{key} = {fg:?}\n"));
        }
        Some(ColorSpec::FgBg(ColorSpecFgBg { fg, bg: Some(bg) })) => {
            body.push_str(&format!("{key} = {{ fg = {fg:?}, bg = {bg:?} }}\n"));
        }
        Some(ColorSpec::FgBg(ColorSpecFgBg { fg, bg: None })) => {
            body.push_str(&format!("{key} = {{ fg = {fg:?} }}\n"));
        }
    }
}

fn write_theme_config(body: &mut String, theme: &ThemeConfig) {
    body.push_str("[theme]\n");
    body.push_str(&format!("name = {:?}\n", theme.name()));
    if !theme.has_overrides() {
        body.push_str(
            "# Optional color patches (same keys as themes/*.toml).\n\
             # Text colors accept \"#hex\" or { fg = \"...\", bg = \"...\" }:\n\
             # [theme.colors]\n\
             # background = \"#11111b\"\n\
             # dim = { fg = \"#6c7086\", bg = \"#313244\" }\n\
             # [theme.levels]\n\
             # error = { fg = \"#1e1e2e\", bg = \"#f38ba8\" }\n\
             # [theme.ui]\n\
             # timestamp = { fg = \"#89b4fa\", bg = \"#11111b\" }\n\n",
        );
        return;
    }
    body.push('\n');
    let o = theme.overrides();
    if !o.colors.is_empty() {
        body.push_str("[theme.colors]\n");
        write_opt(body, "background", &o.colors.background);
        write_color_spec_opt(body, "foreground", &o.colors.foreground);
        write_opt(body, "selection_bg", &o.colors.selection_bg);
        write_opt(body, "selection_fg", &o.colors.selection_fg);
        write_opt(body, "overlay_bg", &o.colors.overlay_bg);
        write_opt(body, "status_bg", &o.colors.status_bg);
        write_opt(body, "status_fg", &o.colors.status_fg);
        write_color_spec_opt(body, "border", &o.colors.border);
        write_color_spec_opt(body, "search_match", &o.colors.search_match);
        write_color_spec_opt(body, "dim", &o.colors.dim);
        body.push('\n');
    }
    if !o.levels.is_empty() {
        body.push_str("[theme.levels]\n");
        write_color_spec_opt(body, "trace", &o.levels.trace);
        write_color_spec_opt(body, "debug", &o.levels.debug);
        write_color_spec_opt(body, "info", &o.levels.info);
        write_color_spec_opt(body, "warn", &o.levels.warn);
        write_color_spec_opt(body, "error", &o.levels.error);
        write_color_spec_opt(body, "fatal", &o.levels.fatal);
        write_color_spec_opt(body, "unknown", &o.levels.unknown);
        body.push('\n');
    }
    if !o.ui.is_empty() {
        body.push_str("[theme.ui]\n");
        write_color_spec_opt(body, "timestamp", &o.ui.timestamp);
        write_color_spec_opt(body, "key", &o.ui.key);
        write_color_spec_opt(body, "string", &o.ui.string);
        write_color_spec_opt(body, "number", &o.ui.number);
        write_color_spec_opt(body, "bool", &o.ui.bool_color);
        write_color_spec_opt(body, "null", &o.ui.null);
        body.push('\n');
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_defaults() {
        let cfg = Config::default();
        let raw = toml::to_string(&cfg).unwrap();
        let parsed: Config = toml::from_str(&raw).unwrap();
        assert_eq!(cfg.theme.name(), parsed.theme.name());
        assert_eq!(cfg.timestamp_format, parsed.timestamp_format);
        assert_eq!(cfg.columns, parsed.columns);
    }

    #[test]
    fn load_merges_partial_keys() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-cfg-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "[theme]\nname = \"nord\"\n[keys]\nq = \"quit\"\nD = \"hide\"\n",
        )
        .unwrap();
        let (cfg, _) = Config::load_from(&path).unwrap();
        assert_eq!(cfg.theme.name(), "nord");
        assert_eq!(cfg.keys.get("d").map(String::as_str), Some("hide"));
        assert_eq!(cfg.keys.get("D").map(String::as_str), Some("hide"));
        assert_eq!(cfg.keys.get("q").map(String::as_str), Some("quit"));
        assert_eq!(cfg.columns, default_columns());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_theme_string() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-theme-str-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(&path, "theme = \"nord\"\n").unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_unknown_field() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-unk-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(&path, "nope = 1\n").unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_invalid_color() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-badcol-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "[theme]\nname = \"catppuccin\"\n[theme.levels]\nerror = \"not-a-color\"\n",
        )
        .unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_unknown_key_command() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-badcmd-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(&path, "[keys]\nq = \"not-a-command\"\n").unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_zero_scroll_lines() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-scroll-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(&path, "scroll_lines = 0\n").unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_emits_theme_table() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-write-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        let mut cfg = Config::default();
        cfg.theme.set_name("nord");
        cfg.write_to(&path).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("[theme]\n"));
        assert!(raw.contains("name = \"nord\"\n"));
        assert!(!raw.contains("theme = \""));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_columns_from_toml() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-cols-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
[[columns]]
source = "level"
width = 5

[[columns]]
source = "annotations.url"
"#,
        )
        .unwrap();
        let (cfg, _) = Config::load_from(&path).unwrap();
        assert_eq!(cfg.columns.len(), 2);
        assert_eq!(cfg.columns[1].source, "annotations.url");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_legacy_line_format() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-legacy-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            "line_format = \"{raw}\"\n[theme]\nname = \"nord\"\n",
        )
        .unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_theme_table_overrides() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-ovr-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r##"
[theme]
name = "catppuccin"
[theme.colors]
background = "#000000"
[theme.levels]
error = "#ff0000"
[theme.ui]
bool = "#00ff00"
"##,
        )
        .unwrap();
        let (cfg, _) = Config::load_from(&path).unwrap();
        assert_eq!(cfg.theme.name(), "catppuccin");
        let o = cfg.theme.overrides();
        assert_eq!(o.colors.background.as_deref(), Some("#000000"));
        assert_eq!(
            o.levels.error,
            Some(crate::theme::ColorSpec::Fg("#ff0000".into()))
        );
        assert_eq!(
            o.ui.bool_color,
            Some(crate::theme::ColorSpec::Fg("#00ff00".into()))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_tone_fg_bg() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-lvl-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r##"
[theme]
name = "catppuccin"
[theme.colors]
dim = { fg = "#6c7086", bg = "#313244" }
[theme.levels]
error = { fg = "#1e1e2e", bg = "#f38ba8" }
warn = "#f9e2af"
[theme.ui]
timestamp = { fg = "#89b4fa", bg = "#11111b" }
"##,
        )
        .unwrap();
        let (cfg, _) = Config::load_from(&path).unwrap();
        let o = cfg.theme.overrides();
        assert_eq!(
            o.colors.dim,
            Some(crate::theme::ColorSpec::FgBg(crate::theme::ColorSpecFgBg {
                fg: "#6c7086".into(),
                bg: Some("#313244".into()),
            }))
        );
        assert_eq!(
            o.levels.error,
            Some(crate::theme::ColorSpec::FgBg(crate::theme::ColorSpecFgBg {
                fg: "#1e1e2e".into(),
                bg: Some("#f38ba8".into()),
            }))
        );
        assert_eq!(
            o.levels.warn,
            Some(crate::theme::ColorSpec::Fg("#f9e2af".into()))
        );
        assert_eq!(
            o.ui.timestamp,
            Some(crate::theme::ColorSpec::FgBg(crate::theme::ColorSpecFgBg {
                fg: "#89b4fa".into(),
                bg: Some("#11111b".into()),
            }))
        );
        let theme =
            crate::theme::Theme::resolve_with_overrides(cfg.theme.name(), &o).unwrap();
        let err = theme.level_color(crate::model::LogLevel::Error);
        assert_eq!(err.fg, ratatui::style::Color::Rgb(0x1e, 0x1e, 0x2e));
        assert_eq!(err.bg, Some(ratatui::style::Color::Rgb(0xf3, 0x8b, 0xa8)));
        assert_eq!(
            theme.timestamp.bg,
            Some(ratatui::style::Color::Rgb(0x11, 0x11, 0x1b))
        );
        assert_eq!(
            theme.dim.bg,
            Some(ratatui::style::Color::Rgb(0x31, 0x32, 0x44))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_legacy_theme_overrides() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-legacy-ovr-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r##"
[theme]
name = "nord"
[theme_overrides.colors]
background = "#010101"
"##,
        )
        .unwrap();
        assert!(Config::load_from(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
