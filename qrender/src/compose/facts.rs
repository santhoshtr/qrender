//! The predicate vocabulary of the rule tables. Each table has a facts
//! type; each predicate is a method; `holds` checks the conditions of one
//! rule. A predicate is computed only when a rule asks for it: `&&` stops
//! at the first condition that fails.
//!
//! To add a predicate: a field in the `When` type of its table
//! (`config/rules.rs`), a method here, and one line in `holds`.

use std::collections::HashSet;

use content::{Datatype, Pid, Statement};

use super::{Composer, qualifier_year};
use crate::config::rules::{BlockMake, GroupWhen, PlacementWhen, PropertyWhen, TemplateWhen, When};
use crate::vocab::{Primitive, Role, RoleSet};
use crate::wikidata::POINT_IN_TIME;

// ------------------------------------------------------------- property

/// The facts of a property, before any of its statements is claimed.
pub struct PropertyFacts {
    /// The property names the office of a holder property (P1313).
    pub office_title: bool,
    pub roles: RoleSet,
}

impl PropertyFacts {
    pub fn new(composer: &Composer, pid: Pid) -> Self {
        Self {
            office_title: composer.is_office_title(pid),
            roles: composer.config.roles(pid),
        }
    }
}

impl When<PropertyFacts> for PropertyWhen {
    fn holds(&self, f: &PropertyFacts) -> bool {
        self.office_title.is_none_or(|b| f.office_title == b)
            && self.role.is_none_or(|r| f.roles.has(r))
    }
}

// ---------------------------------------------------------------- group

/// The facts of a group of values of one property.
pub struct GroupFacts<'c, 'a> {
    composer: &'c Composer<'a>,
    pid: Pid,
    statements: &'c [&'a Statement],
    former: bool,
}

impl<'c, 'a> GroupFacts<'c, 'a> {
    /// `statements` is not empty. `former`: the group holds the values that
    /// ended.
    pub fn new(
        composer: &'c Composer<'a>,
        pid: Pid,
        statements: &'c [&'a Statement],
        former: bool,
    ) -> Self {
        debug_assert!(!statements.is_empty());
        Self {
            composer,
            pid,
            statements,
            former,
        }
    }

    pub fn datatype(&self) -> Datatype {
        self.statements[0].value.datatype()
    }

    pub fn roles(&self) -> RoleSet {
        self.composer.config.roles(self.pid)
    }

    pub fn former(&self) -> bool {
        self.former
    }

    /// Two values or fewer.
    pub fn few(&self) -> bool {
        self.statements.len() <= 2
    }

    /// At least `run_pictured` of the values have a picture.
    pub fn mostly_pictured(&self) -> bool {
        let pictured = self
            .statements
            .iter()
            .filter(|s| self.composer.is_pictured(s))
            .count();
        let share = self.composer.config.layouts.thresholds.run_pictured;
        pictured as f64 >= share * self.statements.len() as f64
    }

    /// The values make a series: each is dated, by distinct years.
    pub fn distinct_years(&self) -> bool {
        dated_years(self.statements).is_some()
    }

    /// The globe of a coordinate value; None for other values.
    pub fn globe(&self) -> Option<content::Qid> {
        match self.statements[0].value {
            content::Value::GlobeCoordinate { globe, .. } => Some(globe),
            _ => None,
        }
    }
}

impl When<GroupFacts<'_, '_>> for GroupWhen {
    fn holds(&self, f: &GroupFacts) -> bool {
        self.datatype.is_none_or(|d| f.datatype() == d)
            && self.role.is_none_or(|r| f.roles().has(r))
            && self
                .relation
                .is_none_or(|b| f.roles().has(Role::Relation) == b)
            && self.former.is_none_or(|b| f.former() == b)
            && self.few.is_none_or(|b| f.few() == b)
            && self
                .mostly_pictured
                .is_none_or(|b| f.mostly_pictured() == b)
            && self.distinct_years.is_none_or(|b| f.distinct_years() == b)
            && self.globe.is_none_or(|g| f.globe() == Some(g))
    }
}

/// The year of each statement (its point in time qualifier), when there
/// are at least three statements, each has a year, and no two share one.
pub fn dated_years(statements: &[&Statement]) -> Option<Vec<i64>> {
    let years: Vec<i64> = statements
        .iter()
        .map(|s| qualifier_year(s, *POINT_IN_TIME).filter(|y| *y != 0))
        .collect::<Option<_>>()?;
    let distinct: HashSet<_> = years.iter().collect();
    (years.len() >= 3 && distinct.len() == years.len()).then_some(years)
}

// ------------------------------------------------------------ placement

