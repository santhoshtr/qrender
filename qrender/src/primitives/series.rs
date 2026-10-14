//! Series: measurements over time, as a line chart. One line from
//! statements dated by year, or from a Commons data file; or one line for
//! each part of a split, when every part is a series. The page has the
//! points as a table, a column for each line; `chart.js` draws the chart
//! from it.

use content::{Cell, Pid, Qid, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::compose::facts::series_points;
use crate::schema::Provenance;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Series {
    /// The split whose parts are the lines; None for one property.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<Qid>,
    pub lines: Vec<Line>,
    /// The statement of the data file (a file series only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<StatementId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// The points of one property, by year.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Line {
    pub pid: Pid,
    pub points: Vec<Point>,
}

/// A row of the table: a year, and a cell for each line.
pub struct TableRow<'a> {
    pub year: i64,
    pub cells: Vec<TableCell<'a>>,
}

/// The point of a line in one year, and the other values of that year.
pub struct TableCell<'a> {
    pub point: Option<&'a Point>,
    pub others: Vec<&'a Point>,
}

impl Series {
    /// The rows of the table: every year of every line, the oldest first.
    pub fn rows(&self) -> Vec<TableRow<'_>> {
        let mut years: Vec<i64> = self
            .lines
            .iter()
            .flat_map(|l| l.points.iter().map(|p| p.year))
            .collect();
        years.sort_unstable();
        years.dedup();
        years
            .into_iter()
            .map(|year| TableRow {
                year,
                cells: self
                    .lines
                    .iter()
                    .map(|l| {
                        let of_year = l.points.iter().filter(|p| p.year == year);
                        TableCell {
                            point: of_year.clone().find(|p| !p.other),
                            others: of_year.filter(|p| p.other).collect(),
                        }
                    })
                    .collect(),
            })
            .collect()
    }
}

/// HTML: the template binding. The markup is templates/primitives/series.html.
#[derive(askama::Template)]
#[template(path = "primitives/series.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Series,
}

/// The CSS of this primitive (design/primitives/series.css).
pub const CSS: &str = include_str!("../../design/primitives/series.css");

/// The script of this primitive: the qr-chart element (assets/chart.js).
pub const JS: &str = include_str!("../../assets/chart.js");

/// The Commons data file of a series.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Source {
    pub file: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Point {
    pub year: i64,
    pub amount: f64,
    /// Another value of the year, from another source: the point of the
    /// year is its preferred value.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub other: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<StatementId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// Dated quantity statements of one property, when they make a series
/// (`series_points`).
pub fn dated(
    c: &Composer,
    pid: Pid,
    statements: &[&Statement],
    rule: Option<&str>,
) -> Option<Series> {
    Some(Series {
        concept: None,
        lines: vec![line(c, pid, statements)?],
        statement: None,
        provenance: None,
        source: None,
        rule: rule.map(str::to_string),
    })
}

/// The parts of a split as one chart, a line for each part, when the
/// statements of every part make a series.
pub fn lines(c: &Composer, concept: Qid, parts: &[(Pid, Vec<&Statement>)]) -> Option<Series> {
    Some(Series {
        concept: Some(concept),
        lines: parts
            .iter()
            .map(|(pid, statements)| line(c, *pid, statements))
            .collect::<Option<_>>()?,
        statement: None,
        provenance: None,
        source: None,
        rule: None,
    })
}

fn line(c: &Composer, pid: Pid, statements: &[&Statement]) -> Option<Line> {
    let mut points: Vec<_> = series_points(statements)?
        .into_iter()
        .map(|p| match p.statement.value {
            Value::Quantity { amount, .. } => Some(Point {
                year: p.year,
                amount,
                other: p.other,
                statement: Some(p.statement.id.clone()),
                provenance: Some(c.provenance(p.statement)),
            }),
            _ => None,
        })
        .collect::<Option<_>>()?;
    points.sort_by_key(|p| (p.year, p.other));
    Some(Line { pid, points })
}

/// A series from a Commons data file: the first column whose name has
/// "year", "date" or "time", and the first other column that holds only
/// numbers.
pub fn tabular(c: &Composer, pid: Pid, statement: &Statement) -> Option<Series> {
    let Value::TabularData { page } = &statement.value else {
        return None;
    };
    let table = c.content.tabular.get(page)?;
    let year_column = table.fields.iter().position(|f| {
        let f = f.to_lowercase();
        ["year", "date", "time"].iter().any(|w| f.contains(w))
    })?;
    // Columns that every row has.
    let width = table.rows.iter().map(Vec::len).min().unwrap_or(0);
    let value_column = (0..width).find(|&i| {
        i != year_column
            && table
                .rows
                .iter()
                .all(|row| matches!(row[i], Cell::Null | Cell::Number(_)))
    })?;
    let mut points: Vec<(i64, f64)> = table
        .rows
        .iter()
        .filter_map(|row| Some((year(&row[year_column])?, number(&row[value_column])?)))
        .collect();
    if points.len() < 3 {
        return None;
    }
    points.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a data file"));
    Some(Series {
        concept: None,
        lines: vec![Line {
            pid,
            points: points
                .iter()
                .map(|&(year, amount)| Point {
                    year,
                    amount,
                    other: false,
                    statement: None,
                    provenance: None,
                })
                .collect(),
        }],
        statement: Some(statement.id.clone()),
        provenance: Some(c.provenance(statement)),
        source: Some(Source {
            file: table.title.clone(),
            license: table.license.clone(),
        }),
        rule: None,
    })
}

/// The year of a cell: its first four characters ("1793", "1793-01-01").
fn year(cell: &Cell) -> Option<i64> {
    let text = match cell {
        Cell::Number(n) => n.to_string(),
        Cell::Text(t) => t.clone(),
        Cell::Null | Cell::Bool(_) => return None,
    };
    text.chars().take(4).collect::<String>().parse().ok()
}

fn number(cell: &Cell) -> Option<f64> {
    match cell {
        Cell::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Markdown: every point, "year: amount"; with lines, one sub-item for
/// each line, under the concept.
pub fn markdown(b: &Series, md: &mut crate::render::markdown::Markdown) {
    if let Some(concept) = b.concept {
        let lines: Vec<String> = b
            .lines
            .iter()
            .map(|l| format!("{}: {}", md.described_property(l.pid), points(l, md)))
            .collect();
        md.items(&md.label(content::EntityId::Item(concept)), &lines);
        return;
    }
    let line = &b.lines[0];
    let mut value = points(line, md);
    if let Some(source) = &b.source {
        value.push_str(&format!(" (source: {})", source.file));
    }
    md.item(&md.described_property(line.pid), &value);
}

fn points(line: &Line, md: &crate::render::markdown::Markdown) -> String {
    line.points
        .iter()
        .map(|p| {
            let point = format!(
                "{}: {}",
                md.view.year(p.year),
                crate::localize::format::number(p.amount)
            );
            if p.other {
                format!("{point} ({})", md.view.msg("qrender-other-value"))
            } else {
                point
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}
