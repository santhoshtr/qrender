//! Title: the label and the description of the item, at the head of the
//! page. The composition holds only the item; localize gives the text.

use content::Qid;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Title {
    pub item: Qid,
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

/// Markdown: the title and the description.
pub fn markdown(b: &Title, md: &mut crate::render::markdown::Markdown) {
    let label = md.view.labels.get(&content::EntityId::Item(b.item));
    md.heading(1, &md.label(content::EntityId::Item(b.item)));
    if let Some(d) = label.and_then(|l| l.description.as_ref()) {
        md.paragraph(&d.text);
    }
}
