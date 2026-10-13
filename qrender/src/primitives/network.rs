//! Network: the item values of a property, as pictures. Two forms:
//! - a set: the values of one property, in one row that scrolls, the
//!   first ones shown and all of them in a dialog;
//! - a gallery: the small values of different properties of one spread,
//!   one picture each.

use content::{Pid, Statement};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::config::rules::Order;
use crate::primitives::mark_shown;
use crate::schema::StatementRef;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Network {
    pub pid: Pid,
    pub values: Vec<StatementRef>,
    pub priority: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Gallery {
    pub role: GalleryRole,
    pub values: Vec<GalleryValue>,
    pub priority: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GalleryRole {
    Gallery,
}

/// A small value: it joins the gallery of its spread.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GalleryValue {
    #[serde(flatten)]
    pub statement: StatementRef,
    /// The priority of its property.
    pub priority: f64,
}

/// HTML: a set (templates/primitives/network.html).
#[derive(askama::Template)]
#[template(path = "primitives/network.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Network,
}

/// HTML: the gallery of a spread (templates/primitives/gallery.html).
#[derive(askama::Template)]
#[template(path = "primitives/gallery.html")]
pub struct GalleryHtml<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Gallery,
}

/// A set, in the order of the order rule; the first `shown` are shown.
pub fn build(
    c: &Composer,
    pid: Pid,
    statements: &[&Statement],
    order: Order,
    shown: usize,
    rule: Option<&str>,
) -> Network {
    let mut sorted = statements.to_vec();
    match order {
        // More references (a stand-in for billing), then pictured, then the
        // item number, which is the same in every language.
        Order::References => sorted.sort_by_key(|s| {
            (
                std::cmp::Reverse(c.content.references.get(&s.id).map_or(0, Vec::len)),
                !c.is_pictured(s),
                s.value.item().map_or(u64::MAX, |q| q.number()),
            )
        }),
        // Pictured first; the sort is stable, so the rest keeps its order.
        Order::Pictured => sorted.sort_by_key(|s| !c.is_pictured(s)),
    }
    Network {
        pid,
        values: mark_shown(sorted.iter().map(|s| c.statement(s)), shown),
        priority: c.priority(pid, statements),
        rule: rule.map(str::to_string),
    }
}

/// The small values of a spread: pictured first, then by priority.
pub fn gallery(mut values: Vec<GalleryValue>, rule: Option<&str>) -> Gallery {
    values.sort_by(|a, b| {
        (a.statement.image.is_none(), -a.priority)
            .partial_cmp(&(b.statement.image.is_none(), -b.priority))
            .expect("priorities are numbers")
    });
    Gallery {
        role: GalleryRole::Gallery,
        priority: values.iter().map(|v| v.priority).fold(f64::MIN, f64::max),
        values,
        rule: rule.map(str::to_string),
    }
}

/// Markdown: every value of the set.
pub fn markdown(b: &Network, md: &mut crate::render::markdown::Markdown) {
    let values: Vec<String> = b.values.iter().map(|v| md.value(v)).collect();
    md.items(&md.property(b.pid), &values);
}

/// Markdown: each small value with its property.
pub fn gallery_markdown(b: &Gallery, md: &mut crate::render::markdown::Markdown) {
    for v in &b.values {
        md.item(&md.property(v.statement.pid), &md.value(&v.statement));
    }
}
