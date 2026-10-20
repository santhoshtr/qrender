//! The configuration: `layouts.toml`, `rules.toml` and `citations.toml`,
//! embedded at build time, parsed and checked once.

pub mod layouts;
pub mod rules;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use content::{CitationStyle, Pid};
use serde::Deserialize;

pub use layouts::{ClassLayout, Layouts, SplitSpec, SpreadSpec, Thresholds};
pub use rules::Rules;

use crate::vocab::{Role, RoleSet, SpreadKind};
use rules::Row;

const LAYOUTS: &str = include_str!("../../layouts.toml");

/// The parts of a split at most: the categorical colours of the design
/// system (`--part-1` to `--part-6`).
pub const PARTS_MAX: usize = 6;
const RULES: &str = include_str!("../../rules.toml");
const CITATIONS: &str = include_str!("../../citations.toml");

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{file}: {source}")]
    Toml {
        file: &'static str,
        source: toml::de::Error,
    },
    #[error("invalid configuration:\n- {}", .0.join("\n- "))]
    Invalid(Vec<String>),
}

#[derive(Debug)]
pub struct Config {
    pub layouts: Layouts,
    pub rules: Rules,
    /// The citation templates, for the fetcher.
    pub citations: Vec<CitationStyle>,
    roles: HashMap<Pid, RoleSet>,
    hash: u64,
}

impl Config {
    /// The embedded configuration. A test checks that it loads, so this
    /// cannot fail in a build that passed its tests.
    pub fn embedded() -> &'static Config {
        static CONFIG: LazyLock<Config> = LazyLock::new(|| {
            Config::parse(LAYOUTS, RULES, CITATIONS)
                .unwrap_or_else(|e| panic!("embedded configuration: {e}"))
        });
        &CONFIG
    }

    pub fn parse(layouts: &str, rules: &str, citations: &str) -> Result<Config, ConfigError> {
        let toml_error = |file| move |source| ConfigError::Toml { file, source };
        let layouts: Layouts = toml::from_str(layouts).map_err(toml_error("layouts.toml"))?;
        let rules: Rules = toml::from_str(rules).map_err(toml_error("rules.toml"))?;
        let citations: Citations =
            toml::from_str(citations).map_err(toml_error("citations.toml"))?;
        let errors = validate(&layouts, &rules);
        if !errors.is_empty() {
            return Err(ConfigError::Invalid(errors));
        }
        Ok(Config {
            roles: role_index(&layouts),
            hash: fnv1a(&[LAYOUTS, RULES]),
            layouts,
            rules,
            citations: citations.style,
        })
    }

    /// What the fetcher needs from the layouts: layout matching, reverse
    /// properties, container properties, qualifier logos, concepts, web
    /// properties and their platforms; and the citation templates.
    pub fn fetch_settings(&self) -> content::Settings {
        let layouts = &self.layouts;
        let mut concepts: Vec<content::Qid> = layouts
            .class_layouts
            .values()
            .flat_map(|l| &l.spreads)
            .flat_map(|s| s.concept.iter().chain(&s.prose))
            .chain(layouts.splits.iter().map(|s| &s.concept))
            .copied()
            .collect();
        concepts.sort();
        concepts.dedup();
        content::Settings {
            layouts: layouts
                .class_layouts
                .iter()
                .map(|(name, l)| content::LayoutMatch {
                    name: name.clone(),
                    match_p31: l.match_p31.clone(),
                    reverse: l
                        .spreads
                        .iter()
                        .flat_map(|s| s.reverse.iter().copied())
                        .collect(),
                })
                .collect(),
            containers: layouts.containers.keys().copied().collect(),
            qualifier_logos: layouts.media.qualifier_logos.clone(),
            concepts,
            timeline: layouts.roles.list(Role::Timeline).to_vec(),
            calendar: layouts.roles.list(Role::Calendar).to_vec(),
            web: layouts.roles.list(Role::Web).to_vec(),
            platforms: layouts
                .web
                .platforms
                .iter()
                .map(|(p, q)| (*p, *q))
                .collect(),
            texts: layouts.roles.text.iter().map(|(i, t)| (*i, *t)).collect(),
            citations: self.citations.clone(),
        }
    }

    /// The roles of a property.
    pub fn roles(&self, pid: Pid) -> RoleSet {
        self.roles.get(&pid).copied().unwrap_or_default()
    }

    /// A hash of both files. It is part of the cache key of a composition
    /// and a view, so a change of the configuration invalidates them.
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

/// `citations.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Citations {
    style: Vec<CitationStyle>,
}

fn role_index(layouts: &Layouts) -> HashMap<Pid, RoleSet> {
    let mut index: HashMap<Pid, RoleSet> = HashMap::new();
    for (role, pids) in &layouts.roles.lists {
        for pid in pids {
            index.entry(*pid).or_default().insert(*role);
        }
    }
    for pid in layouts.roles.office.keys() {
        index.entry(*pid).or_default().insert(Role::Office);
    }
    for pid in layouts.roles.tabular.keys() {
        index.entry(*pid).or_default().insert(Role::Tabular);
    }
    index
}

