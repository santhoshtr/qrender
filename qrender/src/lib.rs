use data_loading::fetch_wikidata_item;
use grouping::{GroupConfig, group_properties, load_grouping_config};
use registry::RendererRegistry;
use serde::Serialize;

use crate::grouping::GroupingConfig;

pub mod cards;
mod custom;
pub mod data_loading;
pub mod error;
pub mod factoid;
pub mod textual;
pub mod grouping;
pub mod model;
mod registry;
mod rendering;

pub use error::QRenderError;
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
    if let RenderFormatOptions::Factoid = render_config.format {
        let item = data_loading::fetch_typed(qid, render_config.language.as_str()).await?;
        let page = cards::synthesize(
            &item,
            &render_config.language,
            &render_config.grouping_config,
            render_config.ignore_ids,
        );
        return factoid::render_page(&page);
    }
    let wikidata_item = fetch_wikidata_item(qid, render_config.language.as_str()).await?;
    render_item(&wikidata_item, render_config)
}

/// Render an already-fetched item. Pure: no network, deterministic output.
pub fn render_item(
    wikidata_item: &model::WikidataItem,
    render_config: &RenderConfig,
) -> Result<String, QRenderError> {
    let mut output = String::new();
    let grouped_properties = group_properties(wikidata_item, &render_config.grouping_config);

    for (group_name, properties) in grouped_properties {
        if properties.is_empty() {
            continue; // Skip empty groups
        }
        if render_config.ignore_ids && group_name == "identifiers" {
            continue; // Skip identifiers group if ignore_ids is true
        }
        let group_config = {
            let this = render_config.grouping_config.groups.get(&group_name);
            match this {
                Some(x) => x,
                None => &GroupConfig {
                    pids: vec![],
                    renderer: None,
                    order: None,
                },
            }
        };
        let renderer_name = group_config.renderer.as_deref().unwrap_or("default"); // Use "default" if None

        let renderer = RendererRegistry::get_renderer(renderer_name)?;

        match render_config.format {
            RenderFormatOptions::HTML => {
                output.push_str(&renderer.render_html(&group_name, &properties)?);
            }
            RenderFormatOptions::Text => {
                output.push_str(&renderer.render_text(&group_name, &properties)?);
                output.push('\n'); // Add a separator between groups
            }
            RenderFormatOptions::Markdown => {
                output.push_str(&renderer.render_markdown(&group_name, &properties)?);
            }
            RenderFormatOptions::Wikitext => {
                output.push_str(&renderer.render_wikitext(&group_name, &properties)?);
            }
            RenderFormatOptions::Factoid => {
                // Factoid needs the typed item; render() routes it before here
                return Err(QRenderError::UnsupportedFormat("factoid".to_string()));
            }
        }
    }

    Ok(output)
}
