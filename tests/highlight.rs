use lnav_rs::highlight::*;
use ratatui::style::{Color, Style};
use regex::RegexBuilder;


fn styles_of(text: &str, pattern: &str) -> Vec<(String, bool)> {
    let re = RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .unwrap();
    let base = Style::default().fg(Color::White);
    let matched = Style::default().fg(Color::Yellow);
    let mut spans = Vec::new();
    push_highlighted(&mut spans, text.to_string(), base, matched, Some(&re));
    spans
        .into_iter()
        .map(|s| {
            let is_match = s.style.fg == Some(Color::Yellow);
            (s.content.to_string(), is_match)
        })
        .collect()
}

#[test]
fn highlights_only_matching_substring() {
    assert_eq!(
        styles_of("hello ERROR world", "error"),
        vec![
            ("hello ".into(), false),
            ("ERROR".into(), true),
            (" world".into(), false),
        ]
    );
}

#[test]
fn highlights_regex_groups() {
    assert_eq!(
        styles_of("status=404 path=/x", r"\d{3}"),
        vec![
            ("status=".into(), false),
            ("404".into(), true),
            (" path=/x".into(), false),
        ]
    );
}
