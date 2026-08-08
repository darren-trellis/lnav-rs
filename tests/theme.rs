use teleminator::model::LogLevel;
use teleminator::theme::*;
use ratatui::style::Color;


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
    assert_eq!(theme.column_border_width, 1);
    assert_eq!(theme.column_border_padding, teleminator::config::Padding::both(1));
    assert_eq!(
        theme.column_border,
        Tone {
            fg: Color::Rgb(0x58, 0x5b, 0x70),
            bg: Some(Color::Rgb(0x1e, 0x1e, 0x2e)),
        }
    );

    let mut theme = theme;
    theme
        .apply_overrides(&ThemeOverrides {
            ui: UiOverrides {
                border_color: Some(ColorSpec::FgBg(ColorSpecFgBg {
                    fg: "#ff0000".into(),
                    bg: Some("#00ff00".into()),
                })),
                border_width: Some(2),
                border_padding: Some(teleminator::config::Padding { left: 2, right: 1 }),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(theme.column_border_width, 2);
    assert_eq!(
        theme.column_border_padding,
        teleminator::config::Padding { left: 2, right: 1 }
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
selection = { fg = "#ffffff", bg = "#333333" }
overlay = { fg = "#ffffff", bg = "#111111" }
status = { fg = "#aaaaaa", bg = "#222222" }
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
