//! The qrender command line. Each command reads and writes the files of
//! one contract, so each layer can be run and checked on its own.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use content::{Content, Text};
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
    /// Content and text JSON -> the localized view JSON, on standard output.
    View {
        /// A content file (content/0.1).
        #[arg(long)]
        content: PathBuf,
        /// A text file of one language (text/0.1).
        #[arg(long)]
        text: PathBuf,
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
            let content: Content = read(&content)?;
            let composition = qrender::compose(&content, Config::embedded());
            println!("{}", serde_json::to_string_pretty(&composition)?);
        }
        Command::View { content, text } => {
            let content: Content = read(&content)?;
            let text: Text = read(&text)?;
            let composition = qrender::compose(&content, Config::embedded());
            let view = qrender::localize(&composition, &text);
            println!("{}", serde_json::to_string_pretty(&view)?);
        }
    }
    Ok(())
}

/// A JSON file of one contract.
fn read<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&json).map_err(|e| format!("{}: {e}", path.display()))
}
