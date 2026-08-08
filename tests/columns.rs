use teleminator::columns::*;
use teleminator::config::{Align, Column};
use teleminator::model::{Field, FieldValue, LineFormat, LogEntry, LogLevel};
use teleminator::timestamp;
use unicode_width::UnicodeWidthStr;

fn entry() -> LogEntry {
    let raw_ts = "2026-07-27T23:58:14.817Z".to_string();
    let parsed = timestamp::parse(&raw_ts);
    LogEntry {
        line_no: 7,
        raw: r#"{"msg":"hi"}"#.into(),
        format: LineFormat::Json,
        level: LogLevel::Error,
        timestamp: Some(raw_ts),
        timestamp_parsed: parsed,
        message: Some("hi".into()),
        fields: vec![
            Field {
                key: "code".into(),
                value: FieldValue::Number("500".into()),
            },
            Field {
                key: "annotations".into(),
                value: FieldValue::Nested(serde_json::json!({
                    "url": "https://example.com/x",
                    "items": [{"id": "a"}, {"id": "b"}]
                })),
            },
        ],
    }
}

fn cols(sources: &[(&str, Option<usize>, Align)]) -> Vec<Column> {
    sources
        .iter()
        .map(|(source, width, align)| Column {
            source: (*source).into(),
            width: *width,
            align: *align,
            padding: teleminator::config::Padding::default(),
            border: None,
            border_color: None,
            border_width: None,
            border_padding: None,
        })
        .collect()
}

#[test]
fn per_column_border_width_overrides_default() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let mut columns = cols(&[
        ("level", Some(5), Align::Left),
        ("message", None, Align::Left),
        ("code", None, Align::Left),
    ]);
    columns[1].border_width = Some(2);
    columns[2].border_width = Some(0);
    let segs = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 1,
            padding: teleminator::config::Padding::default(),
            color: None,
            enabled: true,
        },
    );
    assert_eq!(segs[1].kind, SegmentKind::ColumnBorder);
    assert_eq!(segs[1].text, "││");
    assert_eq!(segs[3].kind, SegmentKind::Literal);
    assert_eq!(segs[3].text, " ");
}

#[test]
fn per_column_border_overrides_global() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let mut columns = cols(&[
        ("level", Some(5), Align::Left),
        ("message", None, Align::Left),
        ("code", None, Align::Left),
    ]);
    columns[1].border = Some(false);
    columns[2].border = Some(true);
    let segs = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 1,
            padding: teleminator::config::Padding::default(),
            color: None,
            enabled: false,
        },
    );
    assert_eq!(segs[1].kind, SegmentKind::Literal);
    assert_eq!(segs[1].text, " ");
    assert_eq!(segs[3].kind, SegmentKind::ColumnBorder);
    assert_eq!(segs[3].text, "│");
}

#[test]
fn per_column_border_color_overrides_default() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let mut columns = cols(&[
        ("level", Some(5), Align::Left),
        ("message", None, Align::Left),
    ]);
    columns[1].border_color = Some(teleminator::theme::ColorSpec::Fg("#ff0000".into()));
    let segs = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 1,
            padding: teleminator::config::Padding::default(),
            color: None,
            enabled: true,
        },
    );
    assert_eq!(segs[1].kind, SegmentKind::ColumnBorder);
    assert_eq!(
        segs[1].border_color,
        Some(teleminator::theme::ColorSpec::Fg("#ff0000".into()))
    );
}

#[test]
fn renders_default_style_columns() {
    let opts = FormatOptions {
        timestamp_format: "%H:%M:%S",
        timestamp_localized: true,
        view_line: 3,
    };
    let entry = entry();
    let local_ts = entry
        .timestamp_parsed
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%H:%M:%S")
        .to_string();
    let columns = cols(&[
        ("level", Some(5), Align::Left),
        ("timestamp", None, Align::Left),
        ("message", None, Align::Left),
        ("line", None, Align::Left),
        ("code", None, Align::Left),
    ]);
    assert_eq!(
        render(&columns, &entry, &opts, &ColumnBorderStyle::default()),
        format!("ERROR {local_ts} hi 3 500")
    );
}

#[test]
fn width_and_align() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let columns = cols(&[("level", Some(8), Align::Right)]);
    assert_eq!(
        render(&columns, &entry(), &opts, &ColumnBorderStyle::default()),
        "   ERROR"
    );
}

#[test]
fn center_aligns_in_fixed_width() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let err = entry();
    let columns = cols(&[("level", Some(5), Align::Center)]);
    assert_eq!(
        render(&columns, &err, &opts, &ColumnBorderStyle::default()),
        "ERROR"
    );

    let mut info = entry();
    info.level = LogLevel::Info;
    let columns = cols(&[("level", Some(5), Align::Center)]);
    assert_eq!(
        render(&columns, &info, &opts, &ColumnBorderStyle::default()),
        " INFO"
    );

    let columns = cols(&[("level", Some(6), Align::Center)]);
    assert_eq!(
        render(&columns, &info, &opts, &ColumnBorderStyle::default()),
        " INFO "
    );

    let columns = cols(&[("level", Some(8), Align::Center)]);
    assert_eq!(
        render(&columns, &err, &opts, &ColumnBorderStyle::default()),
        "  ERROR "
    );
}

