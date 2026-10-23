//! The predicate vocabulary of the rule tables. Each table has a facts
//! type; each predicate is a method; `holds` checks the conditions of one
//! rule. A predicate is computed only when a rule asks for it: `&&` stops
//! at the first condition that fails.
//!
//! To add a predicate: a field in the `When` type of its table
//! (`config/rules.rs`), a method here, and one line in `holds`.

use std::collections::{HashMap, HashSet};

use content::{Datatype, EntityId, Pid, Qualifier, QualifierValue, Rank, Statement, Value};

use super::{Composer, measurement_year};
use crate::config::rules::{BlockMake, GroupWhen, PlacementWhen, PropertyWhen, TemplateWhen, When};
use crate::vocab::{Primitive, Role, RoleSet};
use crate::wikidata::{
    COORDINATE_LOCATION, EARTH, END_TIME, POINT_IN_TIME, PREFERRED_REASON, START_TIME,
};

// ------------------------------------------------------------- property

/// The facts of a property, before any of its statements is claimed.
pub struct PropertyFacts {
    /// The property names the office of a holder property (P1313).
    pub office_title: bool,
    pub roles: RoleSet,
    /// The spread lists the coordinate (P625) and the item has one on the
    /// Earth: the spread has a map, and its extreme points are markers on
    /// it.
    pub map: bool,
}

impl PropertyFacts {
    /// `pids`: the properties of the spread.
    pub fn new(composer: &Composer, pid: Pid, pids: &[Pid]) -> Self {
        // All the statements: the map has claimed the coordinate before.
        let on_earth = || {
            composer
                .statements_of(*COORDINATE_LOCATION)
                .iter()
                .any(|s| matches!(s.value, Value::GlobeCoordinate { globe, .. } if globe == *EARTH))
        };
        Self {
            office_title: composer.is_office_title(pid),
            roles: composer.config.roles(pid),
            map: pids.contains(&COORDINATE_LOCATION) && on_earth(),
        }
    }
}

impl When<PropertyFacts> for PropertyWhen {
    fn holds(&self, f: &PropertyFacts) -> bool {
        self.office_title.is_none_or(|b| f.office_title == b)
            && self.role.is_none_or(|r| f.roles.has(r))
            && self.map.is_none_or(|b| f.map == b)
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

    /// `linked_min` values or more, of which `linked_share` have a
    /// qualifier that names another item. The reason for the preferred
    /// rank does not count: it is for editors.
    pub fn linked(&self) -> bool {
        let thresholds = &self.composer.config.layouts.thresholds;
        let linked = self
            .statements
            .iter()
            .filter(|s| {
                s.qualifiers.iter().any(|q| {
                    q.pid != *PREFERRED_REASON
                        && matches!(
                            q.value,
                            QualifierValue::Entity {
                                entity: EntityId::Item(_)
                            }
                        )
                })
            })
            .count();
        self.statements.len() >= thresholds.linked_min
            && linked as f64 >= thresholds.linked_share * self.statements.len() as f64
    }

    /// The values make a series (`series_points`).
    pub fn series(&self) -> bool {
        series_points(self.composer, self.statements).is_some()
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
            && self.linked.is_none_or(|b| f.linked() == b)
            && self.series.is_none_or(|b| f.series() == b)
            && self.globe.is_none_or(|g| f.globe() == Some(g))
    }
}

/// A point of a series: a statement and its year.
#[derive(Debug)]
pub struct DatedPoint<'s> {
    pub year: i64,
    pub statement: &'s Statement,
    /// Another value of a year whose preferred value is the point.
    pub other: bool,
    /// A value of a year whose values differ by another qualifier (by
    /// sex), with qualifiers that most values of the property do not
    /// have: not a point of the series.
    pub apart: bool,
}

/// The points of a series: `dated_points` in two years or more (values
/// apart do not count). Two points are a series: the chart shows that the
/// data is scarce and asks for more.
pub fn series_points<'s>(
    c: &Composer,
    statements: &[&'s Statement],
) -> Option<Vec<DatedPoint<'s>>> {
    let points = dated_points(c, statements)?;
    let years: HashSet<i64> = points.iter().filter(|p| !p.apart).map(|p| p.year).collect();
    (years.len() >= 2).then_some(points)
}

