//! What the command line and the server share: the fetcher, built from the
//! environment, and the steps from an item to a view.
//!
//! Only fetched parts are cached (content, text, peer sets). The
//! composition and the view are derived from them in milliseconds.
//!
//! Each step writes one line to stderr: the item, the language, the time.
//! A fetch of a few milliseconds came from the cache.

use std::time::Instant;

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
    /// From the environment: `USER_AGENT`, `TOOL_REDIS_URI`, `REDIS_PREFIX`.
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
        let start = Instant::now();
        let content = self.fetcher.content(qid).await?;
        eprintln!(
            "content {qid}: {} statements, {:.1?}",
            content.statements.len(),
            start.elapsed()
        );
        Ok(content)
    }

    pub async fn text(&self, content: &Content, lang: &Lang) -> Result<Text, FetchError> {
        let start = Instant::now();
        let text = self.fetcher.text(content, lang).await?;
        eprintln!(
            "text {}.{lang}: {} labels, {:.1?}",
            content.qid,
            text.labels.len(),
            start.elapsed()
        );
        Ok(text)
    }

    pub async fn composition(&self, qid: Qid) -> Result<Composition, FetchError> {
        Ok(self.compose(&self.content(qid).await?))
    }

    pub async fn view(&self, qid: Qid, lang: &Lang) -> Result<View, FetchError> {
        let content = self.content(qid).await?;
        let text = self.text(&content, lang).await?;
        let composition = self.compose(&content);
        let start = Instant::now();
        let view = qrender::localize(&composition, &text, self.config);
        eprintln!("localize {qid}.{lang}: {:.1?}", start.elapsed());
        Ok(view)
    }

    fn compose(&self, content: &Content) -> Composition {
        let start = Instant::now();
        let composition = qrender::compose(content, self.config);
        eprintln!("compose {}: {:.1?}", content.qid, start.elapsed());
        composition
    }
}
