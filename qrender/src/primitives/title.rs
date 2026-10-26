//! Title: the label and the description of the item, at the head of the
//! page. The composition holds only the item; localize gives the text.

use content::Qid;
use schemars::JsonSchema;
use serde::Serialize;

use crate::schema::{StatementRef, ValueShown};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Title {
    pub item: Qid,
    /// A character that stands for the item (the glyph role), shown after
    /// the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glyph: Option<StatementRef>,
}

impl Title {
    /// The character of the glyph.
    pub fn glyph_text(&self) -> Option<&str> {
        match &self.glyph.as_ref()?.value {
            ValueShown::String { string } => Some(string),
            _ => None,
        }
    }
}

/// HTML: the template binding. The markup is templates/primitives/title.html.
#[derive(askama::Template)]
#[template(path = "primitives/title.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Title,
}

/// The CSS of this primitive (design/primitives/title.css).
pub const CSS: &str = include_str!("../../design/primitives/title.css");

/// Markdown: the title (with its glyph) and the description.
pub fn markdown(b: &Title, md: &mut crate::render::markdown::Markdown) {
    let label = md.view.labels.get(&content::EntityId::Item(b.item));
    let mut heading = md.label(content::EntityId::Item(b.item));
    if let Some(glyph) = b.glyph_text() {
        heading = format!("{heading} {glyph}");
    }
    md.heading(1, &heading);
    if let Some(d) = label.and_then(|l| l.description.as_ref()) {
        md.paragraph(&d.text);
    }
}
