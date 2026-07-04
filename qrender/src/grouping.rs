use crate::error::QRenderError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupingConfig {
    pub groups: HashMap<String, GroupConfig>, // Group Name -> Group Config
    #[serde(default)]
    pub properties: HashMap<String, PropertyConfig>, // PID -> Property Config
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupConfig {
    pub pids: Vec<String>,
    pub order: Option<i32>,
    /// Symbol name from assets/icons/, shown on cards from this group
    pub icon: Option<String>,
    /// Bento grid preferences for cards from this group
    pub cols: Option<u8>,
    pub rows: Option<u8>,
    pub sort: Option<i32>,
    /// Wikimedia-curation meta (categories, templates); cards render
    /// in the collapsed footnote region instead of the main grid
    #[serde(default)]
    pub footnote: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PropertyConfig {
    /// Symbol name from assets/icons/; overrides the group icon
    pub icon: Option<String>,
    /// Never render this property (structural/meta properties like P31)
    #[serde(default)]
    pub ignore: bool,
    /// Bento grid preferences; override the group's
    pub cols: Option<u8>,
    pub rows: Option<u8>,
    pub sort: Option<i32>,
    /// Wikimedia-curation meta; see GroupConfig::footnote
    #[serde(default)]
    pub footnote: bool,
    /// Render this quantity as a gauge (HTML meter element). Not
    /// derivable from data: config declares the scale and what is good.
    pub meter: Option<MeterConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct MeterConfig {
    pub min: f64,
    pub max: f64,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub optimum: Option<f64>,
}

impl GroupingConfig {
    pub fn is_ignored(&self, pid: &str) -> bool {
        self.properties.get(pid).is_some_and(|p| p.ignore)
    }

    pub fn sorted_groups(&self) -> Vec<(&String, &GroupConfig)> {
        let mut groups_vec: Vec<(&String, &GroupConfig)> = self.groups.iter().collect();
        // Groups with an explicit order come first (ascending); the rest
        // follow alphabetically so output is deterministic across runs.
        groups_vec
            .sort_by_key(|(name, config)| (config.order.is_none(), config.order, name.as_str()));
        groups_vec
    }
}

pub fn load_grouping_config() -> Result<GroupingConfig, QRenderError> {
    // Read the TOML file content
    let toml_content = include_str!("../groups.toml");
    // Parse the TOML string into our Config struct
    let config: GroupingConfig = toml::from_str(toml_content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_icons_exist() {
        let config = load_grouping_config().unwrap();
        let group_icons = config.groups.values().filter_map(|g| g.icon.as_deref());
        let property_icons = config.properties.values().filter_map(|p| p.icon.as_deref());
        for icon in group_icons.chain(property_icons) {
            assert!(
                crate::icons::lookup(icon).is_some(),
                "unknown icon in groups.toml: {icon}"
            );
        }
    }
}
