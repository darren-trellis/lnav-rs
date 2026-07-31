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
