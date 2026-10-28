//! The localized view (`view/0.2`): the composition, unchanged, and what
//! depends on the language, in tables. A renderer reads only this.

use std::collections::BTreeMap;

use content::{Cited, EntityId, Lang, Qid};
use schemars::JsonSchema;
use serde::Serialize;

use super::locale::Dir;
use crate::schema::{Picture, Spread, StatementRef, Wanted};

pub const VIEW_SCHEMA: &str = "view/0.2";

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct View {
    pub schema: &'static str,
    pub composition: &'static str,
    pub design: &'static str,
    pub qid: Qid,
    /// The language of the page (a Wikimedia code).
    pub lang: Lang,
    /// The BCP 47 locale for `Intl`, from `lang`.
    pub locale: String,
    pub dir: Dir,
    pub page_title: String,
    /// The label of every item and property that the page shows.
    pub labels: BTreeMap<EntityId, Label>,
    /// Pictures from articles, for item values that have no picture.
    pub page_images: BTreeMap<Qid, PageImage>,
    /// The Wikipedia article of each topic item: in the page language, or
    /// else in English.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub articles: BTreeMap<Qid, Link>,
    /// The interface messages of the page language.
    pub messages: BTreeMap<String, String>,
    /// The lead of the article, in the page language or the fallback.
    pub lead: Option<Article>,
    /// Labels in the page language (or `mul`), over what the reader sees.
    pub coverage: Coverage,
    /// The projects with a page about the item, the most first.
    pub projects: Vec<Project>,
    /// The spreads that have something to show.
    pub spreads: Vec<SpreadView>,
    /// The values that the spreads not on the page ask for: one group of
    /// the page.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<Ask>,
    /// The article sections that no spread matched (their titles), when
    /// the page does not show their content.
    pub unmatched_sections: Vec<Link>,
    /// The same sections with their content, when the page shows it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub more_sections: Vec<ProseSection>,
    /// The values that the page does not show: not of the best rank.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub set_aside: Vec<StatementRef>,
    /// The notes of the prose that the page shows, by number.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
    /// The references of the statements that cite a source, by hash.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub citations: BTreeMap<String, Cited>,
    /// The works that the short footnotes of the notes cite, by anchor.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub works: BTreeMap<String, Cited>,
}

impl View {
    /// The spreads in page order: before the sections of the article, or
    /// after them (`end`: the apparatus).
    pub fn spreads_at(&self, end: bool) -> impl Iterator<Item = &SpreadView> {
        self.spreads
            .iter()
            .filter(move |s| s.spread.kind.is_apparatus() == end)
    }
}

/// The value that a spread asks for when the page does not show the
/// spread: it has no data.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Ask {
    pub spread: String,
    /// The label of the spread's concept.
    pub title: Option<Label>,
    pub wanted: Wanted,
}

/// A note of the article: a reference, or an explanatory note.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Note {
    /// The number of the note on this page: its marks in the prose show
    /// it (`<sup data-note="3"></sup>`).
    pub number: usize,
    /// None for a reference; "lower-alpha" for an explanatory note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The language of the article.
    pub lang: Lang,
    /// The citations; the HTML of free text is cleaned as prose is.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Cited>,
    /// An explanatory note: its text, cleaned, with its own marks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Label {
    pub text: String,
    /// The language of the text; None when there is no label at all.
    pub lang: Option<Lang>,
    /// The label is not in the page language (a `mul` label is not a
    /// fallback).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub fallback: bool,
    /// Where a reader adds the label in the page language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<String>,
    /// The page of the entity on Wikidata.
    pub href: String,
    /// For the item itself and for the values whose description is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Description {
    pub text: String,
    pub lang: Lang,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub fallback: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PageImage {
    #[serde(flatten)]
    pub picture: Picture,
    /// The wiki of the article the picture comes from.
    pub from: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Article {
    pub lang: Lang,
    /// The article is in the page language.
    pub local: bool,
    pub host: String,
    pub url: String,
    /// The blocks of the lead, as the blocks of a section.
    pub blocks: Vec<ProseBlock>,
    /// The first figure of the lead, when it has one image and the page
    /// has a head picture: it takes the place of that picture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<ProseFigure>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Coverage {
    pub have: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Project {
    pub name: String,
    pub count: usize,
    /// The project has a page in the page language.
    pub local: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SpreadView {
    #[serde(flatten)]
    pub spread: Spread,
    /// The label of the spread's concept.
    pub title: Option<Label>,
    /// The first paragraph of the article section of the spread, when the
    /// page does not show its content.
    pub prose_text: Option<SectionProse>,
    /// The article sections of the spread.
    pub prose_links: Vec<Link>,
    /// The article sections of the spread with their content.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<ProseSection>,
}

/// A section of the article, with its content and its subsections.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProseSection {
    pub title: String,
    pub anchor: String,
    /// The section in the article.
    pub href: String,
    pub lang: Lang,
    /// The article is in the page language: nothing to translate.
    pub local: bool,
    /// 2 for a section, 3 for its subsection, and so on.
    pub level: u8,
    pub blocks: Vec<ProseBlock>,
    pub sections: Vec<ProseSection>,
}

/// A block of an article section: a text, or a figure.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ProseBlock {
    /// Cleaned HTML, as the lead: `p`, `ul`, `ol`, `dl`, `blockquote`.
    Text(String),
    Figure(ProseFigure),
}

/// The images of a thumbnail, a multiple-image template or a gallery of
/// the article: files on Commons.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProseFigure {
    pub images: Vec<ProseImage>,
    /// The number of images in each row, in order: the rows of a
    /// multiple-image template. Empty: a row for each image.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<usize>,
    /// The caption of the whole figure: cleaned HTML, as the lead.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub caption: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProseImage {
    /// The name of the file on Commons, without "File:".
    pub file: String,
    /// Width / height.
    pub aspect: f64,
    /// The caption of this image in a group: cleaned HTML.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub caption: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SectionProse {
    /// Cleaned HTML, as the lead.
    pub html: String,
    pub lang: Lang,
    pub local: bool,
    pub title: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Link {
    pub text: String,
    pub lang: Lang,
    pub href: String,
}
