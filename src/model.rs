use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StatementType {
    Literal,
    Uri,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Statement {
    #[serde(rename = "type")]
    pub statement_type: StatementType, // Use a more descriptive name
    pub datatype: String,
    pub value: String,
    pub rank: String,
    pub unit_of_measure: Option<String>,
    pub unit_of_measure_label: Option<String>,
    pub qualifiers: Option<Vec<Qualifier>>,
    pub qid: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    pub pid: String,
    pub wd_label: String,
    pub statements: Vec<Statement>,
}

pub type WikidataProperties = HashMap<String, Property>; // QID -> Property

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WikidataItem {
    pub properties: WikidataProperties,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Qualifier {
    pub datatype: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub pid: String,
    pub value: String,
    pub label: String,
}
