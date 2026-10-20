//! Links: the item's sites and accounts (the web role), a card for each
//! value with the logo of its platform. An account is a link: its value in
//! the formatter URL of its property. A count of followers goes on the
//! card of the account that it names.

use content::{Pid, Qid, QualifierValue, Statement, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, measurement_year};
use crate::schema::{Picture, Provenance, StatementRef, ValueShown};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Links {
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Card {
    pub statement: StatementRef,
    /// The address: a URL value, or an account in the formatter URL of its
    /// property. None for an item value (it links to its item) and for an
    /// account whose property has no formatter URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    /// The platform of the property (X), when the configuration names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Qid>,
    /// The logo of the platform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Picture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<Followers>,
}

/// The number of followers of an account: the latest count.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Followers {
    pub pid: Pid,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i64>,
    pub statement: StatementId,
    pub provenance: Provenance,
    /// The references that cite a source, by hash.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<String>,
}

/// HTML: the template binding. The markup is templates/primitives/links.html.
#[derive(askama::Template)]
#[template(path = "primitives/links.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Links,
}

/// The CSS of this primitive (design/primitives/links.css).
pub const CSS: &str = include_str!("../../design/primitives/links.css");

/// The cards of the statements of `pids`, in the order of `pids`, and the
/// counts of followers that are not on a card (an older count, a count of
/// no account). Claims all of them.
pub fn build<'a>(c: &mut Composer<'a>, pids: &[Pid]) -> (Option<Links>, Vec<&'a Statement>) {
    let statements = c.take(pids.iter().flat_map(|p| c.free(*p)).collect::<Vec<_>>());
    let (counts, values): (Vec<&Statement>, Vec<&Statement>) = statements
        .into_iter()
        .partition(|s| matches!(s.value, Value::Quantity { .. }));
    let mut cards: Vec<Card> = values
        .iter()
        .map(|s| {
            let platform = c.config.layouts.web.platforms.get(&s.pid).copied();
            Card {
                statement: c.statement(s),
                href: href(c, s),
                platform,
                logo: platform
                    .and_then(|p| c.content.logos.get(&p))
                    .map(|file| c.picture(file, true)),
                followers: None,
            }
        })
        .collect();
    let mut others = Vec::new();
    // The latest count first: the first that finds its card stays.
    let mut counts = counts;
    counts.sort_by_key(|s| std::cmp::Reverse(measurement_year(s)));
    for s in counts {
        // The count names its account by an identifier: the account itself
        // (subreddit "india"), or an identifier that the account statement
        // has as a qualifier (the X numeric user ID).
        let card = cards.iter_mut().zip(&values).find_map(|(card, v)| {
            s.qualifiers
                .iter()
                .filter(|q| matches!(q.value, QualifierValue::String { .. }))
                .any(|q| {
                    let own = matches!(&q.value, QualifierValue::String { string }
                        if Some(string.as_str()) == account(v));
                    (q.pid == v.pid && own) || v.qualifiers.contains(q)
                })
                .then_some(card)
        });
        match card {
            Some(card) if card.followers.is_none() => {
                card.followers = Some(Followers {
                    pid: s.pid,
                    amount: match s.value {
                        Value::Quantity { amount, .. } => amount,
                        _ => 0.0,
                    },
                    year: measurement_year(s),
                    statement: s.id.clone(),
                    provenance: c.provenance(s),
                    citations: c.citations(s),
                });
            }
            _ => others.push(s),
        }
    }
    let links = (!cards.is_empty()).then_some(Links { cards });
    (links, others)
}

/// The name of an account: an identifier or a string value.
fn account(s: &Statement) -> Option<&str> {
    match &s.value {
        Value::ExternalId { id } => Some(id),
        Value::String { string } => Some(string),
        _ => None,
    }
}

/// The address of a value: a URL, or an account in the formatter URL.
fn href(c: &Composer, s: &Statement) -> Option<String> {
    if let Value::Url { url } = &s.value {
        return Some(url.clone());
    }
    let formatter = c.content.formatters.get(&s.pid)?;
    Some(formatter.replace("$1", &account(s)?.replace(' ', "_")))
}

