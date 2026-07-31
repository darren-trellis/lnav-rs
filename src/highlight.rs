use ratatui::style::Style;
use ratatui::text::Span;
use regex::Regex;

/// Split `text` into spans, styling regex matches with `match_style`.
pub fn push_highlighted(
    spans: &mut Vec<Span<'static>>,
    text: String,
    base: Style,
    match_style: Style,
    regex: Option<&Regex>,
) {
    let Some(re) = regex else {
        spans.push(Span::styled(text, base));
        return;
    };

    let mut last = 0;
    let mut found = false;
    for m in re.find_iter(&text) {
        if m.start() == m.end() {
            continue;
        }
        found = true;
        if m.start() > last {
            spans.push(Span::styled(text[last..m.start()].to_string(), base));
        }
        spans.push(Span::styled(
            text[m.start()..m.end()].to_string(),
            match_style,
        ));
        last = m.end();
    }
    if !found {
        spans.push(Span::styled(text, base));
        return;
    }
    if last < text.len() {
        spans.push(Span::styled(text[last..].to_string(), base));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
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
}
