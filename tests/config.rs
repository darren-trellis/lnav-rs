use std::fs;

use teleminator::config::*;

#[test]
fn roundtrip_defaults() {
    let dir = std::env::temp_dir().join(format!("teleminator-roundtrip-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let cfg = Config::default();
    cfg.write_to(&path).unwrap();
    let (parsed, _) = Config::load_from(&path).unwrap();
    assert_eq!(cfg.theme.name(), parsed.theme.name());
    assert_eq!(cfg.timestamp_format, parsed.timestamp_format);
    assert_eq!(cfg.columns, parsed.columns);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_merges_partial_keys() {
    let dir = std::env::temp_dir().join(format!("teleminator-cfg-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "[theme]\nname = \"nord\"\n[keys]\nq = \"quit\"\nD = \"hide\"\n",
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert_eq!(cfg.theme.name(), "nord");
    assert_eq!(cfg.keys.bindings.get("d").map(String::as_str), Some("hide"));
    assert_eq!(cfg.keys.bindings.get("D").map(String::as_str), Some("hide"));
    assert_eq!(cfg.keys.bindings.get("q").map(String::as_str), Some("quit"));
    assert_eq!(cfg.columns, default_columns());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_theme_string() {
    let dir = std::env::temp_dir().join(format!("teleminator-theme-str-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "theme = \"nord\"\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_unknown_field() {
    let dir = std::env::temp_dir().join(format!("teleminator-unk-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "nope = 1\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_invalid_color() {
    let dir = std::env::temp_dir().join(format!("teleminator-badcol-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "[theme]\nname = \"catppuccin\"\n[levels]\nerror = \"not-a-color\"\n",
    )
    .unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_unknown_key_command() {
    let dir = std::env::temp_dir().join(format!("teleminator-badcmd-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[keys]\nq = \"not-a-command\"\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn accepts_chained_key_commands() {
    let dir = std::env::temp_dir().join(format!("teleminator-chain-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "[keys]\nr = \"view details; focus toggle\"\n",
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert_eq!(
        cfg.keys.bindings.get("r").map(String::as_str),
        Some("view details; focus toggle")
    );
    fs::write(&path, "[keys]\nr = \"view details; nope\"\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_nested_keys_details_and_sidebar() {
    let dir = std::env::temp_dir().join(format!("teleminator-nested-keys-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "[keys]\nr = \"quit\"\n[keys.details]\nspace = \"copy\"\n[keys.sidebar]\nd = \"filter delete line\"\n",
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert_eq!(cfg.keys.bindings.get("r").map(String::as_str), Some("quit"));
    assert_eq!(
        cfg.keys.details.get("space").map(String::as_str),
        Some("copy")
    );
    assert_eq!(
        cfg.keys.sidebar.get("d").map(String::as_str),
        Some("filter delete line")
    );
    assert_eq!(
        cfg.keys.sidebar.get("space").map(String::as_str),
        Some("filter set toggle")
    );
    fs::write(&path, "[details_keys]\nspace = \"copy\"\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn write_nested_key_overlays() {
    let dir = std::env::temp_dir().join(format!("teleminator-write-nested-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let mut cfg = Config::default();
    cfg.keys.details.insert("c".into(), "fold toggle".into());
    cfg.write_to(&path).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("[keys.details]"));
    assert!(raw.contains("c = \"fold toggle\""));
    assert!(!raw.contains("details_keys"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_zero_scroll_lines() {
    let dir = std::env::temp_dir().join(format!("teleminator-scroll-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[main]\nscroll_lines = 0\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_narrow_sidebar_width() {
    let dir = std::env::temp_dir().join(format!("teleminator-sidebar-w-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[sidebar]\nwidth = 8\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn loads_sidebar_section() {
    let dir = std::env::temp_dir().join(format!("teleminator-sidebar-sec-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"[sidebar]
enabled = true
width = 36
position = "left"
scrollbar_vertical = false
scrollbar_horizontal = false
"#,
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert!(cfg.sidebar);
    assert_eq!(cfg.sidebar_width, 36);
    assert_eq!(cfg.sidebar_position, SidebarPosition::Left);
    assert!(!cfg.sidebar_scrollbar_vertical);
    assert!(!cfg.sidebar_scrollbar_horizontal);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_legacy_main_sidebar_keys() {
    let dir = std::env::temp_dir().join(format!("teleminator-sidebar-leg-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[main]\nsidebar = true\nsidebar_width = 40\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_root_level_main_settings() {
    let dir = std::env::temp_dir().join(format!("teleminator-root-main-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "follow = true\nline_numbers = true\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_legacy_scrollbar_key() {
    let dir = std::env::temp_dir().join(format!("teleminator-legacy-bar-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[main]\nscrollbar = false\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn write_emits_theme_table() {
    let dir = std::env::temp_dir().join(format!("teleminator-write-{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("teleminator-write-keys-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    Config::default().write_to(&path).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(!raw.contains("[keys]"));
    assert!(!raw.contains("[main]"));
    assert!(!raw.contains("follow = "));
    assert!(!raw.contains("wrap_details = "));
    assert!(!raw.contains("details_json_tree = "));
    assert!(!raw.contains("details_max_height = "));
    assert!(!raw.contains("details_tab_width = "));
    assert!(!raw.contains("line_numbers = "));
    assert!(!raw.contains("list_scrollbar_vertical = "));
    assert!(!raw.contains("list_scrollbar_horizontal = "));
    assert!(!raw.contains("details_scrollbar_vertical = "));
    assert!(!raw.contains("scrollbar = "));
    assert!(!raw.contains("border = "));
    assert!(!raw.contains("autosave = "));
    assert!(!raw.contains("autoreload = "));
    assert!(!raw.contains("page_lines = "));
    assert!(!raw.contains("[details]"));
    assert!(!raw.contains("[sidebar]"));
    assert!(!raw.contains("sidebar_width = "));
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
    let dir = std::env::temp_dir().join(format!("teleminator-case-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[main]\ncase_mode = \"smartcase\"\n").unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert_eq!(cfg.case_mode, CaseMode::Smart);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn sidebar_position_left_and_aliases() {
    assert_eq!(SidebarPosition::parse("left"), Some(SidebarPosition::Left));
    assert_eq!(SidebarPosition::parse("L"), Some(SidebarPosition::Left));
    assert_eq!(SidebarPosition::parse("right"), Some(SidebarPosition::Right));
    assert_eq!(SidebarPosition::default(), SidebarPosition::Right);
    let dir = std::env::temp_dir().join(format!("teleminator-side-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[sidebar]\nposition = \"left\"\n").unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert_eq!(cfg.sidebar_position, SidebarPosition::Left);
    let mut out = cfg;
    out.sidebar_position = SidebarPosition::Left;
    out.write_to(&path).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("[sidebar]"));
    assert!(raw.contains("position = \"left\""));
    assert!(!raw.contains("sidebar_position = "));
    assert!(!raw.contains("[main]"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn write_emits_sidebar_section_not_main_keys() {
    let dir = std::env::temp_dir().join(format!("teleminator-write-side-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let mut cfg = Config::default();
    cfg.sidebar = true;
    cfg.sidebar_width = 36;
    cfg.sidebar_position = SidebarPosition::Left;
    cfg.sidebar_scrollbar_vertical = false;
    cfg.sidebar_scrollbar_horizontal = false;
    cfg.write_to(&path).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("[sidebar]"));
    assert!(raw.contains("enabled = true"));
    assert!(raw.contains("width = 36"));
    assert!(raw.contains("position = \"left\""));
    assert!(raw.contains("scrollbar_vertical = false"));
    assert!(raw.contains("scrollbar_horizontal = false"));
    assert!(!raw.contains("sidebar_width = "));
    assert!(!raw.contains("sidebar_position = "));
    assert!(!raw.contains("sidebar_scrollbar_"));
    assert!(!raw.contains("[main]\nsidebar"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn loads_details_section() {
    let dir = std::env::temp_dir().join(format!("teleminator-details-sec-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"[details]
wrap = false
json_tree = false
max_height = 12
tab_width = 2
scrollbar_vertical = false
"#,
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert!(!cfg.wrap_details);
    assert!(!cfg.details_json_tree);
    assert_eq!(cfg.details_max_height, 12);
    assert_eq!(cfg.details_tab_width, 2);
    assert!(!cfg.details_scrollbar_vertical);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_legacy_main_details_keys() {
    let dir = std::env::temp_dir().join(format!("teleminator-details-leg-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[main]\nwrap_details = false\ndetails_max_height = 16\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn write_emits_details_section_not_main_keys() {
    let dir = std::env::temp_dir().join(format!("teleminator-write-det-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let mut cfg = Config::default();
    cfg.wrap_details = false;
    cfg.details_json_tree = false;
    cfg.details_max_height = 12;
    cfg.details_tab_width = 2;
    cfg.details_scrollbar_vertical = false;
    cfg.write_to(&path).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("[details]"));
    assert!(raw.contains("wrap = false"));
    assert!(raw.contains("json_tree = false"));
    assert!(raw.contains("max_height = 12"));
    assert!(raw.contains("tab_width = 2"));
    assert!(raw.contains("scrollbar_vertical = false"));
    assert!(!raw.contains("wrap_details = "));
    assert!(!raw.contains("details_json_tree = "));
    assert!(!raw.contains("details_max_height = "));
    assert!(!raw.contains("details_tab_width = "));
    assert!(!raw.contains("details_scrollbar_vertical = "));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_narrow_details_max_height() {
    let dir = std::env::temp_dir().join(format!("teleminator-details-h-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "[details]\nmax_height = 2\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn write_main_section_before_theme_tables_roundtrip() {
    let dir = std::env::temp_dir().join(format!("teleminator-write-order-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");

    let mut cfg = Config {
        line_numbers: true,
        ..Config::default()
    };
    cfg.levels.info = Some(teleminator::theme::ColorSpec::Fg("#a6e3a1".into()));
    cfg.columns = vec![
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
            source: "annotations.url".into(),
            width: None,
            align: Align::Left,
            padding: Padding::default(),
            border: None,
            border_color: None,
            border_width: None,
            border_padding: None,
        },
    ];

    cfg.write_to(&path).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    let main_pos = raw.find("[main]").expect("[main] in file");
    let line_nums_pos = raw
        .find("line_numbers = true")
        .expect("line_numbers in file");
    let levels_pos = raw.find("[levels]").expect("[levels] in file");
    assert!(
        main_pos < line_nums_pos && line_nums_pos < levels_pos,
        "[main] line_numbers must appear before [levels]\n{raw}"
    );

    let (loaded, _) = Config::load_from(&path).unwrap();
    assert!(loaded.line_numbers);
    assert_eq!(
        loaded.levels.info,
        Some(teleminator::theme::ColorSpec::Fg("#a6e3a1".into()))
    );
    assert_eq!(loaded.columns.len(), 2);
    assert_eq!(loaded.columns[1].source, "annotations.url");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_columns_from_toml() {
    let dir = std::env::temp_dir().join(format!("teleminator-cols-{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("teleminator-pad-{}", std::process::id()));
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
    assert_eq!(cfg.columns[1].padding, Padding { left: 1, right: 2 });
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_legacy_line_format() {
    let dir = std::env::temp_dir().join(format!("teleminator-legacy-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "line_format = \"{raw}\"\n[theme]\nname = \"nord\"\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_theme_table_overrides() {
    let dir = std::env::temp_dir().join(format!("teleminator-ovr-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r##"
[theme]
name = "catppuccin"
[colors]
background = "#000000"
[levels]
error = "#ff0000"
[ui]
bool = "#00ff00"
"##,
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    assert_eq!(cfg.theme.name(), "catppuccin");
    let o = cfg.theme_overrides();
    assert_eq!(o.colors.background.as_deref(), Some("#000000"));
    assert_eq!(
        o.levels.error,
        Some(teleminator::theme::ColorSpec::Fg("#ff0000".into()))
    );
    assert_eq!(
        o.ui.bool_color,
        Some(teleminator::theme::ColorSpec::Fg("#00ff00".into()))
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_tone_fg_bg() {
    let dir = std::env::temp_dir().join(format!("teleminator-lvl-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r##"
[theme]
name = "catppuccin"
[colors]
dim = { fg = "#6c7086", bg = "#313244" }
[levels]
error = { fg = "#1e1e2e", bg = "#f38ba8" }
warn = "#f9e2af"
[ui]
timestamp = { fg = "#89b4fa", bg = "#11111b" }
"##,
    )
    .unwrap();
    let (cfg, _) = Config::load_from(&path).unwrap();
    let o = cfg.theme_overrides();
    assert_eq!(
        o.colors.dim,
        Some(teleminator::theme::ColorSpec::FgBg(teleminator::theme::ColorSpecFgBg {
            fg: "#6c7086".into(),
            bg: Some("#313244".into()),
        }))
    );
    assert_eq!(
        o.levels.error,
        Some(teleminator::theme::ColorSpec::FgBg(teleminator::theme::ColorSpecFgBg {
            fg: "#1e1e2e".into(),
            bg: Some("#f38ba8".into()),
        }))
    );
    assert_eq!(
        o.levels.warn,
        Some(teleminator::theme::ColorSpec::Fg("#f9e2af".into()))
    );
    assert_eq!(
        o.ui.timestamp,
        Some(teleminator::theme::ColorSpec::FgBg(teleminator::theme::ColorSpecFgBg {
            fg: "#89b4fa".into(),
            bg: Some("#11111b".into()),
        }))
    );
    let theme = teleminator::theme::Theme::resolve_with_overrides(cfg.theme.name(), &o).unwrap();
    let err = theme.level_color(teleminator::model::LogLevel::Error);
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
    let dir = std::env::temp_dir().join(format!("teleminator-legacy-ovr-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r##"
[theme]
name = "nord"
[theme.colors]
background = "#010101"
"##,
    )
    .unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}
