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
    Center,
    Right,
}

/// Horizontal padding around a column cell.
///
/// Config accepts `padding = 1` (both sides) or `padding = { left = 1, right = 2 }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Padding {
    pub left: usize,
    pub right: usize,
}

impl Padding {
    pub fn both(n: usize) -> Self {
        Self { left: n, right: n }
    }

    pub fn is_zero(self) -> bool {
        self.left == 0 && self.right == 0
    }
}

impl Serialize for Padding {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.left == self.right {
            serializer.serialize_u64(self.left as u64)
        } else {
            use serde::ser::SerializeMap;
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("left", &self.left)?;
            map.serialize_entry("right", &self.right)?;
            map.end()
        }
    }
}

impl<'de> Deserialize<'de> for Padding {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Spec {
            Both(usize),
            Sides {
                #[serde(default)]
                left: usize,
                #[serde(default)]
                right: usize,
            },
        }
        match Spec::deserialize(deserializer)? {
            Spec::Both(n) => Ok(Self::both(n)),
            Spec::Sides { left, right } => Ok(Self { left, right }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub source: String,
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub align: Align,
    #[serde(default)]
    pub padding: Padding,
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

/// Case matching for `/` search and `:filter` regexes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaseMode {
    /// Always case-sensitive.
    Sensitive,
    /// Always case-insensitive.
    Insensitive,
    /// Case-insensitive unless the pattern contains an uppercase letter (vim smartcase).
    #[default]
    #[serde(alias = "smartcase")]
    Smart,
}

impl CaseMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sensitive => "sensitive",
            Self::Insensitive => "insensitive",
            Self::Smart => "smart",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "sensitive" => Some(Self::Sensitive),
            "insensitive" | "ignore" => Some(Self::Insensitive),
            "smart" | "smartcase" => Some(Self::Smart),
            _ => None,
        }
    }

    /// Whether the compiled regex should ignore case for this pattern.
    pub fn ignore_case(self, pattern: &str) -> bool {
        match self {
            Self::Sensitive => false,
            Self::Insensitive => true,
            Self::Smart => !pattern.chars().any(|c| c.is_uppercase()),
        }
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

    /// Render nested JSON fields as an indented tree in the details overlay.
    #[serde(default = "default_true")]
    pub details_json_tree: bool,

    /// Maximum height of the details overlay (rows, including border).
    #[serde(default = "default_details_max_height")]
    pub details_max_height: usize,

    /// Indent width (columns) per nesting level in the details JSON tree.
    #[serde(default = "default_details_tab_width")]
    pub details_tab_width: usize,

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

    /// Case matching for search and filters: `sensitive`, `insensitive`, or `smart`.
    #[serde(default)]
    pub case_mode: CaseMode,

    /// List-view columns. Empty / missing → default level / timestamp / message.
    #[serde(default)]
    pub columns: Vec<Column>,

    /// Key → command map. User values override defaults; `""` unbinds.
    #[serde(default)]
    pub keys: BTreeMap<String, String>,

    /// Key overrides while the details overlay is focused (merged over `keys`).
    #[serde(default)]
    pub details_keys: BTreeMap<String, String>,

    /// Persist filters per log file under `~/.local/share/lnav-rs/sessions/`.
    #[serde(default = "default_true")]
    pub session_filters: bool,

    /// Persist filters for stdin under a shared `sessions/stdin.toml`.
    #[serde(default = "default_true")]
    pub session_stdin: bool,
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

fn default_details_max_height() -> usize {
    24
}

fn default_details_tab_width() -> usize {
    4
}

fn default_timestamp_format() -> String {
    timestamp::DEFAULT_FORMAT.into()
}

pub fn default_columns() -> Vec<Column> {
    vec![
        Column {
            source: "level".into(),
            width: Some(5),
            align: Align::Center,
            padding: Padding::both(1),
        },
        Column {
            source: "timestamp".into(),
            width: None,
            align: Align::Left,
            padding: Padding::default(),
        },
        Column {
            source: "message".into(),
            width: None,
            align: Align::Left,
            padding: Padding::default(),
        },
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            follow: true,
            wrap_details: true,
            details_json_tree: true,
            details_max_height: default_details_max_height(),
            details_tab_width: default_details_tab_width(),
            line_numbers: false,
            relative_line_numbers: false,
            scroll_lines: default_scroll_lines(),
            timestamp_format: default_timestamp_format(),
            case_mode: CaseMode::default(),
            columns: default_columns(),
            keys: keys::defaults(),
            details_keys: keys::details_defaults(),
            session_filters: true,
            session_stdin: true,
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

    pub fn data_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("lnav-rs");
            }
        }
        dirs_home()
            .map(|h| h.join(".local").join("share").join("lnav-rs"))
            .unwrap_or_else(|| PathBuf::from(".lnav-rs-data"))
    }

    pub fn sessions_dir() -> PathBuf {
        Self::data_dir().join("sessions")
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
        cfg.details_keys =
            keys::merge(keys::details_defaults(), std::mem::take(&mut cfg.details_keys));
        if cfg.columns.is_empty() {
            cfg.columns = default_columns();
        }
        Ok((cfg, Some(path.to_path_buf())))
    }

    fn validate(&self) -> Result<()> {
        if self.scroll_lines == 0 {
            bail!("scroll_lines must be >= 1");
        }
        if self.details_max_height < 4 {
            bail!("details_max_height must be >= 4");
        }
        if self.details_tab_width < 2 {
            bail!("details_tab_width must be >= 2");
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
        validate_key_map("keys", &self.keys, &known)?;
        validate_key_map("details_keys", &self.details_keys, &known)?;
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

        let defaults = Self::default();
        let mut body = String::new();

        // Root scalars must come before [theme.*] tables — TOML keeps assigning
        // keys to the most recent table header until the next one.
        if self.follow != defaults.follow {
            body.push_str(&format!("follow = {}\n", self.follow));
        }
        if self.wrap_details != defaults.wrap_details {
            body.push_str(&format!("wrap_details = {}\n", self.wrap_details));
        }
        if self.details_json_tree != defaults.details_json_tree {
            body.push_str(&format!("details_json_tree = {}\n", self.details_json_tree));
        }
        if self.details_max_height != defaults.details_max_height {
            body.push_str(&format!(
                "details_max_height = {}\n",
                self.details_max_height.max(4)
            ));
        }
        if self.details_tab_width != defaults.details_tab_width {
            body.push_str(&format!(
                "details_tab_width = {}\n",
                self.details_tab_width.max(2)
            ));
        }
        if self.line_numbers != defaults.line_numbers {
            body.push_str(&format!("line_numbers = {}\n", self.line_numbers));
        }
        if self.relative_line_numbers != defaults.relative_line_numbers {
            body.push_str(&format!(
                "relative_line_numbers = {}\n",
                self.relative_line_numbers
            ));
        }
        if self.scroll_lines.max(1) != defaults.scroll_lines {
            body.push_str(&format!("scroll_lines = {}\n", self.scroll_lines.max(1)));
        }
        if self.timestamp_format != defaults.timestamp_format {
            body.push_str(&format!("timestamp_format = {:?}\n", self.timestamp_format));
        }
        if self.case_mode != defaults.case_mode {
            body.push_str(&format!("case_mode = {:?}\n", self.case_mode.as_str()));
        }
        if self.session_filters != defaults.session_filters {
            body.push_str(&format!("session_filters = {}\n", self.session_filters));
        }
        if self.session_stdin != defaults.session_stdin {
            body.push_str(&format!("session_stdin = {}\n", self.session_stdin));
        }
        if !body.is_empty() {
            body.push('\n');
        }

        write_theme_config(&mut body, &self.theme);

        if self.columns != defaults.columns {
            for col in &self.columns {
                body.push_str("[[columns]]\n");
                body.push_str(&format!("source = {:?}\n", col.source));
                if let Some(w) = col.width {
                    body.push_str(&format!("width = {w}\n"));
                }
                match col.align {
                    Align::Left => {}
                    Align::Center => body.push_str("align = \"center\"\n"),
                    Align::Right => body.push_str("align = \"right\"\n"),
                }
                if !col.padding.is_zero() {
                    if col.padding.left == col.padding.right {
                        body.push_str(&format!("padding = {}\n", col.padding.left));
                    } else {
                        body.push_str(&format!(
                            "padding = {{ left = {}, right = {} }}\n",
                            col.padding.left, col.padding.right
                        ));
                    }
                }
                body.push('\n');
            }
        }

        write_key_section(&mut body, "keys", &self.keys, &keys::defaults());
        write_key_section(
            &mut body,
            "details_keys",
            &self.details_keys,
            &keys::details_defaults(),
        );

        fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path.to_path_buf())
    }
}

