//! The text of one item in one language (`text/0.1`): labels and
//! descriptions with the language each came from, the concepts that title
//! the spreads, the Wikipedia article, and page images. Everything here
//! depends on the language; nothing here is an editorial decision.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::citation::{Citation, Cited};
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
    /// The lead: the content of each paragraph, as HTML with the marks of
    /// notes, and no attribute but `href` and `alt`.
    pub paragraphs: Vec<String>,
    /// The item of each article that a paragraph links to, by page title.
    pub links: IndexMap<String, Qid>,
    pub sections: Vec<Section>,
    /// The anchors of the sections that each concept matches.
    #[serde(default)]
    pub concept_sections: IndexMap<Qid, Vec<String>>,
    /// The first paragraph of each matched section, by anchor: its
    /// content, as the lead.
    #[serde(default)]
    pub section_text: IndexMap<String, String>,
    /// The sections with their content: only for the fallback article of
    /// a page whose language has no article.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outline: Vec<OutlineSection>,
    /// The notes that the text above marks, by the id of their item in the
    /// list of notes ("cite_note-12"). A mark in the HTML is
    /// `<sup data-note="cite_note-12"></sup>`.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub notes: IndexMap<String, Note>,
    /// The works that the short footnotes of the notes cite, by their
    /// anchor ("CITEREFFaingold2018"): entries of the bibliography.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub works: IndexMap<String, Cited>,
}

/// A note of an article: a reference (the default group) or an
/// explanatory note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// None for the default group (numbers); "lower-alpha" for notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The citations in the note, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Cited>,
    /// An explanatory note: its text as HTML, with its own marks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Note {
    /// The anchors of the works that the short footnotes of this note
    /// cite.
    pub fn works(&self) -> impl Iterator<Item = &str> {
        self.citations.iter().filter_map(|c| match &c.citation {
            Citation::Short(s) => Some(s.work.as_str()),
            _ => None,
        })
    }
}

/// A section of an article with its content, and its subsections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutlineSection {
    pub title: String,
    pub anchor: String,
    /// 2 for a section, 3 for its subsection, and so on.
    pub level: u8,
    /// The blocks before the first subsection (`p`, `ul`, `ol`, `dl`,
    /// `blockquote`), as HTML: the marks of notes, and no attributes but
    /// `href` and `alt`.
    pub blocks: Vec<String>,
    pub sections: Vec<OutlineSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub title: String,
    pub anchor: String,
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
