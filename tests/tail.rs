use std::fs::{self, File};
use std::io::Write;
use std::thread;
use std::time::Duration;

use teleminator::tail::*;

#[cfg(unix)]
#[test]
fn stdin_source_reads_json_lines() {
    use std::os::fd::FromRawFd;
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let mut writer = unsafe { File::from_raw_fd(fds[1]) };
    let reader = unsafe { File::from_raw_fd(fds[0]) };

    writeln!(writer, r#"{{"level":"info","msg":"from pipe","n":1}}"#).unwrap();
    writeln!(writer, r#"{{"level":"error","msg":"boom","n":2}}"#).unwrap();
    drop(writer);

    let mut source = LogSource::open_stdin(reader).unwrap();
    let mut added = 0;
    for _ in 0..50 {
        if let RefreshOutcome::Appended(n) = source.refresh().unwrap() {
            added += n;
        }
        if added >= 2 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(source.len(), 2);
    assert_eq!(source.display_name(), "<stdin>");
    assert_eq!(source.entries()[0].message.as_deref(), Some("from pipe"));
    assert_eq!(source.entries()[1].level, teleminator::model::LogLevel::Error);
}

#[test]
fn delete_entries_rewrites_file() {
    let dir = std::env::temp_dir().join(format!("teleminator-del-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("test.jsonl");
    fs::write(
        &path,
        "{\n  \"level\": \"info\",\n  \"msg\": \"keep-me\"\n}\n{\"level\":\"error\",\"msg\":\"drop\"}\n{\"level\":\"info\",\"msg\":\"also-keep\"}\n",
    )
    .unwrap();

    let mut source = LogSource::open_file(&path).unwrap();
    // Pretty object is assembled into one entry, then one-line error, then keep.
    assert_eq!(source.len(), 3);
    let err_idx = source
        .entries()
        .iter()
        .position(|e| e.raw.contains("drop"))
        .unwrap();
    let removed = source.delete_entries(&[err_idx]).unwrap();
    assert_eq!(removed, 1);
    assert!(source.entries().iter().all(|e| !e.raw.contains("drop")));
    assert!(source.entries().iter().any(|e| e.raw.contains("also-keep")));
    assert!(source.entries().iter().any(|e| e.raw.contains("keep-me")));
    let _ = fs::remove_dir_all(&dir);
}
