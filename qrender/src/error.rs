use thiserror::Error;

#[derive(Error, Debug)]
pub enum QRenderError {
    #[error("failed to fetch Wikidata item: {0}")]
    Fetch(#[from] qjson::QjsonError),
    #[error("invalid grouping config: {0}")]
    Config(#[from] toml::de::Error),
    #[error("factoid template error: {0}")]
    Factoid(#[from] askama::Error),
}
