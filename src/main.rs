use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::Parser;

use teleminator::app::App;
use teleminator::config::Config;
use teleminator::tail::LogSource;
use teleminator::tty;

#[derive(Debug, Parser)]
#[command(
    name = "teleminator",
    about = "Teleminator — a modern log and trace navigator",
    version
)]
struct Cli {
    /// Log file to open, or `-` for stdin (also: `prog | teleminator`)
    file: Option<PathBuf>,

    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Write a default config file and exit
    #[arg(long)]
    init_config: bool,
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

    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(Config::default_path);
    let (config, loaded_from) = Config::load_from(&config_path)?;

    let source = open_source(cli.file.as_ref())?;

    let mut terminal = ratatui::init();
    let result = (|| {
        let mut app = App::new(source, config, config_path)?;
        if let Some(path) = loaded_from {
            app.status_message = Some(format!("config: {}", path.display()));
        }
        app.run(&mut terminal)
    })();
    ratatui::restore();
    // Theme picker enables mouse capture; ensure it is always cleared on exit.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
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
                     teleminator app.jsonl\n  \
                     myapp | teleminator"
                );
            }
            let pipe = tty::require_piped_stdin()?;
            LogSource::open_stdin(pipe)
        }
    }
}
