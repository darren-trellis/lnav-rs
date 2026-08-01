use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::command_catalog;
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
    /// Leading separator on/off (`None` → top-level `border`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<bool>,
    /// Leading separator color (`None` → theme `ui.border_color`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<crate::theme::ColorSpec>,
    /// Leading separator width before this column (`None` → theme `ui.border_width`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_width: Option<usize>,
    /// Leading separator padding (`None` → theme `ui.border_padding`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_padding: Option<Padding>,
}

/// Theme name selection (`[theme]`). Color patches live at the config root
/// under `[colors]` / `[levels]` / `[ui]`.
///
/// ```toml
/// [theme]
/// name = "catppuccin"
///
/// [colors]
/// background = "#11111b"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub name: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme(),
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

    /// Patches for theme `[colors]` (config root `[colors]`).
    #[serde(
        default,
        skip_serializing_if = "crate::theme::ColorOverrides::is_empty"
    )]
    pub colors: crate::theme::ColorOverrides,

    /// Patches for theme `[levels]` (config root `[levels]`).
    #[serde(
        default,
        skip_serializing_if = "crate::theme::LevelOverrides::is_empty"
    )]
    pub levels: crate::theme::LevelOverrides,

    /// Patches for theme `[ui]` (config root `[ui]`).
    #[serde(default, skip_serializing_if = "crate::theme::UiOverrides::is_empty")]
    pub ui: crate::theme::UiOverrides,

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

    /// Show a vertical scrollbar on the right of the list and details panes.
    #[serde(default = "default_true")]
    pub scrollbar: bool,

    /// Draw vertical rules between list columns (theme/column width still apply when on).
    #[serde(default = "default_true")]
    pub border: bool,

    /// When true, `:config set` writes the config file after a successful change.
    #[serde(default = "default_true")]
    pub autosave: bool,

    /// When true, reload settings when the config file changes on disk.
    #[serde(default = "default_true")]
    pub autoreload: bool,

    /// Show a filters sidebar listing active filters.
    #[serde(default)]
    pub sidebar: bool,

    /// Lines to move per mouse-wheel notch in the log list.
    #[serde(default = "default_scroll_lines")]
    pub scroll_lines: usize,

    /// When true, mouse-wheel scrolling moves the selection/cursor.
    /// When false, the wheel scrolls the viewport only (list, details, sidebar).
    #[serde(default = "default_true")]
    pub scroll_moves_selection: bool,

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

    /// Key overrides while the filters sidebar is focused (merged over `keys`).
    #[serde(default)]
    pub sidebar_keys: BTreeMap<String, String>,

    /// Persist filters per log file under `~/.local/share/lnav-rs/sessions/`.
    #[serde(default = "default_true")]
    pub session_filters: bool,

    /// Persist filters for stdin under a shared `sessions/stdin.toml`.
    #[serde(default = "default_true")]
    pub session_stdin: bool,
}

#[derive(Serialize)]
struct PersistedConfig<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    follow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrap_details: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details_json_tree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details_max_height: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details_tab_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_numbers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relative_line_numbers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scrollbar: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autosave: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autoreload: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sidebar: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_moves_selection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp_format: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    case_mode: Option<CaseMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_filters: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_stdin: Option<bool>,
    theme: &'a ThemeConfig,
    #[serde(skip_serializing_if = "crate::theme::ColorOverrides::is_empty")]
    colors: &'a crate::theme::ColorOverrides,
    #[serde(skip_serializing_if = "crate::theme::LevelOverrides::is_empty")]
    levels: &'a crate::theme::LevelOverrides,
    #[serde(skip_serializing_if = "crate::theme::UiOverrides::is_empty")]
    ui: &'a crate::theme::UiOverrides,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    columns: Vec<PersistedColumn<'a>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    keys: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    details_keys: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    sidebar_keys: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct PersistedColumn<'a> {
    source: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    align: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<Padding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_color: Option<&'a crate::theme::ColorSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_padding: Option<Padding>,
}