/// The statements as points, in their order, when each has a year
/// (`measurement_year`: its point in time, else its start or end time).
///
/// A year with more than one value (from two sources) has one point: the
/// value with preferred rank; else the value with the most references to a
/// real source; else the first in the order of the data. Its other values
/// are marked `other`. Values that differ by another qualifier (by sex, by
/// place) are not one measurement. In such a year, the values with the
/// other qualifiers of more than half of the values are the measurement;
/// the rest are marked `apart`. Without such a majority: no points.
pub fn dated_points<'s>(c: &Composer, statements: &[&'s Statement]) -> Option<Vec<DatedPoint<'s>>> {
    let years: Vec<i64> = statements
        .iter()
        .map(|s| measurement_year(s).filter(|y| *y != 0))
        .collect::<Option<_>>()?;
    let measures: Vec<Vec<&Qualifier>> = statements.iter().map(|s| other_qualifiers(s)).collect();
    // The other qualifiers of more than half of the values.
    let main = measures
        .iter()
        .find(|m| 2 * measures.iter().filter(|n| n == m).count() > measures.len());
    let mut by_year: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, year) in years.iter().enumerate() {
        by_year.entry(*year).or_default().push(i);
    }
    let mut other = vec![false; statements.len()];
    let mut apart = vec![false; statements.len()];
    for same in by_year.values().filter(|same| same.len() > 1) {
        let mut same = same.clone();
        if !same.iter().all(|&i| measures[i] == measures[same[0]]) {
            let main = main?;
            for &i in &same {
                apart[i] = measures[i] != *main;
            }
            same.retain(|&i| !apart[i]);
            if same.len() < 2 {
                continue;
            }
        }
        // The last of the largest keys: the first index wins a tie.
        let point = same.iter().copied().max_by_key(|&i| {
            (
                statements[i].rank == Rank::Preferred,
                c.sources(statements[i]),
                std::cmp::Reverse(i),
            )
        })?;
        for &i in &same {
            other[i] = i != point;
        }
    }
    Some(
        statements
            .iter()
            .zip(years)
            .zip(other.into_iter().zip(apart))
            .map(|((statement, year), (other, apart))| DatedPoint {
                year,
                statement,
                other,
                apart,
            })
            .collect(),
    )
}

/// The qualifiers of a statement other than its dates and the reason for
/// its rank: the qualifiers of what it measures.
pub(crate) fn other_qualifiers(statement: &Statement) -> Vec<&Qualifier> {
    statement
        .qualifiers
        .iter()
        .filter(|q| ![*POINT_IN_TIME, *START_TIME, *END_TIME, *PREFERRED_REASON].contains(&q.pid))
        .collect()
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
    /// The block is a strip of pictures.
    pub strip: bool,
}

impl When<PlacementFacts> for PlacementWhen {
    fn holds(&self, f: &PlacementFacts) -> bool {
        self.make.is_none_or(|m| f.make == Some(m))
            && self.primitive.is_none_or(|p| f.primitive == Some(p))
            && self.audio.is_none_or(|b| f.audio == b)
            && self.strip.is_none_or(|b| f.strip == b)
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

    /// The id of the first property rule, in a spread of `pids` of an item
    /// with `statements`.
    fn property_rule_in(p: &str, statements: Value, pids: &[&str]) -> (&'static str, PropertyMake) {
        let c = content(statements, json!({}));
        let k = Composer::new(&c, Config::embedded());
        let pids: Vec<Pid> = pids.iter().map(|p| pid(p)).collect();
        let rule = first(&rules().property, &PropertyFacts::new(&k, pid(p), &pids));
        (rule.id(), rule.make)
    }

    fn property_rule(p: &str) -> (&'static str, PropertyMake) {
        property_rule_in(p, json!([]), &[])
    }

    #[test]
    fn property_rules() {
        use PropertyMake::*;
        assert_eq!(property_rule("P1313"), ("office-title", Skip));
        // An extreme point is a marker on the map of the spread; without a
        // map of the Earth, a value like any other.
        let on = |globe: &str| json!([{"pid": "P625", "value": {"type": "GlobeCoordinate", "lat": 1.0, "lon": 2.0, "globe": globe}}]);
        assert_eq!(
            property_rule_in("P1332", on("Q2"), &["P625", "P1332"]),
            ("extreme-point", Skip)
        );
        assert_eq!(
            property_rule_in("P1332", on("Q2"), &["P1332"]),
            ("values", Values)
        );
        assert_eq!(
            property_rule_in("P1332", on("Q405"), &["P625", "P1332"]),
            ("values", Values)
        );
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
            group_rule(
                split,
                json!([dated("P1082", 1.0, 2000)]),
                none.clone(),
                false
            ),
            "together"
        );
        // Holidays (the calendar role) stay one list.
        assert_eq!(
            group_rule(split, json!([item("P832", "Q2")]), none, false),
            "calendar-together"
        );
        let former = rules().split.iter().find(|r| r.id == "former-values");
        assert_eq!(former.map(|r| r.make), Some(SplitMake::SplitEnded));
    }

