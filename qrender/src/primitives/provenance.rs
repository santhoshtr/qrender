//! Provenance: how many of the item's statements are sourced, imported
//! from a Wikipedia, or without a reference. Identifiers are not counted.

use content::Datatype;
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Provenance {
    pub sourced: usize,
    pub imported: usize,
    pub none: usize,
}

/// HTML: the template binding. The markup is templates/primitives/provenance.html.
#[derive(askama::Template)]
#[template(path = "primitives/provenance.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Provenance,
}

/// The CSS of this primitive (design/primitives/provenance.css).
pub const CSS: &str = include_str!("../../design/primitives/provenance.css");

pub fn build(c: &Composer) -> Provenance {
    let mut counts = Provenance {
        sourced: 0,
        imported: 0,
        none: 0,
    };
    for s in &c.content.statements {
        if s.value.datatype() == Datatype::ExternalId {
            continue;
        }
        match c.provenance(s) {
            schema::Provenance::Sourced => counts.sourced += 1,
            schema::Provenance::Imported => counts.imported += 1,
            schema::Provenance::None => counts.none += 1,
        }
    }
    counts
}

/// Markdown: the provenance counts and the label coverage.
pub fn markdown(b: &Provenance, md: &mut crate::render::markdown::Markdown) {
    let coverage = &md.view.coverage;
    md.paragraph(&format!(
        "Statements: {} sourced, {} imported from Wikipedia, {} without a reference. Labels in {}: {} of {}.",
        b.sourced, b.imported, b.none, md.view.lang, coverage.have, coverage.total
    ));
}
