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
