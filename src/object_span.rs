use crate::model::{LineFormat, LogEntry};

/// Inclusive range of source indices that form the current record.
///
/// - JSON (or a line starting with `{`): the whole object, spanning lines until
///   braces balance (covers JSONL one-liners and pretty-printed objects).
/// - Anything else: just that single line.
pub fn object_span(entries: &[LogEntry], start: usize) -> std::ops::RangeInclusive<usize> {
    if start >= entries.len() {
        return start..=start;
    }

    let first = &entries[start];
    let looks_json = first.format == LineFormat::Json
        || first.raw.trim_start().starts_with('{')
        || first.raw.trim_start().starts_with('[');

    if !looks_json {
        return start..=start;
    }

    let mut depth = 0i32;
    let mut seen_open = false;

    for (idx, entry) in entries.iter().enumerate().skip(start) {
        update_depth(&entry.raw, &mut depth, &mut seen_open);
        if seen_open && depth == 0 {
            return start..=idx;
        }
        // Cap runaway scans on malformed input.
        if idx - start > 10_000 {
            break;
        }
    }

    // Unclosed object — take what we have through the end (or just the line).
    if seen_open {
        start..=(entries.len() - 1)
    } else {
        start..=start
    }
}

fn update_depth(line: &str, depth: &mut i32, seen_open: &mut bool) {
    let mut in_string = false;
    let mut escape = false;

    for ch in line.chars() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' | '[' => {
                *depth += 1;
                *seen_open = true;
            }
            '}' | ']' if *seen_open => *depth -= 1,
            _ => {}
        }
    }
}
