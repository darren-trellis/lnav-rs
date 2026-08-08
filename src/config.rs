use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::command_catalog;
use crate::keys::KeysConfig;
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
    /// Leading separator on/off (`None` → `[main].border`).
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

/// Which side of the main pane the filters sidebar occupies.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SidebarPosition {
    Left,
    #[default]
    Right,
}

impl SidebarPosition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "left" | "l" => Some(Self::Left),
            "right" | "r" => Some(Self::Right),
            _ => None,
        }
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

/// Runtime config. File format nests scalars under `[main]`; see `ConfigDocument`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeConfig,

    /// Patches for theme `[colors]` (config root `[colors]`).
    pub colors: crate::theme::ColorOverrides,

    /// Patches for theme `[levels]` (config root `[levels]`).
    pub levels: crate::theme::LevelOverrides,

    /// Patches for theme `[ui]` (config root `[ui]`).
    pub ui: crate::theme::UiOverrides,

    /// When true, live-tail as the file or pipe grows (`[view.main] tail_mode`).
    pub tail_mode: bool,

    /// When true, wrap the details overlay content.
    pub wrap_details: bool,

    /// Render nested JSON fields as an indented tree in the details overlay.
    pub details_json_tree: bool,

    /// Maximum height of the details overlay (rows, including border).
    pub details_max_height: usize,

    /// Indent width (columns) per nesting level in the details JSON tree.
    pub details_tab_width: usize,

    /// Show 1-based view line numbers in the list (not file line numbers).
    pub line_numbers: bool,

    /// Show distances from the cursor instead of absolute numbers (vim `relativenumber`).
    /// With `line_numbers`, the current line stays absolute.
    pub relative_line_numbers: bool,

    /// Vertical scrollbar on the main list.
    pub list_scrollbar_vertical: bool,
    /// Horizontal scrollbar on the main list.
    pub list_scrollbar_horizontal: bool,
    /// Vertical scrollbar on the filters/hidden sidebar.
    pub sidebar_scrollbar_vertical: bool,
    /// Horizontal scrollbar on the filters/hidden sidebar.
    pub sidebar_scrollbar_horizontal: bool,
    /// Vertical scrollbar on the details overlay.
    pub details_scrollbar_vertical: bool,

    /// Draw vertical rules between list columns (theme/column width still apply when on).
    pub border: bool,

    /// When true, `:config set` writes the config file after a successful change.
    pub autosave: bool,

    /// When true, reload settings when the config file changes on disk.
    pub autoreload: bool,

    /// Persist filters under `~/.local/share/teleminator/sessions/` (files and stdin).
    pub session_filters: bool,

    /// Show a filters sidebar listing active filters.
    pub sidebar: bool,

    /// Preferred sidebar width in columns (clamped to fit the terminal).
    pub sidebar_width: usize,

    /// Place the filters sidebar on the `left` or `right` of the main pane.
    pub sidebar_position: SidebarPosition,

    /// When true, enable terminal mouse capture (clicks, wheel, drag).
    pub mouse: bool,

    /// Lines to move per mouse-wheel notch in the log list.
    pub scroll_lines: usize,

    /// Lines to move per page up/down. `0` (default) uses the current viewport height.
    pub page_lines: usize,

    /// When true, mouse-wheel scrolling moves the selection/cursor.
    /// When false, the wheel scrolls the viewport only (list, details, sidebar).
    pub scroll_moves_selection: bool,

    /// strftime format for timestamp columns (chrono syntax), or `"raw"`.
    pub timestamp_format: String,

    /// When true, display timestamps in the local timezone; otherwise keep UTC.
    pub timestamp_localized: bool,

    /// Case matching for search and filters: `sensitive`, `insensitive`, or `smart`.
    pub case_mode: CaseMode,

    /// List-view columns. Empty / missing → default level / timestamp / message.
    pub columns: Vec<Column>,

    /// Keybindings: `[keys]`, `[keys.details]`, `[keys.sidebar]`, `[keys.spans]`.
    pub keys: KeysConfig,
}

