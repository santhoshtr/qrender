//! "More": every statement that no spread listed, kept and demoted: rows,
//! the most important first. A series is a chart, as in any spread.
//! Nothing is lost.

use content::{Datatype, Pid, Statement};
use indexmap::IndexMap;

use crate::compose::Composer;
use crate::compose::blocks::Made;
use crate::compose::facts::series_points;
use crate::config::SpreadSpec;
use crate::primitives::fact;
use crate::schema::Block;

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> super::Built {
    let mut left: IndexMap<Pid, Vec<&Statement>> = IndexMap::new();
    for s in c.unclaimed() {
        if s.value.datatype() != Datatype::ExternalId {
            left.entry(s.pid).or_default().push(s);
        }
    }
    let mut charts = Vec::new();
    let mut rows = Vec::new();
    for (pid, statements) in left {
        let statements = c.take(statements);
        // A series of quantities: a chart, by the block rules.
        if statements[0].value.datatype() == Datatype::Quantity
            && series_points(c, &statements).is_some()
        {
            charts.extend(c.blocks_for(pid, statements, &[]));
        } else {
            rows.push(fact::build(c, pid, &statements, None, None));
        }
    }
    if charts.is_empty() && rows.is_empty() {
        return Err(Vec::new());
    }
    // The most important first; the sort is stable.
    rows.sort_by(|a, b| b.priority.total_cmp(&a.priority));
    let made = charts
        .into_iter()
        .chain(rows.into_iter().map(|f| Made::Block {
            make: None,
            block: Block::Fact(f),
        }));
    let (slots, template) = c.place(made.collect());
    Ok(c.spread(spec, slots, template, Vec::new()))
}
