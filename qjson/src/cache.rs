//! Redis cache for transformed items, matching the Go tool's behavior:
//! keys `qjson:{qid}:{lang}`, 7-day TTL, and any cache failure degrades
//! to a direct SPARQL fetch instead of an error.

use redis::AsyncCommands;

use crate::model::WikidataItem;

const KEY_PREFIX: &str = "qjson";
const CACHE_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

pub struct Cache {
    client: Option<redis::Client>,
}

impl Cache {
    /// Reads REDIS_URL. Accepts both a redis:// URL and the Go tool's bare
    /// host:port form. No/invalid REDIS_URL means caching is disabled.
    pub fn from_env() -> Cache {
        let client = std::env::var("REDIS_URL").ok().and_then(|url| {
            let url = if url.contains("://") {
                url
            } else {
                format!("redis://{}", url)
            };
            redis::Client::open(url)
                .inspect_err(|e| eprintln!("qjson: ignoring invalid REDIS_URL: {e}"))
                .ok()
        });
        Cache { client }
    }

    fn key(qid: &str, language: &str) -> String {
        format!("{KEY_PREFIX}:{qid}:{language}")
    }

    pub async fn get(&self, qid: &str, language: &str) -> Option<WikidataItem> {
        let client = self.client.as_ref()?;
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .inspect_err(|e| eprintln!("qjson: redis connection failed: {e}"))
            .ok()?;
        let data: Option<String> = connection.get(Self::key(qid, language)).await.ok()?;
        serde_json::from_str(&data?).ok()
    }

    pub async fn set(&self, qid: &str, language: &str, item: &WikidataItem) {
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(mut connection) = client.get_multiplexed_async_connection().await else {
            return;
        };
        let Ok(json) = serde_json::to_string(item) else {
            return;
        };
        let result: Result<(), _> = connection
            .set_ex(Self::key(qid, language), json, CACHE_TTL_SECONDS)
            .await;
        if let Err(e) = result {
            eprintln!("qjson: failed to cache {qid}/{language}: {e}");
        }
    }
}