fn validate(layouts: &Layouts, rules: &Rules) -> Vec<String> {
    let mut errors = Vec::new();
    table("property", &rules.property, &mut errors);
    table("split", &rules.split, &mut errors);
    table("block", &rules.block, &mut errors);
    table("order", &rules.order, &mut errors);
    table("placement", &rules.placement, &mut errors);
    table("template", &rules.template, &mut errors);
    if rules.lead.primitives.is_empty() {
        errors.push("lead: no primitives".into());
    }

    for table in [Role::Office, Role::Tabular] {
        if layouts.roles.lists.contains_key(&table) {
            errors.push(format!("roles: {table:?} is a table, not a list"));
        }
    }
    match layouts.class_layouts.get("generic") {
        None => errors.push("class_layouts: no generic layout".into()),
        Some(generic) if !generic.match_p31.is_empty() => {
            errors.push("class_layouts.generic: match_p31 must be empty".into())
        }
        Some(_) => {}
    }
    let mut spread_ids = HashSet::new();
    for (name, layout) in &layouts.class_layouts {
        class_layout(name, layout, &mut errors);
        spread_ids.extend(layout.spreads.iter().map(|s| s.id.as_str()));
    }
    for (pid, spread) in &layouts.topics {
        if !spread_ids.contains(spread.as_str()) {
            errors.push(format!(
                "topics: {pid} names spread {spread:?}, which no layout has"
            ));
        }
    }
    splits(&layouts.splits, &mut errors);
    for (pid, index) in &layouts.indexes {
        match (index.min, index.max) {
            (None, None) => {}
            (Some(min), Some(max)) if min < max => {}
            _ => errors.push(format!(
                "indexes.{pid}: give min and max, with min below max, or none"
            )),
        }
    }
    let t = &layouts.thresholds;
    for (name, share) in [("run_pictured", t.run_pictured), ("absence", t.absence)] {
        if !(0.0..=1.0).contains(&share) {
            errors.push(format!(
                "thresholds.{name}: {share} is not a share (0 to 1)"
            ));
        }
    }
    errors
}

/// A table has rules, unique ids, and a last rule without a condition.
fn table<R: Row>(name: &str, rows: &[R], errors: &mut Vec<String>) {
    let mut ids = HashSet::new();
    for row in rows {
        if !ids.insert(row.id()) {
            errors.push(format!("{name}: rule id {:?} is not unique", row.id()));
        }
    }
    match rows.last() {
        None => errors.push(format!("{name}: no rules")),
        Some(last) if !last.unconditional() => errors.push(format!(
            "{name}: the last rule ({:?}) must have no condition",
            last.id()
        )),
        Some(_) => {}
    }
}

/// A split has 2 to `PARTS_MAX` parts (the categorical colours of the
/// design system); a property is a part of one split only; ids are unique.
fn splits(splits: &[SplitSpec], errors: &mut Vec<String>) {
    let mut ids = HashSet::new();
    let mut parts = HashSet::new();
    for split in splits {
        if !ids.insert(split.id.as_str()) {
            errors.push(format!("splits: id {:?} twice", split.id));
        }
        if !(2..=PARTS_MAX).contains(&split.parts.len()) {
            errors.push(format!(
                "splits.{}: {} parts; a split has 2 to {PARTS_MAX}",
                split.id,
                split.parts.len()
            ));
        }
        for pid in &split.parts {
            if !parts.insert(*pid) {
                errors.push(format!(
                    "splits.{}: {pid} is a part of another split",
                    split.id
                ));
            }
        }
    }
}

/// The identity spread comes first; spread ids are unique; "more" and
/// "evidence" are there at most once.
fn class_layout(name: &str, layout: &ClassLayout, errors: &mut Vec<String>) {
    if layout.spreads.first().map(|s| s.kind) != Some(SpreadKind::Identity) {
        errors.push(format!(
            "class_layouts.{name}: the first spread must be identity"
        ));
    }
    let mut ids = HashSet::new();
    for spread in &layout.spreads {
        if !ids.insert(spread.id.as_str()) {
            errors.push(format!(
                "class_layouts.{name}: spread id {:?} twice",
                spread.id
            ));
        }
    }
    for kind in [SpreadKind::Identity, SpreadKind::More, SpreadKind::Evidence] {
        if layout.spreads.iter().filter(|s| s.kind == kind).count() > 1 {
            errors.push(format!(
                "class_layouts.{name}: more than one {kind:?} spread"
            ));
        }
    }
}

