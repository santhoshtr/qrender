//! A facts spread: the properties the layout lists, each by the shape of
//! its data and its role, through the rule tables.

use content::{Datatype, Pid, Value};

use crate::compose::Composer;
use crate::compose::blocks::Made;
use crate::compose::facts::PropertyFacts;
use crate::config::SpreadSpec;
use crate::config::rules::{PropertyMake, first};
use crate::primitives::fact::Age;
use crate::primitives::{credits, names, split, succession, works};
use crate::schema::Block;
use crate::vocab::Role;
use crate::wikidata::{AGE, DATE_OF_BIRTH, DATE_OF_DEATH, INSTANCE_OF};

pub fn build<'a>(c: &mut Composer<'a>, spec: &'a SpreadSpec) -> super::Built {
    let pids = properties(c, spec);
    let config = c.config;
    let thresholds = &config.layouts.thresholds;
    let mut made = Vec::new();
    for pid in &pids {
        // The names in many languages, at the place of the first.
        if let Some(block) = names::at(c, *pid, spec, &pids) {
            made.push(Made::Block { make: None, block });
        }
        // A split, at the place of its first part; its parts' other
        // statements go on below.
        if let Some(block) = split::at(c, *pid, &pids) {
            made.push(Made::Block { make: None, block });
        }
        let rule = first(&config.rules.property, &PropertyFacts::new(c, *pid));
        let id = Some(rule.id.as_str());
        match rule.make {
            PropertyMake::Skip => {}
            // All the crew properties of the spread: one block, by person,
            // where the first of them is.
            PropertyMake::Credits => {
                let done = made.iter().any(|m| {
                    matches!(
                        m,
                        Made::Block {
                            block: Block::Credits(_),
                            ..
                        }
                    )
                });
                if !done {
                    let crew: Vec<Pid> = pids
                        .iter()
                        .copied()
                        .filter(|p| c.has_role(*p, Role::Crew))
                        .collect();
                    if let Some(block) = credits::build(c, &crew, thresholds.set_max, id) {
                        made.push(Made::Block {
                            make: None,
                            block: Block::Credits(block),
                        });
                    }
                }
            }
            PropertyMake::Succession => {
                let offices = &config.layouts.roles.office;
                let pairs: Vec<_> = pids
                    .iter()
                    .filter_map(|p| Some((*p, *offices.get(p)?)))
                    .collect();
                let roles = succession::same_office(c, (*pid, offices[pid]), &pairs);
                if let Some(block) = succession::build(c, &roles, thresholds.succession_shown, id) {
                    made.push(Made::Block {
                        make: None,
                        block: Block::Succession(block),
                    });
                }
            }
            PropertyMake::Values => {
                let statements = c.free(*pid);
                let statements = c.take(statements);
                if !statements.is_empty() {
                    made.extend(c.blocks_for(*pid, statements, &pids));
                }
            }
        }
    }
    add_age(c, &mut made);
    if let Some(block) = works::build(c, &spec.reverse) {
        made.push(Made::Block {
            make: None,
            block: Block::Works(block),
        });
    }
    if made.is_empty() {
        return Err(Vec::new());
    }
    let (slots, template) = c.place(made);
    Ok(c.spread(spec, slots, template, Vec::new()))
}

/// The spread's properties; with `pictured`, also any item property whose
/// values are mostly pictured, in content order.
fn properties(c: &Composer, spec: &SpreadSpec) -> Vec<Pid> {
    let mut pids = spec.pids.clone();
    if !spec.pictured {
        return pids;
    }
    let layouts = &c.config.layouts;
    let skip = |p: &Pid| {
        layouts.evidence.pids.contains(p)
            || layouts.roles.list(Role::Web).contains(p)
            // Events of their own: the timeline of the history spread.
            || layouts.roles.list(Role::Timeline).contains(p)
            || layouts.topics.contains_key(p)
            || *p == *INSTANCE_OF
    };
    for (pid, statements) in c.properties() {
        let live: Vec<_> = statements.iter().filter(|s| !c.is_claimed(s)).collect();
        let pictured = live.iter().filter(|s| c.is_pictured(s)).count();
        if !pids.contains(&pid)
            && !skip(&pid)
            && live
                .first()
                .is_some_and(|s| s.value.datatype() == Datatype::WikibaseItem)
            && pictured as f64 >= layouts.thresholds.run_pictured * live.len() as f64
        {
            pids.push(pid);
        }
    }
    pids
}

/// The age at death, from the first dates of birth and death: an
/// intrinsic fact, on the row of the date of death.
fn add_age(c: &Composer, made: &mut [Made]) {
    let date = |pid| match c.statements_of(pid).first().map(|s| &s.value) {
        Some(Value::Time { time, .. }) => {
            let year = content::model::year_of(time)?;
            Some((year, time.get(5..10).unwrap_or_default().to_string()))
        }
        _ => None,
    };
    let (Some((born, born_day)), Some((died, died_day))) =
        (date(*DATE_OF_BIRTH), date(*DATE_OF_DEATH))
    else {
        return;
    };
    for m in made {
        if let Made::Block {
            block: Block::Fact(fact),
            ..
        } = m
            && fact.pid == *DATE_OF_DEATH
        {
            // Month and day as "MM-DD": a birthday not yet reached is one
            // year less.
            let years = died - born - i64::from(died_day < born_day);
            fact.age = Some(Age { pid: *AGE, years });
        }
    }
}
