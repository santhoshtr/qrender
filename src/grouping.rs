use crate::model::{Property, WikidataItem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupingConfig {
    pub groups: HashMap<String, GroupConfig>, // Group Name -> Group Config
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupConfig {
    pub pids: Vec<String>,
    pub renderer: Option<String>, // Optional renderer name
    pub order: Option<i32>,
}

impl GroupingConfig {
    pub fn sorted_groups(&self) -> Vec<(&String, &GroupConfig)> {
        let mut groups_vec: Vec<(&String, &GroupConfig)> = self.groups.iter().collect();
        groups_vec.sort_by_key(|(_, config)| config.order);
        // Reverse to get descending order
        groups_vec.reverse();
        groups_vec
    }
}

pub fn load_grouping_config() -> Result<GroupingConfig, Box<dyn std::error::Error>> {
    // Read the TOML file content
    let toml_content = include_str!("../groups.toml");
    // Parse the TOML string into our Config struct
    let config: GroupingConfig = toml::from_str(toml_content)?;
    Ok(config)
}

pub fn group_properties(
    item: &WikidataItem,
    config: &GroupingConfig,
) -> Vec<(String, Vec<Property>)> {
    let sorted_groups = config.sorted_groups();
    let mut grouped_properties: Vec<(String, Vec<Property>)> = Vec::new();

    for (group_name, group_config) in sorted_groups {
        let mut properties_in_group: Vec<Property> = Vec::new();
        for pid in &group_config.pids {
            if let Some(property) = item.properties.get(pid) {
                properties_in_group.push(property.clone());
            }
        }
        grouped_properties.push((group_name.clone(), properties_in_group));
    }

    // For the properties that does not belong to any group, add them to a default group
    let default_group_name = "default".to_string();
    let mut default_group_properties: Vec<Property> = Vec::new();
    for (pid, property) in &item.properties {
        if !config.groups.values().any(|g| g.pids.contains(pid)) {
            default_group_properties.push(property.clone());
        }
    }
    if !default_group_properties.is_empty() {
        grouped_properties.push((default_group_name, default_group_properties));
    }

    grouped_properties
}
