//! Typed model of a Wikidata item, built from SPARQL query results.
//!
//! Every value keeps the localized display string the Wikidata label
//! service produced (via `display()`); the typed payload is additional
//! signal for renderers that need numbers, dates or coordinates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WikidataItem {
    pub qid: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub properties: HashMap<String, Property>, // PID -> Property
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    pub pid: String,
    pub label: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Statement {
    pub value: Value,
    pub rank: Rank,
    pub qualifiers: Vec<Qualifier>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Qualifier {
    pub pid: String,
    pub label: String,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rank {
    Preferred,
    #[default]
    Normal,
    Deprecated,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Value {
    Text {
        text: String,
    },
    MonolingualText {
        text: String,
        language: String,
    },
    ItemRef {
        qid: String,
        label: String,
        /// P18 image of the referenced item, when available
        image_url: Option<String>,
    },
    Time {
        /// ISO 8601 timestamp as stored by Wikidata, e.g. "1899-01-01T00:00:00Z"
        iso: String,
        /// Wikidata precision (9 = year, 10 = month, 11 = day, ...)
        precision: Option<u8>,
    },
    Quantity {
        amount: f64,
        /// Original decimal literal, kept for display fidelity
        raw: String,
        unit_qid: Option<String>,
        unit_label: Option<String>,
    },
    Coordinate {
        lat: f64,
        lon: f64,
        /// Original WKT literal, e.g. "Point(36.817222222 -1.286388888)"
        raw: String,
    },
    CommonsMedia {
        /// Commons file name, e.g. "Nairobi banner.jpg"
        file_name: String,
        /// Special:FilePath URL as returned by SPARQL
        url: String,
    },
    Url {
        url: String,
    },
    ExternalId {
        id: String,
    },
}

impl Value {
    /// The human-readable string for this value — what the Wikidata label
    /// service resolved, or the literal itself. This is what text-oriented
    /// renderers print.
    pub fn display(&self) -> &str {
        match self {
            Value::Text { text } => text,
            Value::MonolingualText { text, .. } => text,
            Value::ItemRef { label, .. } => label,
            Value::Time { iso, .. } => iso,
            Value::Quantity { raw, .. } => raw,
            Value::Coordinate { raw, .. } => raw,
            Value::CommonsMedia { url, .. } => url,
            Value::Url { url } => url,
            Value::ExternalId { id } => id,
        }
    }
}
