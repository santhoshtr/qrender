//! The qrender command line. Each command reads and writes the files of
//! one contract, so each layer can be run and checked on its own.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use content::Content;
use qrender::config::Config;

#[derive(Parser)]
#[command(about = "Compose Wikidata items into pages")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Content JSON -> composition JSON, on standard output.
    Compose {
        /// A content file (content/0.1).
        #[arg(long)]
        content: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qrender: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Compose { content } => {
            let text = std::fs::read_to_string(&content)
                .map_err(|e| format!("{}: {e}", content.display()))?;
            let content: Content =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", content.display()))?;
            let composition = qrender::compose(&content, Config::embedded());
            println!("{}", serde_json::to_string_pretty(&composition)?);
        }
    }
    Ok(())
}
