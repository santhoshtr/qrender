//! Sitelinks: the Wikimedia projects that have a page about the item, by
//! project family, with the language codes of each.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Sitelinks {
    /// Family ("wikipedia", "wikiquote", ...) -> language codes, sorted.
    pub families: IndexMap<String, Vec<String>>,
}

/// HTML: the template binding. The markup is templates/primitives/sitelinks.html.
#[derive(askama::Template)]
#[template(path = "primitives/sitelinks.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Sitelinks,
}

/// The CSS of this primitive (design/primitives/sitelinks.css).
pub const CSS: &str = include_str!("../../design/primitives/sitelinks.css");

/// Site id suffixes of the language projects. "wiki" is Wikipedia.
const FAMILIES: [&str; 8] = [
    "wiki",
    "wikiquote",
    "wikivoyage",
    "wikinews",
    "wikisource",
    "wikibooks",
    "wikiversity",
    "wiktionary",
];

/// Projects that are not in a language.
const NOT_LANGUAGE: [&str; 5] = [
    "commonswiki",
    "specieswiki",
    "wikidatawiki",
    "metawiki",
    "mediawikiwiki",
];

pub fn build(sitelinks: &IndexMap<String, String>) -> Sitelinks {
    let mut families: IndexMap<String, Vec<String>> = IndexMap::new();
    for site in sitelinks.keys() {
        if NOT_LANGUAGE.contains(&site.as_str()) {
            continue;
        }
        // The shortest language code that leaves a family suffix.
        let split = (1..site.len())
            .filter(|&i| site.is_char_boundary(i))
            .find_map(|i| FAMILIES.contains(&&site[i..]).then(|| site.split_at(i)));
        if let Some((code, family)) = split {
            let family = if family == "wiki" {
                "wikipedia"
            } else {
                family
            };
            families
                .entry(family.to_string())
                .or_default()
                .push(code.to_string());
        }
    }
    for codes in families.values_mut() {
        codes.sort();
    }
    Sitelinks { families }
}

/// Markdown: the projects with a page about the item.
pub fn markdown(_: &Sitelinks, md: &mut crate::render::markdown::Markdown) {
    let projects: Vec<String> = md
        .view
        .projects
        .iter()
        .map(|p| {
            format!(
                "{} {}{}",
                p.name,
                p.count,
                if p.local { " (this language)" } else { "" }
            )
        })
        .collect();
    if !projects.is_empty() {
        md.paragraph(&format!("Projects: {}", projects.join(", ")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families() {
        let sites: IndexMap<String, String> = [
            "enwiki",
            "mlwikiquote",
            "enwiktionary",
            "commonswiki",
            "be_x_oldwiki",
            "afwiki",
        ]
        .iter()
        .map(|s| (s.to_string(), String::new()))
        .collect();
        let f = build(&sites).families;
        assert_eq!(f["wikipedia"], ["af", "be_x_old", "en"]);
        assert_eq!(f["wikiquote"], ["ml"]);
        assert_eq!(f["wiktionary"], ["en"]);
        assert_eq!(f.len(), 3);
    }
}