/// Scalar settings stored under `[main]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct MainConfig {
    #[serde(default = "default_true")]
    border: bool,
    #[serde(default)]
    page_lines: usize,
    #[serde(default)]
    case_mode: CaseMode,
}

impl Default for MainConfig {
    fn default() -> Self {
        Self {
            border: true,
            page_lines: 0,
            case_mode: CaseMode::default(),
        }
    }
}

/// Vertical + horizontal scrollbar toggles (`[view.*.scrollbar]`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ScrollbarAxesFileConfig {
    #[serde(default = "default_true")]
    vertical: bool,
    #[serde(default = "default_true")]
    horizontal: bool,
}

impl Default for ScrollbarAxesFileConfig {
    fn default() -> Self {
        Self {
            vertical: true,
            horizontal: true,
        }
    }
}

/// Vertical-only scrollbar (`[view.details.scrollbar]`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ScrollbarVerticalFileConfig {
    #[serde(default = "default_true")]
    vertical: bool,
}

impl Default for ScrollbarVerticalFileConfig {
    fn default() -> Self {
        Self { vertical: true }
    }
}

/// Main-list view settings under `[view.main]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ViewMainFileConfig {
    #[serde(default = "default_true")]
    tail_mode: bool,
    #[serde(default)]
    scrollbar: ScrollbarAxesFileConfig,
}

impl Default for ViewMainFileConfig {
    fn default() -> Self {
        Self {
            tail_mode: true,
            scrollbar: ScrollbarAxesFileConfig::default(),
        }
    }
}

/// Pane layout settings under `[view]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ViewFileConfig {
    #[serde(default)]
    main: ViewMainFileConfig,
    #[serde(default)]
    details: DetailsFileConfig,
    #[serde(default)]
    sidebar: SidebarFileConfig,
}

/// Timestamp display settings under `[timestamp]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TimestampFileConfig {
    #[serde(default = "default_timestamp_format")]
    format: String,
    #[serde(default = "default_true")]
    localized: bool,
}

impl Default for TimestampFileConfig {
    fn default() -> Self {
        Self {
            format: default_timestamp_format(),
            localized: true,
        }
    }
}

/// Meta settings under `[config]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ConfigFileConfig {
    #[serde(default = "default_true")]
    autosave: bool,
    #[serde(default = "default_true")]
    autoreload: bool,
}

impl Default for ConfigFileConfig {
    fn default() -> Self {
        Self {
            autosave: true,
            autoreload: true,
        }
    }
}

/// Persistence settings under `[persist]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PersistFileConfig {
    #[serde(default = "default_true")]
    filters: bool,
}

impl Default for PersistFileConfig {
    fn default() -> Self {
        Self { filters: true }
    }
}

/// Line number gutter settings under `[line_numbers]` in `config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LineNumbersFileConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    relative: bool,
}

/// Mouse settings under `[mouse]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct MouseFileConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_scroll_lines")]
    scroll_lines: usize,
    #[serde(default = "default_true")]
    scroll_moves_selection: bool,
}

impl Default for MouseFileConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scroll_lines: default_scroll_lines(),
            scroll_moves_selection: true,
        }
    }
}

/// Details overlay settings under `[view.details]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DetailsFileConfig {
    #[serde(default = "default_true")]
    wrap: bool,
    #[serde(default = "default_true")]
    json_tree: bool,
    #[serde(default = "default_details_max_height")]
    max_height: usize,
    #[serde(default = "default_details_tab_width")]
    tab_width: usize,
    #[serde(default)]
    scrollbar: ScrollbarVerticalFileConfig,
}

impl Default for DetailsFileConfig {
    fn default() -> Self {
        Self {
            wrap: true,
            json_tree: true,
            max_height: default_details_max_height(),
            tab_width: default_details_tab_width(),
            scrollbar: ScrollbarVerticalFileConfig::default(),
        }
    }
}

