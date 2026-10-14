//! Succession: an office and its holders. The office is the title, the
//! holder now leads, and the holders before are newest first. Holder
//! properties that name the same office with the same holder now (a
//! president who is head of state and head of government) are one block,
//! with every role of the holder.

use content::{Pid, Statement};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, qualifier_year};
use crate::primitives::mark_shown;
use crate::schema::StatementRef;
use crate::wikidata::{END_TIME, START_TIME};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Succession {
    /// The holder properties: the roles of the holder (head of government,
    /// head of state).
    pub pids: Vec<Pid>,
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

/// The CSS of this primitive (design/primitives/succession.css).
pub const CSS: &str = include_str!("../../design/primitives/succession.css");

/// The holder properties of one block: `holder` (P6, with its office
/// property P1313), and each other pair of `pairs` whose office is the same
/// item and whose holder now is the same person (P35, P1906 in the USA).
pub fn same_office(c: &Composer, holder: (Pid, Pid), pairs: &[(Pid, Pid)]) -> Vec<(Pid, Pid)> {
    let now = |(holder, office): (Pid, Pid)| {
        let item = |pid| c.current(c.free(pid)).first().and_then(|s| s.value.item());
        Some((item(holder)?, item(office)?))
    };
    let mut out = vec![holder];
    if let Some(first) = now(holder) {
        out.extend(
            pairs
                .iter()
                .copied()
                .filter(|p| p.0 != holder.0 && now(*p) == Some(first)),
        );
    }
    out
}

/// `roles`: the holder properties (P6) with the properties that name their
/// office (P1313); the first names the block. Claims the holders and the
/// current offices. A holding in two roles (the same person from the same
/// year, under two holder properties) is listed once.
pub fn build(
    c: &mut Composer,
    roles: &[(Pid, Pid)],
    shown: usize,
    rule: Option<&str>,
) -> Option<Succession> {
    let all: Vec<_> = roles.iter().flat_map(|(h, _)| c.free(*h)).collect();
    let holders = c.current(all);
    if holders.is_empty() {
        return None;
    }
    c.take(holders.iter().copied());
    let mut office = None;
    for (_, pid) in roles {
        let offices = c.current(c.free(*pid));
        let taken = c.take(offices.into_iter().take(1));
        office = office.or(taken.first().copied());
    }
    // The same holding in another role (the president as head of state):
    // the same person from the same year, under another holder property.
    // Two statements of one property are two holdings (acting, then
    // elected).
    let same = |a: &Statement, b: &Statement| {
        a.pid != b.pid
            && a.value.item() == b.value.item()
            && qualifier_year(a, *START_TIME) == qualifier_year(b, *START_TIME)
    };
    let mut kept: Vec<&Statement> = vec![holders[0]];
    for s in &holders[1..] {
        if !kept.iter().any(|k| same(k, s)) {
            kept.push(s);
        }
    }
    let mut before = kept[1..].to_vec();
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
        pids: roles.iter().map(|(h, _)| *h).collect(),
        office: office.map(|s| c.statement(s)),
        holder: c.statement(holders[0]),
        before: mark_shown(before.iter().map(|s| c.statement(s)), shown),
        priority: c.priority(roles[0].0, &holders),
        rule: rule.map(str::to_string),
    })
}

/// Markdown: the office, the holder now, the holders before.
pub fn markdown(b: &Succession, md: &mut crate::render::markdown::Markdown) {
    let office = match &b.office {
        Some(o) => md.value(o),
        None => md.property(b.pids[0]),
    };
    md.item(&office, &md.value(&b.holder));
    let before: Vec<String> = b.before.iter().map(|v| md.value(v)).collect();
    if !before.is_empty() {
        md.items(&format!("{office} (before)"), &before);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value as Json, json};

    use super::*;
    use crate::compose::test_content::{content, item};
    use crate::config::Config;

    fn pid(s: &str) -> Pid {
        s.parse().unwrap()
    }

    fn held(p: &str, who: &str, start: u32, end: Option<u32>) -> Json {
        let mut qualifiers = vec![json!({"pid": "P580", "value": {"type": "time",
            "time": format!("{start}-01-20T00:00:00Z")}})];
        if let Some(end) = end {
            qualifiers.push(json!({"pid": "P582", "value": {"type": "time",
                "time": format!("{end}-01-20T00:00:00Z")}}));
        }
        json!({"pid": p, "value": {"type": "WikibaseItem", "item": who},
               "qualifiers": qualifiers})
    }

    const PAIRS: [(&str, &str); 2] = [("P6", "P1313"), ("P35", "P1906")];

    fn blocks(statements: Json) -> Vec<Succession> {
        let c = content(statements, json!({}));
        let mut k = Composer::new(&c, Config::embedded());
        let pairs: Vec<_> = PAIRS.iter().map(|(h, o)| (pid(h), pid(o))).collect();
        let mut out = Vec::new();
        for pair in &pairs {
            let roles = same_office(&k, *pair, &pairs);
            out.extend(build(&mut k, &roles, 6, None));
        }
        out
    }

    #[test]
    fn one_office_two_roles_is_one_block() {
        // The USA: the president is head of government and head of state.
        let b = blocks(json!([
            held("P6", "Q2", 2025, None),
            held("P6", "Q3", 2021, Some(2025)),
            held("P35", "Q2", 2025, None),
            item("P1313", "Q11696"),
            item("P1906", "Q11696"),
        ]));
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].pids, [pid("P6"), pid("P35")]);
        assert_eq!(b[0].holder.value.item(), Some("Q2".parse().unwrap()));
        // The holder now is not one of the holders before.
        assert_eq!(b[0].before.len(), 1);
    }

    #[test]
    fn two_holdings_of_one_role_are_two() {
        // V. V. Giri: acting president in 1969, then elected in 1969.
        let b = blocks(json!([
            held("P35", "Q2", 2022, None),
            held("P35", "Q3", 1969, Some(1969)),
            held("P35", "Q3", 1969, Some(1974)),
            item("P1906", "Q193320"),
        ]));
        assert_eq!(b[0].before.len(), 2);
    }

    #[test]
    fn two_offices_are_two_blocks() {
        // India: a prime minister and a president.
        let b = blocks(json!([
            held("P6", "Q2", 2014, None),
            held("P35", "Q3", 2022, None),
            item("P1313", "Q192711"),
            item("P1906", "Q193320"),
        ]));
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].pids, [pid("P6")]);
        assert_eq!(b[1].pids, [pid("P35")]);
    }
}
