use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use anyhow::{Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::model::LogEntry;
use crate::parse;

pub struct LogSource {
    path: PathBuf,
    entries: Vec<LogEntry>,
    offset: u64,
    next_line_no: usize,
    _watcher: RecommendedWatcher,
    rx: Receiver<()>,
}

impl LogSource {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::channel();
        let watch_path = path.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any
                ) {
                    let _ = tx.send(());
                }
            }
        })
        .context("failed to create file watcher")?;

        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .context("failed to watch log file")?;

        let mut source = Self {
            path,
            entries: Vec::new(),
            offset: 0,
            next_line_no: 1,
            _watcher: watcher,
            rx,
        };
        source.read_new_lines()?;
        Ok(source)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    fn read_new_lines(&mut self) -> Result<usize> {
        let mut file = File::open(&self.path)
            .with_context(|| format!("failed to open {}", self.path.display()))?;
        let metadata = file.metadata()?;
        let len = metadata.len();

        if len < self.offset {
            // Truncated / rotated — reload from start.
            self.offset = 0;
            self.entries.clear();
            self.next_line_no = 1;
        }

        if len == self.offset {
            return Ok(0);
        }

        file.seek(SeekFrom::Start(self.offset))?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        let mut added = 0usize;

        loop {
            buf.clear();
            let n = reader.read_line(&mut buf)?;
            if n == 0 {
                break;
            }

            // Incomplete trailing line: wait for more data.
            if !buf.ends_with('\n') && self.offset + n as u64 >= len {
                break;
            }

            self.offset += n as u64;
            let raw = buf.trim_end_matches(['\r', '\n']).to_string();
            if raw.is_empty() {
                continue;
            }
            self.entries.push(parse::parse_line(self.next_line_no, raw));
            self.next_line_no += 1;
            added += 1;
        }

        Ok(added)
    }

    /// Re-read any unread bytes (polled from the UI loop; notify wakes are coalesced).
    pub fn refresh(&mut self) -> Result<usize> {
        let mut notified = false;
        while self.rx.try_recv().is_ok() {
            notified = true;
        }
        if notified {
            // Coalesce bursty writers before reading.
            std::thread::sleep(std::time::Duration::from_millis(15));
            while self.rx.try_recv().is_ok() {}
        }
        self.read_new_lines()
    }
}