    #[test]
    fn block_rules() {
        let block = &rules().block;
        let none = json!({});
        // Ten pictured relations, each with its embassy: rows. Without the
        // embassies, or fewer of them: a strip of pictures.
        let relation = |i: u32, embassy: bool| {
            let mut v = pictured("P530", &format!("Q{}", 100 + i));
            if embassy {
                v["qualifiers"] = json!([{"pid": "P531",
                    "value": {"type": "entity", "entity": format!("Q{}", 200 + i)}}]);
            }
            v
        };
        let linked: Vec<Value> = (0..10).map(|i| relation(i, true)).collect();
        assert_eq!(
            group_rule(block, json!(linked), none.clone(), false),
            "linked-set"
        );
        let plain: Vec<Value> = (0..10).map(|i| relation(i, i < 3)).collect();
        assert_eq!(group_rule(block, json!(plain), none.clone(), false), "set");
        let few: Vec<Value> = (0..9).map(|i| relation(i, true)).collect();
        assert_eq!(group_rule(block, json!(few), none.clone(), false), "set");
        let media = json!([{"pid": "P18", "value": {"type": "CommonsMedia", "file": "A.jpg"}}]);
        assert_eq!(
            group_rule(block, media.clone(), none.clone(), false),
            "media"
        );
        // Files that ended (the flags before): earlier.
        assert_eq!(group_rule(block, media, none.clone(), true), "earlier");
        let point = json!([{"pid": "P625", "value": {"type": "GlobeCoordinate", "lat": 1.0, "lon": 2.0, "globe": "Q2"}}]);
        assert_eq!(group_rule(block, point, none.clone(), false), "coordinate");
        // On the Moon: no Earth map, a row.
        let moon = json!([{"pid": "P625", "value": {"type": "GlobeCoordinate", "lat": 0.0, "lon": 0.0, "globe": "Q405"}}]);
        assert_eq!(group_rule(block, moon, none.clone(), false), "row");
        // An extreme point that reaches the block rules has no map: a row.
        let pole = json!([{"pid": "P1332", "value": {"type": "GlobeCoordinate", "lat": 90.0, "lon": 0.0, "globe": "Q2"}}]);
        assert_eq!(
            group_rule(block, pole, none.clone(), false),
            "extreme-point-row"
        );
        let series = json!([
            dated("P1082", 1.0, 2000),
            dated("P1082", 2.0, 2010),
            dated("P1082", 3.0, 2020)
        ]);
        assert_eq!(group_rule(block, series, none.clone(), false), "series");
        // Two values in one year (two sources): still a series; one of them
        // is the point, the other an "other value" (`dated_points`).
        let colliding = json!([
            dated("P1082", 1.0, 2000),
            dated("P1082", 2.0, 2000),
            dated("P1082", 3.0, 2020)
        ]);
        assert_eq!(group_rule(block, colliding, none.clone(), false), "series");
        // Any dated quantity is a series, from two points.
        let rate = json!([dated("P8763", 17.0, 2019), dated("P8763", 16.4, 2021)]);
        assert_eq!(group_rule(block, rate, none.clone(), false), "series");
        assert_eq!(
            group_rule(
                block,
                json!([dated("P8763", 17.0, 2019)]),
                none.clone(),
                false
            ),
            "row"
        );
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
            "significance"
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
                strip: false,
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
        let strip = first(
            &rules().placement,
            &PlacementFacts {
                make: None,
                primitive: Some(Network),
                audio: false,
                strip: true,
            },
        );
        assert_eq!((strip.id(), strip.slot), ("strip", Slot::Strips));
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
        let k = Composer::new(&c, Config::embedded());
        let years = |sts: &[&Statement]| {
            series_points(&k, sts)
                .map(|ps| ps.iter().map(|p| (p.year, p.other)).collect::<Vec<_>>())
        };
        assert_eq!(
            years(&sts),
            Some(vec![(2010, false), (2000, false), (2020, false)])
        );
        assert_eq!(years(&sts[..2]), Some(vec![(2010, false), (2000, false)]));
        assert_eq!(years(&sts[..1]), None, "one point");
    }

