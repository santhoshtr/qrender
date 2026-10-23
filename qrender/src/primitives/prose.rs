//! Prose: the lead of the Wikipedia article, beside the head picture. The
//! composition holds only how many languages have an article; localize
//! chooses and cleans the article.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

use crate::localize::ProseBlock;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Prose {
    pub source: ProseSource,
    /// The number of Wikipedias with an article about the item.
    pub languages: usize,
}

/// HTML: the template binding. The markup is templates/primitives/prose.html.
#[derive(askama::Template)]
#[template(path = "primitives/prose.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Prose,
}

/// The CSS of this primitive (design/primitives/prose.css).
pub const CSS: &str = include_str!("../../design/primitives/prose.css");

#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProseSource {
    Lead,
}

/// Sites that end in "wiki" but are not a Wikipedia.
const NOT_WIKIPEDIA: [&str; 4] = ["commonswiki", "specieswiki", "wikidatawiki", "metawiki"];

pub fn lead(sitelinks: &IndexMap<String, String>) -> Prose {
    let wikipedia = |site: &str| {
        site.strip_suffix("wiki").is_some_and(|code| {
            !code.is_empty()
                && code
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_' || b == b'-')
        }) && !NOT_WIKIPEDIA.contains(&site)
    };
    Prose {
        source: ProseSource::Lead,
        languages: sitelinks.keys().filter(|s| wikipedia(s)).count(),
    }
}

/// Markdown: the texts of the lead of the article, with its source.
pub fn markdown(b: &Prose, md: &mut crate::render::markdown::Markdown) {
    let Some(article) = &md.view.lead else {
        return;
    };
    let text: Vec<String> = article
        .blocks
        .iter()
        .filter_map(|b| match b {
            ProseBlock::Text(html) => Some(crate::render::markdown::plain(html)),
            ProseBlock::Figure(_) => None,
        })
        .collect();
    let source = format!(
        "Source: [{}]({}); {} Wikipedias have an article.",
        article.host, article.url, b.languages
    );
    md.paragraph(&text.join("\n\n"));
    md.paragraph(&source);
}
