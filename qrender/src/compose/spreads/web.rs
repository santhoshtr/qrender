//! The web spread: the item's sites and accounts (the web role), a card
//! each. Counts of followers that name no account are rows.

use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::{fact, links};
use crate::schema::Block;
use crate::vocab::{Role, Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> super::Built {
    let pids = c.config.layouts.roles.list(Role::Web).to_vec();
    let (links, others) = links::build(c, &pids);
    let mut slots = IndexMap::new();
    if let Some(links) = links {
        slots.insert(Slot::Stack, vec![Block::Links(links)]);
    }
    if !others.is_empty() {
        let mut rows = Vec::new();
        let mut pids: Vec<_> = others.iter().map(|s| s.pid).collect();
        pids.dedup();
        for pid in pids {
            let statements: Vec<_> = others.iter().copied().filter(|s| s.pid == pid).collect();
            rows.push(Block::Fact(fact::build(c, pid, &statements, None, None)));
        }
        slots.insert(Slot::Sheet, rows);
    }
    if slots.is_empty() {
        return Err(Vec::new());
    }
    Ok(c.spread(spec, slots, Template::Columns, Vec::new()))
}
