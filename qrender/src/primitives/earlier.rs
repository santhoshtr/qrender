//! Earlier: the files of a property that ended (the flags of a country
//! before the current one), one block with the years they cover and a
//! dialog of the files, the oldest first. The current files of the
//! property are pictures, as before.

use content::{Pid, Statement};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, qualifier_year};
use crate::primitives::timeline::When;
use crate::schema::StatementRef;
use crate::wikidata::{END_TIME, START_TIME};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Earlier {
    pub pid: Pid,
    /// The oldest first.
    pub values: Vec<StatementRef>,
    /// The first start time and the last end time of the files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

impl Earlier {
    /// The years the files cover, as a span.
    pub fn span(&self) -> When {
        When {
            year: None,
            start: self.from,
            end: self.to,
            own: false,
        }
    }
}

/// HTML: the template binding. The markup is templates/primitives/earlier.html.
#[derive(askama::Template)]
#[template(path = "primitives/earlier.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Earlier,
}

/// The CSS of this primitive (design/primitives/earlier.css).
pub const CSS: &str = include_str!("../../design/primitives/earlier.css");

pub fn build(c: &Composer, pid: Pid, group: &[&Statement], rule: Option<&str>) -> Earlier {
    let start = |s: &Statement| qualifier_year(s, *START_TIME).filter(|y| *y != 0);
    let end = |s: &Statement| qualifier_year(s, *END_TIME).filter(|y| *y != 0);
    let mut group = group.to_vec();
    // Stable: by start, else by end; the undated last.
    group.sort_by_key(|s| start(s).or_else(|| end(s)).unwrap_or(i64::MAX));
    Earlier {
        pid,
        from: group.iter().filter_map(|s| start(s)).min(),
        to: group.iter().filter_map(|s| end(s)).max(),
        values: group.iter().map(|s| c.statement(s)).collect(),
        rule: rule.map(str::to_string),
    }
}

/// Markdown: every file, with its years.
pub fn markdown(b: &Earlier, md: &mut crate::render::markdown::Markdown) {
    let values: Vec<String> = b.values.iter().map(|s| md.value(s)).collect();
    md.items(
        &format!(
            "{} ({})",
            md.property(b.pid),
            md.view.msg("qrender-earlier")
        ),
        &values,
    );
}
