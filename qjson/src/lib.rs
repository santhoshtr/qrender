//! Fetch a Wikidata item's statements via SPARQL and expose them as a
//! typed model. Rust port of the qjson Go tool (qjson.toolforge.org).

pub mod error;
pub mod model;

pub use error::QjsonError;
pub use model::{Property, Qualifier, Rank, Statement, Value, WikidataItem};