/// Sidebar settings under `[view.sidebar]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SidebarFileConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_sidebar_width")]
    width: usize,
    #[serde(default)]
    position: SidebarPosition,
    #[serde(default)]
    scrollbar: ScrollbarAxesFileConfig,
}

impl Default for SidebarFileConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            width: default_sidebar_width(),
            position: SidebarPosition::default(),
            scrollbar: ScrollbarAxesFileConfig::default(),
        }
    }
}

/// On-disk TOML shape: scalars live under `[main]` / `[config]` / `[persist]` / `[view]` / …
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigDocument {
    #[serde(default)]
    main: MainConfig,
    #[serde(default)]
    config: ConfigFileConfig,
    #[serde(default)]
    persist: PersistFileConfig,
    #[serde(default)]
    view: ViewFileConfig,
    #[serde(default)]
    mouse: MouseFileConfig,
    #[serde(default)]
    line_numbers: LineNumbersFileConfig,
    #[serde(default)]
    timestamp: TimestampFileConfig,
    #[serde(default)]
    theme: ThemeConfig,
    #[serde(default)]
    colors: crate::theme::ColorOverrides,
    #[serde(default)]
    levels: crate::theme::LevelOverrides,
    #[serde(default)]
    ui: crate::theme::UiOverrides,
    #[serde(default)]
    columns: Vec<Column>,
    #[serde(default)]
    keys: KeysConfig,
}

impl ConfigDocument {
    fn into_config(self) -> Config {
        Config {
            theme: self.theme,
            colors: self.colors,
            levels: self.levels,
            ui: self.ui,
            tail_mode: self.view.main.tail_mode,
            wrap_details: self.view.details.wrap,
            details_json_tree: self.view.details.json_tree,
            details_max_height: self.view.details.max_height,
            details_tab_width: self.view.details.tab_width,
            line_numbers: self.line_numbers.enabled,
            relative_line_numbers: self.line_numbers.relative,
            list_scrollbar_vertical: self.view.main.scrollbar.vertical,
            list_scrollbar_horizontal: self.view.main.scrollbar.horizontal,
            sidebar_scrollbar_vertical: self.view.sidebar.scrollbar.vertical,
            sidebar_scrollbar_horizontal: self.view.sidebar.scrollbar.horizontal,
            details_scrollbar_vertical: self.view.details.scrollbar.vertical,
            border: self.main.border,
            autosave: self.config.autosave,
            autoreload: self.config.autoreload,
            session_filters: self.persist.filters,
            sidebar: self.view.sidebar.enabled,
            sidebar_width: self.view.sidebar.width,
            sidebar_position: self.view.sidebar.position,
            mouse: self.mouse.enabled,
            scroll_lines: self.mouse.scroll_lines,
            page_lines: self.main.page_lines,
            scroll_moves_selection: self.mouse.scroll_moves_selection,
            timestamp_format: self.timestamp.format,
            timestamp_localized: self.timestamp.localized,
            case_mode: self.main.case_mode,
            columns: self.columns,
            keys: self.keys,
        }
    }
}

#[derive(Serialize)]
struct PersistedConfig<'a> {
    #[serde(skip_serializing_if = "PersistedMain::is_empty")]
    main: PersistedMain,
    #[serde(skip_serializing_if = "PersistedConfigMeta::is_empty")]
    config: PersistedConfigMeta,
    #[serde(skip_serializing_if = "PersistedPersist::is_empty")]
    persist: PersistedPersist,
    #[serde(skip_serializing_if = "PersistedView::is_empty")]
    view: PersistedView,
    #[serde(skip_serializing_if = "PersistedMouse::is_empty")]
    mouse: PersistedMouse,
    #[serde(skip_serializing_if = "PersistedLineNumbers::is_empty")]
    line_numbers: PersistedLineNumbers,
    #[serde(skip_serializing_if = "PersistedTimestamp::is_empty")]
    timestamp: PersistedTimestamp<'a>,
    theme: &'a ThemeConfig,
    #[serde(skip_serializing_if = "crate::theme::ColorOverrides::is_empty")]
    colors: &'a crate::theme::ColorOverrides,
    #[serde(skip_serializing_if = "crate::theme::LevelOverrides::is_empty")]
    levels: &'a crate::theme::LevelOverrides,
    #[serde(skip_serializing_if = "crate::theme::UiOverrides::is_empty")]
    ui: &'a crate::theme::UiOverrides,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    columns: Vec<PersistedColumn<'a>>,
    #[serde(skip_serializing_if = "PersistedKeys::is_empty")]
    keys: PersistedKeys,
}

