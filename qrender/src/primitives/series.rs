//! Series: measurements of one property over time, as columns. From
//! statements dated by distinct years, or from a Commons data file.

use content::{Cell, Pid, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::compose::facts::dated_years;
use crate::schema::{Provenance, round_to};

/// With more points than this, only the ends and the peak carry numbers.
const LABELLED_MAX: usize = 12;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Series {
    pub pid: Pid,
    /// The statement of the data file (a file series only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<StatementId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub points: Vec<Point>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// HTML: the template binding. The markup is templates/primitives/series.html.
#[derive(askama::Template)]
#[template(path = "primitives/series.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Series,
}

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
    /// The amount as a share of the largest amount of the series.
    pub fraction: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<StatementId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    /// The point carries its number (a file series only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labelled: Option<bool>,
}

/// Dated quantity statements of one property, when their years are
/// distinct (`dated_years`).
pub fn dated(
    c: &Composer,
    pid: Pid,
    statements: &[&Statement],
    rule: Option<&str>,
) -> Option<Series> {
    let years = dated_years(statements)?;
    let amounts: Vec<f64> = statements
        .iter()
        .map(|s| match s.value {
            Value::Quantity { amount, .. } => Some(amount),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let top = amounts.iter().copied().fold(f64::MIN, f64::max);
    let mut points: Vec<_> = years
        .into_iter()
        .zip(amounts)
        .zip(statements)
        .map(|((year, amount), s)| Point {
            year,
            amount,
            fraction: share(amount, top),
            statement: Some(s.id.clone()),
            provenance: Some(c.provenance(s)),
            labelled: None,
        })
        .collect();
    points.sort_by_key(|p| p.year);
    Some(Series {
        pid,
        statement: None,
        provenance: None,
        source: None,
        points,
        rule: rule.map(str::to_string),
    })
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
    let top = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
    // The first point with the largest amount.
    let peak = points
        .iter()
        .copied()
        .reduce(|best, p| if p.1 > best.1 { p } else { best })?;
    let ends = [points[0], points[points.len() - 1], peak];
    let few = points.len() <= LABELLED_MAX;
    Some(Series {
        pid,
        statement: Some(statement.id.clone()),
        provenance: Some(c.provenance(statement)),
        source: Some(Source {
            file: table.title.clone(),
            license: table.license.clone(),
        }),
        points: points
            .iter()
            .map(|&(year, amount)| Point {
                year,
                amount,
                fraction: share(amount, top),
                statement: None,
                provenance: None,
                labelled: Some(few || ends.contains(&(year, amount))),
            })
            .collect(),
        rule: None,
    })
}

fn share(amount: f64, top: f64) -> f64 {
    if top == 0.0 {
        0.0
    } else {
        round_to(amount / top, 4)
    }
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

/// Markdown: every point, "year: amount".
pub fn markdown(b: &Series, md: &mut crate::render::markdown::Markdown) {
    let points: Vec<String> = b
        .points
        .iter()
        .map(|p| {
            format!(
                "{}: {}",
                md.view.year(p.year),
                crate::localize::format::number(p.amount)
            )
        })
        .collect();
    let mut value = points.join("; ");
    if let Some(source) = &b.source {
        value.push_str(&format!(" (source: {})", source.file));
    }
    md.item(&md.property(b.pid), &value);
}
