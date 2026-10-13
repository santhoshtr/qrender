//! Credits: the people who made a work, each person once, with all of
//! their roles ("director · screenwriter · producer"). The entries keep
//! the order of the properties: the director first.

use content::{Pid, Value};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::primitives::mark_shown;
use crate::schema::StatementRef;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Credits {
    /// The crew properties that have values, in the spread's order.
    pub pids: Vec<Pid>,
    pub values: Vec<Person>,
    pub priority: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// HTML: the template binding. The markup is templates/primitives/credits.html.
#[derive(askama::Template)]
#[template(path = "primitives/credits.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Credits,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Person {
    /// The person's first statement.
    #[serde(flatten)]
    pub statement: StatementRef,
    /// Every role of the person.
    pub pids: Vec<Pid>,
}

/// One block for the crew properties `pids` of a spread. Claims them all.
pub fn build(c: &mut Composer, pids: &[Pid], shown: usize, rule: Option<&str>) -> Option<Credits> {
    // A person is an item; a value that is not an item is its own entry.
    let mut people: IndexMap<String, Person> = IndexMap::new();
    for pid in pids {
        let statements = c.free(*pid);
        for s in c.take(statements) {
            let key = match &s.value {
                Value::WikibaseItem { item } => item.to_string(),
                other => format!("{other:?}"),
            };
            let person = people.entry(key).or_insert_with(|| Person {
                statement: c.statement(s),
                pids: Vec::new(),
            });
            if !person.pids.contains(pid) {
                person.pids.push(*pid);
            }
        }
    }
    if people.is_empty() {
        return None;
    }
    let (statements, roles): (Vec<_>, Vec<_>) =
        people.into_values().map(|p| (p.statement, p.pids)).unzip();
    let values = mark_shown(statements, shown)
        .into_iter()
        .zip(roles)
        .map(|(statement, pids)| Person { statement, pids })
        .collect::<Vec<_>>();
    Some(Credits {
        pids: pids
            .iter()
            .filter(|p| values.iter().any(|v| v.pids.contains(p)))
            .copied()
            .collect(),
        priority: pids
            .iter()
            .map(|p| c.priority(*p, &[]))
            .fold(f64::MIN, f64::max),
        values,
        rule: rule.map(str::to_string),
    })
}

/// Markdown: each person with all of their roles.
pub fn markdown(b: &Credits, md: &mut crate::render::markdown::Markdown) {
    for p in &b.values {
        let roles: Vec<String> = p.pids.iter().map(|r| md.property(*r)).collect();
        md.item(&roles.join(", "), &md.value(&p.statement));
    }
}
