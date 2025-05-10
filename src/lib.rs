use data_loading::fetch_wikidata_item;
use grouping::{GroupConfig, group_properties, load_grouping_config};
use registry::RendererRegistry;
use serde::Serialize;

use crate::grouping::GroupingConfig;

pub mod config;
mod custom;
pub mod data_loading;
pub mod grouping;
mod model;
mod registry;
mod rendering;
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderFormatOptions {
    #[default]
    Text,
    HTML,
    Markdown,
    Wikitext,
}

pub struct RenderConfig {
    pub format: RenderFormatOptions,
    pub ignore_ids: bool,
    pub grouping_config: GroupingConfig,
    pub language: String,
}

impl RenderConfig {
    pub fn new(format: RenderFormatOptions, ignore_ids: bool, language: &str) -> Self {
        let grouping_config = load_grouping_config().unwrap();
        RenderConfig {
            format,
            ignore_ids,
            grouping_config,
            language: language.to_owned(),
        }
    }

    pub fn new_text_renderer(language: &str) -> Self {
        let grouping_config = load_grouping_config().unwrap();
        RenderConfig {
            format: RenderFormatOptions::Text,
            ignore_ids: true,
            grouping_config,
            language: language.to_string(),
        }
    }

    pub fn new_markdow_renderer(language: &str) -> Self {
        let grouping_config = load_grouping_config().unwrap();
        RenderConfig {
            format: RenderFormatOptions::Markdown,
            ignore_ids: true,
            grouping_config,
            language: language.to_string(),
        }
    }
}

pub async fn render(
    qid: &str,
    render_config: &RenderConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();
    let wikidata_item = fetch_wikidata_item(qid, render_config.language.as_str()).await?;
    let grouped_properties = group_properties(&wikidata_item, &render_config.grouping_config);

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

        let renderer = RendererRegistry::get_renderer(renderer_name);

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
        }
    }

    Ok(output)
}