/// The facts of a block, for its slot.
pub struct PlacementFacts {
    /// The block rule that made the block, when a block rule made it.
    pub make: Option<BlockMake>,
    /// None for a single value: it is not a block.
    pub primitive: Option<Primitive>,
    /// The block is an audio file.
    pub audio: bool,
}

impl When<PlacementFacts> for PlacementWhen {
    fn holds(&self, f: &PlacementFacts) -> bool {
        self.make.is_none_or(|m| f.make == Some(m))
            && self.primitive.is_none_or(|p| f.primitive == Some(p))
            && self.audio.is_none_or(|b| f.audio == b)
    }
}

// ------------------------------------------------------------- template

/// The facts of a spread, for its template.
pub struct TemplateFacts {
    /// The primitive of the block that leads the spread.
    pub lead: Option<Primitive>,
}

impl When<TemplateFacts> for TemplateWhen {
    fn holds(&self, f: &TemplateFacts) -> bool {
        self.lead.is_none_or(|p| f.lead == Some(p))
    }
}

#[cfg(test)]
mod tests {
    //! One test for each row of each table in `rules.toml`.

    use serde_json::{Value, json};

    use super::*;
    use crate::compose::pid;
    use crate::compose::test_content::{content, item};
    use crate::config::Config;
    use crate::config::rules::{Order, PropertyMake, Row, SplitMake, first};
    use crate::vocab::{Slot, Template};

