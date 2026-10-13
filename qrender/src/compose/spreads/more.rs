//! "More": every statement that no spread listed, kept and demoted: rows,
//! the most important first, collapsed. Nothing is lost.

use content::{Datatype, Pid, Statement};
use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::fact;
use crate::schema::{Block, Spread};
use crate::vocab::{Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> Option<Spread> {
    let mut left: IndexMap<Pid, Vec<&Statement>> = IndexMap::new();
    for s in c.unclaimed() {
        if s.value.datatype() != Datatype::ExternalId {
            left.entry(s.pid).or_default().push(s);
        }
    }
    let mut rows: Vec<_> = left
        .into_iter()
        .map(|(pid, statements)| {
            let statements = c.take(statements);
            fact::build(c, pid, &statements, None, None)
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    // The most important first; the sort is stable.
    rows.sort_by(|a, b| b.priority.total_cmp(&a.priority));
    let slots = IndexMap::from([(Slot::Sheet, rows.into_iter().map(Block::Fact).collect())]);
    Some(c.spread(spec, slots, Template::Collapsed, Vec::new()))
}
