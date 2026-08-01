use ratatui::style::Style;
use ratatui::text::Span;

use lnav_rs::text::*;

#[test]
fn slice_width_skips_and_takes_display_columns() {
    assert_eq!(slice_width("abcdef", 2, 3), "cde");
    assert_eq!(slice_width("abcdef", 0, 2), "ab");
    assert_eq!(slice_width("abcdef", 4, 10), "ef");
}

#[test]
fn slice_spans_preserves_styles_across_boundaries() {
    let spans = [
        Span::styled(
            String::from("abc"),
            Style::default().fg(ratatui::style::Color::Red),
        ),
        Span::styled(
            String::from("def"),
            Style::default().fg(ratatui::style::Color::Blue),
        ),
    ];
    let sliced = slice_spans(&spans, 2, 3);
    assert_eq!(spans_width(&sliced), 3);
    assert_eq!(sliced.len(), 2);
    assert_eq!(sliced[0].content.as_ref(), "c");
    assert_eq!(sliced[0].style.fg, Some(ratatui::style::Color::Red));
    assert_eq!(sliced[1].content.as_ref(), "de");
    assert_eq!(sliced[1].style.fg, Some(ratatui::style::Color::Blue));
}
