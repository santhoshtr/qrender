//! Time: a time axis of the item's history. Events of the item (by role)
//! and the spans of relations that changed what it is (former countries,
//! capital of). Dated measurements are series, never events.

use content::{Pid, Qid, Statement, StatementId, Value};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, qualifier_year};
use crate::schema::{Provenance, ValueShown, round_to};
use crate::vocab::Role;
use crate::wikidata::{END_TIME, POINT_IN_TIME, START_TIME};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Time {
    pub from: i64,
    pub to: i64,
    /// One track for each property, the earliest first.
    pub tracks: Vec<Track>,
}

/// HTML: the template binding. The markup is templates/primitives/time.html.
#[derive(askama::Template)]
#[template(path = "primitives/time.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Time,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Track {
    pub pid: Pid,
    pub marks: Vec<Mark>,
}

/// A point (`year`) or a span (`start`, `end`; either can be unknown). The
/// positions are fractions of the item's own time span.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Mark {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_pos: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_pos: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_pos: Option<f64>,
    pub statement: StatementId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<Qid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u8>,
    pub provenance: Provenance,
}

impl Mark {
    /// The year that places the mark: its year, else its start, else its end.
    fn first_year(&self) -> i64 {
        [self.year, self.start, self.end]
            .into_iter()
            .flatten()
            .find(|y| *y != 0)
            .unwrap_or(0)
    }
}

/// The axis, when its marks have at least `time_points` distinct years.
/// Else the number of distinct years, for the silence record. Claims the
/// events and the ended history spans; a span still holds where its
/// relation is shown, and the axis repeats it on purpose.
pub fn build(c: &mut Composer) -> Result<Time, usize> {
    let events = c.config.layouts.roles.list(Role::Event);
    let spans = c.config.layouts.roles.list(Role::HistorySpan);
    let mut tracks: IndexMap<Pid, Vec<Mark>> = IndexMap::new();
    let mut years = Vec::new();
    for s in &c.content.statements {
        let Some(mark) = mark(c, s, events.contains(&s.pid), spans.contains(&s.pid)) else {
            continue;
        };
        years.extend([mark.year, mark.start, mark.end].into_iter().flatten());
        tracks.entry(s.pid).or_default().push(mark);
    }
    let mut distinct = years.clone();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() < c.config.layouts.thresholds.time_points {
        return Err(distinct.len());
    }
    let (from, to) = (distinct[0], distinct[distinct.len() - 1]);
    let span = (to - from).max(1) as f64;
    let position = |y: Option<i64>| y.map(|y| round_to((y - from) as f64 / span, 4));
    for mark in tracks.values_mut().flatten() {
        mark.year_pos = position(mark.year);
        mark.start_pos = position(mark.start);
        mark.end_pos = position(mark.end);
    }
    let claimed: Vec<&Statement> = c
        .content
        .statements
        .iter()
        .filter(|s| events.contains(&s.pid) || (spans.contains(&s.pid) && Composer::ended(s)))
        .collect();
    c.take(claimed);
    let mut tracks: Vec<Track> = tracks
        .into_iter()
        .map(|(pid, mut marks)| {
            marks.sort_by_key(Mark::first_year);
            Track { pid, marks }
        })
        .collect();
    tracks.sort_by_key(|t| t.marks.iter().map(Mark::first_year).min());
    Ok(Time { from, to, tracks })
}

/// The mark of an event (a date, a point in time, or a span) or of a
/// history span (a span).
fn mark(c: &Composer, s: &Statement, event: bool, span: bool) -> Option<Mark> {
    let (start, end) = (qualifier_year(s, *START_TIME), qualifier_year(s, *END_TIME));
    let has_span = start.is_some() || end.is_some();
    let (year, start, end) = if event {
        if let Value::Time { time, .. } = &s.value
            && let Some(year) = content::model::year_of(time)
        {
            (Some(year), None, None)
        } else if let Some(year) = qualifier_year(s, *POINT_IN_TIME) {
            (Some(year), None, None)
        } else if has_span {
            (None, start, end)
        } else {
            return None;
        }
    } else if span && has_span {
        (None, start, end)
    } else {
        return None;
    };
    let shown = c.statement(s);
    let (item, time, precision) = match shown.value {
        ValueShown::Item { item } => (item, None, None),
        ValueShown::Time { time, precision } => (None, Some(time), precision),
        _ => (None, None, None),
    };
    Some(Mark {
        year,
        start,
        end,
        year_pos: None,
        start_pos: None,
        end_pos: None,
        statement: s.id.clone(),
        item,
        time,
        precision,
        provenance: shown.provenance,
    })
}

/// Markdown: each track, its points and spans in order.
pub fn markdown(b: &Time, md: &mut crate::render::markdown::Markdown) {
    for track in &b.tracks {
        let marks: Vec<String> = track
            .marks
            .iter()
            .map(|m| {
                let when = match m.year {
                    Some(y) => md.view.year(y),
                    None => format!(
                        "{} – {}",
                        m.start.map(|y| md.view.year(y)).unwrap_or_default(),
                        m.end.map(|y| md.view.year(y)).unwrap_or_default()
                    ),
                };
                match m.item {
                    Some(item) => format!("{when} {}", md.label(content::EntityId::Item(item))),
                    None => when,
                }
            })
            .collect();
        md.items(&md.property(track.pid), &marks);
    }
}
