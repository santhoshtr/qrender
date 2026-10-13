//! The text of one item in one language (`text/0.1`): labels and
//! descriptions with the language each came from, the concepts that title
//! the spreads, the Wikipedia article, and page images. Everything here
//! depends on the language; nothing here is an editorial decision.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::id::{EntityId, Lang, Qid};

pub const TEXT_SCHEMA: &str = "text/0.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub schema: String,
    pub lang: Lang,
    /// Every identifier that the content uses.
    pub labels: IndexMap<EntityId, Term>,
    /// The items that title the spreads and name their prose sections.
    pub concepts: IndexMap<Qid, Concept>,
    /// The article in the page language.
    pub article: Option<Article>,
    /// The English article, when the page language is not English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_article: Option<Article>,
    /// Pictures from the articles of items that have no P18.
    pub page_images: IndexMap<Qid, PageImage>,
}

/// A text and the language it is in. By the chain `{lang}, mul, en`, the
/// language can differ from the page language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LangText {
    pub text: String,
    pub lang: Lang,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Term {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<LangText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<LangText>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub label: Option<LangText>,
    /// The label and the aliases in the page language: the section titles
    /// that this concept matches.
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub title: String,
    /// The lead, as HTML paragraphs.
    pub paragraphs: Vec<String>,
    /// The item of each article that a paragraph links to, by page title.
    pub links: IndexMap<String, Qid>,
    pub sections: Vec<Section>,
    /// The anchors of the sections that each concept matches.
    #[serde(default)]
    pub concept_sections: IndexMap<Qid, Vec<String>>,
    /// The first paragraph of each matched section, by anchor.
    #[serde(default)]
    pub section_text: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub title: String,
    pub anchor: String,
    /// The section index of the parse API (a string: "T-1" is possible).
    pub index: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageImage {
    pub file: String,
    pub width: u32,
    pub height: u32,
    pub artist: String,
    pub license: String,
    /// The wiki the picture came from: "en.wikipedia.org".
    pub source: String,
}
