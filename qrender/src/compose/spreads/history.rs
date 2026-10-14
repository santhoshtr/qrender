//! A history spread: the timeline of the events that are items of their
//! own (significant events). Without one, the spread stays when it has a
//! topic ("history of India") or prose concepts: the view keeps it when
//! the article has the section.

use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::timeline;
use crate::schema::Block;
use crate::vocab::{Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> super::Built {
    let timelines: Vec<Block> = timeline::build(c)
        .into_iter()
        .map(Block::Timeline)
        .collect();
    let mut slots = IndexMap::new();
    if !timelines.is_empty() {
        slots.insert(Slot::Stack, timelines);
    } else {
        let topics = &c.config.layouts.topics;
        let has_topic = c
            .properties()
            .any(|(p, _)| topics.get(&p).is_some_and(|s| *s == spec.id));
        if !has_topic && spec.prose.is_empty() {
            return Err(Vec::new());
        }
    }
    Ok(c.spread(spec, slots, Template::Columns, Vec::new()))
}
