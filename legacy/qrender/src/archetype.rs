//! Archetype resolution: which composition recipe a page gets, from
//! archetypes.toml. Curated P31 (instance of) QIDs select directly;
//! otherwise signal-property presence votes, so items whose P31 the
//! curated list misses still land in the right recipe. Misresolution
//! only affects presentation - generic is always a safe answer.

use qjson::WikidataItem;
use serde::Deserialize;
use std::collections::HashMap;

use crate::error::QRenderError;

pub const GENERIC: &str = "generic";

/// Minimum signal-property hits for a shape-scored match; below this
/// the page stays generic.
const SIGNAL_THRESHOLD: usize = 4;

#[derive(Debug, Deserialize)]
pub struct ArchetypesConfig {
    pub archetypes: HashMap<String, ArchetypeConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ArchetypeConfig {
    /// P31 values that select this archetype directly
    #[serde(default)]
    pub p31: Vec<String>,
    /// Properties whose presence votes for this archetype
    #[serde(default)]
    pub signals: Vec<String>,
    /// Facts pulled into the page header
    pub hero: Option<HeroConfig>,
    /// Composed regions, in reading order; a card is claimed by the
    /// first section whose pids intersect its sources. Sections whose
    /// data is absent collapse; unclaimed cards stay in the overflow
    /// grid, so a sparse item degrades to the generic page.
    #[serde(default)]
    pub sections: Vec<SectionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct HeroConfig {
    /// [start, end] time properties formatted as a range under the
    /// title (person: birth/death)
    #[serde(default)]
    pub dates: Vec<String>,
    /// Item property whose value labels join into a tagline
    pub tagline: Option<String>,
    /// Single-image property shown inside the header (person: P109
    /// signature); its statement is consumed like the P18 hero
    pub emblem: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SectionConfig {
    pub name: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub pids: Vec<String>,
    /// Synthesize a cross-property timeline card at the head of this
    /// section from dated statements of these properties
    pub timeline: Option<TimelineConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TimelineConfig {
    pub pids: Vec<String>,
    /// Below this many events the timeline does not render at all -
    /// no half-empty showpieces
    pub min_events: usize,
}

pub fn load_archetypes_config() -> Result<ArchetypesConfig, QRenderError> {
    Ok(toml::from_str(include_str!("../archetypes.toml"))?)
}

/// P31 match wins; else the highest signal score at or above the
/// threshold; else generic. Archetypes are visited in name order so
/// resolution is deterministic.
pub fn resolve(item: &WikidataItem, config: &ArchetypesConfig) -> String {
    let instance_of: Vec<&str> = item
        .properties
        .get("P31")
        .map(|p| {
            p.statements
                .iter()
                .filter_map(|s| match &s.value {
                    qjson::Value::ItemRef { qid, .. } => Some(qid.as_str()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    let mut names: Vec<&String> = config.archetypes.keys().collect();
    names.sort();

    for name in &names {
        let p31 = &config.archetypes[name.as_str()].p31;
        if instance_of.iter().any(|qid| p31.iter().any(|p| p == qid)) {
            return (*name).clone();
        }
    }

    let mut best: Option<(&str, usize)> = None;
    for name in &names {
        let hits = config.archetypes[name.as_str()]
            .signals
            .iter()
            .filter(|pid| item.properties.contains_key(pid.as_str()))
            .count();
        if hits >= SIGNAL_THRESHOLD && best.is_none_or(|(_, b)| hits > b) {
            best = Some((name, hits));
        }
    }
    best.map_or_else(|| GENERIC.to_string(), |(name, _)| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(qid: &str, fixture: &str) -> WikidataItem {
        let response: qjson::sparql::SparqlResponse = serde_json::from_str(fixture).unwrap();
        qjson::transform::transform(qid, &response.results.bindings)
    }

    #[test]
    fn q42_resolves_person_via_p31() {
        let item = item(
            "Q42",
            include_str!("../../qjson/tests/fixtures/Q42.sparql.json"),
        );
        assert_eq!(resolve(&item, &load_archetypes_config().unwrap()), "person");
    }

    #[test]
    fn q3870_resolves_place_via_p31() {
        let item = item(
            "Q3870",
            include_str!("../../qjson/tests/fixtures/Q3870.sparql.json"),
        );
        // Nairobi's P31 is "big city" Q1549591
        assert_eq!(resolve(&item, &load_archetypes_config().unwrap()), "place");
    }

    #[test]
    fn signals_back_up_a_missing_p31() {
        let mut item = item(
            "Q3870",
            include_str!("../../qjson/tests/fixtures/Q3870.sparql.json"),
        );
        item.properties.remove("P31");
        assert_eq!(resolve(&item, &load_archetypes_config().unwrap()), "place");
    }

    #[test]
    fn sparse_item_stays_generic() {
        let item = WikidataItem {
            qid: "Q1".to_string(),
            label: Some("Testland".to_string()),
            description: None,
            properties: HashMap::new(),
        };
        assert_eq!(resolve(&item, &load_archetypes_config().unwrap()), GENERIC);
    }
}