    #[test]
    fn series_with_two_values_in_a_year() {
        let preferred = |mut v: Value| {
            v["rank"] = json!("preferred");
            v
        };
        let sex = |mut v: Value, q: &str| {
            v["qualifiers"]
                .as_array_mut()
                .unwrap()
                .push(json!({"pid": "P21", "value": {"type": "entity", "entity": q}}));
            v
        };
        let years_with = |values: Value, extra: Value| {
            let c = content(values, extra);
            let k = Composer::new(&c, Config::embedded());
            let sts: Vec<_> = c.statements.iter().collect();
            series_points(&k, &sts)
                .map(|ps| ps.iter().map(|p| (p.year, p.other)).collect::<Vec<_>>())
        };
        let years = |values: Value| years_with(values, json!({}));
        // Two sources for 2021: the preferred value is the point.
        assert_eq!(
            years(json!([
                dated("P2250", 66.0, 2019),
                preferred(dated("P2250", 70.03, 2021)),
                dated("P2250", 67.0, 2021)
            ])),
            Some(vec![(2019, false), (2021, false), (2021, true)])
        );
        // No preferred value: the value with a real source is the point.
        assert_eq!(
            years_with(
                json!([
                    dated("P2250", 66.0, 2019),
                    dated("P2250", 70.03, 2021),
                    dated("P2250", 67.0, 2021)
                ]),
                json!({"references": {"Q1-0000-0002": [["P248"]]}})
            ),
            Some(vec![(2019, false), (2021, true), (2021, false)])
        );
        // An equal tie: the first in the order of the data.
        assert_eq!(
            years(json!([
                dated("P2250", 66.0, 2019),
                dated("P2250", 70.03, 2021),
                dated("P2250", 67.0, 2021)
            ])),
            Some(vec![(2019, false), (2021, false), (2021, true)])
        );
        // Dated by a start time, not a point in time (a minimum wage, a
        // Gini coefficient of a survey period): the start year.
        let from = |p: &str, amount: f64, year: u32| {
            json!({"pid": p, "value": {"type": "Quantity", "amount": amount},
                   "qualifiers": [{"pid": "P580", "value": {"type": "time",
                                   "time": format!("{year}-01-01T00:00:00Z")}}]})
        };
        assert_eq!(
            years(json!([
                dated("P1125", 28.0, 1998),
                from("P1125", 29.9, 2001),
                dated("P1125", 30.2, 2004)
            ])),
            Some(vec![(1998, false), (2001, false), (2004, false)])
        );
        // Values by sex are two measurements, not two sources. Without a
        // measurement of most values: no series.
        assert_eq!(
            years(json!([
                dated("P1082", 1.0, 2011),
                preferred(sex(dated("P1082", 2.0, 2021), "Q6581072")),
                sex(dated("P1082", 3.0, 2021), "Q6581097")
            ])),
            None
        );
        // Germany's life expectancy: one year by sex among years without
        // it. The values of that year are apart; the series goes on. The
        // reason for a preferred rank is not a measurement.
        let apart = |values: Value| {
            let c = content(values, json!({}));
            let k = Composer::new(&c, Config::embedded());
            let sts: Vec<_> = c.statements.iter().collect();
            series_points(&k, &sts)
                .map(|ps| ps.iter().map(|p| (p.year, p.apart)).collect::<Vec<_>>())
        };
        let mut recent = preferred(dated("P2250", 81.9, 2024));
        recent["qualifiers"]
            .as_array_mut()
            .unwrap()
            .push(json!({"pid": "P7452", "value": {"type": "entity", "entity": "Q71533355"}}));
        assert_eq!(
            apart(json!([
                dated("P2250", 80.6, 2016),
                sex(dated("P2250", 83.3, 2017), "Q6581072"),
                sex(dated("P2250", 78.5, 2017), "Q6581097"),
                dated("P2250", 81.0, 2019),
                recent
            ])),
            Some(vec![
                (2016, false),
                (2017, true),
                (2017, true),
                (2019, false),
                (2024, false)
            ])
        );
    }
}
