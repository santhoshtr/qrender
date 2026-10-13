//! Data loading via the qjson crate: in-process SPARQL fetch with an
//! optional Redis cache.

use crate::error::QRenderError;

pub async fn fetch_typed(qid: &str, language: &str) -> Result<qjson::WikidataItem, QRenderError> {
    Ok(qjson::Client::new().get_item(qid, language).await?)
}