impl<'a> PersistedConfig<'a> {
    fn new(config: &'a Config) -> Self {
        let defaults = Config::default();
        let details_max_height = config.details_max_height.max(4);
        let details_tab_width = config.details_tab_width.max(2);
        let scroll_lines = config.scroll_lines.max(1);
        let columns = if config.columns == defaults.columns {
            Vec::new()
        } else {
            config
                .columns
                .iter()
                .map(|column| PersistedColumn {
                    source: &column.source,
                    width: column.width,
                    align: (column.align != Align::Left).then_some(column.align),
                    padding: (!column.padding.is_zero()).then_some(column.padding),
                    border: column.border,
                    border_color: column.border_color.as_ref(),
                    border_width: column.border_width,
                    border_padding: column.border_padding,
                })
                .collect()
        };

        Self {
            follow: (config.follow != defaults.follow).then_some(config.follow),
            wrap_details: (config.wrap_details != defaults.wrap_details)
                .then_some(config.wrap_details),
            details_json_tree: (config.details_json_tree != defaults.details_json_tree)
                .then_some(config.details_json_tree),
            details_max_height: (details_max_height != defaults.details_max_height)
                .then_some(details_max_height),
            details_tab_width: (details_tab_width != defaults.details_tab_width)
                .then_some(details_tab_width),
            line_numbers: (config.line_numbers != defaults.line_numbers)
                .then_some(config.line_numbers),
            relative_line_numbers: (config.relative_line_numbers != defaults.relative_line_numbers)
                .then_some(config.relative_line_numbers),
            scrollbar: (config.scrollbar != defaults.scrollbar).then_some(config.scrollbar),
            border: (config.border != defaults.border).then_some(config.border),
            autosave: (config.autosave != defaults.autosave).then_some(config.autosave),
            autoreload: (config.autoreload != defaults.autoreload).then_some(config.autoreload),
            sidebar: (config.sidebar != defaults.sidebar).then_some(config.sidebar),
            scroll_lines: (scroll_lines != defaults.scroll_lines).then_some(scroll_lines),
            scroll_moves_selection: (config.scroll_moves_selection
                != defaults.scroll_moves_selection)
                .then_some(config.scroll_moves_selection),
            timestamp_format: (config.timestamp_format != defaults.timestamp_format)
                .then_some(config.timestamp_format.as_str()),
            case_mode: (config.case_mode != defaults.case_mode).then_some(config.case_mode),
            session_filters: (config.session_filters != defaults.session_filters)
                .then_some(config.session_filters),
            session_stdin: (config.session_stdin != defaults.session_stdin)
                .then_some(config.session_stdin),
            theme: &config.theme,
            colors: &config.colors,
            levels: &config.levels,
            ui: &config.ui,
            columns,
            keys: key_differences(&config.keys, &defaults.keys),
            details_keys: key_differences(&config.details_keys, &defaults.details_keys),
            sidebar_keys: key_differences(&config.sidebar_keys, &defaults.sidebar_keys),
        }
    }
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
            border: None,
            border_color: None,
            border_width: None,
            border_padding: None,
        },
        Column {
            source: "timestamp".into(),
            width: None,
            align: Align::Left,
            padding: Padding::default(),
            border: None,
            border_color: None,
            border_width: None,
            border_padding: None,
        },
        Column {
            source: "message".into(),
            width: None,
            align: Align::Left,
            padding: Padding::default(),
            border: None,
            border_color: None,
            border_width: None,
            border_padding: None,
        },
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            colors: crate::theme::ColorOverrides::default(),
            levels: crate::theme::LevelOverrides::default(),
            ui: crate::theme::UiOverrides::default(),
            follow: true,
            wrap_details: true,
            details_json_tree: true,
            details_max_height: default_details_max_height(),
            details_tab_width: default_details_tab_width(),
            line_numbers: false,
            relative_line_numbers: false,
            scrollbar: true,
            border: true,
            autosave: true,
            autoreload: true,
            sidebar: false,
            scroll_lines: default_scroll_lines(),
            scroll_moves_selection: true,
            timestamp_format: default_timestamp_format(),
            case_mode: CaseMode::default(),
            columns: default_columns(),
            keys: keys::defaults(),
            details_keys: keys::details_defaults(),
            sidebar_keys: keys::sidebar_defaults(),
            session_filters: true,
            session_stdin: true,
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("lnav-rs");
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
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("lnav-rs");
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
        let mut cfg: Self =
            toml::from_str(&raw).with_context(|| format!("invalid config {}", path.display()))?;
        cfg.validate()
            .with_context(|| format!("invalid config {}", path.display()))?;
        cfg.keys = keys::merge(keys::defaults(), std::mem::take(&mut cfg.keys));
        cfg.details_keys = keys::merge_overlay(
            keys::details_defaults(),
            std::mem::take(&mut cfg.details_keys),
        );
        cfg.sidebar_keys = keys::merge_overlay(
            keys::sidebar_defaults(),
            std::mem::take(&mut cfg.sidebar_keys),
        );
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
        self.theme_overrides()
            .validate()
            .context("invalid theme color override")?;
        for (i, col) in self.columns.iter().enumerate() {
            if col.source.trim().is_empty() {
                bail!("columns[{i}].source must not be empty");
            }
            if let Some(border) = &col.border_color {
                border
                    .validate()
                    .with_context(|| format!("invalid columns[{i}].border_color"))?;
            }
        }
        let known: Vec<&str> = command_catalog::catalog().iter().map(|c| c.name).collect();
        validate_key_map("keys", &self.keys, &known)?;
        validate_key_map("details_keys", &self.details_keys, &known)?;
        validate_key_map("sidebar_keys", &self.sidebar_keys, &known)?;
        Ok(())
    }

    pub fn theme_overrides(&self) -> crate::theme::ThemeOverrides {
        crate::theme::ThemeOverrides {
            colors: self.colors.clone(),
            levels: self.levels.clone(),
            ui: self.ui.clone(),
        }
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

        let body =
            toml::to_string(&PersistedConfig::new(self)).context("failed to serialize config")?;

        fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path.to_path_buf())
    }
}

fn validate_key_map(section: &str, map: &BTreeMap<String, String>, known: &[&str]) -> Result<()> {
    for (key, cmd) in map {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            continue;
        }
        let name = cmd.split_whitespace().next().unwrap_or(cmd);
        if !command_catalog::is_known_command(name) {
            bail!(
                "unknown command {cmd:?} for {section}.{key} (try: {})",
                known.join(", ")
            );
        }
    }
    Ok(())
}

fn key_differences(
    map: &BTreeMap<String, String>,
    defaults: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut differences = BTreeMap::new();
    for (key, command) in map {
        if defaults.get(key) != Some(command) {
            differences.insert(key.clone(), command.clone());
        }
    }
    for key in defaults.keys() {
        if !map.contains_key(key) {
            differences.insert(key.clone(), String::new());
        }
    }
    differences
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
