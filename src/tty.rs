//! When logs arrive on a pipe, stdin is not a TTY. Crossterm still needs a
//! terminal for keyboard input, so we steal the pipe fd and point fd 0 at `/dev/tty`.

use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal};
use std::os::fd::{AsRawFd, FromRawFd};

use anyhow::{bail, Context, Result};

/// If stdin is a pipe/file (not a TTY), duplicate it and replace stdin with `/dev/tty`.
/// Returns the duplicated pipe for the log reader.
pub fn take_piped_stdin() -> Result<Option<File>> {
    if io::stdin().is_terminal() {
        return Ok(None);
    }

    #[cfg(unix)]
    {
        take_piped_stdin_unix()
    }

    #[cfg(not(unix))]
    {
        bail!("reading logs from a pipe is only supported on Unix")
    }
}

#[cfg(unix)]
fn take_piped_stdin_unix() -> Result<Option<File>> {
    let stdin_fd = io::stdin().as_raw_fd();
    let pipe_fd = unsafe { libc::dup(stdin_fd) };
    if pipe_fd < 0 {
        return Err(io::Error::last_os_error())
            .context("failed to duplicate stdin pipe");
    }
    let pipe = unsafe { File::from_raw_fd(pipe_fd) };

    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .context(
            "failed to open /dev/tty for keyboard input \
             (required when reading logs from a pipe)",
        )?;

    if unsafe { libc::dup2(tty.as_raw_fd(), stdin_fd) } < 0 {
        return Err(io::Error::last_os_error())
            .context("failed to attach /dev/tty to stdin");
    }

    Ok(Some(pipe))
}

pub fn require_piped_stdin() -> Result<File> {
    match take_piped_stdin()? {
        Some(pipe) => Ok(pipe),
        None => bail!(
            "stdin is a terminal; pipe JSON/log lines in, e.g.\n  \
             myapp | lnav-rs\n  \
             or: lnav-rs - < app.jsonl"
        ),
    }
}