/// FNV-1a, 64 bit: a hash that stays the same across Rust releases, so a
/// cache key made by one build is valid for the next.
fn fnv1a(parts: &[&str]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in parts.iter().flat_map(|p| p.bytes().chain([0])) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::{SpreadKind, Template};

    #[test]
    fn embedded_config_loads() {
        let config = Config::embedded();
        let layouts = &config.layouts;
        assert_eq!(
            layouts.class_layouts.keys().collect::<Vec<_>>(),
            [
                "settlement",
                "person",
                "film",
                "astronomical",
                "country",
                "generic"
            ]
        );
        let settlement = &layouts.class_layouts["settlement"];
        assert_eq!(settlement.spreads[0].kind, SpreadKind::Identity);
        assert_eq!(
            settlement.spreads.last().unwrap().kind,
            SpreadKind::Evidence
        );
        assert_eq!(layouts.thresholds.set_max, 12);
        assert_eq!(config.rules.block.last().unwrap().id, "row");
        assert_eq!(
            config.rules.template.last().unwrap().template,
            Template::Columns
        );
        assert_eq!(config.citations[0].name, "CS1");
        assert_eq!(config.citations[0].templates.web, ["cite web"]);
    }

    #[test]
    fn roles_by_property() {
        let config = Config::embedded();
        let pid = |s: &str| s.parse::<Pid>().unwrap();
        assert!(config.roles(pid("P6")).has(Role::Office));
        assert!(config.roles(pid("P26")).has(Role::Relation));
        assert!(!config.roles(pid("P31")).has(Role::Relation));
        assert_eq!(
            config.layouts.roles.office[&pid("P6")],
            pid("P1313"),
            "head of government -> office held by head of government"
        );
        assert!(config.layouts.roles.list(Role::ExtremePoint).len() == 5);
        assert!(config.roles(pid("P4179")).has(Role::Tabular));
        assert_eq!(config.layouts.roles.tabular[&pid("P4179")], pid("P1082"));
    }

    fn invalid(layouts: &str, rules: &str) -> String {
        Config::parse(layouts, rules, CITATIONS)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn unknown_names_are_errors() {
        let bad_rule = RULES.replacen("make = \"fact\"", "make = \"facts\"", 1);
        assert!(invalid(LAYOUTS, &bad_rule).contains("rules.toml"));
        let bad_predicate = RULES.replacen("few = true", "fewer = true", 1);
        assert!(invalid(LAYOUTS, &bad_predicate).contains("rules.toml"));
        let bad_role = LAYOUTS.replacen("timeline = [", "timelines = [", 1);
        assert!(invalid(&bad_role, RULES).contains("layouts.toml"));
        let bad_pid = LAYOUTS.replacen("\"P1082\",", "\"1082\",", 1);
        assert!(invalid(&bad_pid, RULES).contains("layouts.toml"));
        let bad_kind = CITATIONS.replacen("web = [\"cite web\"]", "webs = [\"cite web\"]", 1);
        let error = Config::parse(LAYOUTS, RULES, &bad_kind).unwrap_err();
        assert!(error.to_string().contains("citations.toml"));
    }

    #[test]
    fn tables_end_with_an_unconditional_rule() {
        let rules = RULES.replace(
            "id = \"plain\"\ntemplate = \"columns\"",
            "id = \"plain\"\nwhen = { lead = \"map\" }\ntemplate = \"columns\"",
        );
        assert!(invalid(LAYOUTS, &rules).contains("template: the last rule (\"plain\")"));
        let rules = RULES.replace("id = \"map\"\nwhen", "id = \"office\"\nwhen");
        assert!(invalid(LAYOUTS, &rules).contains("\"office\" is not unique"));
    }

    #[test]
    fn layouts_are_checked() {
        let layouts = LAYOUTS.replacen("P2184 = \"history\"", "P2184 = \"histories\"", 1);
        assert!(invalid(&layouts, RULES).contains("\"histories\", which no layout has"));
        // A part of two splits; a split of one part.
        let layouts = LAYOUTS.replacen(
            "\"P6344\",  # rural population\n]",
            "\"P1539\",  # rural population\n]",
            1,
        );
        assert!(invalid(&layouts, RULES).contains("P1539 is a part of another split"));
        let layouts = LAYOUTS.replacen("  \"P1540\",  # male population\n]", "]", 1);
        assert!(invalid(&layouts, RULES).contains("splits.sex: 1 parts"));
        // An index range with one end.
        let layouts = LAYOUTS.replacen("{ min = 0, max = 100 }", "{ min = 0 }", 1);
        assert!(invalid(&layouts, RULES).contains("indexes.P11547: give min and max"));
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(fnv1a(&["a"]), fnv1a(&["a"]));
        assert_ne!(fnv1a(&["ab", "c"]), fnv1a(&["a", "bc"]));
        assert_eq!(fnv1a(&[]), 0xcbf2_9ce4_8422_2325);
    }
}
