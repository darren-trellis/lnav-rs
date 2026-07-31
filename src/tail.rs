use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use anyhow::{bail, Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::model::LogEntry;
use crate::parse;

enum Backend {
    File {
        path: PathBuf,
        offset: u64,
        _watcher: RecommendedWatcher,
        notify_rx: Receiver<()>,
    },
    Stdin {
        line_rx: Receiver<String>,
        _reader: thread::JoinHandle<()>,
    },
}

pub struct LogSource {
    label: String,
    entries: Vec<LogEntry>,
    next_line_no: usize,
    backend: Backend,
}

impl LogSource {
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let (tx, notify_rx) = mpsc::channel();
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
            label: path.display().to_string(),
            entries: Vec::new(),
            next_line_no: 1,
            backend: Backend::File {
                path,
                offset: 0,
                _watcher: watcher,
                notify_rx,
            },
        };
        source.read_file_lines()?;
        Ok(source)
    }

    pub fn open_stdin(pipe: File) -> Result<Self> {
        let (tx, line_rx) = mpsc::sync_channel::<String>(8192);
        let reader = thread::Builder::new()
            .name("lnav-rs-stdin".into())
            .spawn(move || {
                let mut reader = BufReader::new(pipe);
                let mut buf = String::new();
                loop {
                    buf.clear();
                    match reader.read_line(&mut buf) {
                        Ok(0) => break,
                        Ok(_) => {
                            let line = buf.trim_end_matches(['\r', '\n']).to_string();
                            if line.is_empty() {
                                continue;
                            }
                            if tx.send(line).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
            .context("failed to spawn stdin reader")?;

        Ok(Self {
            label: "<stdin>".into(),
            entries: Vec::new(),
            next_line_no: 1,
            backend: Backend::Stdin {
                line_rx,
                _reader: reader,
            },
        })
    }

    pub fn display_name(&self) -> &str {
        &self.label
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_file(&self) -> bool {
        matches!(self.backend, Backend::File { .. })
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.backend {
            Backend::File { path, .. } => Some(path.as_path()),
            Backend::Stdin { .. } => None,
        }
    }

    /// Remove the given source indices from the on-disk file and reload.
    ///
    /// Rewrites **in place** on the same inode (truncate + write) so writers
    /// that still hold the file open — e.g. `tee -a` — keep appending to the
    /// same file instead of an orphaned inode from a rename replace.
    pub fn delete_entries(&mut self, indices: &[usize]) -> Result<usize> {
        let path = match &self.backend {
            Backend::File { path, .. } => path.clone(),
            Backend::Stdin { .. } => bail!("cannot delete lines from stdin"),
        };

        if indices.is_empty() {
            return Ok(0);
        }

        let remove: HashSet<usize> = indices.iter().copied().collect();
        let mut body = String::new();
        let mut removed = 0usize;
        for (i, entry) in self.entries.iter().enumerate() {
            if remove.contains(&i) {
                removed += 1;
                continue;
            }
            body.push_str(&entry.raw);
            body.push('\n');
        }

        {
            let mut f = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&path)
                .with_context(|| format!("failed to open {} for rewrite", path.display()))?;
            f.write_all(body.as_bytes())
                .with_context(|| format!("failed to rewrite {}", path.display()))?;
            f.sync_all()
                .with_context(|| format!("failed to sync {}", path.display()))?;
        }

        // Full reload so line numbers / offsets stay consistent.
        self.entries.clear();
        self.next_line_no = 1;
        if let Backend::File { offset, .. } = &mut self.backend {
            *offset = 0;
        }
        // Drain stale notify events from our own rewrite.
        if let Backend::File { notify_rx, .. } = &self.backend {
            while notify_rx.try_recv().is_ok() {}
        }
        self.read_file_lines()?;
        Ok(removed)
    }

    fn read_file_lines(&mut self) -> Result<usize> {
        let (path, offset) = match &mut self.backend {
            Backend::File { path, offset, .. } => (path.clone(), offset),
            _ => bail!("not a file source"),
        };

        let mut file =
            File::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
        let metadata = file.metadata()?;
        let len = metadata.len();

        if len < *offset {
            *offset = 0;
            self.entries.clear();
            self.next_line_no = 1;
        }

        if len == *offset {
            return Ok(0);
        }

        file.seek(SeekFrom::Start(*offset))?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        let mut added = 0usize;
        let mut new_offset = *offset;

        loop {
            buf.clear();
            let n = reader.read_line(&mut buf)?;
            if n == 0 {
                break;
            }

            if !buf.ends_with('\n') && new_offset + n as u64 >= len {
                break;
            }

            new_offset += n as u64;
            let raw = buf.trim_end_matches(['\r', '\n']).to_string();
            if raw.is_empty() {
                continue;
            }
            self.entries.push(parse::parse_line(self.next_line_no, raw));
            self.next_line_no += 1;
            added += 1;
        }

        if let Backend::File { offset, .. } = &mut self.backend {
            *offset = new_offset;
        }

        Ok(added)
    }

    fn drain_stdin_lines(&mut self) -> usize {
        let mut batch = Vec::new();
        if let Backend::Stdin { line_rx, .. } = &self.backend {
            while let Ok(raw) = line_rx.try_recv() {
                batch.push(raw);
                if batch.len() >= 2000 {
                    break;
                }
            }
        } else {
            return 0;
        }

        let mut added = 0usize;
        for raw in batch {
            self.entries.push(parse::parse_line(self.next_line_no, raw));
            self.next_line_no += 1;
            added += 1;
        }
        added
    }

    pub fn refresh(&mut self) -> Result<usize> {
        if matches!(self.backend, Backend::Stdin { .. }) {
            return Ok(self.drain_stdin_lines());
        }

        let notified = {
            let Backend::File { notify_rx, .. } = &self.backend else {
                bail!("not a file source");
            };
            let mut notified = false;
            while notify_rx.try_recv().is_ok() {
                notified = true;
            }
            notified
        };

        if notified {
            std::thread::sleep(std::time::Duration::from_millis(15));
            if let Backend::File { notify_rx, .. } = &self.backend {
                while notify_rx.try_recv().is_ok() {}
            }
        }

        self.read_file_lines()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::Duration;

    #[cfg(unix)]
    #[test]
    fn stdin_source_reads_json_lines() {
        use std::os::fd::FromRawFd;
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        let mut writer = unsafe { File::from_raw_fd(fds[1]) };
        let reader = unsafe { File::from_raw_fd(fds[0]) };

        writeln!(
            writer,
            r#"{{"level":"info","msg":"from pipe","n":1}}"#
        )
        .unwrap();
        writeln!(writer, r#"{{"level":"error","msg":"boom","n":2}}"#).unwrap();
        drop(writer);

        let mut source = LogSource::open_stdin(reader).unwrap();
        let mut added = 0;
        for _ in 0..50 {
            added += source.refresh().unwrap();
            if added >= 2 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(source.len(), 2);
        assert_eq!(source.display_name(), "<stdin>");
        assert_eq!(source.entries()[0].message.as_deref(), Some("from pipe"));
        assert_eq!(source.entries()[1].level, crate::model::LogLevel::Error);
    }

    #[test]
    fn delete_entries_rewrites_file() {
        let dir = std::env::temp_dir().join(format!("lnav-rs-del-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.jsonl");
        fs::write(
            &path,
            "{\n  \"level\": \"info\",\n  \"msg\": \"keep-me\"\n}\n{\"level\":\"error\",\"msg\":\"drop\"}\n{\"level\":\"info\",\"msg\":\"also-keep\"}\n",
        )
        .unwrap();

        let mut source = LogSource::open_file(&path).unwrap();
        // Pretty object is indices 0..=3, then one-line error, then keep.
        // Find the error line and delete its object span (single line).
        let err_idx = source
            .entries()
            .iter()
            .position(|e| e.raw.contains("drop"))
            .unwrap();
        let removed = source.delete_entries(&[err_idx]).unwrap();
        assert_eq!(removed, 1);
        assert!(source.entries().iter().all(|e| !e.raw.contains("drop")));
        assert!(source.entries().iter().any(|e| e.raw.contains("also-keep")));
        let _ = fs::remove_dir_all(&dir);
    }
}
