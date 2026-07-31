use crate::model::{Field, FieldValue, LogLevel};

pub fn parse_logfmt(line: &str) -> Option<(LogLevel, Option<String>, Option<String>, Vec<Field>)> {
    let fields = tokenize(line)?;
    if fields.is_empty() {
        return None;
    }

    let mut level = LogLevel::Unknown;
    let mut timestamp = None;
    let mut message = None;
    let mut out = Vec::with_capacity(fields.len());

    for (key, value) in fields {
        let key_l = key.to_ascii_lowercase();
        match key_l.as_str() {
            "level" | "lvl" | "severity" | "log_level" => {
                level = LogLevel::parse(&value);
            }
            "time" | "timestamp" | "ts" | "@timestamp" | "datetime" | "date" => {
                timestamp = Some(value.clone());
            }
            "msg" | "message" | "text" | "log" => {
                message = Some(value.clone());
            }
            _ => {}
        }
        out.push(Field {
            key,
            value: FieldValue::String(value),
        });
    }

    Some((level, timestamp, message, out))
}

fn tokenize(input: &str) -> Option<Vec<(String, String)>> {
    let mut fields = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return None;
        }
        let key = &input[key_start..i];
        if key.is_empty() || !looks_like_key(key) {
            return None;
        }
        i += 1;

        let value = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let mut out = String::new();
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' if i + 1 < bytes.len() => {
                        out.push(bytes[i + 1] as char);
                        i += 2;
                    }
                    b'"' => {
                        i += 1;
                        break;
                    }
                    c => {
                        out.push(c as char);
                        i += 1;
                    }
                }
            }
            out
        } else {
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            input[start..i].to_string()
        };

        fields.push((key.to_string(), value));
    }

    if fields.len() < 2 {
        return None;
    }
    Some(fields)
}

fn looks_like_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' || c == '@')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_logfmt() {
        let (level, ts, msg, fields) =
            parse_logfmt(r#"time=2024-01-01T00:00:00Z level=info msg="hello world" user=ada"#)
                .unwrap();
        assert_eq!(level, LogLevel::Info);
        assert_eq!(ts.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(msg.as_deref(), Some("hello world"));
        assert_eq!(fields.len(), 4);
    }
}
