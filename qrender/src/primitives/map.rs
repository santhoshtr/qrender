//! Map: the coordinate of the item, with the points that mark its extent
//! and the outlines of its geoshapes. The composition gives the extent of
//! the item; each renderer computes the zoom for its own frame.

use content::{Pid, Rank, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::{Provenance, StatementRef, ValueShown};
use crate::vocab::Role;
use crate::wikidata::{AREA, GEOSHAPE};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Map {
    #[serde(flatten)]
    pub statement: StatementRef,
    /// The side of a square with the item's area, in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent_m: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    /// The geoshapes of the item, the preferred first: outlines on the
    /// map. Two shapes can be two views of a disputed border; their
    /// qualifiers say whose.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<MapShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MapShape {
    #[serde(flatten)]
    pub statement: StatementRef,
    /// The Commons page: "Data:India.map".
    pub title: String,
    /// The outline: longitude, latitude (`content::Shape`).
    pub lines: Vec<Vec<[f64; 2]>>,
    /// West, south, east, north.
    pub bbox: [f64; 4],
    pub license: String,
}

/// HTML: the template binding. The markup is templates/primitives/map.html.
#[derive(askama::Template)]
#[template(path = "primitives/map.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Map,
}

/// The CSS of this primitive (design/primitives/map.css).
pub const CSS: &str = include_str!("../../design/primitives/map.css");

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Marker {
    pub pid: Pid,
    #[serde(flatten)]
    pub value: ValueShown,
    pub statement: StatementId,
    pub provenance: Provenance,
    /// The references that cite a source, by hash.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<String>,
}

/// The extreme points and the geoshapes of the spread's properties become
/// markers and outlines; they are claimed here. A geoshape without its
/// outline stays for the rows.
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
                    citations: c.citations(s),
                });
            }
        }
    }
    let mut shapes = Vec::new();
    if spread_pids.contains(&GEOSHAPE) {
        let mut statements: Vec<_> = c
            .free(*GEOSHAPE)
            .into_iter()
            .filter_map(|s| match &s.value {
                Value::GeoShape { page } => Some((s, c.content.shapes.get(page)?)),
                _ => None,
            })
            .collect();
        statements.sort_by_key(|(s, _)| s.rank != Rank::Preferred);
        for (s, shape) in statements {
            c.take([s]);
            shapes.push(MapShape {
                statement: c.statement(s),
                title: shape.title.clone(),
                lines: shape.lines.clone(),
                bbox: shape.bbox,
                license: shape.license.clone(),
            });
        }
    }
    Map {
        statement: c.statement(statement),
        extent_m: area.map(|a| a.amount.sqrt().round_ties_even() as i64),
        markers,
        shapes,
        rule: rule.map(str::to_string),
    }
}

/// Markdown: the coordinate, the extreme points, and the geoshapes with
/// their qualifiers (whose view each is).
pub fn markdown(b: &Map, md: &mut crate::render::markdown::Markdown) {
    md.item(&md.property(b.statement.pid), &md.value(&b.statement));
    for m in &b.markers {
        let value = md.part(&md.view.part(&m.value));
        md.item(&md.property(m.pid), &value);
    }
    for s in &b.shapes {
        let mut value = format!(
            "[{}](https://commons.wikimedia.org/wiki/{})",
            s.title,
            s.title.replace(' ', "_")
        );
        // Whose view the shape is: "statement disputed by: A, B".
        let groups: Vec<String> = md
            .view
            .labelled_note(&s.statement.qualifiers)
            .iter()
            .map(|(pid, parts)| {
                let values: Vec<String> = parts.iter().map(|p| md.part(p)).collect();
                format!("{}: {}", md.property(*pid), values.join(", "))
            })
            .collect();
        if !groups.is_empty() {
            value.push_str(&format!(" ({})", groups.join("; ")));
        }
        md.item(&md.property(s.statement.pid), &value);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::compose::test_content::content;
    use crate::config::Config;

    #[test]
    fn the_map_takes_the_shapes_with_outlines() {
        let url = |t: &str| format!("http://commons.wikimedia.org/data/main/Data:{t}.map");
        let c = content(
            json!([
                {"pid": "P625", "value": {"type": "GlobeCoordinate", "lat": 22.8, "lon": 83.0, "globe": "Q2"}},
                {"pid": "P3896", "value": {"type": "GeoShape", "page": url("A")}},
                {"pid": "P3896", "rank": "preferred", "value": {"type": "GeoShape", "page": url("B")}},
                {"pid": "P3896", "value": {"type": "GeoShape", "page": url("C")}},
            ]),
            json!({"shapes": {
                url("A"): {"title": "Data:A.map", "lines": [[[80.0, 20.0], [85.0, 25.0]]],
                           "bbox": [80.0, 20.0, 85.0, 25.0], "license": "CC0-1.0"},
                url("B"): {"title": "Data:B.map", "lines": [[[80.0, 20.0], [86.0, 26.0]]],
                           "bbox": [80.0, 20.0, 86.0, 26.0], "license": "CC0-1.0"}
            }}),
        );
        let mut k = Composer::new(&c, Config::embedded());
        let pids = ["P625", "P3896"].map(|p| p.parse::<Pid>().unwrap());
        let map = build(&mut k, &c.statements[0], &pids, None);
        // The preferred first; C has no outline and stays for the rows.
        let titles: Vec<_> = map.shapes.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["Data:B.map", "Data:A.map"]);
        assert_eq!(k.free(pids[1]).len(), 1);
    }
}
