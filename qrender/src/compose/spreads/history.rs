//! A history spread: the timeline of the events that are items of their
//! own (significant events), and a row for each property the layout lists
//! (inception, dissolution). Without either, the spread stays when it has
//! a topic ("history of India") or prose concepts: the view keeps it when
//! the article has the section.

use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::{fact, timeline};
use crate::schema::Block;
use crate::vocab::{Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> super::Built {
    let timelines: Vec<Block> = timeline::build(c)
        .into_iter()
        .map(Block::Timeline)
        .collect();
    let config = c.config;
    let topics = &config.layouts.topics;
    let mut rows = Vec::new();
    for p in &spec.pids {
        // A topic property is the topic of its spread (`Composer::spread`).
        if topics.contains_key(p) {
            continue;
        }
        let statements = c.free(*p);
        let statements = c.take(statements);
        if !statements.is_empty() {
            rows.push(Block::Fact(fact::build(c, *p, &statements, None, None)));
        }
    }
    let mut slots = IndexMap::new();
    if !timelines.is_empty() {
        slots.insert(Slot::Stack, timelines);
    }
    if !rows.is_empty() {
        slots.insert(Slot::Sheet, rows);
    }
    if slots.is_empty() {
        let has_topic = c
            .properties()
            .any(|(p, _)| topics.get(&p).is_some_and(|s| *s == spec.id));
        if !has_topic && spec.prose.is_empty() {
            return Err(Vec::new());
        }
    }
    Ok(c.spread(spec, slots, Template::Columns, Vec::new()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::compose::pid;
    use crate::compose::test_content::{content, item};
    use crate::config::Config;

    #[test]
    fn a_listed_date_is_a_row_and_a_topic_is_not() {
        let c = content(
            json!([
                {"pid": "P571", "value": {"type": "Time", "time": "1947-08-15T00:00:00Z", "precision": 11}},
                item("P2184", "Q1"),
            ]),
            json!({}),
        );
        let config = Config::embedded();
        let spec = config.layouts.class_layouts["settlement"]
            .spreads
            .iter()
            .find(|s| s.id == "history")
            .unwrap();
        let mut k = Composer::new(&c, config);
        let spread = build(&mut k, spec).unwrap();
        let Some([Block::Fact(row)]) = spread.slots.get(&Slot::Sheet).map(Vec::as_slice) else {
            panic!("one row");
        };
        assert_eq!(row.pid, pid("P571"));
        assert_eq!(spread.topics.len(), 1);
        assert!(k.unclaimed().is_empty());
    }
}