    fn rules() -> &'static crate::config::Rules {
        &Config::embedded().rules
    }

    fn property_rule(p: &str) -> (&'static str, PropertyMake) {
        let c = content(json!([]), json!({}));
        let k = Composer::new(&c, Config::embedded());
        let rule = first(&rules().property, &PropertyFacts::new(&k, pid(p)));
        (rule.id(), rule.make)
    }

    #[test]
    fn property_rules() {
        use PropertyMake::*;
        assert_eq!(property_rule("P1313"), ("office-title", Skip));
        assert_eq!(property_rule("P1332"), ("extreme-point", Skip));
        assert_eq!(property_rule("P6"), ("office", Succession));
        assert_eq!(property_rule("P57"), ("crew", Credits));
        assert_eq!(property_rule("P31"), ("values", Values));
    }

    /// The id of the first rule of a group table for these statements.
    fn group_rule<R>(
        table: &'static [R],
        statements: Value,
        extra: Value,
        former: bool,
    ) -> &'static str
    where
        R: Row<When = GroupWhen>,
    {
        let c = content(statements, extra);
        let k = Composer::new(&c, Config::embedded());
        let sts: Vec<_> = c.statements.iter().collect();
        first(table, &GroupFacts::new(&k, sts[0].pid, &sts, former)).id()
    }

    fn pictured(p: &str, q: &str) -> Value {
        json!({"pid": p, "value": {"type": "WikibaseItem", "item": q},
               "value_image": {"file": format!("{q}.jpg")}})
    }

    fn dated(p: &str, amount: f64, year: u32) -> Value {
        json!({"pid": p, "value": {"type": "Quantity", "amount": amount},
               "qualifiers": [{"pid": "P585", "value": {"type": "time",
                               "time": format!("{year}-01-01T00:00:00Z")}}]})
    }

    fn string(p: &str, s: &str) -> Value {
        json!({"pid": p, "value": {"type": "String", "string": s}})
    }

    #[test]
    fn split_rules() {
        let split = &rules().split;
        let none = json!({});
        assert_eq!(
            group_rule(split, json!([item("P17", "Q2")]), none.clone(), false),
            "former-values"
        );
        // A relation keeps its ended values with the others.
        assert_eq!(
            group_rule(split, json!([item("P26", "Q2")]), none.clone(), false),
            "together"
        );
        assert_eq!(
            group_rule(split, json!([dated("P1082", 1.0, 2000)]), none, false),
            "together"
        );
        assert_eq!(rules().split[0].make, SplitMake::SplitEnded);
    }

    #[test]
    fn block_rules() {
        let block = &rules().block;
        let none = json!({});
        let media = json!([{"pid": "P18", "value": {"type": "CommonsMedia", "file": "A.jpg"}}]);
        assert_eq!(group_rule(block, media, none.clone(), false), "media");
        let point = json!([{"pid": "P625", "value": {"type": "GlobeCoordinate", "lat": 1.0, "lon": 2.0, "globe": "Q2"}}]);
        assert_eq!(group_rule(block, point, none.clone(), false), "coordinate");
        // On the Moon: no Earth map, a row.
        let moon = json!([{"pid": "P625", "value": {"type": "GlobeCoordinate", "lat": 0.0, "lon": 0.0, "globe": "Q405"}}]);
        assert_eq!(group_rule(block, moon, none.clone(), false), "row");
        let series = json!([
            dated("P1082", 1.0, 2000),
            dated("P1082", 2.0, 2010),
            dated("P1082", 3.0, 2020)
        ]);
        assert_eq!(group_rule(block, series, none.clone(), false), "series");
        // Two values with the same year are not a series: rows.
        let colliding = json!([
            dated("P1082", 1.0, 2000),
            dated("P1082", 2.0, 2000),
            dated("P1082", 3.0, 2020)
        ]);
        assert_eq!(group_rule(block, colliding, none.clone(), false), "row");
        // Not a series role: rows, even with distinct years.
        let area = json!([
            dated("P2046", 1.0, 2000),
            dated("P2046", 2.0, 2010),
            dated("P2046", 3.0, 2020)
        ]);
        assert_eq!(group_rule(block, area, none.clone(), false), "row");
        assert_eq!(
            group_rule(block, json!([pictured("P17", "Q2")]), none.clone(), true),
            "former"
        );
        let unpictured = json!([
            item("P463", "Q2"),
            item("P463", "Q3"),
            pictured("P463", "Q4")
        ]);
        assert_eq!(
            group_rule(block, unpictured, none.clone(), false),
            "few-pictures"
        );
        let two = json!([pictured("P463", "Q2"), item("P463", "Q3")]);
        assert_eq!(group_rule(block, two, none.clone(), false), "single");
        let set = json!([
            pictured("P47", "Q2"),
            pictured("P47", "Q3"),
            item("P47", "Q4")
        ]);
        assert_eq!(group_rule(block, set, none.clone(), false), "set");
        // A class value has no picture, so a set of classes is a list.
        let classes = json!([
            pictured("P106", "Q2"),
            pictured("P106", "Q3"),
            pictured("P106", "Q4")
        ]);
        let are_classes = json!({"value_classes": ["Q2", "Q3"]});
        assert_eq!(
            group_rule(block, classes, are_classes, false),
            "few-pictures"
        );
        assert_eq!(
            group_rule(block, json!([string("P444", "94%")]), none.clone(), false),
            "score"
        );
        assert_eq!(
            group_rule(block, json!([string("P281", "00100")]), none, false),
            "row"
        );
    }

    #[test]
    fn order_rules() {
        let order = &rules().order;
        let none = json!({});
        assert_eq!(
            group_rule(order, json!([item("P161", "Q2")]), none.clone(), false),
            "billing"
        );
        assert_eq!(
            group_rule(order, json!([item("P47", "Q2")]), none, false),
            "pictured"
        );
        assert_eq!(rules().order[0].order, Order::References);
    }

    fn slot(make: Option<BlockMake>, primitive: Primitive, audio: bool) -> (&'static str, Slot) {
        let rule = first(
            &rules().placement,
            &PlacementFacts {
                make,
                primitive: Some(primitive),
                audio,
            },
        );
        (rule.id(), rule.slot)
    }

    #[test]
    fn placement_rules() {
        use Primitive::*;
        assert_eq!(
            slot(Some(BlockMake::Single), Network, false),
            ("single", Slot::Gallery)
        );
        assert_eq!(
            slot(Some(BlockMake::FormerList), Fact, false),
            ("row", Slot::Sheet)
        );
        assert_eq!(
            slot(Some(BlockMake::Media), Media, false),
            ("picture", Slot::Gallery)
        );
        assert_eq!(
            slot(Some(BlockMake::Media), Media, true),
            ("block", Slot::Stack)
        );
        assert_eq!(slot(None, Succession, false), ("block", Slot::Stack));
    }

    #[test]
    fn template_rules() {
        let template = |lead| {
            let rule = first(&rules().template, &TemplateFacts { lead });
            (rule.id(), rule.template)
        };
        assert_eq!(
            template(Some(Primitive::Succession)),
            ("office", Template::LeadBeside)
        );
        assert_eq!(
            template(Some(Primitive::Map)),
            ("map", Template::HeroPlusStack)
        );
        assert_eq!(template(None), ("plain", Template::Columns));
        assert_eq!(
            rules().lead.primitives,
            [Primitive::Succession, Primitive::Map]
        );
    }

    #[test]
    fn series_years() {
        let c = content(
            json!([
                dated("P1082", 1.0, 2010),
                dated("P1082", 2.0, 2000),
                dated("P1082", 3.0, 2020)
            ]),
            json!({}),
        );
        let sts: Vec<_> = c.statements.iter().collect();
        assert_eq!(dated_years(&sts), Some(vec![2010, 2000, 2020]));
        assert_eq!(dated_years(&sts[..2]), None, "fewer than three");
    }
}
