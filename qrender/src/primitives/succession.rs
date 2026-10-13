//! Succession: an office and its holders. The office is the title, the
//! holder now leads, and the holders before are newest first.

use content::Pid;
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, qualifier_year};
use crate::primitives::mark_shown;
use crate::schema::StatementRef;
use crate::wikidata::{END_TIME, START_TIME};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Succession {
    pub pid: Pid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub office: Option<StatementRef>,
    pub holder: StatementRef,
    pub before: Vec<StatementRef>,
    pub priority: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// HTML: the template binding. The markup is templates/primitives/succession.html.
#[derive(askama::Template)]
#[template(path = "primitives/succession.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Succession,
}

/// `holder`: the holder property (P6); `office`: the property that names
/// its office (P1313). Claims the holders and the current office.
pub fn build(
    c: &mut Composer,
    holder: Pid,
    office: Pid,
    shown: usize,
    rule: Option<&str>,
) -> Option<Succession> {
    let holders = c.current(c.free(holder));
    if holders.is_empty() {
        return None;
    }
    c.take(holders.iter().copied());
    let offices = c.current(c.free(office));
    let office = c.take(offices.into_iter().take(1)).first().copied();
    let mut before = holders[1..].to_vec();
    // Newest first: by start, else by end.
    before.sort_by_key(|s| {
        std::cmp::Reverse(
            qualifier_year(s, *START_TIME)
                .filter(|y| *y != 0)
                .or_else(|| qualifier_year(s, *END_TIME).filter(|y| *y != 0))
                .unwrap_or(-1_000_000),
        )
    });
    Some(Succession {
        pid: holder,
        office: office.map(|s| c.statement(s)),
        holder: c.statement(holders[0]),
        before: mark_shown(before.iter().map(|s| c.statement(s)), shown),
        priority: c.priority(holder, &holders),
        rule: rule.map(str::to_string),
    })
}

/// Markdown: the office, the holder now, the holders before.
pub fn markdown(b: &Succession, md: &mut crate::render::markdown::Markdown) {
    let office = match &b.office {
        Some(o) => md.value(o),
        None => md.property(b.pid),
    };
    md.item(&office, &md.value(&b.holder));
    let before: Vec<String> = b.before.iter().map(|v| md.value(v)).collect();
    if !before.is_empty() {
        md.items(&format!("{office} (before)"), &before);
    }
}
