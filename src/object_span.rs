use crate::assemble;
use crate::model::{LineFormat, LogEntry};

/// Inclusive range of source indices that form the current record.
///
/// - Assembled multi-line JSON/OTel entries (`raw` may contain `\n`): just that entry.
/// - Otherwise, a `{`/`[` opener scans forward until braces balance.
/// - Anything else: just that single line.
pub fn object_span(entries: &[LogEntry], start: usize) -> std::ops::RangeInclusive<usize> {
    if start >= entries.len() {
        return start..=start;
    }

    let first = &entries[start];

    // Ingest already joined multi-line objects into one entry.
    if first.raw.contains('\n') {
        return start..=start;
    }

    let looks_object = matches!(first.format, LineFormat::Json | LineFormat::Otel)
        || first.raw.trim_start().starts_with('{')
        || first.raw.trim_start().starts_with('[');

    if !looks_object {
        return start..=start;
    }

    let mut depth = 0i32;
    let mut seen_open = false;
    assemble::update_depth(&first.raw, &mut depth, &mut seen_open);
    if seen_open && depth == 0 {
        return start..=start;
    }

    for (idx, entry) in entries.iter().enumerate().skip(start + 1) {
        assemble::update_depth(&entry.raw, &mut depth, &mut seen_open);
        if seen_open && depth == 0 {
            return start..=idx;
        }
        if idx - start > 10_000 {
            break;
        }
    }

    if seen_open {
        start..=(entries.len() - 1)
    } else {
        start..=start
    }
}
