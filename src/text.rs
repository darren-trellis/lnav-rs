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
