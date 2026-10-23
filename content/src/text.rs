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
    /// The Wikipedia article of each topic item (the "... of topic"
    /// values): its title, in the page language or else in English.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub articles: IndexMap<Qid, LangText>,
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
    /// The lead: the blocks of section 0, as the blocks of a section.
    pub lead: Vec<OutlineBlock>,
    /// The item of each article that a text links to, by page title.
    pub links: IndexMap<String, Qid>,
    pub sections: Vec<Section>,
    /// The anchors of the sections that each concept matches.
    #[serde(default)]
    pub concept_sections: IndexMap<Qid, Vec<String>>,
    /// The first paragraph of each matched section, by anchor: its
    /// content, as the lead.
    #[serde(default)]
    pub section_text: IndexMap<String, String>,
    /// The sections with their content: for the article that the page
    /// shows (the article in the page language, or the fallback article
    /// when the page language has none).
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
    /// The blocks before the first subsection, in order.
    pub blocks: Vec<OutlineBlock>,
    pub sections: Vec<OutlineSection>,
}

/// A block of a section: a text, or a figure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutlineBlock {
    /// `p`, `ul`, `ol`, `dl` or `blockquote`, as HTML: the marks of
    /// notes, and no attributes but `href` and `alt`.
    Text(String),
    Figure(ArticleFigure),
}

/// The images of a thumbnail, a multiple-image template or a gallery:
/// the files on Commons only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleFigure {
    pub images: Vec<ArticleImage>,
    /// The caption of the whole figure, as HTML (as a text block).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub caption: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleImage {
    /// The name of the file on Commons, without "File:".
    pub file: String,
    pub width: u32,
    pub height: u32,
    /// The caption of this image in a group, as HTML.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub caption: String,
}

impl OutlineBlock {
    /// The HTML texts of the block: the text, or the captions.
    pub fn texts(&self) -> Vec<&str> {
        match self {
            Self::Text(html) => vec![html],
            Self::Figure(f) => f
                .images
                .iter()
                .map(|i| i.caption.as_str())
                .chain([f.caption.as_str()])
                .filter(|c| !c.is_empty())
                .collect(),
        }
    }
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
