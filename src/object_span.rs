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
            '}' | ']' => {
                if *seen_open {
                    *depth -= 1;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LogLevel;
    use crate::parse;

    fn entries(lines: &[&str]) -> Vec<LogEntry> {
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| parse::parse_line(i + 1, (*l).to_string()))
            .collect()
    }

    #[test]
    fn single_line_json_object() {
        let e = entries(&[r#"{"level":"info","msg":"hi"}"#, r#"{"level":"error"}"#]);
        assert_eq!(*object_span(&e, 0).start(), 0);
        assert_eq!(*object_span(&e, 0).end(), 0);
        assert_eq!(e[0].format, LineFormat::Json);
        assert_eq!(e[0].level, LogLevel::Info);
    }

    #[test]
    fn multiline_json_object() {
        let e = entries(&[
            "{",
            r#"  "level": "info","#,
            r#"  "msg": "hi""#,
            "}",
            r#"{"level":"error"}"#,
        ]);
        assert_eq!(object_span(&e, 0), 0..=3);
        assert_eq!(object_span(&e, 4), 4..=4);
    }

    #[test]
    fn plain_line_is_single() {
        let e = entries(&["just a line", "another"]);
        assert_eq!(object_span(&e, 0), 0..=0);
    }
}
