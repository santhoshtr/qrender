use serde::Serialize;

pub mod cards;
pub mod data_loading;
pub mod error;
pub mod factoid;
pub mod grouping;
pub mod icons;
pub mod textual;

pub use error::QRenderError;

use crate::grouping::{GroupingConfig, load_grouping_config};

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderFormatOptions {
    #[default]
    Text,
    HTML,
    Markdown,
    Wikitext,
    Factoid,
}

pub struct RenderConfig {
    pub format: RenderFormatOptions,
    pub ignore_ids: bool,
    pub grouping_config: GroupingConfig,
    pub language: String,
}

impl RenderConfig {
    pub fn new(
        format: RenderFormatOptions,
        ignore_ids: bool,
        language: &str,
    ) -> Result<Self, QRenderError> {
        let grouping_config = load_grouping_config()?;
        Ok(RenderConfig {
            format,
            ignore_ids,
            grouping_config,
            language: language.to_owned(),
        })
    }

    pub fn new_text_renderer(language: &str) -> Result<Self, QRenderError> {
        Self::new(RenderFormatOptions::Text, true, language)
    }

    pub fn new_markdow_renderer(language: &str) -> Result<Self, QRenderError> {
        Self::new(RenderFormatOptions::Markdown, true, language)
    }
}

pub async fn render(qid: &str, render_config: &RenderConfig) -> Result<String, QRenderError> {
    let item = data_loading::fetch_typed(qid, render_config.language.as_str()).await?;
    render_typed_item(&item, render_config)
}

/// Render an already-fetched item. Pure: no network, deterministic output.
pub fn render_typed_item(
    item: &qjson::WikidataItem,
    render_config: &RenderConfig,
) -> Result<String, QRenderError> {
    let page = cards::synthesize(
        item,
        &render_config.language,
        &render_config.grouping_config,
        render_config.ignore_ids,
    );
    Ok(match render_config.format {
        RenderFormatOptions::Text => textual::render_text(&page),
        RenderFormatOptions::Markdown => textual::render_markdown(&page),
        RenderFormatOptions::Wikitext => textual::render_wikitext(&page),
        RenderFormatOptions::HTML => textual::render_html(&page),
        RenderFormatOptions::Factoid => factoid::render_page(&page)?,
    })
}
