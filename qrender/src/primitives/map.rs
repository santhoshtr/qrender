//! Map: the coordinate of the item, with the points that mark its extent.
//! The composition gives the extent of the item; each renderer computes
//! the zoom for its own frame.

use content::{Pid, Statement, StatementId};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::{Provenance, StatementRef, ValueShown};
use crate::vocab::Role;
use crate::wikidata::AREA;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Map {
    #[serde(flatten)]
    pub statement: StatementRef,
    /// The side of a square with the item's area, in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent_m: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Marker {
    pub pid: Pid,
    #[serde(flatten)]
    pub value: ValueShown,
    pub statement: StatementId,
    pub provenance: Provenance,
}

/// The extreme points of the spread's properties become markers; they are
/// claimed here.
pub fn build(
    c: &mut Composer,
    statement: &Statement,
    spread_pids: &[Pid],
    rule: Option<&str>,
) -> Map {
    let area = c
        .content
        .quantities
        .get(&c.content.qid)
        .and_then(|q| q.values.get(&*AREA));
    let mut markers = Vec::new();
    for pid in c.config.layouts.roles.list(Role::ExtremePoint) {
        if spread_pids.contains(pid) {
            let points = c.free(*pid);
            for s in c.take(points) {
                markers.push(Marker {
                    pid: *pid,
                    value: ValueShown::from(&s.value),
                    statement: s.id.clone(),
                    provenance: c.provenance(s),
                });
            }
        }
    }
    Map {
        statement: c.statement(statement),
        extent_m: area.map(|a| a.amount.sqrt().round_ties_even() as i64),
        markers,
        rule: rule.map(str::to_string),
    }
}