fn validate_key_map(
    section: &str,
    map: &BTreeMap<String, String>,
    known: &[&str],
) -> Result<()> {
    for (key, cmd) in map {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            continue;
        }
        let name = cmd.split_whitespace().next().unwrap_or(cmd);
        if !command::is_known_command(name) {
            bail!(
                "unknown command {cmd:?} for {section}.{key} (try: {})",
                known.join(", ")
            );
        }
    }
    Ok(())
}

fn write_key_section(
    body: &mut String,
    section: &str,
    map: &BTreeMap<String, String>,
    defaults: &BTreeMap<String, String>,
) {
    let overrides: Vec<(&String, &String)> = map
        .iter()
        .filter(|(key, cmd)| defaults.get(*key) != Some(*cmd))
        .collect();
    let unbinds: Vec<&String> = defaults
        .keys()
        .filter(|key| !map.contains_key(*key))
        .collect();
    if overrides.is_empty() && unbinds.is_empty() {
        return;
    }
    body.push_str(&format!("[{section}]\n"));
    for (key, cmd) in overrides {
        body.push_str(&format!("{} = {:?}\n", toml_key(key), cmd));
    }
    for key in unbinds {
        body.push_str(&format!("{} = \"\"\n", toml_key(key)));
    }
    body.push('\n');
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
    body.push_str(&format!("name = {:?}\n\n", theme.name()));
    if !theme.has_overrides() {
        return;
    }
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
        write_color_spec_opt(body, "column_border", &o.ui.column_border);
        if let Some(w) = o.ui.column_border_width {
            body.push_str(&format!("column_border_width = {w}\n"));
        }
        if let Some(p) = o.ui.column_border_padding {
            if p.left == p.right {
                body.push_str(&format!("column_border_padding = {}\n", p.left));
            } else {
                body.push_str(&format!(
                    "column_border_padding = {{ left = {}, right = {} }}\n",
                    p.left, p.right
                ));
            }
        }
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
    fn write_omits_default_keys() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-write-keys-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        Config::default().write_to(&path).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("[keys]"));
        assert!(!raw.contains("follow = "));
        assert!(!raw.contains("wrap_details = "));
        assert!(!raw.contains("details_json_tree = "));
        assert!(!raw.contains("details_max_height = "));
        assert!(!raw.contains("details_tab_width = "));
        assert!(!raw.contains("line_numbers = "));
        assert!(!raw.contains("session_filters = "));
        assert!(!raw.contains("session_stdin = "));
        assert!(!raw.contains("case_mode = "));
        assert!(!raw.contains("[[columns]]"));
        assert!(raw.contains("[theme]"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn case_mode_smartcase_and_aliases() {
        assert!(!CaseMode::Sensitive.ignore_case("error"));
        assert!(CaseMode::Insensitive.ignore_case("ERROR"));
        assert!(CaseMode::Smart.ignore_case("error"));
        assert!(!CaseMode::Smart.ignore_case("Error"));
        assert_eq!(CaseMode::parse("smartcase"), Some(CaseMode::Smart));
        let cfg: Config = toml::from_str("case_mode = \"smartcase\"\n").unwrap();
        assert_eq!(cfg.case_mode, CaseMode::Smart);
    }

    #[test]
    fn write_root_scalars_before_theme_tables_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "lnav-rs-write-order-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");

        let mut cfg = Config::default();
        cfg.line_numbers = true;
        cfg.theme.levels.info = Some(crate::theme::ColorSpec::Fg("#a6e3a1".into()));
        cfg.columns = vec![
            Column {
                source: "level".into(),
                width: Some(5),
                align: Align::Center,
                padding: Padding::both(1),
            },
            Column {
                source: "annotations.url".into(),
                width: None,
                align: Align::Left,
                padding: Padding::default(),
            },
        ];

        cfg.write_to(&path).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        let line_nums_pos = raw.find("line_numbers = true").expect("line_numbers in file");
        let theme_levels_pos = raw
            .find("[theme.levels]")
            .expect("[theme.levels] in file");
        assert!(
            line_nums_pos < theme_levels_pos,
            "line_numbers must appear before [theme.levels]\n{raw}"
        );

        let (loaded, _) = Config::load_from(&path).unwrap();
        assert!(loaded.line_numbers);
        assert_eq!(
            loaded.theme.overrides().levels.info,
            Some(crate::theme::ColorSpec::Fg("#a6e3a1".into()))
        );
        assert_eq!(loaded.columns.len(), 2);
        assert_eq!(loaded.columns[1].source, "annotations.url");

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
    fn load_column_padding_from_toml() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-pad-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
[[columns]]
source = "level"
width = 5
padding = 1

[[columns]]
source = "message"
padding = { left = 1, right = 2 }
"#,
        )
        .unwrap();
        let (cfg, _) = Config::load_from(&path).unwrap();
        assert_eq!(cfg.columns.len(), 2);
        assert_eq!(cfg.columns[0].padding, Padding::both(1));
        assert_eq!(
            cfg.columns[1].padding,
            Padding {
                left: 1,
                right: 2
            }
        );
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
