//! A scale spread: the item against its container and its peers, and the
//! series of its measures.

use content::Datatype;
use indexmap::IndexMap;

use crate::compose::Composer;
use crate::compose::blocks::Made;
use crate::config::SpreadSpec;
use crate::primitives::{fact, peers, scale, series, split};
use crate::schema::{Block, Silence, SilenceReason};
use crate::vocab::{Primitive, Slot, Template};

pub fn build<'a>(c: &mut Composer<'a>, spec: &'a SpreadSpec) -> super::Built {
    let pids = &spec.pids;
    let mut slots = IndexMap::new();
    let mut silent = Vec::new();
    match scale::build(c, pids) {
        Some(block) => {
            slots.insert(Slot::Hero, vec![Block::Scale(block)]);
        }
        None => silent.push(Silence {
            primitive: Primitive::Scale,
            reason: SilenceReason::NoComparableQuantity,
        }),
    }
    // The peer lineups do not depend on the spread's properties (as in v3).
    let mut stack: Vec<Block> = peers::build(c).into_iter().map(Block::Peers).collect();
    let mut rows = Vec::new();

    // A Commons data file first; the dated statements it replaces stay as
    // one fact, the current value.
    let tabular = c
        .config
        .layouts
        .roles
        .tabular
        .iter()
        .filter(|(p, _)| pids.contains(p))
        .find_map(|(p, replaces)| Some((*c.free(*p).first()?, *replaces)));
    let file_series = tabular.and_then(|(statement, replaces)| {
        let series = series::tabular(c, statement.pid, statement)?;
        c.take([statement]);
        let counts = c.current(c.free(replaces));
        if !counts.is_empty() {
            rows.push(Block::Fact(fact::build(
                c,
                replaces,
                &counts[..1],
                None,
                None,
            )));
            c.take(counts);
        }
        Some(series)
    });
    for p in pids {
        // A split, at the place of its first part; its parts' other
        // statements go on below.
        if let Some(block) = split::at(c, *p, pids) {
            stack.push(block);
        }
        let free = c.free(*p);
        if !free.is_empty()
            && let Some(block) = series::dated(c, *p, &free, None)
        {
            c.take(free);
            stack.push(Block::Series(block));
        }
    }
    if let Some(series) = file_series {
        stack.insert(0, Block::Series(series));
    }

    for p in pids {
        let statements = c.free(*p);
        let statements = c.take(statements);
        if statements.is_empty() {
            continue;
        }
        if statements[0].value.datatype() != Datatype::WikibaseItem {
            rows.push(Block::Fact(fact::build(c, *p, &statements, None, None)));
            continue;
        }
        // This spread has no gallery: a small value is a row here.
        let mut singles = Vec::new();
        for made in c.blocks_for(*p, statements.clone(), &[]) {
            match made {
                Made::Block {
                    block: Block::Fact(f),
                    ..
                } => rows.push(Block::Fact(f)),
                Made::Block { block, .. } => stack.push(block),
                Made::Single { value, .. } => singles.push(value.statement.statement),
            }
        }
        for id in singles {
            let s = statements
                .iter()
                .copied()
                .filter(|s| s.id == id)
                .collect::<Vec<_>>();
            rows.push(Block::Fact(fact::build(c, *p, &s, None, None)));
        }
    }
    if !stack.is_empty() {
        slots.insert(Slot::Stack, stack);
    }
    if !rows.is_empty() {
        slots.insert(Slot::Sheet, rows);
    }
    if slots.is_empty() {
        return Err(silent);
    }
    Ok(c.spread(spec, slots, Template::Columns, silent))
}
