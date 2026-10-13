//! What the command line and the server share: the fetcher, built from the
//! environment, and the steps from an item to a view.
//!
//! Only fetched parts are cached (content, text, peer sets). The
//! composition and the view are derived from them in milliseconds.

use content::cache::Cache;
use content::fetch::{FetchError, Fetcher, Http, Mode};
use content::{Content, Lang, Qid, Text};
use qrender::config::Config;
use qrender::localize::View;
use qrender::schema::Composition;

/// The Wikimedia User-Agent policy asks for a way to reach the operator.
pub const DEFAULT_USER_AGENT: &str = "qrender/0.1 (https://qrender.toolforge.org)";

pub struct Service {
    pub fetcher: Fetcher<Cache>,
    pub config: &'static Config,
}

impl Service {
    /// From the environment: `USER_AGENT`, `REDIS_URL`.
    pub fn from_env(mode: Mode) -> Result<Self, FetchError> {
        let agent = std::env::var("USER_AGENT").unwrap_or_else(|_| DEFAULT_USER_AGENT.to_string());
        let config = Config::embedded();
        Ok(Self {
            fetcher: Fetcher::new(
                Http::new(&agent, mode)?,
                config.fetch_settings(),
                Cache::from_env(),
            ),
            config,
        })
    }

    pub async fn content(&self, qid: Qid) -> Result<Content, FetchError> {
        self.fetcher.content(qid).await
    }

    pub async fn text(&self, content: &Content, lang: &Lang) -> Result<Text, FetchError> {
        self.fetcher.text(content, lang).await
    }

    pub async fn composition(&self, qid: Qid) -> Result<Composition, FetchError> {
        Ok(qrender::compose(&self.content(qid).await?, self.config))
    }

    pub async fn view(&self, qid: Qid, lang: &Lang) -> Result<View, FetchError> {
        let content = self.content(qid).await?;
        let text = self.text(&content, lang).await?;
        Ok(qrender::localize(
            &qrender::compose(&content, self.config),
            &text,
            self.config,
        ))
    }
}
