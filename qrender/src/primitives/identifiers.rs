//! Identifiers: the item's identifiers in other databases, in a dialog.

use content::{Datatype, Pid, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Identifiers {
    pub values: Vec<Identifier>,
}

/// HTML: the template binding. The markup is templates/primitives/identifiers.html.
#[derive(askama::Template)]
#[template(path = "primitives/identifiers.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Identifiers,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Identifier {
    pub pid: Pid,
    pub string: String,
    pub statement: StatementId,
}

/// Claims every identifier statement that no spread claimed.
pub fn build(c: &mut Composer) -> Identifiers {
    let ids: Vec<&Statement> = c
        .content
        .statements
        .iter()
        .filter(|s| s.value.datatype() == Datatype::ExternalId)
        .collect();
    Identifiers {
        values: c
            .take(ids)
            .iter()
            .map(|s| Identifier {
                pid: s.pid,
                string: match &s.value {
                    Value::ExternalId { id } => id.clone(),
                    _ => String::new(),
                },
                statement: s.id.clone(),
            })
            .collect(),
    }
}

/// Markdown: every identifier.
pub fn markdown(b: &Identifiers, md: &mut crate::render::markdown::Markdown) {
    for v in &b.values {
        md.item(&md.property(v.pid), &v.string);
    }
    md.end();
}
