//! The localized view (`view/0.2`): the composition, unchanged, and what
//! depends on the language, in tables. A renderer reads only this.

use std::collections::BTreeMap;

use content::{EntityId, Lang, Qid};
use schemars::JsonSchema;
use serde::Serialize;

use super::locale::Dir;
use crate::schema::{Picture, Spread, StatementRef};

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
    /// The article sections that no spread matched (their titles), when
    /// the page does not show their content.
    pub unmatched_sections: Vec<Link>,
    /// The same sections with their content: for a fallback article.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub more_sections: Vec<ProseSection>,
    /// The values that the page does not show: not of the best rank.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub set_aside: Vec<StatementRef>,
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
    /// The first paragraph, cleaned (HTML: `a`, `b`, `i`, ruby glosses).
    pub paragraphs: Vec<String>,
    /// The other paragraphs of the lead.
    pub more: Vec<String>,
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
    /// The article sections of the spread with their content: for a
    /// fallback article.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<ProseSection>,
}

/// A section of the fallback article, with its content and its
/// subsections.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProseSection {
    pub title: String,
    pub anchor: String,
    /// The section in the article.
    pub href: String,
    pub lang: Lang,
    /// 2 for a section, 3 for its subsection, and so on.
    pub level: u8,
    /// Cleaned HTML, as the lead: `p`, `ul`, `ol`, `dl`, `blockquote`.
    pub blocks: Vec<String>,
    pub sections: Vec<ProseSection>,
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
