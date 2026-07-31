use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::model::LogEntry;
use crate::parse;

enum Backend {
    File {
        path: PathBuf,
        offset: u64,
        identity: Option<FileIdentity>,
        last_check: Instant,
        _watcher: RecommendedWatcher,
        notify_rx: Receiver<()>,
    },
    Stdin {
        line_rx: Receiver<String>,
        _reader: thread::JoinHandle<()>,
    },
}

#[cfg(unix)]
type FileIdentity = (u64, u64);

#[cfg(not(unix))]
type FileIdentity = ();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    Unchanged,
    Appended(usize),
    Truncated(usize),
    Replaced(usize),
}

impl RefreshOutcome {
    pub fn changed(self) -> bool {
        !matches!(self, Self::Unchanged)
    }

    pub fn reset(self) -> bool {
        matches!(self, Self::Truncated(_) | Self::Replaced(_))
    }
}

pub struct LogSource {
    label: String,
    entries: Vec<LogEntry>,
    source_ranges: Vec<std::ops::Range<u64>>,
    next_line_no: usize,
    backend: Backend,
}

impl LogSource {
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let (tx, notify_rx) = mpsc::channel();
        let watch_path = path.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res
                && matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any
                )
            {
                let _ = tx.send(());
            }
        })
        .context("failed to create file watcher")?;

        watcher
            .watch(&watch_path, RecursiveMode::NonRecursive)
            .context("failed to watch log file")?;

        let mut source = Self {
            label: path.display().to_string(),
            entries: Vec::new(),
            source_ranges: Vec::new(),
            next_line_no: 1,
            backend: Backend::File {
                path,
                offset: 0,
                identity: None,
                last_check: Instant::now(),
                _watcher: watcher,
                notify_rx,
            },
        };
        let _ = source.read_file_lines()?;
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
            source_ranges: Vec::new(),
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
        let mut sorted_indices: Vec<_> = remove.iter().copied().collect();
        sorted_indices.sort_unstable();
        if sorted_indices
            .iter()
            .any(|&i| i >= self.source_ranges.len())
        {
            bail!("file changed; refresh before deleting");
        }
        let mut ranges: Vec<std::ops::Range<u64>> = Vec::new();
        for idx in sorted_indices {
            let range = self.source_ranges[idx].clone();
            if let Some(last) = ranges.last_mut()
                && idx > 0
                && self.source_ranges[idx - 1].end == last.end
            {
                last.end = range.end;
            } else {
                ranges.push(range);
            }
        }

        let expected_identity = match &self.backend {
            Backend::File { identity, .. } => *identity,
            Backend::Stdin { .. } => None,
        };
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("failed to open {} for rewrite", path.display()))?;
        let metadata = file.metadata()?;
        if expected_identity.is_some_and(|expected| expected != file_identity(&metadata)) {
            bail!("file was replaced; refresh before deleting");
        }
        let mut original = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut original)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if ranges
            .last()
            .is_some_and(|range| range.end > original.len() as u64)
        {
            bail!("file was truncated; refresh before deleting");
        }

        let mut rewritten = Vec::with_capacity(original.len());
        let mut cursor = 0usize;
        for range in &ranges {
            let start = range.start as usize;
            let end = range.end as usize;
            rewritten.extend_from_slice(&original[cursor..start]);
            cursor = end;
        }
        rewritten.extend_from_slice(&original[cursor..]);

        file.seek(SeekFrom::Start(0))?;
        file.write_all(&rewritten)
            .with_context(|| format!("failed to rewrite {}", path.display()))?;
        file.set_len(rewritten.len() as u64)
            .with_context(|| format!("failed to truncate {}", path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync {}", path.display()))?;

        self.entries.clear();
        self.source_ranges.clear();
        self.next_line_no = 1;
        if let Backend::File { offset, .. } = &mut self.backend {
            *offset = 0;
        }
        if let Backend::File { notify_rx, .. } = &self.backend {
            while notify_rx.try_recv().is_ok() {}
        }
        let _ = self.read_file_lines()?;
        Ok(remove.len())
    }

    fn read_file_lines(&mut self) -> Result<RefreshOutcome> {
        let (path, old_offset, old_identity) = match &self.backend {
            Backend::File {
                path,
                offset,
                identity,
                ..
            } => (path.clone(), *offset, *identity),
            _ => bail!("not a file source"),
        };

        let mut file =
            File::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
        let metadata = file.metadata()?;
        let len = metadata.len();
        let identity = file_identity(&metadata);
        let replaced = old_identity.is_some_and(|old| old != identity);
        let truncated = !replaced && len < old_offset;
        let reset = replaced || truncated;
        let offset = if reset { 0 } else { old_offset };

        if reset {
            self.entries.clear();
            self.source_ranges.clear();
            self.next_line_no = 1;
        }

        if len == offset {
            if let Backend::File {
                offset: stored,
                identity: stored_identity,
                ..
            } = &mut self.backend
            {
                *stored = offset;
                *stored_identity = Some(identity);
            }
            return Ok(if replaced {
                RefreshOutcome::Replaced(0)
            } else if truncated {
                RefreshOutcome::Truncated(0)
            } else {
                RefreshOutcome::Unchanged
            });
        }

        file.seek(SeekFrom::Start(offset))?;
        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        let mut added = 0usize;
        let mut new_offset = offset;

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
            self.source_ranges.push((new_offset - n as u64)..new_offset);
            self.next_line_no += 1;
            added += 1;
        }

        if let Backend::File {
            offset,
            identity: stored_identity,
            ..
        } = &mut self.backend
        {
            *offset = new_offset;
            *stored_identity = Some(identity);
        }

        Ok(if replaced {
            RefreshOutcome::Replaced(added)
        } else if truncated {
            RefreshOutcome::Truncated(added)
        } else if added > 0 {
            RefreshOutcome::Appended(added)
        } else {
            RefreshOutcome::Unchanged
        })
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

    pub fn refresh(&mut self) -> Result<RefreshOutcome> {
        if matches!(self.backend, Backend::Stdin { .. }) {
            let added = self.drain_stdin_lines();
            return Ok(if added > 0 {
                RefreshOutcome::Appended(added)
            } else {
                RefreshOutcome::Unchanged
            });
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

        let should_check = match &self.backend {
            Backend::File { last_check, .. } => {
                notified || last_check.elapsed() >= Duration::from_secs(1)
            }
            Backend::Stdin { .. } => false,
        };
        if !should_check {
            return Ok(RefreshOutcome::Unchanged);
        }
        if let Backend::File { last_check, .. } = &mut self.backend {
            *last_check = Instant::now();
        }

        if notified {
            std::thread::sleep(Duration::from_millis(15));
            if let Backend::File { notify_rx, .. } = &self.backend {
                while notify_rx.try_recv().is_ok() {}
            }
        }

        self.read_file_lines()
    }
}

#[cfg(unix)]
fn file_identity(metadata: &std::fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;
    (metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn file_identity(_metadata: &std::fs::Metadata) -> FileIdentity {}
