use clap::Parser;
use qrender::{RenderConfig, RenderFormatOptions, render};

use std::error::Error;

#[derive(Parser, Debug)]
#[command(author = "Santhosh Thottingal", version, about = "Wikidata Renderer")]
struct Args {
    /// The QID of the Wikidata item to render
    #[arg(short, long, default_value = "Q405")]
    qid: String,
    /// The language to use
    #[arg(short, long, default_value = "en")]
    language: String,

    /// Render format
    #[arg(short, long, default_value_t = RenderFormatOptions::Text, value_enum)]
    format: RenderFormatOptions,

    /// Ignore IDs in the output
    #[arg(short, long, default_value_t = true, default_value = "true")]
    ignore_ids: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let render_config = RenderConfig::new(args.format, args.ignore_ids, args.language.as_str());

    let rendered_text = render(args.qid.as_str(), &render_config).await?;

    println!("{}", rendered_text);
    Ok(())
}
