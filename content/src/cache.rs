//! A Redis store for fetched and derived parts. Without `REDIS_URL`, or
//! when Redis fails, nothing is cached and every part is fetched: a cache
//! failure is never an error.

use redis::AsyncCommands;

use crate::fetch::Store;

pub struct Cache {
    client: Option<redis::Client>,
}

impl Cache {
    /// From `REDIS_URL`: a `redis://` URL, or a bare `host:port`.
    pub fn from_env() -> Self {
        let client = std::env::var("REDIS_URL").ok().and_then(|url| {
            let url = if url.contains("://") {
                url
            } else {
                format!("redis://{url}")
            };
            redis::Client::open(url)
                .inspect_err(|e| eprintln!("qrender: ignoring REDIS_URL: {e}"))
                .ok()
        });
        Self { client }
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
}

impl Store for Cache {
    async fn get(&self, key: &str) -> Option<String> {
        self.connection().await?.get(key).await.ok()?
    }

    async fn set(&self, key: &str, value: String, ttl: u64) {
        let Some(mut connection) = self.connection().await else {
            return;
        };
        let result: Result<(), _> = connection.set_ex(key, value, ttl).await;
        if let Err(e) = result {
            eprintln!("qrender: redis: could not store {key}: {e}");
        }
    }
}
