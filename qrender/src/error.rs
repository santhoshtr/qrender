use thiserror::Error;

#[derive(Error, Debug)]
pub enum QRenderError {
    #[error("failed to fetch Wikidata item: {0}")]
    Fetch(#[from] qjson::QjsonError),
    #[error("failed to parse response JSON: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("invalid grouping config: {0}")]
    Config(#[from] toml::de::Error),
    #[error("template error: {0}")]
    Template(#[from] Box<handlebars::TemplateError>),
    #[error("render error: {0}")]
    Render(#[from] handlebars::RenderError),
    #[error("factoid template error: {0}")]
    Factoid(#[from] askama::Error),
    #[error("unknown renderer: {0}")]
    UnknownRenderer(String),
    #[error("format not supported by this render path: {0}")]
    UnsupportedFormat(String),
}
