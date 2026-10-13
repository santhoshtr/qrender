//! Prose: the lead of the Wikipedia article, beside the head picture. The
//! composition holds only how many languages have an article; localize
//! chooses and cleans the article.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Prose {
    pub source: ProseSource,
    /// The number of Wikipedias with an article about the item.
    pub languages: usize,
}

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
