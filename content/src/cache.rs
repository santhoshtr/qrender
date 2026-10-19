//! A Redis store for fetched and derived parts. Without `TOOL_REDIS_URI`, or
//! when Redis fails, nothing is cached and every part is fetched: a cache
//! failure is never an error.

use redis::AsyncCommands;

use crate::fetch::Store;

pub struct Cache {
    client: Option<redis::Client>,
    /// Put before every key; empty, or ends with ':'.
    prefix: String,
}

impl Cache {
    /// From `TOOL_REDIS_URI` (Toolforge sets it): a `redis://` URL, or a
    /// bare `host:port`. And `REDIS_PREFIX`, put before every key: on a
    /// shared Redis (Toolforge), make it a long random secret, so that other
    /// users cannot find or change the keys.
    pub fn from_env() -> Self {
        let client = std::env::var("TOOL_REDIS_URI").ok().and_then(|url| {
            let url = if url.contains("://") {
                url
            } else {
                format!("redis://{url}")
            };
            redis::Client::open(url)
                .inspect_err(|e| eprintln!("qrender: ignoring TOOL_REDIS_URI: {e}"))
                .ok()
        });
        let prefix = std::env::var("REDIS_PREFIX")
            .ok()
            .filter(|p| !p.is_empty())
            .map(|p| format!("{p}:"))
            .unwrap_or_default();
        if client.is_some() && prefix.is_empty() {
            eprintln!(
                "qrender: no REDIS_PREFIX: on a shared Redis, other users can read and change the keys"
            );
        }
        Self { client, prefix }
    }

    pub fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    async fn connection(&self) -> Option<redis::aio::MultiplexedConnection> {
        self.client
            .as_ref()?
            .get_multiplexed_async_connection()
            .await
            .inspect_err(|e| eprintln!("qrender: redis: {e}"))
            .ok()
    }

    fn key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }
}

impl Store for Cache {
    async fn get(&self, key: &str) -> Option<String> {
        self.connection().await?.get(self.key(key)).await.ok()?
    }

    async fn set(&self, key: &str, value: String, ttl: u64) {
        let Some(mut connection) = self.connection().await else {
            return;
        };
        let result: Result<(), _> = connection.set_ex(self.key(key), value, ttl).await;
        if let Err(e) = result {
            eprintln!("qrender: redis: could not store {key}: {e}");
        }
    }
}
