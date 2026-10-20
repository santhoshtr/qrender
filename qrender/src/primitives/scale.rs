//! Scale: the item against its container. "Nairobi is 11 % of the people
//! of Kenya"; "the Moon has 1.2 % of the mass of the Earth".

use content::{Pid, Qid, StatementId};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::{Provenance, round_to};
use crate::vocab::ContainerMode;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Scale {
    pub mode: ContainerMode,
    pub container: Container,
    pub measures: Vec<Measure>,
}

/// HTML: the template binding. The markup is templates/primitives/scale.html.
#[derive(askama::Template)]
#[template(path = "primitives/scale.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Scale,
}

/// The CSS of this primitive (design/primitives/scale.css).
pub const CSS: &str = include_str!("../../design/primitives/scale.css");

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Container {
    pub pid: Pid,
    pub item: Qid,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Measure {
    pub pid: Pid,
    /// The item's amount as a share of the container's.
    pub ratio: f64,
    pub amount: f64,
    pub whole: f64,
    /// The SI unit of both amounts. None for a count.
    pub unit: Option<Qid>,
    pub statement: StatementId,
    pub provenance: Provenance,
    /// The references that cite a source, by hash.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<String>,
}

/// The comparable quantities of the spread that both the item and its
/// container have. None when there is no container mode or no such
/// quantity.
pub fn build(c: &Composer, spread_pids: &[Pid]) -> Option<Scale> {
    let container = &c.content.container;
    let (pid, item) = (container.pid?, container.qid?);
    // A country is its own country (P17): it is not its container.
    if item == c.content.qid {
        return None;
    }
    let mode = *c.config.layouts.containers.get(&pid)?;
    let quantities = &c.content.quantities;
    let mine = &quantities.get(&c.content.qid)?.values;
    let theirs = &quantities.get(&item)?.values;
    let measures: Vec<_> = c
        .config
        .layouts
        .compare
        .pids
        .iter()
        .filter(|p| spread_pids.contains(p))
        .filter_map(|p| {
            let (own, whole) = (mine.get(p)?, theirs.get(p)?);
            if whole.amount == 0.0 {
                return None;
            }
            let statement = *c.current(c.statements_of(*p).to_vec()).first()?;
            Some(Measure {
                pid: *p,
                ratio: round_to(own.amount / whole.amount, 6),
                amount: own.amount,
                whole: whole.amount,
                unit: Some(own.unit).filter(|u| u.number() != 199),
                statement: statement.id.clone(),
                provenance: c.provenance(statement),
                citations: c.citations(statement),
            })
        })
        .collect();
    (!measures.is_empty()).then_some(Scale {
        mode,
        container: Container { pid, item },
        measures,
    })
}

/// Markdown: each measure as a share of the container.
pub fn markdown(b: &Scale, md: &mut crate::render::markdown::Markdown) {
    let container = md.label(content::EntityId::Item(b.container.item));
    for m in &b.measures {
        let crate::localize::format::Part::Number { text, .. } =
            crate::localize::format::percent(m.ratio)
        else {
            continue;
        };
        md.item(&md.property(m.pid), &format!("{text} of {container}"));
    }
    md.end();
}
