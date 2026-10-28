//! The qrender command line. Each command reads and writes the files of
//! one contract, so each layer can be run and checked on its own:
//!
//!   qrender fetch --qid Q3870 --lang en --out data     content and text files
//!   qrender compose --content data/content/q3870.json  composition JSON
//!   qrender view --content … --text …                  view JSON
//!   qrender page --content … --text … [--format markdown]
//!   qrender page --qid Q3870 --lang en                 all steps, live

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use app::Service;
use clap::{Args, Parser, Subcommand};
use content::fetch::Mode;
use content::{Content, Lang, Qid, Text};
use qrender::config::Config;
use qrender::localize::View;
use qrender::render::html::{self, Styles};
use qrender::render::markdown;

#[derive(Parser)]
#[command(about = "Compose Wikidata items into pages")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fetch an item: write content/<qid>.json and text/<qid>.<lang>.json.
    Fetch {
        #[command(flatten)]
        item: Live,
        /// The directory for the two files.
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
    /// Content JSON -> composition JSON, on standard output.
    Compose {
        #[arg(long)]
        content: PathBuf,
    },
    /// Content and text JSON -> the view JSON, on standard output.
    View {
        #[arg(long)]
        content: PathBuf,
        #[arg(long)]
        text: PathBuf,
    },
    /// A page, on standard output: from content and text files, or live.
    Page {
        #[arg(long, requires = "text", conflicts_with = "qid")]
        content: Option<PathBuf>,
        #[arg(long, requires = "content")]
        text: Option<PathBuf>,
        /// Fetch this item (with --lang), in place of the files.
        #[arg(long, requires = "lang")]
        qid: Option<Qid>,
        #[arg(long, requires = "qid")]
        lang: Option<Lang>,
        #[arg(long, value_enum, default_value = "html")]
        format: Format,
        /// Link the stylesheets from this base URL; without it they are in
        /// the page (HTML only).
        #[arg(long)]
        design: Option<String>,
    },
}

/// An item to fetch.
#[derive(Args)]
struct Live {
    #[arg(long)]
    qid: Qid,
    #[arg(long)]
    lang: Lang,
    /// Save every HTTP response in this directory.
    #[arg(long, conflicts_with = "replay")]
    record: Option<PathBuf>,
    /// Take every HTTP response from this directory: no network.
    #[arg(long)]
    replay: Option<PathBuf>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format {
    Html,
    Markdown,
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qrender: {e}");
            ExitCode::FAILURE
        }
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Fetch { item, out } => {
            let (content, text) = fetch(&item).await?;
            let q = item.qid.to_string().to_lowercase();
            write(&out.join(format!("content/{q}.json")), &content)?;
            write(&out.join(format!("text/{q}.{}.json", item.lang)), &text)?;
        }
        Command::Compose { content } => {
            let content: Content = read(&content)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&qrender::compose(&content, Config::embedded()))?
            );
        }
        Command::View { content, text } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&view(&read(&content)?, &read(&text)?))?
            );
        }
        Command::Page {
            content,
            text,
            qid,
            lang,
            format,
            design,
        } => {
            let view = match (content, text, qid, lang) {
                (Some(content), Some(text), _, _) => view(&read(&content)?, &read(&text)?),
                (_, _, Some(qid), Some(lang)) => {
                    let item = Live {
                        qid,
                        lang,
                        record: None,
                        replay: None,
                    };
                    let (content, text) = fetch(&item).await?;
                    view(&content, &text)
                }
                _ => return Err("give --content and --text, or --qid and --lang".into()),
            };
            match format {
                Format::Markdown => print!("{}", markdown::page(&view)),
                Format::Html => {
                    let styles = design.as_deref().map_or(Styles::Inline, Styles::Linked);
                    print!("{}", html::page(&view, styles, None));
                }
            }
        }
    }
    Ok(())
}

async fn fetch(item: &Live) -> Result<(Content, Text)> {
    let mode = match (&item.record, &item.replay) {
        (Some(dir), _) => Mode::Record(dir.clone()),
        (_, Some(dir)) => Mode::Replay(dir.clone()),
        _ => Mode::Live,
    };
    let service = Service::from_env(mode)?;
    let content = service.content(item.qid).await?;
    let text = service.text(&content, &item.lang).await?;
    Ok((content, text))
}

fn view(content: &Content, text: &Text) -> View {
    qrender::localize(
        &qrender::compose(content, Config::embedded()),
        text,
        Config::embedded(),
    )
}

/// A JSON file of one contract.
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(serde_json::from_str(&json).map_err(|e| format!("{}: {e}", path.display()))?)
}

fn write(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
    std::fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    eprintln!("wrote {}", path.display());
    Ok(())
}
