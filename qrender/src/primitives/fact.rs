//! Fact: a row of the fact sheet. Its form follows the shape of the data.

use std::collections::HashSet;

use content::{Datatype, EntityId, Pid, QualifierValue, Range, Statement, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::{Composer, measurement_year};
use crate::primitives::mark_shown;
use crate::schema::{StatementRef, round_to};
use crate::vocab::RangeEnd;
use crate::wikidata::PERCENT;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Fact {
    pub pid: Pid,
    pub form: FactForm,
    /// The first `row_shown` are shown; the rest are behind "+N".
    pub statements: Vec<StatementRef>,
    pub priority: f64,
    /// The values ended: history, not the present.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub former: bool,
    /// The values are measurements dated by point in time: the description
    /// of the property says what the number is ("per 1,000 population").
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub measured: bool,
    /// One value on a scale: a proportion or an index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge: Option<Gauge>,
    /// The age at death, from the dates of birth and death.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<Age>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// HTML: a fact outside a sheet slot, as a sheet of one row
/// (templates/primitives/fact.html).
#[derive(askama::Template)]
#[template(path = "primitives/fact.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Fact,
}

/// The CSS of this primitive (design/primitives/fact.css).
pub const CSS: &str = include_str!("../../design/primitives/fact.css");

/// HTML: a row of the fact sheet (templates/primitives/fact-row.html).
#[derive(askama::Template)]
#[template(path = "primitives/fact-row.html")]
pub struct Row<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Fact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FactForm {
    /// One value, or a few.
    Value,
    /// Two quantities that are the minimum and the maximum.
    Range,
    /// Quantities that differ by a qualifier (equatorial, polar radius).
    Variants,
    /// Many values, one per line.
    List,
}

impl FactForm {
    /// The name the design system uses (`r-<name>`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Range => "range",
            Self::Variants => "variants",
            Self::List => "list",
        }
    }
}

/// A value on a scale from `min` to `max`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Gauge {
    pub kind: GaugeKind,
    pub min: f64,
    pub max: f64,
    /// Where the value is: 0 at `min`, 1 at `max`; rounded to 4 places.
    pub at: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GaugeKind {
    /// A part of a whole (a percentage, 0 to 100): a filled bar.
    Proportion,
    /// A position on a scale (Gini coefficient, 0 to 100): a marker.
    Index,
}

impl GaugeKind {
    /// The name the design system uses (`g-<name>`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Proportion => "proportion",
            Self::Index => "index",
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Age {
    /// The property that names an age (P3629).
    pub pid: Pid,
    pub years: i64,
}

/// `form`: None to take the form from the shape of the data.
pub fn build(
    c: &Composer,
    pid: Pid,
    statements: &[&Statement],
    form: Option<FactForm>,
    rule: Option<&str>,
) -> Fact {
    let form = form.unwrap_or_else(|| shape(c, statements));
    let mut statements = statements.to_vec();
    if form == FactForm::Range {
        // The minimum first, whatever the order of the data.
        statements.sort_by_key(|s| end(c, s) != Some(RangeEnd::Min));
    } else if statements[0].value.datatype() == Datatype::Quantity {
        // Measurements by their point in time, the newest first; the
        // undated after them.
        statements.sort_by_key(|s| std::cmp::Reverse(measurement_year(s)));
    }
    let statements = statements.as_slice();
    let measured = statements[0].value.datatype() == Datatype::Quantity
        && statements.iter().any(|s| measurement_year(s).is_some());
    let values = statements.iter().map(|s| c.statement(s));
    Fact {
        pid,
        form,
        statements: mark_shown(values, c.config.layouts.thresholds.row_shown),
        priority: c.priority(pid, statements),
        former: false,
        measured,
        gauge: gauge(c, pid, statements),
        age: None,
        rule: rule.map(str::to_string),
    }
}

/// The scale of one quantity value. A percentage whose property allows 0
/// to 100 is a proportion. An index (configuration) is on the range
/// constraint of its property, or on the range that the configuration
/// gives. A value outside its range has no gauge.
fn gauge(c: &Composer, pid: Pid, statements: &[&Statement]) -> Option<Gauge> {
    let [statement] = statements else {
        return None;
    };
    let Value::Quantity { amount, unit } = statement.value else {
        return None;
    };
    let constraint = c.content.ranges.get(&pid).copied();
    let (kind, range) = match c.config.layouts.indexes.get(&pid) {
        Some(index) => {
            let given = index
                .min
                .zip(index.max)
                .map(|(min, max)| Range { min, max });
            (GaugeKind::Index, given.or(constraint)?)
        }
        None if unit == Some(*PERCENT)
            && constraint
                == Some(Range {
                    min: 0.0,
                    max: 100.0,
                }) =>
        {
            (GaugeKind::Proportion, constraint?)
        }
        None => return None,
    };
    if range.min >= range.max || !(range.min..=range.max).contains(&amount) {
        return None;
    }
    Some(Gauge {
        kind,
        min: range.min,
        max: range.max,
        at: round_to((amount - range.min) / (range.max - range.min), 4),
    })
}

/// The end of a range that a statement's qualifiers mark (minimum or
/// maximum), if any.
fn end(c: &Composer, statement: &Statement) -> Option<RangeEnd> {
    statement.qualifiers.iter().find_map(|q| match q.value {
        QualifierValue::Entity {
            entity: EntityId::Item(item),
        } => c.config.layouts.range_ends.get(&item).copied(),
        _ => None,
    })
}

fn shape(c: &Composer, statements: &[&Statement]) -> FactForm {
    let ends: HashSet<RangeEnd> = statements.iter().filter_map(|s| end(c, s)).collect();
    let quantity = statements[0].value.datatype() == Datatype::Quantity;
    if quantity
        && statements.len() == 2
        && ends.contains(&RangeEnd::Min)
        && ends.contains(&RangeEnd::Max)
    {
        FactForm::Range
    } else if quantity && statements.len() > 1 {
        FactForm::Variants
    } else if statements.len() > 6 {
        FactForm::List
    } else {
        FactForm::Value
    }
}

/// Markdown: the row, with all of its values.
pub fn markdown(b: &Fact, md: &mut crate::render::markdown::Markdown) {
    let mut values: Vec<String> = b.statements.iter().map(|s| md.value(s)).collect();
    if let (Some(age), Some(first)) = (&b.age, values.first_mut()) {
        let words = crate::localize::messages::format(
            md.view.msg("qrender-age"),
            &[&age.years.to_string()],
        );
        first.push_str(&format!(" ({words})"));
    }
    // An index: its scale, which the number does not say.
    if let (Some(g), Some(first)) = (&b.gauge, values.first_mut())
        && g.kind == GaugeKind::Index
    {
        first.push_str(&format!(
            " (scale {}–{})",
            crate::localize::format::number(g.min),
            crate::localize::format::number(g.max)
        ));
    }
    let key = if b.former {
        format!("{} (former)", md.property(b.pid))
    } else {
        md.described_property(b.pid)
    };
    md.items(&key, &values);
}

#[cfg(test)]
mod tests {
    use serde_json::{Value as Json, json};