#[test]
fn truncates_wide_columns() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let columns = cols(&[("message", Some(2), Align::Left)]);
    let mut e = entry();
    e.message = Some("hello".into());
    assert_eq!(
        render(&columns, &e, &opts, &ColumnBorderStyle::default()),
        "h…"
    );
}

#[test]
fn resolves_dotted_nested_fields() {
    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let columns = cols(&[("annotations.url", None, Align::Left)]);
    assert_eq!(
        render(&columns, &entry(), &opts, &ColumnBorderStyle::default()),
        "https://example.com/x"
    );
    let columns = cols(&[("annotations.items.1.id", None, Align::Left)]);
    assert_eq!(
        render(&columns, &entry(), &opts, &ColumnBorderStyle::default()),
        "b"
    );
}

#[test]
fn segments_preserve_kinds() {
    let opts = FormatOptions {
        timestamp_format: "%H:%M:%S",
        timestamp_localized: true,
        view_line: 1,
    };
    let columns = cols(&[
        ("level", None, Align::Left),
        ("timestamp", None, Align::Left),
        ("message", None, Align::Left),
    ]);
    let segs = render_segments(&columns, &entry(), &opts, &ColumnBorderStyle::default());
    assert_eq!(segs[0].kind, SegmentKind::Level);
    assert_eq!(segs[1].kind, SegmentKind::Literal);
    assert_eq!(segs[2].kind, SegmentKind::Timestamp);
    assert_eq!(segs[4].kind, SegmentKind::Message);

    let bordered = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 1,
            padding: teleminator::config::Padding::default(),
            color: None,
            enabled: true,
        },
    );
    assert_eq!(bordered[1].kind, SegmentKind::ColumnBorder);
    assert_eq!(bordered[1].text, "│");
    let wide = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 2,
            padding: teleminator::config::Padding::default(),
            color: None,
            enabled: true,
        },
    );
    assert_eq!(wide[1].text, "││");
    let padded = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 1,
            padding: teleminator::config::Padding::both(1),
            color: None,
            enabled: true,
        },
    );
    assert_eq!(padded[1].text, " │ ");
    let asymmetric = render_segments(
        &columns,
        &entry(),
        &opts,
        &ColumnBorderStyle {
            width: 1,
            padding: teleminator::config::Padding { left: 2, right: 1 },
            color: None,
            enabled: true,
        },
    );
    assert_eq!(asymmetric[1].text, "  │ ");
}

#[test]
fn padding_wraps_fitted_content() {
    use teleminator::config::Padding;

    let opts = FormatOptions {
        timestamp_format: "raw",
        timestamp_localized: true,
        view_line: 1,
    };
    let columns = vec![Column {
        source: "level".into(),
        width: Some(5),
        align: Align::Left,
        padding: Padding::both(1),
        border: None,
        border_color: None,
        border_width: None,
        border_padding: None,
    }];
    assert_eq!(
        render(&columns, &entry(), &opts, &ColumnBorderStyle::default()),
        " ERROR "
    );
}

#[test]
fn auto_widths_align_across_rows() {
    let columns = cols(&[
        ("level", Some(5), Align::Left),
        ("message", None, Align::Left),
        ("code", None, Align::Left),
    ]);
    let mut short = entry();
    short.message = Some("hi".into());
    short.fields[0].value = FieldValue::Number("1".into());
    let mut long = entry();
    long.message = Some("[Network] Response".into());
    long.fields[0].value = FieldValue::Number("500".into());

    let rows = [(&short as &LogEntry, 1usize), (&long, 2)];
    let widths = measure_widths(&columns, &rows, "raw", true);
    assert_eq!(widths[0], 5);
    assert_eq!(widths[1], UnicodeWidthStr::width("[Network] Response"));
    assert_eq!(widths[2], 3); // "500"

    let short_row: String = render_segments_sized(
        &columns,
        &widths,
        &short,
        &FormatOptions {
            timestamp_format: "raw",
            timestamp_localized: true,
            view_line: 1,
        },
        &ColumnBorderStyle::default(),
    )
    .into_iter()
    .map(|s| s.text)
    .collect();
    let long_row: String = render_segments_sized(
        &columns,
        &widths,
        &long,
        &FormatOptions {
            timestamp_format: "raw",
            timestamp_localized: true,
            view_line: 2,
        },
        &ColumnBorderStyle::default(),
    )
    .into_iter()
    .map(|s| s.text)
    .collect();

    let code_start = 5 + 1 + widths[1] + 1;
    assert_eq!(
        UnicodeWidthStr::width(&short_row[..code_start]),
        UnicodeWidthStr::width(&long_row[..code_start])
    );
    assert!(short_row[code_start..].starts_with('1'));
    assert!(long_row[code_start..].starts_with("500"));
    assert_eq!(
        UnicodeWidthStr::width(short_row.as_str()),
        UnicodeWidthStr::width(long_row.as_str())
    );
}
