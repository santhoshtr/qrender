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
