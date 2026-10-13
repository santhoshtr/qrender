//! The types of `rules.toml`: the ordered rule tables of the composer.
//!
//! In each table, the first rule whose conditions all hold wins. A
//! condition is an AND of named predicates; an absent predicate is not
//! checked. Each table has its own condition type, so each table has its
//! own vocabulary. The last rule of a table has no condition, so a table
//! always gives a rule.

use content::Datatype;
use serde::Deserialize;

use crate::vocab::{Primitive, Role, Slot, Template};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub property: Vec<PropertyRule>,
    pub split: Vec<SplitRule>,
    pub block: Vec<BlockRule>,
    pub order: Vec<OrderRule>,
    pub lead: Lead,
    pub placement: Vec<PlacementRule>,
    pub template: Vec<TemplateRule>,
}

/// A row of a rule table: its id and its condition.
pub trait Row {
    type When: PartialEq + Default;

    fn id(&self) -> &str;
    fn when(&self) -> &Self::When;

    fn unconditional(&self) -> bool {
        *self.when() == Self::When::default()
    }
}

/// A condition that can be checked against the facts of type `F`.
pub trait When<F> {
    fn holds(&self, facts: &F) -> bool;
}

/// The first rule whose condition holds.
pub fn first<'r, R, F>(rules: &'r [R], facts: &F) -> &'r R
where
    R: Row,
    R::When: When<F>,
{
    rules
        .iter()
        .find(|rule| rule.when().holds(facts))
        .expect("validated at load: the last rule of a table has no condition")
}

macro_rules! row {
    ($($rule:ty => $when:ty),* $(,)?) => {$(
        impl Row for $rule {
            type When = $when;

            fn id(&self) -> &str {
                &self.id
            }

            fn when(&self) -> &$when {
                &self.when
            }
        }
    )*};
}

row!(
    PropertyRule => PropertyWhen,
    SplitRule => GroupWhen,
    BlockRule => GroupWhen,
    OrderRule => GroupWhen,
    PlacementRule => PlacementWhen,
    TemplateRule => TemplateWhen,
);

// ------------------------------------------------------------- property

/// Before any statement of a property is claimed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyRule {
    pub id: String,
    #[serde(default)]
    pub when: PropertyWhen,
    pub make: PropertyMake,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyWhen {
    /// The property names the office of a holder property (P1313).
    pub office_title: Option<bool>,
    pub role: Option<Role>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PropertyMake {
    /// Not claimed here: another block shows it.
    Skip,
    Succession,
    /// All the crew properties of the spread, as one block by person.
    Credits,
    /// Claim the statements; the split and block tables decide the rest.
    Values,
}

// ------------------------------------------------- split, block, order

/// The conditions on a group of values of one property.
#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupWhen {
    #[serde(rename = "type")]
    pub datatype: Option<Datatype>,
    pub role: Option<Role>,
    /// The property has the role `relation`.
    pub relation: Option<bool>,
    /// The group holds the values that ended.
    pub former: Option<bool>,
    /// The group has two values or fewer.
    pub few: Option<bool>,
    /// At least `run_pictured` of the values have a picture.
    pub mostly_pictured: Option<bool>,
    /// The values make a series: dated by distinct years.
    pub distinct_years: Option<bool>,
}

/// Values of an item property that ended are history: a group of their own.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitRule {
    pub id: String,
    #[serde(default)]
    pub when: GroupWhen,
    pub make: SplitMake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SplitMake {
    /// The ended values are one group, the others another.
    SplitEnded,
    /// One group.
    Together,
}

/// The block of one group of values.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockRule {
    pub id: String,
    #[serde(default)]
    pub when: GroupWhen,
    pub make: BlockMake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockMake {
    Media,
    Map,
    Series,
    /// A list of values that ended, marked as former.
    FormerList,
    /// A list of item values (few have pictures).
    ItemList,
    /// Each value joins the gallery of the spread.
    Single,
    /// A set of pictured values.
    Network,
    Score,
    /// A row of the fact sheet.
    Fact,
}

/// The order of the values of a set.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderRule {
    pub id: String,
    #[serde(default)]
    pub when: GroupWhen,
    pub order: Order,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Order {
    /// More references first (a stand-in for billing), then pictured
    /// values, then the item number.
    References,
    /// Pictured values first; else the order stays.
    Pictured,
}

// ------------------------------------------------- lead, slot, template

/// The first block of one of these primitives leads its spread.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lead {
    pub primitives: Vec<Primitive>,
}

/// The slot of a block that does not lead.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementRule {
    pub id: String,
    #[serde(default)]
    pub when: PlacementWhen,
    pub slot: Slot,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementWhen {
    /// The block rule that made the block.
    pub make: Option<BlockMake>,
    pub primitive: Option<Primitive>,
    /// The block is an audio file.
    pub audio: Option<bool>,
}

/// The template of a facts spread, from its lead.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateRule {
    pub id: String,
    #[serde(default)]
    pub when: TemplateWhen,
    pub template: Template,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateWhen {
    pub lead: Option<Primitive>,
}
