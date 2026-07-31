use std::fs;
use std::io::Write;

use lnav_rs::config::Config;
use lnav_rs::session::*;
use lnav_rs::tail::LogSource;

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
        .into_filters(lnav_rs::config::CaseMode::Insensitive)
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
    let dir = std::env::temp_dir().join(format!("lnav-rs-sess-{}", std::process::id()));
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
    assert!(config.session_stdin);
    let path = session_path(&source, &config).unwrap();
    assert!(path.extension().is_some_and(|e| e == "toml"));

    config.session_filters = false;
    assert!(session_path(&source, &config).is_none());
    let _ = fs::remove_dir_all(&dir);
}
