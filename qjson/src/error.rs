use thiserror::Error;

#[derive(Error, Debug)]
pub enum QjsonError {
    #[error("invalid QID: {0}")]
    InvalidQid(String),
    #[error("invalid language code: {0}")]
    InvalidLanguage(String),
    #[error("WDQS request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("WDQS returned HTTP {0}")]
    WdqsStatus(u16),
    #[error("failed to decode SPARQL response: {0}")]
    Decode(#[from] serde_json::Error),
}
