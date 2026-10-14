//! Timeline: events that are items of their own (significant events), as
//! a vertical list in the history spread, the oldest first: for each event
//! its date, its picture, its label and its description. The date is the
//! statement's (point in time, or start and end time), else the event
//! item's own (`event_date`).

use content::{Pid, Statement};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, qualifier_year};
use crate::schema::StatementRef;
use crate::vocab::Role;
use crate::wikidata::{END_TIME, POINT_IN_TIME, START_TIME};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Timeline {
    pub pid: Pid,
    /// The oldest first; the events without a date last.
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Event {
    #[serde(flatten)]
    pub statement: StatementRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

/// A year, or a span of years (either end can be unknown).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct When {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    /// The date is the event item's own: the statement has none.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
}

impl When {
    /// The year that places the event: its year, else its start, else its
    /// end.
    fn first(&self) -> Option<i64> {
        self.year.or(self.start).or(self.end)
    }

    /// The one year of the event: its year, or a span within one year.
    pub fn single(&self) -> Option<i64> {
        self.year.or(self.start.filter(|s| Some(*s) == self.end))
    }
}

/// HTML: the template binding. The markup is templates/primitives/timeline.html.
#[derive(askama::Template)]
#[template(path = "primitives/timeline.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Timeline,
}

/// The CSS of this primitive (design/primitives/timeline.css).
pub const CSS: &str = include_str!("../../design/primitives/timeline.css");

/// The date of an event statement: its qualifiers (point in time, else
/// start and end time), else the dates of its item (`Content.event_dates`:
/// start and end time, else point in time).
pub fn event_date(c: &Composer, s: &Statement) -> Option<When> {
    let span = |start: Option<i64>, end: Option<i64>, own| {
        (start.is_some() || end.is_some()).then_some(When {
            year: None,
            start,
            end,
            own,
        })
    };
    let year = |year, own| When {
        year: Some(year),
        start: None,
        end: None,
        own,
    };
    if let Some(y) = qualifier_year(s, *POINT_IN_TIME) {
        return Some(year(y, false));
    }
    if let Some(when) = span(
        qualifier_year(s, *START_TIME),
        qualifier_year(s, *END_TIME),
        false,
    ) {
        return Some(when);
    }
    // The item's span first: a war has a start and an end, and its point
    // in time is often only one of them.
    let dates = c.content.event_dates.get(&s.value.item()?)?;
    let of = |t: &Option<String>| t.as_deref().and_then(content::model::year_of);
    span(of(&dates.start_time), of(&dates.end_time), true)
        .or_else(|| of(&dates.point_in_time).map(|y| year(y, true)))
}

/// The timeline of each timeline property of the item (the role), with
/// its statements that no spread has claimed.
pub fn build(c: &mut Composer) -> Vec<Timeline> {
    let config = c.config;
    let mut out = Vec::new();
    for pid in config.layouts.roles.list(Role::Timeline) {
        let statements = c.free(*pid);
        let statements = c.take(statements);
        if statements.is_empty() {
            continue;
        }
        let mut events: Vec<Event> = statements
            .iter()
            .map(|s| {
                let shown = c.statement(s);
                Event {
                    statement: StatementRef {
                        described: s.value.item().is_some(),
                        // The date qualifiers are the date of the event.
                        qualifiers: shown
                            .qualifiers
                            .into_iter()
                            .filter(|q| ![*POINT_IN_TIME, *START_TIME, *END_TIME].contains(&q.pid))
                            .collect(),
                        ..shown
                    },
                    when: event_date(c, s),
                }
            })
            .collect();
        // Stable: by the first year, then the one that ends first; the
        // undated last, in the order of the data.
        events.sort_by_key(|e| match e.when {
            Some(w) => (w.first().unwrap_or(i64::MAX), w.end.unwrap_or(i64::MIN)),
            None => (i64::MAX, i64::MAX),
        });
        out.push(Timeline { pid: *pid, events });
    }
    out
}

/// Markdown: each event, "when: label — description".
pub fn markdown(b: &Timeline, md: &mut crate::render::markdown::Markdown) {
    let events: Vec<String> = b
        .events
        .iter()
        .map(|e| {
            let value = md.value(&e.statement);
            match e.when {
                Some(w) => format!("{}: {value}", when(&w, md)),
                None => value,
            }
        })
        .collect();
    md.items(&md.property(b.pid), &events);
}

fn when(w: &When, md: &crate::render::markdown::Markdown) -> String {
    let year = |y: Option<i64>| y.map(|y| md.view.year(y)).unwrap_or_default();
    match w.single() {
        Some(y) => md.view.year(y),
        None => format!("{} – {}", year(w.start), year(w.end))
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value as Json, json};

    use super::*;
    use crate::compose::test_content::content;
    use crate::config::Config;

    fn event(item: &str, qualifiers: Json) -> Json {
        json!({"pid": "P793", "value": {"type": "WikibaseItem", "item": item},
               "qualifiers": qualifiers})
    }

    fn time(pid: &str, year: u32) -> Json {
        json!({"pid": pid, "value": {"type": "time", "time": format!("{year}-01-01T00:00:00Z")}})
    }

    #[test]
    fn the_statement_date_else_the_item_date() {
        let c = content(
            json!([
                event("Q2", json!([time("P580", 1947), time("P582", 1975)])),
                event("Q3", json!([])),
                event("Q4", json!([])),
            ]),
            json!({"event_dates": {
                "Q2": {"point_in_time": "1950-01-01T00:00:00Z"},
                "Q3": {"start_time": "2003-03-20T00:00:00Z", "end_time": "2011-12-15T00:00:00Z"}
            }}),
        );
        let k = Composer::new(&c, Config::embedded());
        let when: Vec<_> = c.statements.iter().map(|s| event_date(&k, s)).collect();
        // The statement's span, not the item's point in time.
        assert_eq!(
            when[0],
            Some(When {
                year: None,
                start: Some(1947),
                end: Some(1975),
                own: false
            })
        );
        // No date on the statement: the item's own.
        assert_eq!(
            when[1],
            Some(When {
                year: None,
                start: Some(2003),
                end: Some(2011),
                own: true
            })
        );
        assert_eq!(when[2], None);
    }

    #[test]
    fn oldest_first_undated_last() {
        let c = content(
            json!([
                event("Q4", json!([])),
                event("Q3", json!([time("P585", 2001)])),
                event("Q2", json!([time("P585", 1947)])),
            ]),
            json!({}),
        );
        let mut k = Composer::new(&c, Config::embedded());
        let timelines = build(&mut k);
        let items: Vec<_> = timelines[0]
            .events
            .iter()
            .map(|e| e.statement.value.item().map(|q| q.to_string()))
            .collect();
        assert_eq!(
            items,
            [Some("Q2".into()), Some("Q3".into()), Some("Q4".into())]
        );
        assert!(timelines[0].events.iter().all(|e| e.statement.described));
    }
}
