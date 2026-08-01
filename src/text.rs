use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn truncate_width(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(value) <= max_width {
        return value.to_string();
    }

    let ellipsis = '…';
    let ellipsis_width = ellipsis.width().unwrap_or(1);
    if max_width <= ellipsis_width {
        return ellipsis.to_string();
    }

    let mut out = String::new();
    let mut width = 0;
    for ch in value.chars() {
        let char_width = ch.width().unwrap_or(0);
        if width + char_width + ellipsis_width > max_width {
            break;
        }
        out.push(ch);
        width += char_width;
    }
    out.push(ellipsis);
    out
}

/// Skip `skip` display columns, then take up to `take` display columns.
pub fn slice_width(value: &str, skip: usize, take: usize) -> String {
    if take == 0 {
        return String::new();
    }
    let mut skipped = 0;
    let mut taken = 0;
    let mut out = String::new();
    let mut started = skip == 0;
    for ch in value.chars() {
        let char_width = ch.width().unwrap_or(0);
        if !started {
            if skipped + char_width > skip {
                // Partial wide char at the left edge — skip it.
                skipped = skip;
                started = true;
                continue;
            }
            skipped += char_width;
            if skipped >= skip {
                started = true;
            }
            continue;
        }
        if taken + char_width > take {
            break;
        }
        out.push(ch);
        taken += char_width;
        if taken >= take {
            break;
        }
    }
    out
}

/// Total display width of a span list.
pub fn spans_width(spans: &[ratatui::text::Span<'_>]) -> usize {
    spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum()
}

/// Skip `skip` display columns across styled spans, then take up to `take`.
pub fn slice_spans(
    spans: &[ratatui::text::Span<'_>],
    skip: usize,
    take: usize,
) -> Vec<ratatui::text::Span<'static>> {
    if take == 0 {
        return Vec::new();
    }
    let mut skipped = 0usize;
    let mut taken = 0usize;
    let mut out = Vec::new();
    let mut started = skip == 0;

    for span in spans {
        let content = span.content.as_ref();
        let width = UnicodeWidthStr::width(content);
        if width == 0 {
            if started {
                out.push(ratatui::text::Span::styled(
                    content.to_string(),
                    span.style,
                ));
            }
            continue;
        }

        if !started {
            if skipped + width <= skip {
                skipped += width;
                if skipped >= skip {
                    started = true;
                }
                continue;
            }
            let need_skip = skip - skipped;
            let rest = slice_width(content, need_skip, usize::MAX);
            skipped = skip;
            started = true;
            let rest_w = UnicodeWidthStr::width(rest.as_str());
            if rest_w == 0 {
                continue;
            }
            if taken + rest_w > take {
                let part = slice_width(&rest, 0, take - taken);
                if !part.is_empty() {
                    out.push(ratatui::text::Span::styled(part, span.style));
                }
                break;
            }
            out.push(ratatui::text::Span::styled(rest, span.style));
            taken += rest_w;
            if taken >= take {
                break;
            }
            continue;
        }

        if taken + width <= take {
            out.push(ratatui::text::Span::styled(
                content.to_string(),
                span.style,
            ));
            taken += width;
            if taken >= take {
                break;
            }
            continue;
        }

        let part = slice_width(content, 0, take - taken);
        if !part.is_empty() {
            out.push(ratatui::text::Span::styled(part, span.style));
        }
        break;
    }
    out
}
