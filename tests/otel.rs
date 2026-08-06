use std::fs;

use teleminator::assemble::RecordAssembler;
use teleminator::model::LineFormat;
use teleminator::parse::parse_line;
use teleminator::tail::LogSource;
use teleminator::trace;

#[test]
fn assembles_json_and_otel_into_entries() {
    let dir = std::env::temp_dir().join(format!("teleminator-otel-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("mixed.txt");
    fs::write(&path, include_str!("../examples/sample-otel.txt")).unwrap();

    let source = LogSource::open_file(&path).unwrap();
    // 2 JSON lines + 2 OTel spans (each multi-line dump → 1 entry)
    assert_eq!(source.len(), 4);
    assert_eq!(source.entries()[0].format, LineFormat::Json);
    assert_eq!(source.entries()[1].format, LineFormat::Otel);
    assert_eq!(source.entries()[2].format, LineFormat::Otel);
    assert_eq!(source.entries()[3].format, LineFormat::Json);
    assert_eq!(
        source.entries()[1].message.as_deref(),
        Some("jwt.verify")
    );
    assert_eq!(source.entries()[2].message.as_deref(), Some("HTTP GET"));

    let indices: Vec<_> = (0..source.len()).collect();
    let forest = trace::build_forest(source.entries(), &indices);
    assert_eq!(forest.traces.len(), 1);
    assert_eq!(forest.traces[0].span_count(), 2);
    // HTTP GET is parent of jwt.verify
    let root = forest.traces[0].roots[0];
    assert_eq!(forest.traces[0].spans[root].name, "HTTP GET");
    assert_eq!(forest.traces[0].spans[root].children.len(), 1);
    let child = forest.traces[0].spans[root].children[0];
    assert_eq!(forest.traces[0].spans[child].name, "jwt.verify");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn assembler_joins_multiline_before_parse() {
    let mut a = RecordAssembler::default();
    let mut records = Vec::new();
    let sample = include_str!("../examples/sample-otel.txt");
    for (i, line) in sample.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        if let Some(rec) = a.feed(line, i + 1, None, None) {
            records.push(parse_line(rec.start_line_no, rec.text));
        }
    }
    assert!(!a.is_pending());
    assert_eq!(records.len(), 4);
    assert!(records.iter().any(|e| e.format == LineFormat::Otel));
}
