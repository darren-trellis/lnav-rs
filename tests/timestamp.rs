use chrono::Local;

use teleminator::timestamp::*;

#[test]
fn parses_rfc3339() {
    let dt = parse("2026-07-27T23:58:14.817Z").unwrap();
    assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-07-27");
}

#[test]
fn formats_in_local_timezone() {
    let raw = "2026-07-27T23:58:14.817Z";
    let dt = parse(raw).unwrap();
    let expected = dt.with_timezone(&Local).format("%H:%M:%S").to_string();
    assert_eq!(format(raw, Some(&dt), "%H:%M:%S", true), expected);
    // Sanity: local display should differ from UTC when offset is non-zero.
    let utc = dt.format("%H:%M:%S").to_string();
    let local = format(raw, Some(&dt), "%H:%M:%S", true);
    if Local::now().offset().local_minus_utc() != 0 {
        assert_ne!(local, utc);
    }
}

#[test]
fn formats_in_utc_when_not_localized() {
    let raw = "2026-07-27T23:58:14.817Z";
    let dt = parse(raw).unwrap();
    assert_eq!(format(raw, Some(&dt), "%H:%M:%S", false), "23:58:14");
}

#[test]
fn raw_passthrough() {
    assert_eq!(format("abc", None, "raw", true), "abc");
    assert_eq!(format("abc", None, "", true), "abc");
}
