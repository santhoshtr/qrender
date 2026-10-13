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
