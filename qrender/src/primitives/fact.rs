//! Fact: a row of the fact sheet. Its form follows the shape of the data.

use std::collections::HashSet;

use content::{Datatype, EntityId, Pid, QualifierValue, Statement};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::primitives::mark_shown;
use crate::schema::StatementRef;
use crate::vocab::RangeEnd;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Fact {
    pub pid: Pid,
    pub form: FactForm,
    /// The first `list_shown` are shown; the rest are behind "+N".
    pub statements: Vec<StatementRef>,
    pub priority: f64,
    /// The values ended: history, not the present.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub former: bool,
    /// The age at death, from the dates of birth and death.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<Age>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
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
    let values = statements.iter().map(|s| c.statement(s));
    Fact {
        pid,
        form: form.unwrap_or_else(|| shape(c, statements)),
        statements: mark_shown(values, c.config.layouts.thresholds.list_shown),
        priority: c.priority(pid, statements),
        former: false,
        age: None,
        rule: rule.map(str::to_string),
    }
}

fn shape(c: &Composer, statements: &[&Statement]) -> FactForm {
    let range_ends = &c.config.layouts.range_ends;
    let ends: HashSet<RangeEnd> = statements
        .iter()
        .flat_map(|s| &s.qualifiers)
        .filter_map(|q| match q.value {
            QualifierValue::Entity {
                entity: EntityId::Item(item),
            } => range_ends.get(&item).copied(),
            _ => None,
        })
        .collect();
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
