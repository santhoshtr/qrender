//! Score: review scores, one card each, with the logo of the source (IMDb,
//! Rotten Tomatoes). A logo is known at a glance, in every language.
//!
//! The qualifiers are the ones of the review score property (P444) itself,
//! so they are named here: this module exists for that property.

use content::{EntityId, Pid, Qid, QualifierValue, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::{Picture, Provenance};

const SOURCE: &str = "P447"; // review score by
const METHOD: &str = "P459"; // determination method or standard
const COUNT: &str = "P7887"; // number of reviews/ratings
const DATE: &str = "P585"; // point in time

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Score {
    pub pid: Pid,
    pub values: Vec<ScoreValue>,
    pub priority: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// HTML: the template binding. The markup is templates/primitives/score.html.
#[derive(askama::Template)]
#[template(path = "primitives/score.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Score,
}

/// The CSS of this primitive (design/primitives/score.css).
pub const CSS: &str = include_str!("../../design/primitives/score.css");

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScoreValue {
    pub statement: StatementId,
    pub pid: Pid,
    /// The score as Wikidata has it: "94%", "7.9/10".
    pub string: String,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ScoreSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<ItemRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<Count>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i64>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScoreSource {
    pub item: Qid,
    /// The logo of the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Picture>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ItemRef {
    pub item: Qid,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Count {
    pub pid: Pid,
    pub amount: f64,
}

/// The cards, the most reviewed first.
pub fn build(c: &Composer, pid: Pid, statements: &[&Statement], rule: Option<&str>) -> Score {
    let p = |id: &str| id.parse::<Pid>().expect("a property id");
    let (source, method, count, date) = (p(SOURCE), p(METHOD), p(COUNT), p(DATE));
    let mut values: Vec<ScoreValue> = statements
        .iter()
        .map(|s| {
            // The last qualifier of a property wins, as in a map.
            let qualifier = |q: Pid| {
                s.qualifiers
                    .iter()
                    .rev()
                    .find(|x| x.pid == q)
                    .map(|x| &x.value)
            };
            let item = |q: Pid| match qualifier(q) {
                Some(QualifierValue::Entity {
                    entity: EntityId::Item(item),
                }) => Some(*item),
                _ => None,
            };
            ScoreValue {
                statement: s.id.clone(),
                pid,
                string: match &s.value {
                    Value::String { string } => string.clone(),
                    other => format!("{other:?}"),
                },
                provenance: c.provenance(s),
                source: item(source).map(|item| ScoreSource {
                    item,
                    image: c.content.logos.get(&item).map(|logo| c.picture(logo, true)),
                }),
                method: item(method).map(|item| ItemRef { item }),
                count: match qualifier(count) {
                    Some(QualifierValue::Amount { amount }) => Some(Count {
                        pid: count,
                        amount: *amount,
                    }),
                    _ => None,
                },
                year: qualifier(date).and_then(QualifierValue::year),
            }
        })
        .collect();
    // An order that is the same in every language. The sort is stable.
    values.sort_by(|a, b| {
        let amount = |v: &ScoreValue| v.count.as_ref().map_or(0.0, |c| c.amount);
        amount(b).total_cmp(&amount(a))
    });
    Score {
        pid,
        values,
        priority: c.priority(pid, statements),
        rule: rule.map(str::to_string),
    }
}

/// Markdown: each score with its source, method, count and year.
pub fn markdown(b: &Score, md: &mut crate::render::markdown::Markdown) {
    let values: Vec<String> = b
        .values
        .iter()
        .map(|v| {
            let mut about = Vec::new();
            if let Some(s) = &v.source {
                about.push(md.label(content::EntityId::Item(s.item)));
            }
            if let Some(m) = &v.method {
                about.push(md.label(content::EntityId::Item(m.item)));
            }
            if let Some(c) = &v.count {
                about.push(format!(
                    "{} {}",
                    md.property(c.pid),
                    crate::localize::format::number(c.amount)
                ));
            }
            if let Some(y) = v.year {
                about.push(md.view.year(y));
            }
            format!(
                "{} ({}){}",
                v.string,
                about.join(", "),
                crate::render::markdown::provenance_words(v.provenance)
            )
        })
        .collect();
    md.items(&md.property(b.pid), &values);
}