    use super::*;
    use crate::compose::test_content::content;
    use crate::config::Config;

    fn quantity(p: &str, amount: f64, unit: Option<&str>) -> Json {
        json!({"pid": p, "value": {"type": "Quantity", "amount": amount, "unit": unit}})
    }

    /// The gauge of the row of one property.
    fn gauge_of(statement: Json, ranges: Json) -> Option<(GaugeKind, f64, f64, f64)> {
        let c = content(json!([statement]), json!({ "ranges": ranges }));
        let k = Composer::new(&c, Config::embedded());
        let s: Vec<_> = c.statements.iter().collect();
        let g = build(&k, s[0].pid, &s, None, None).gauge?;
        Some((g.kind, g.min, g.max, g.at))
    }

    #[test]
    fn a_percentage_of_0_to_100_is_a_proportion() {
        let range = json!({"P1198": {"min": 0.0, "max": 100.0}});
        assert_eq!(
            gauge_of(quantity("P1198", 4.0, Some("Q11229")), range.clone()),
            Some((GaugeKind::Proportion, 0.0, 100.0, 0.04))
        );
        // Without the percent unit, or without the range: no gauge.
        assert_eq!(gauge_of(quantity("P1198", 4.0, None), range), None);
        assert_eq!(
            gauge_of(quantity("P1279", 4.0, Some("Q11229")), json!({})),
            None
        );
    }

    #[test]
    fn an_index_is_on_its_range() {
        // The range constraint of Wikidata.
        assert_eq!(
            gauge_of(
                quantity("P1125", 35.7, None),
                json!({"P1125": {"min": 0.0, "max": 100.0}})
            ),
            Some((GaugeKind::Index, 0.0, 100.0, 0.357))
        );
        // No constraint: the range of the configuration (Happy Planet Index).
        assert_eq!(
            gauge_of(quantity("P11547", 36.4, None), json!({})),
            Some((GaugeKind::Index, 0.0, 100.0, 0.364))
        );
        // A range constraint alone is not a scale (suicide rate: 0 to 400).
        assert_eq!(
            gauge_of(
                quantity("P3864", 12.9, None),
                json!({"P3864": {"min": 0.0, "max": 400.0}})
            ),
            None
        );
        // A value outside the range: no gauge.
        assert_eq!(
            gauge_of(
                quantity("P1125", 135.0, None),
                json!({"P1125": {"min": 0.0, "max": 100.0}})
            ),
            None
        );
    }
}