impl Card {
    /// The text of the link: a site without its scheme, "www." and final
    /// "/"; an account by its name.
    pub fn text(&self) -> &str {
        match &self.statement.value {
            ValueShown::Url { url } => {
                let url = url.split_once("://").map_or(url.as_str(), |(_, rest)| rest);
                let url = url.strip_prefix("www.").unwrap_or(url);
                url.strip_suffix('/').unwrap_or(url)
            }
            ValueShown::String { string } => string,
            _ => "",
        }
    }
}

/// Markdown: each value with its address, its note and its followers.
pub fn markdown(b: &Links, md: &mut crate::render::markdown::Markdown) {
    for card in &b.cards {
        let mut value = match &card.href {
            Some(href) => {
                let note = md.view.note(&card.statement.qualifiers);
                let mut link = format!("[{}]({href})", card.text());
                if !note.is_empty() {
                    link.push_str(&format!(" ({})", md.note(&note)));
                }
                link.push_str(crate::render::markdown::provenance_words(
                    card.statement.provenance,
                ));
                link
            }
            None => md.value(&card.statement),
        };
        if let Some(f) = &card.followers {
            value.push_str(&format!(
                " ({}: {}{})",
                md.property(f.pid),
                crate::localize::format::number(f.amount),
                f.year.map(|y| format!(", {y}")).unwrap_or_default()
            ));
        }
        md.item(&md.property(card.statement.pid), &value);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::compose::test_content::content;
    use crate::config::Config;

    #[test]
    fn accounts_are_links_with_their_followers() {
        let c = content(
            json!([
                {"pid": "P856", "value": {"type": "Url", "url": "https://india.gov.in/"}},
                {"pid": "P3984", "value": {"type": "ExternalId", "id": "india"}},
                {"pid": "P8687", "value": {"type": "Quantity", "amount": 1.0},
                 "qualifiers": [
                    {"pid": "P585", "value": {"type": "time", "time": "2021-01-01T00:00:00Z"}},
                    {"pid": "P3984", "value": {"type": "string", "string": "india"}}]},
                {"pid": "P8687", "value": {"type": "Quantity", "amount": 2.0},
                 "qualifiers": [
                    {"pid": "P585", "value": {"type": "time", "time": "2023-01-01T00:00:00Z"}},
                    {"pid": "P3984", "value": {"type": "string", "string": "india"}}]},
                {"pid": "P8687", "value": {"type": "Quantity", "amount": 3.0}},
                {"pid": "P2002", "value": {"type": "ExternalId", "id": "usagov"},
                 "qualifiers": [{"pid": "P6552", "value": {"type": "string", "string": "14074515"}}]},
                {"pid": "P8687", "value": {"type": "Quantity", "amount": 4.0},
                 "qualifiers": [{"pid": "P6552", "value": {"type": "string", "string": "14074515"}}]}
            ]),
            json!({"formatters": {"P3984": "https://www.reddit.com/r/$1/"},
                   "logos": {"Q1136": "Reddit wordmark.svg"}}),
        );
        let mut k = Composer::new(&c, Config::embedded());
        let pids: Vec<Pid> = ["P856", "P3984", "P2002", "P8687"]
            .iter()
            .map(|p| p.parse().unwrap())
            .collect();
        let (links, others) = build(&mut k, &pids);
        let cards = links.unwrap().cards;
        assert_eq!(cards[0].href.as_deref(), Some("https://india.gov.in/"));
        assert!(cards[0].logo.is_none());
        assert_eq!(
            cards[1].href.as_deref(),
            Some("https://www.reddit.com/r/india/")
        );
        assert_eq!(cards[1].logo.as_ref().unwrap().file, "Reddit wordmark.svg");
        // The latest count of the account; a count that names no account
        // stays a row.
        let followers = cards[1].followers.as_ref().unwrap();
        assert_eq!((followers.amount, followers.year), (2.0, Some(2023)));
        assert_eq!(others.len(), 2);
        // An X account, by the numeric user ID that both have.
        assert_eq!(cards[2].followers.as_ref().unwrap().amount, 4.0);
    }
}
