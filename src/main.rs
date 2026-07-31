mod app;
mod model;
mod parse;
mod tail;
mod theme;
mod ui;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Parser;

use crate::app::App;
use crate::theme::Theme;

#[derive(Debug, Parser)]
#[command(
    name = "lnav-rs",
    about = "A modern log file navigator — Rust + ratatui rewrite of lnav essentials",
    version
)]
struct Cli {
    /// Log file to open
    file: Option<PathBuf>,

    /// Theme name: catppuccin, nord, tokyo-night
    #[arg(short, long, default_value = "catppuccin")]
    theme: String,

    /// List built-in themes and exit
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

    if cli.list_themes {
        for name in Theme::available() {
            println!("{name}");
        }
        return Ok(());
    }

    let Some(file) = cli.file else {
        bail!("a log file is required (see --help)");
    };
    if !file.is_file() {
        bail!("not a file: {}", file.display());
    }

    let mut terminal = ratatui::init();
    let result = (|| {
        let mut app = App::new(file, &cli.theme)?;
        app.run(&mut terminal)
    })();
    ratatui::restore();
    result
}
