//! Brace-balanced multi-line record assembly for JSON and util.inspect dumps.

const MAX_LINES: usize = 10_000;

#[derive(Debug, Default)]
pub struct RecordAssembler {
    buf: String,
    depth: i32,
    seen_open: bool,
    lines: usize,
    /// File byte offset where the buffered record started (if known).
    pub start_offset: Option<u64>,
    pub start_line_no: Option<usize>,
}

#[derive(Debug)]
pub struct CompletedRecord {
    pub text: String,
    pub start_offset: Option<u64>,
    pub start_line_no: usize,
    pub line_count: usize,
    pub end_offset: Option<u64>,
}

impl RecordAssembler {
    pub fn is_pending(&self) -> bool {
        self.seen_open && self.depth > 0
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.depth = 0;
        self.seen_open = false;
        self.lines = 0;
        self.start_offset = None;
        self.start_line_no = None;
    }

    /// Feed one physical line (no trailing newline). Returns a completed record when
    /// a brace-balanced block finishes, or immediately for non-object lines.
    pub fn feed(
        &mut self,
        line: &str,
        line_no: usize,
        line_start_offset: Option<u64>,
        line_end_offset: Option<u64>,
    ) -> Option<CompletedRecord> {
        if !self.is_pending() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
                return Some(CompletedRecord {
                    text: line.to_string(),
                    start_offset: line_start_offset,
                    start_line_no: line_no,
                    line_count: 1,
                    end_offset: line_end_offset,
                });
            }
            self.start_offset = line_start_offset;
            self.start_line_no = Some(line_no);
            self.buf.clear();
            self.depth = 0;
            self.seen_open = false;
            self.lines = 0;
        }

        if !self.buf.is_empty() {
            self.buf.push('\n');
        }
        self.buf.push_str(line);
        self.lines += 1;
        update_depth(line, &mut self.depth, &mut self.seen_open);

        if self.seen_open && self.depth == 0 {
            return Some(self.take_completed(line_end_offset));
        }

        if self.lines >= MAX_LINES {
            return Some(self.take_completed(line_end_offset));
        }

        None
    }

    fn take_completed(&mut self, end_offset: Option<u64>) -> CompletedRecord {
        let text = std::mem::take(&mut self.buf);
        let start_line_no = self.start_line_no.unwrap_or(1);
        let line_count = self.lines.max(1);
        let start_offset = self.start_offset;
        self.clear();
        CompletedRecord {
            text,
            start_offset,
            start_line_no,
            line_count,
            end_offset,
        }
    }
}

/// Update brace/bracket depth, ignoring content inside `"` or `'` strings.
pub fn update_depth(line: &str, depth: &mut i32, seen_open: &mut bool) {
    let mut chars = line.chars().peekable();
    let mut in_double = false;
    let mut in_single = false;
    let mut escape = false;

    while let Some(ch) = chars.next() {
        if in_double {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_single {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '{' | '[' => {
                *depth += 1;
                *seen_open = true;
            }
            '}' | ']' if *seen_open => *depth -= 1,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_json_completes() {
        let mut a = RecordAssembler::default();
        let done = a.feed(r#"{"a":1}"#, 1, Some(0), Some(7));
        let rec = done.expect("complete");
        assert_eq!(rec.text, r#"{"a":1}"#);
        assert!(!a.is_pending());
    }

    #[test]
    fn multiline_inspect_completes() {
        let mut a = RecordAssembler::default();
        assert!(a.feed("{", 1, Some(0), Some(2)).is_none());
        assert!(a.feed("  traceId: 'abc',", 2, None, None).is_none());
        assert!(a.feed("  id: 'def',", 3, None, None).is_none());
        assert!(a.feed("  nested: { foo: 1 },", 4, None, None).is_none());
        let done = a.feed("}", 5, None, Some(100));
        let rec = done.expect("complete");
        assert!(rec.text.contains("traceId"));
        assert_eq!(rec.line_count, 5);
        assert!(!a.is_pending());
    }

    #[test]
    fn braces_inside_single_quotes_ignored() {
        let mut depth = 0;
        let mut seen = false;
        update_depth("{ name: 'a{b}c' }", &mut depth, &mut seen);
        assert!(seen);
        assert_eq!(depth, 0);
    }

    #[test]
    fn plain_line_passthrough() {
        let mut a = RecordAssembler::default();
        let done = a.feed("hello", 1, None, None).unwrap();
        assert_eq!(done.text, "hello");
    }
}
