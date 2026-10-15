//! Network: the item values of a property, as pictures. Two forms:
//! - a set: the values of one property, the first ones shown and all of
//!   them in a dialog; a strip of pictures that scrolls, or rows with the
//!   qualifiers when they matter (`NetworkForm`);
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
    pub form: NetworkForm,
    pub values: Vec<StatementRef>,
    pub priority: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// How a set shows its first values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NetworkForm {
    /// A strip of pictures: the picture is the value.
    Strip,
    /// A row for each value, its picture small, its qualifiers in full:
    /// they name other items (an embassy, a border).
    Rows,
}

impl NetworkForm {
    /// The name the design system uses.
    pub fn name(self) -> &'static str {
        match self {
            Self::Strip => "strip",
            Self::Rows => "rows",
        }
    }
}

impl Network {
    /// The values shown before "+N": the first ones.
    pub fn shown(&self) -> &[StatementRef] {
        let n = self
            .values
            .iter()
            .take_while(|v| v.shown != Some(false))
            .count();
        &self.values[..n]
    }
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
    form: NetworkForm,
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
        // The most known first, then pictured; the sort is stable, so the
        // rest keeps its order.
        Order::Significance => sorted.sort_by_key(|s| {
            let known = s
                .value
                .item()
                .and_then(|q| c.content.value_sitelinks.get(&q));
            (
                std::cmp::Reverse(known.copied().unwrap_or(0)),
                !c.is_pictured(s),
            )
        }),
    }
    Network {
        pid,
        form,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::compose::test_content::{content, item};
    use crate::config::Config;

    #[test]
    fn the_most_known_first() {
        // Member of: a small body first in the data, the UN after it.
        let c = content(
            json!([
                item("P463", "Q7172"),
                item("P463", "Q1065"),
                item("P463", "Q8475")
            ]),
            json!({"value_sitelinks": {"Q1065": 250, "Q7172": 60}}),
        );
        let k = Composer::new(&c, Config::embedded());
        let statements: Vec<&Statement> = c.statements.iter().collect();
        let set = build(
            &k,
            "P463".parse().unwrap(),
            &statements,
            NetworkForm::Strip,
            Order::Significance,
            5,
            None,
        );
        let order: Vec<String> = set
            .values
            .iter()
            .map(|v| v.value.item().unwrap().to_string())
            .collect();
        // Without a count, the value keeps its place after the known ones.
        assert_eq!(order, ["Q1065", "Q7172", "Q8475"]);
    }
}
