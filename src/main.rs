mod app;
mod command;
mod completion;
mod config;
mod details;
mod filter;
mod keys;
mod columns;
mod model;
mod object_span;
mod parse;
mod session;
mod tail;
mod theme;
mod timestamp;
mod tty;
mod ui;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Parser;

use crate::app::App;
use crate::config::Config;
use crate::tail::LogSource;
use crate::theme::Theme;

#[derive(Debug, Parser)]
#[command(
    name = "lnav-rs",
    about = "A modern log file navigator — Rust + ratatui rewrite of lnav essentials",
    version
)]
struct Cli {
    /// Log file to open, or `-` for stdin (also: `prog | lnav-rs`)
    file: Option<PathBuf>,

    /// Theme name (overrides config)
    #[arg(short, long)]
    theme: Option<String>,

    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Write a default config file and exit
    #[arg(long)]
    init_config: bool,

    /// List themes (built-in + user) and exit
    #[arg(long)]
    list_themes: bool,
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.init_config {
        let path = Config::default().write()?;
        println!("wrote {}", path.display());
        println!("themes dir: {}", Config::themes_dir().display());
        return Ok(());
    }

    if cli.list_themes {
        for name in Theme::list_names() {
            println!("{name}");
        }
        return Ok(());
    }

    let (mut config, loaded_from) = if let Some(path) = &cli.config {
        Config::load_from(path)?
    } else {
        Config::load()?
    };

    if let Some(theme) = &cli.theme {
        config.theme.set_name(theme);
    }

    let source = open_source(cli.file.as_ref())?;

    let mut terminal = ratatui::init();
    let result = (|| {
        let mut app = App::new(source, config)?;
        if let Some(path) = loaded_from {
            app.status_message = Some(format!("config: {}", path.display()));
        }
        app.run(&mut terminal)
    })();
    ratatui::restore();
    // Theme picker enables mouse capture; ensure it is always cleared on exit.
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture
    );
    result
}

fn open_source(file: Option<&PathBuf>) -> Result<LogSource> {
    let stdin_is_tty = std::io::stdin().is_terminal();

    match file.map(|p| p.as_os_str()) {
        // Explicit stdin.
        Some(p) if p == "-" => {
            let pipe = tty::require_piped_stdin()?;
            LogSource::open_stdin(pipe)
        }
        // Regular file path.
        Some(_) => {
            let path = file.expect("checked");
            if !path.is_file() {
                bail!("not a file: {}", path.display());
            }
            LogSource::open_file(path)
        }
        // No path: use pipe if present, otherwise ask for a file.
        None => {
            if stdin_is_tty {
                bail!(
                    "a log file is required, or pipe data to stdin\n  \
                     lnav-rs app.jsonl\n  \
                     myapp | lnav-rs"
                );
            }
            let pipe = tty::require_piped_stdin()?;
            LogSource::open_stdin(pipe)
        }
    }
}
