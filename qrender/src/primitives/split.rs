//! Split: the parts of one whole (female and male population), as a bar
//! of 100 % for each year that every part has. The whole is the sum of the
//! parts, so the shares add up to 100 %. The configuration declares the
//! splits (`[[splits]]`); a split whose every part is a series is one chart
//! with a line for each part (`series::lines`).

use std::collections::HashMap;

use content::{Pid, Qid, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::compose::facts::dated_points;
use crate::config::SplitSpec;
use crate::primitives::series;
use crate::schema::{Block, Provenance, QualifierShown, round_to};
use crate::wikidata::POINT_IN_TIME;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Split {
    pub id: String,
    pub concept: Qid,
    /// The parts, in the order of the configuration (the order of their
    /// colours).
    pub pids: Vec<Pid>,
    /// One bar for each year, the oldest first.
    pub bars: Vec<Bar>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Bar {
    pub year: i64,
    /// The sum of the parts.
    pub total: f64,
    /// In the order of `pids`.
    pub shares: Vec<Share>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Share {
    pub amount: f64,
    /// The amount as a share of the total, rounded to 4 places.
    pub share: f64,
    pub statement: StatementId,
    pub provenance: Provenance,
    /// The qualifiers but the point in time (the year of the bar): the
    /// parts of a part ("female population: 328875190").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub qualifiers: Vec<QualifierShown>,
}

/// HTML: the template binding. The markup is templates/primitives/split.html.
#[derive(askama::Template)]
#[template(path = "primitives/split.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Split,
}

/// The CSS of this primitive (design/primitives/split.css).
pub const CSS: &str = include_str!("../../design/primitives/split.css");

/// The block of a split, at the place of its first part in `pids` (the
/// properties of a spread), when every part is in `pids`. It claims the
/// statements it shows; the others stay for the series and the rows of
/// their property.
pub fn at<'a>(c: &mut Composer<'a>, pid: Pid, pids: &[Pid]) -> Option<Block> {
    let config = c.config;
    let spec = config
        .layouts
        .splits
        .iter()
        .find(|s| s.parts.contains(&pid))?;
    let first = pids.iter().find(|p| spec.parts.contains(p))?;
    if *first != pid || !spec.parts.iter().all(|p| pids.contains(p)) {
        return None;
    }
    let parts: Vec<(Pid, Vec<&'a Statement>)> =
        spec.parts.iter().map(|p| (*p, c.free(*p))).collect();
    let quantities = |s: &Vec<&Statement>| {
        !s.is_empty() && s.iter().all(|s| matches!(s.value, Value::Quantity { .. }))
    };
    if !parts.iter().all(|(_, s)| quantities(s)) {
        return None;
    }
    if let Some(series) = series::lines(c, spec.concept, &parts) {
        c.take(parts.iter().flat_map(|(_, s)| s.iter().copied()));
        return Some(Block::Series(series));
    }
    bars(c, spec, &parts).map(Block::Split)
}

/// A bar for each year in which every part has a point.
fn bars<'a>(
    c: &mut Composer<'a>,
    spec: &SplitSpec,
    parts: &[(Pid, Vec<&'a Statement>)],
) -> Option<Split> {
    let points: Vec<HashMap<i64, &'a Statement>> = parts
        .iter()
        .map(|(_, statements)| {
            Some(
                dated_points(statements)?
                    .into_iter()
                    .filter(|p| !p.other)
                    .map(|p| (p.year, p.statement))
                    .collect(),
            )
        })
        .collect::<Option<_>>()?;
    let mut years: Vec<i64> = points[0]
        .keys()
        .copied()
        .filter(|y| points.iter().all(|by_year| by_year.contains_key(y)))
        .collect();
    years.sort_unstable();
    let mut bars = Vec::new();
    let mut shown = Vec::new();
    for year in years {
        let statements: Vec<&'a Statement> = points.iter().map(|by_year| by_year[&year]).collect();
        let amounts: Vec<f64> = statements.iter().map(|s| amount(s)).collect();
        let total: f64 = amounts.iter().sum();
        if total <= 0.0 {
            continue;
        }
        bars.push(Bar {
            year,
            total,
            shares: statements
                .iter()
                .zip(amounts)
                .map(|(s, amount)| Share {
                    amount,
                    share: round_to(amount / total, 4),
                    statement: s.id.clone(),
                    provenance: c.provenance(s),
                    qualifiers: s
                        .qualifiers
                        .iter()
                        .filter(|q| q.pid != *POINT_IN_TIME)
                        .filter_map(|q| QualifierShown::new(q.pid, &q.value))
                        .collect(),
                })
                .collect(),
        });
        shown.extend(statements);
    }
    if bars.is_empty() {
        return None;
    }
    c.take(shown);
    Some(Split {
        id: spec.id.clone(),
        concept: spec.concept,
        pids: spec.parts.clone(),
        bars,
    })
}

fn amount(statement: &Statement) -> f64 {
    match statement.value {
        Value::Quantity { amount, .. } => amount,
        _ => 0.0,
    }
}

/// Markdown: one item for each bar, "concept (year)", with each part, its
/// amount and its share.
pub fn markdown(b: &Split, md: &mut crate::render::markdown::Markdown) {
    let concept = md.label(content::EntityId::Item(b.concept));
    for bar in &b.bars {
        let parts: Vec<String> = b
            .pids
            .iter()
            .zip(&bar.shares)
            .map(|(pid, s)| {
                let mut part = format!(
                    "{} {} ({})",
                    md.property(*pid),
                    crate::localize::format::number(s.amount),
                    md.part(&crate::localize::format::percent(s.share))
                );
                let note = md.view.note(&s.qualifiers);
                if !note.is_empty() {
                    part.push_str(&format!(" ({})", md.note(&note)));
                }
                part
            })
            .collect();
        md.item(
            &format!("{concept} ({})", md.view.year(bar.year)),
            &parts.join("; "),
        );
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::compose::test_content::content;
    use crate::config::Config;

    fn pid(s: &str) -> Pid {
        s.parse().unwrap()
    }

    fn dated(p: &str, amount: f64, year: u32) -> Value {
        json!({"pid": p, "value": {"type": "Quantity", "amount": amount},
               "qualifiers": [{"pid": "P585", "value": {"type": "time",
                               "time": format!("{year}-01-01T00:00:00Z")}}]})
    }

    /// The block of the sex split (P1539, P1540) in a spread of both.
    fn split_of(statements: Value) -> (Option<Block>, usize) {
        let c = content(statements, json!({}));
        let mut k = Composer::new(&c, Config::embedded());
        let pids = [pid("P1082"), pid("P1539"), pid("P1540")];
        let block = at(&mut k, pid("P1539"), &pids);
        let left = k.free(pid("P1539")).len() + k.free(pid("P1540")).len();
        (block, left)
    }

    #[test]
    fn one_year_is_a_bar() {
        let (block, left) = split_of(json!([
            dated("P1539", 662380000.0, 2021),
            dated("P1540", 700620000.0, 2021)
        ]));
        let Some(Block::Split(split)) = block else {
            panic!("a split: {block:?}");
        };
        assert_eq!(split.bars.len(), 1);
        let bar = &split.bars[0];
        assert_eq!((bar.year, bar.total), (2021, 1363000000.0));
        assert_eq!(
            bar.shares.iter().map(|s| s.share).collect::<Vec<_>>(),
            [0.486, 0.514]
        );
        assert_eq!(left, 0);
    }

    #[test]
    fn every_part_a_series_is_one_chart() {
        let (block, left) = split_of(json!([
            dated("P1539", 1.0, 2011),
            dated("P1539", 2.0, 2021),
            dated("P1540", 3.0, 2011),
            dated("P1540", 4.0, 2021)
        ]));
        let Some(Block::Series(series)) = block else {
            panic!("a series: {block:?}");
        };
        assert_eq!(series.concept, Some("Q290".parse().unwrap()));
        assert_eq!(
            series.lines.iter().map(|l| l.pid).collect::<Vec<_>>(),
            [pid("P1539"), pid("P1540")]
        );
        assert_eq!(left, 0);
    }

    #[test]
    fn a_bar_only_for_the_common_year() {
        // Female in two years, male in one: a bar for 2021; the female
        // value of 2011 stays for the rows.
        let (block, left) = split_of(json!([
            dated("P1539", 1.0, 2011),
            dated("P1539", 2.0, 2021),
            dated("P1540", 3.0, 2021)
        ]));
        let Some(Block::Split(split)) = block else {
            panic!("a split: {block:?}");
        };
        assert_eq!(
            split.bars.iter().map(|b| b.year).collect::<Vec<_>>(),
            [2021]
        );
        assert_eq!(left, 1);
    }

    #[test]
    fn no_split_without_every_part() {
        let (block, left) = split_of(json!([dated("P1539", 1.0, 2021)]));
        assert!(block.is_none());
        assert_eq!(left, 1);
        // Not at a part that is not the first in the spread.
        let c = content(
            json!([dated("P1539", 1.0, 2021), dated("P1540", 2.0, 2021)]),
            json!({}),
        );
        let mut k = Composer::new(&c, Config::embedded());
        assert!(at(&mut k, pid("P1540"), &[pid("P1539"), pid("P1540")]).is_none());
    }
}