#[derive(Serialize)]
struct PersistedMain {
    #[serde(skip_serializing_if = "Option::is_none")]
    border: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    case_mode: Option<CaseMode>,
}

impl PersistedMain {
    fn is_empty(&self) -> bool {
        self.border.is_none() && self.page_lines.is_none() && self.case_mode.is_none()
    }
}

#[derive(Serialize, Default)]
struct PersistedView {
    #[serde(skip_serializing_if = "PersistedViewMain::is_empty")]
    main: PersistedViewMain,
    #[serde(skip_serializing_if = "PersistedDetails::is_empty")]
    details: PersistedDetails,
    #[serde(skip_serializing_if = "PersistedSidebar::is_empty")]
    sidebar: PersistedSidebar,
}

impl PersistedView {
    fn is_empty(&self) -> bool {
        self.main.is_empty() && self.details.is_empty() && self.sidebar.is_empty()
    }
}

#[derive(Serialize, Default)]
struct PersistedViewMain {
    #[serde(skip_serializing_if = "Option::is_none")]
    tail_mode: Option<bool>,
    #[serde(skip_serializing_if = "PersistedScrollbarAxes::is_empty")]
    scrollbar: PersistedScrollbarAxes,
}

impl PersistedViewMain {
    fn is_empty(&self) -> bool {
        self.tail_mode.is_none() && self.scrollbar.is_empty()
    }
}

#[derive(Serialize, Default)]
struct PersistedScrollbarAxes {
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizontal: Option<bool>,
}

impl PersistedScrollbarAxes {
    fn is_empty(&self) -> bool {
        self.vertical.is_none() && self.horizontal.is_none()
    }
}

#[derive(Serialize, Default)]
struct PersistedScrollbarVertical {
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical: Option<bool>,
}

impl PersistedScrollbarVertical {
    fn is_empty(&self) -> bool {
        self.vertical.is_none()
    }
}

#[derive(Serialize)]
struct PersistedTimestamp<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    localized: Option<bool>,
}

impl PersistedTimestamp<'_> {
    fn is_empty(&self) -> bool {
        self.format.is_none() && self.localized.is_none()
    }
}

#[derive(Serialize)]
struct PersistedConfigMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    autosave: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autoreload: Option<bool>,
}

impl PersistedConfigMeta {
    fn is_empty(&self) -> bool {
        self.autosave.is_none() && self.autoreload.is_none()
    }
}

#[derive(Serialize)]
struct PersistedPersist {
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<bool>,
}

impl PersistedPersist {
    fn is_empty(&self) -> bool {
        self.filters.is_none()
    }
}

#[derive(Serialize, Default)]
struct PersistedDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_tree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_height: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tab_width: Option<usize>,
    #[serde(skip_serializing_if = "PersistedScrollbarVertical::is_empty")]
    scrollbar: PersistedScrollbarVertical,
}

impl PersistedDetails {
    fn is_empty(&self) -> bool {
        self.wrap.is_none()
            && self.json_tree.is_none()
            && self.max_height.is_none()
            && self.tab_width.is_none()
            && self.scrollbar.is_empty()
    }
}

#[derive(Serialize, Default)]
struct PersistedSidebar {
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<SidebarPosition>,
    #[serde(skip_serializing_if = "PersistedScrollbarAxes::is_empty")]
    scrollbar: PersistedScrollbarAxes,
}

impl PersistedSidebar {
    fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.width.is_none()
            && self.position.is_none()
            && self.scrollbar.is_empty()
    }
}

