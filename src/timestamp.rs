use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

/// Parse a log timestamp into UTC.
///
/// Accepts RFC3339 / ISO-8601, common naive formats, and unix seconds/millis.
pub fn parse(raw: &str) -> Option<DateTime<Utc>> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Tolerate ISO without timezone → assume UTC.
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%d/%b/%Y:%H:%M:%S",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    // Bracketed time-of-day like [16:58:14.532] — attach today's UTC date.
    let tod = s.trim_matches(|c| c == '[' || c == ']');
    for fmt in ["%H:%M:%S%.f", "%H:%M:%S"] {
        if let Ok(t) = chrono::NaiveTime::parse_from_str(tod, fmt) {
            let naive = Utc::now().date_naive().and_time(t);
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    // Unix epoch seconds / milliseconds.
    if let Ok(n) = s.parse::<i64>() {
        if n > 1_000_000_000_000 {
            return Utc.timestamp_millis_opt(n).single();
        }
        if n > 1_000_000_000 {
            return Utc.timestamp_opt(n, 0).single();
        }
    }
    if let Ok(n) = s.parse::<f64>() {
        let secs = n.trunc() as i64;
        let nanos = ((n.fract()) * 1_000_000_000.0) as u32;
        if secs > 1_000_000_000 {
            return Utc.timestamp_opt(secs, nanos).single();
        }
    }

    None
}

/// Format a timestamp.
///
/// When `localized` is true, convert to the local timezone before formatting;
/// otherwise keep UTC. `fmt` of `""` or `"raw"` returns the original string.
pub fn format(raw: &str, parsed: Option<&DateTime<Utc>>, fmt: &str, localized: bool) -> String {
    let fmt = fmt.trim();
    if fmt.is_empty() || fmt.eq_ignore_ascii_case("raw") {
        return raw.to_string();
    }

    let dt = parsed.copied().or_else(|| parse(raw));
    match dt {
        Some(dt) if localized => dt.with_timezone(&Local).format(fmt).to_string(),
        Some(dt) => dt.format(fmt).to_string(),
        None => raw.to_string(),
    }
}

pub const DEFAULT_FORMAT: &str = "%H:%M:%S";
