//! Kind: what the item is (its classes) and where it is (its current
//! container), under the head picture.

use content::Pid;
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::primitives::mark_shown;
use crate::schema::StatementRef;
use crate::wikidata::INSTANCE_OF;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Kind {
    pub chain: Vec<ChainLink>,
}

/// HTML: the template binding. The markup is templates/primitives/kind.html.
#[derive(askama::Template)]
#[template(path = "primitives/kind.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Kind,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ChainLink {
    pub pid: Pid,
    pub values: Vec<StatementRef>,
}

/// The classes (current first, the first `chain_max` shown) and the
/// current value of the container property. Claims both.
pub fn build(c: &mut Composer) -> Kind {
    let mut chain = Vec::new();
    let classes = c.current(c.free(*INSTANCE_OF));
    let classes = c.take(classes);
    if !classes.is_empty() {
        let values = classes.iter().map(|s| c.statement(s));
        chain.push(ChainLink {
            pid: *INSTANCE_OF,
            values: mark_shown(values, c.config.layouts.thresholds.chain_max),
        });
    }
    let container = &c.content.container;
    if let (Some(pid), Some(qid)) = (container.pid, container.qid) {
        let now: Vec<_> = c
            .free(pid)
            .into_iter()
            .filter(|s| s.value.item() == Some(qid))
            .collect();
        if !now.is_empty() {
            let first = c.current(now).into_iter().take(1);
            let values = c.take(first).iter().map(|s| c.statement(s)).collect();
            chain.push(ChainLink { pid, values });
        }
    }
    Kind { chain }
}

/// Markdown: every class and the container (no "+N" in text).
pub fn markdown(b: &Kind, md: &mut crate::render::markdown::Markdown) {
    for link in &b.chain {
        let values: Vec<String> = link.values.iter().map(|v| md.value(v)).collect();
        md.item(&md.property(link.pid), &values.join(", "));
    }
    md.end();
}
