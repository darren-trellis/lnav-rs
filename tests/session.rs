use std::fs;
use std::io::Write;
use std::path::Path;

use teleminator::config::Config;
use teleminator::session::*;
use teleminator::tail::LogSource;

#[test]
fn path_key_is_stable_fnv1a() {
    // Golden FNV-1a 64-bit of the literal path bytes. The path must not exist
    // so path_key skips canonicalize and hashes the string as given.
    let path = Path::new("/tmp/teleminator-nonexistent/session-key-fixture.jsonl");
    assert_eq!(path_key(path), "61ff4ad7b1c66197");
}

#[test]
fn roundtrip_filters() {
    let session = Session {
        filtering_enabled: false,
        filters: vec![
            SessionFilter {
                kind: "out".into(),
                pattern: "health".into(),
                enabled: true,
            },
            SessionFilter {
                kind: "in".into(),
                pattern: "ERROR".into(),
                enabled: false,
            },
        ],
    };
    let raw = toml::to_string(&session).unwrap();
    let parsed: Session = toml::from_str(&raw).unwrap();
    let (filters, enabled) = parsed
        .into_filters(teleminator::config::CaseMode::Insensitive)
        .unwrap();
    assert!(!enabled);
    assert_eq!(filters.len(), 2);
    assert_eq!(filters[0].label(), "out");
    assert_eq!(filters[0].pattern, "health");
    assert!(filters[0].enabled);
    assert_eq!(filters[1].label(), "in");
    assert!(!filters[1].enabled);
}

#[test]
fn session_path_respects_flags() {
    let dir = std::env::temp_dir().join(format!("teleminator-sess-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let log = dir.join("app.jsonl");
    {
        let mut f = fs::File::create(&log).unwrap();
        writeln!(f, r#"{{"level":"info","message":"hi"}}"#).unwrap();
    }
    let source = LogSource::open_file(&log).unwrap();
    let mut config = Config::default();
    assert!(config.session_filters);
    let path = session_path(&source, &config).unwrap();
    assert!(path.extension().is_some_and(|e| e == "toml"));

    config.session_filters = false;
    assert!(session_path(&source, &config).is_none());
    let _ = fs::remove_dir_all(&dir);
}
