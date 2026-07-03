use crate::error::QRenderError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupingConfig {
    pub groups: HashMap<String, GroupConfig>, // Group Name -> Group Config
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupConfig {
    pub pids: Vec<String>,
    pub order: Option<i32>,
}

impl GroupingConfig {
    pub fn sorted_groups(&self) -> Vec<(&String, &GroupConfig)> {
        let mut groups_vec: Vec<(&String, &GroupConfig)> = self.groups.iter().collect();
        // Groups with an explicit order come first (ascending); the rest
        // follow alphabetically so output is deterministic across runs.
        groups_vec.sort_by_key(|(name, config)| {
            (config.order.is_none(), config.order, name.as_str())
        });
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
