use std::fs;

use lnav_rs::config::*;

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
    assert_eq!(cfg.keys.bindings.get("d").map(String::as_str), Some("hide"));
    assert_eq!(cfg.keys.bindings.get("D").map(String::as_str), Some("hide"));
    assert_eq!(cfg.keys.bindings.get("q").map(String::as_str), Some("quit"));
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
        "[theme]\nname = \"catppuccin\"\n[levels]\nerror = \"not-a-color\"\n",
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
fn accepts_chained_key_commands() {
    let dir = std::env::temp_dir().join(format!("lnav-rs-chain-{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("lnav-rs-nested-keys-{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("lnav-rs-write-nested-{}", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("lnav-rs-scroll-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "scroll_lines = 0\n").unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_narrow_sidebar_width() {
    let dir = std::env::temp_dir().join(format!("lnav-rs-sidebar-w-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "sidebar_width = 8\n").unwrap();
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
    assert!(!raw.contains("scrollbar = "));
    assert!(!raw.contains("border = "));
    assert!(!raw.contains("autosave = "));
    assert!(!raw.contains("autoreload = "));
    assert!(!raw.contains("page_lines = "));
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
    let cfg: Config = toml::from_str("case_mode = \"smartcase\"\n").unwrap();
    assert_eq!(cfg.case_mode, CaseMode::Smart);
}

#[test]
fn write_root_scalars_before_theme_tables_roundtrip() {
    let dir = std::env::temp_dir().join(format!("lnav-rs-write-order-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");

    let mut cfg = Config {
        line_numbers: true,
        ..Config::default()
    };
    cfg.levels.info = Some(lnav_rs::theme::ColorSpec::Fg("#a6e3a1".into()));
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
    let line_nums_pos = raw
        .find("line_numbers = true")
        .expect("line_numbers in file");
    let levels_pos = raw.find("[levels]").expect("[levels] in file");
    assert!(
        line_nums_pos < levels_pos,
        "line_numbers must appear before [levels]\n{raw}"
    );

    let (loaded, _) = Config::load_from(&path).unwrap();
    assert!(loaded.line_numbers);
    assert_eq!(
        loaded.levels.info,
        Some(lnav_rs::theme::ColorSpec::Fg("#a6e3a1".into()))
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
    assert_eq!(cfg.columns[1].padding, Padding { left: 1, right: 2 });
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rejects_legacy_line_format() {
    let dir = std::env::temp_dir().join(format!("lnav-rs-legacy-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    fs::write(&path, "line_format = \"{raw}\"\n[theme]\nname = \"nord\"\n").unwrap();
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
        Some(lnav_rs::theme::ColorSpec::Fg("#ff0000".into()))
    );
    assert_eq!(
        o.ui.bool_color,
        Some(lnav_rs::theme::ColorSpec::Fg("#00ff00".into()))
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
        Some(lnav_rs::theme::ColorSpec::FgBg(lnav_rs::theme::ColorSpecFgBg {
            fg: "#6c7086".into(),
            bg: Some("#313244".into()),
        }))
    );
    assert_eq!(
        o.levels.error,
        Some(lnav_rs::theme::ColorSpec::FgBg(lnav_rs::theme::ColorSpecFgBg {
            fg: "#1e1e2e".into(),
            bg: Some("#f38ba8".into()),
        }))
    );
    assert_eq!(
        o.levels.warn,
        Some(lnav_rs::theme::ColorSpec::Fg("#f9e2af".into()))
    );
    assert_eq!(
        o.ui.timestamp,
        Some(lnav_rs::theme::ColorSpec::FgBg(lnav_rs::theme::ColorSpecFgBg {
            fg: "#89b4fa".into(),
            bg: Some("#11111b".into()),
        }))
    );
    let theme = lnav_rs::theme::Theme::resolve_with_overrides(cfg.theme.name(), &o).unwrap();
    let err = theme.level_color(lnav_rs::model::LogLevel::Error);
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
[theme.colors]
background = "#010101"
"##,
    )
    .unwrap();
    assert!(Config::load_from(&path).is_err());
    let _ = fs::remove_dir_all(&dir);
}