#[derive(Serialize)]
struct PersistedMouse {
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_moves_selection: Option<bool>,
}

impl PersistedMouse {
    fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.scroll_lines.is_none()
            && self.scroll_moves_selection.is_none()
    }
}

#[derive(Serialize)]
struct PersistedLineNumbers {
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relative: Option<bool>,
}

impl PersistedLineNumbers {
    fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.relative.is_none()
    }
}

#[derive(Serialize)]
struct PersistedKeys {
    #[serde(flatten)]
    bindings: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    details: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    sidebar: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    spans: BTreeMap<String, String>,
}

impl PersistedKeys {
    fn is_empty(&self) -> bool {
        self.bindings.is_empty()
            && self.details.is_empty()
            && self.sidebar.is_empty()
            && self.spans.is_empty()
    }

    fn from_config(config: &Config, defaults: &Config) -> Self {
        Self {
            bindings: key_differences(&config.keys.bindings, &defaults.keys.bindings),
            details: key_differences(&config.keys.details, &defaults.keys.details),
            sidebar: key_differences(&config.keys.sidebar, &defaults.keys.sidebar),
            spans: key_differences(&config.keys.spans, &defaults.keys.spans),
        }
    }
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
        let sidebar_width = config.sidebar_width.max(default_sidebar_width_min());
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
            main: PersistedMain {
                border: (config.border != defaults.border).then_some(config.border),
                page_lines: (config.page_lines != defaults.page_lines).then_some(config.page_lines),
                case_mode: (config.case_mode != defaults.case_mode).then_some(config.case_mode),
            },
            config: PersistedConfigMeta {
                autosave: (config.autosave != defaults.autosave).then_some(config.autosave),
                autoreload: (config.autoreload != defaults.autoreload).then_some(config.autoreload),
            },
            persist: PersistedPersist {
                filters: (config.session_filters != defaults.session_filters)
                    .then_some(config.session_filters),
            },
            view: PersistedView {
                main: PersistedViewMain {
                    tail_mode: (config.tail_mode != defaults.tail_mode).then_some(config.tail_mode),
                    scrollbar: PersistedScrollbarAxes {
                        vertical: (config.list_scrollbar_vertical
                            != defaults.list_scrollbar_vertical)
                            .then_some(config.list_scrollbar_vertical),
                        horizontal: (config.list_scrollbar_horizontal
                            != defaults.list_scrollbar_horizontal)
                            .then_some(config.list_scrollbar_horizontal),
                    },
                },
                details: PersistedDetails {
                    wrap: (config.wrap_details != defaults.wrap_details)
                        .then_some(config.wrap_details),
                    json_tree: (config.details_json_tree != defaults.details_json_tree)
                        .then_some(config.details_json_tree),
                    max_height: (details_max_height != defaults.details_max_height)
                        .then_some(details_max_height),
                    tab_width: (details_tab_width != defaults.details_tab_width)
                        .then_some(details_tab_width),
                    scrollbar: PersistedScrollbarVertical {
                        vertical: (config.details_scrollbar_vertical
                            != defaults.details_scrollbar_vertical)
                            .then_some(config.details_scrollbar_vertical),
                    },
                },
                sidebar: PersistedSidebar {
                    enabled: (config.sidebar != defaults.sidebar).then_some(config.sidebar),
                    width: (sidebar_width != defaults.sidebar_width).then_some(sidebar_width),
                    position: (config.sidebar_position != defaults.sidebar_position)
                        .then_some(config.sidebar_position),
                    scrollbar: PersistedScrollbarAxes {
                        vertical: (config.sidebar_scrollbar_vertical
                            != defaults.sidebar_scrollbar_vertical)
                            .then_some(config.sidebar_scrollbar_vertical),
                        horizontal: (config.sidebar_scrollbar_horizontal
                            != defaults.sidebar_scrollbar_horizontal)
                            .then_some(config.sidebar_scrollbar_horizontal),
                    },
                },
            },
            mouse: PersistedMouse {
                enabled: (config.mouse != defaults.mouse).then_some(config.mouse),
                scroll_lines: (scroll_lines != defaults.scroll_lines).then_some(scroll_lines),
                scroll_moves_selection: (config.scroll_moves_selection
                    != defaults.scroll_moves_selection)
                    .then_some(config.scroll_moves_selection),
            },
            line_numbers: PersistedLineNumbers {
                enabled: (config.line_numbers != defaults.line_numbers)
                    .then_some(config.line_numbers),
                relative: (config.relative_line_numbers != defaults.relative_line_numbers)
                    .then_some(config.relative_line_numbers),
            },
            timestamp: PersistedTimestamp {
                format: (config.timestamp_format != defaults.timestamp_format)
                    .then_some(config.timestamp_format.as_str()),
                localized: (config.timestamp_localized != defaults.timestamp_localized)
                    .then_some(config.timestamp_localized),
            },
            theme: &config.theme,
            colors: &config.colors,
            levels: &config.levels,
            ui: &config.ui,
            columns,
            keys: PersistedKeys::from_config(config, &defaults),
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

fn default_sidebar_width() -> usize {
    28
}

pub fn default_sidebar_width_min() -> usize {
    12
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
        ConfigDocument {
            main: MainConfig::default(),
            config: ConfigFileConfig::default(),
            persist: PersistFileConfig::default(),
            view: ViewFileConfig::default(),
            mouse: MouseFileConfig::default(),
            line_numbers: LineNumbersFileConfig::default(),
            timestamp: TimestampFileConfig::default(),
            theme: ThemeConfig::default(),
            colors: crate::theme::ColorOverrides::default(),
            levels: crate::theme::LevelOverrides::default(),
            ui: crate::theme::UiOverrides::default(),
            columns: default_columns(),
            keys: KeysConfig::with_defaults(),
        }
        .into_config()
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("teleminator");
        }
        dirs_home()
            .map(|h| h.join(".config").join("teleminator"))
            .unwrap_or_else(|| PathBuf::from(".teleminator"))
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
            return PathBuf::from(xdg).join("teleminator");
        }
        dirs_home()
            .map(|h| h.join(".local").join("share").join("teleminator"))
            .unwrap_or_else(|| PathBuf::from(".teleminator-data"))
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
        let doc: ConfigDocument =
            toml::from_str(&raw).with_context(|| format!("invalid config {}", path.display()))?;
        let mut cfg = doc.into_config();
        cfg.validate()
            .with_context(|| format!("invalid config {}", path.display()))?;
        cfg.keys = KeysConfig::merge_user(std::mem::take(&mut cfg.keys));
        if cfg.columns.is_empty() {
            cfg.columns = default_columns();
        }
        Ok((cfg, Some(path.to_path_buf())))
    }

    fn validate(&self) -> Result<()> {
        if self.scroll_lines == 0 {
            bail!("mouse.scroll_lines must be >= 1");
        }
        if self.sidebar_width < default_sidebar_width_min() {
            bail!(
                "view.sidebar.width must be >= {}",
                default_sidebar_width_min()
            );
        }
        if self.details_max_height < 4 {
            bail!("view.details.max_height must be >= 4");
        }
        if self.details_tab_width < 2 {
            bail!("view.details.tab_width must be >= 2");
        }
        if self.timestamp_format.trim().is_empty() {
            bail!("timestamp.format must not be empty");
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
        validate_key_map("keys", &self.keys.bindings, &known)?;
        validate_key_map("keys.details", &self.keys.details, &known)?;
        validate_key_map("keys.sidebar", &self.keys.sidebar, &known)?;
        validate_key_map("keys.spans", &self.keys.spans, &known)?;
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
        for part in cmd.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let name = part.split_whitespace().next().unwrap_or(part);
            if !command_catalog::is_known_command(name) {
                bail!(
                    "unknown command {part:?} in {cmd:?} for {section}.{key} (try: {})",
                    known.join(", ")
                );
            }
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
