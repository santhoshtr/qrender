//! Fetch a Wikidata item's statements via SPARQL and expose them as a
//! typed model. Rust port of the qjson Go tool (qjson.toolforge.org).

pub mod cache;
pub mod error;
pub mod model;
pub mod sparql;
pub mod transform;

pub use error::QjsonError;
pub use model::{Property, Qualifier, Rank, Statement, Value, WikidataItem};

pub struct Client {
    cache: cache::Cache,
}

impl Client {
    /// Caching is configured from REDIS_URL; without it every call
    /// queries WDQS directly.
    pub fn new() -> Client {
        Client {
            cache: cache::Cache::from_env(),
        }
    }

    pub async fn get_item(&self, qid: &str, language: &str) -> Result<WikidataItem, QjsonError> {
        if let Some(item) = self.cache.get(qid, language).await {
            return Ok(item);
        }
        let bindings = sparql::fetch_bindings(qid, language).await?;
        let item = transform::transform(qid, &bindings);
        self.cache.set(qid, language, &item).await;
        Ok(item)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
