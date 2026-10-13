//! The evidence spread: provenance counts, web presence, curation data,
//! the projects that have a page, identifiers, and what most peers have
//! that the item lacks.

use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::{absent, fact, identifiers, provenance, sitelinks};
use crate::schema::{Block, Spread};
use crate::vocab::{Role, Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> Spread {
    let layouts = &c.config.layouts;
    let mut slots = IndexMap::new();
    slots.insert(Slot::Summary, vec![Block::Provenance(provenance::build(c))]);
    let web = rows(c, layouts.roles.list(Role::Web));
    if !web.is_empty() {
        slots.insert(Slot::Web, web);
    }
    // A block with nothing to show is left out: no empty sheet, no
    // button for zero identifiers.
    let curation = rows(c, &layouts.evidence.pids);
    if !curation.is_empty() {
        slots.insert(Slot::Sheet, curation);
    }
    let projects = sitelinks::build(&c.content.sitelinks);
    if !projects.families.is_empty() {
        slots.insert(Slot::Projects, vec![Block::Sitelinks(projects)]);
    }
    let identifiers = identifiers::build(c);
    if !identifiers.values.is_empty() {
        slots.insert(Slot::Identifiers, vec![Block::Identifiers(identifiers)]);
    }
    let absent: Vec<_> = absent::build(c).into_iter().map(Block::Absent).collect();
    if !absent.is_empty() {
        slots.insert(Slot::Absent, absent);
    }
    c.spread(spec, slots, Template::Sheet, Vec::new())
}

/// A row for each property that has unclaimed statements. Claims them.
fn rows(c: &mut Composer, pids: &[content::Pid]) -> Vec<Block> {
    let mut out = Vec::new();
    for pid in pids {
        let statements = c.free(*pid);
        if !statements.is_empty() {
            let statements = c.take(statements);
            out.push(Block::Fact(fact::build(c, *pid, &statements, None, None)));
        }
    }
    out
}
