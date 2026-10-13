//! The one transport of the fetcher: a polite client for the Wikimedia
//! services, which can also record every response or replay recorded
//! ones (for tests without a network).

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Semaphore;

use super::FetchError;

/// Queries at the same time to the query service, and to the APIs.
const WDQS_PARALLEL: usize = 2;
const API_PARALLEL: usize = 8;
const ATTEMPTS: u32 = 3;
const TIMEOUT: Duration = Duration::from_secs(120);
/// The wait before a retry, when the service gives no Retry-After.
const BACKOFF: Duration = Duration::from_secs(10);

pub struct Http {
    client: reqwest::Client,
    wdqs: Semaphore,
    api: Semaphore,
    mode: Mode,
}

pub enum Mode {
    Live,
    /// Live, and every response body is saved in this directory.
    Record(PathBuf),
    /// No network: responses come from this directory.
    Replay(PathBuf),
}

/// Which limit a request counts against.
#[derive(Clone, Copy)]
pub enum Service {
    QueryService,
    Api,
}

impl Http {
    /// `user_agent`: the Wikimedia User-Agent policy asks for a contact.
    pub fn new(user_agent: &str, mode: Mode) -> Result<Self, FetchError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(user_agent)
                .timeout(TIMEOUT)
                .build()?,
            wdqs: Semaphore::new(WDQS_PARALLEL),
            api: Semaphore::new(API_PARALLEL),
            mode,
        })
    }

    /// GET a JSON document. Retries on 429, 503 and 504.
    pub async fn json(
        &self,
        service: Service,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, FetchError> {
        let key = key(url, params);
        if let Mode::Replay(dir) = &self.mode {
            let path = dir.join(format!("{key}.json"));
            let body = std::fs::read_to_string(&path)
                .map_err(|_| FetchError::NotRecorded(request(url, params)))?;
            return Ok(serde_json::from_str(&body)?);
        }
        let _permit = match service {
            Service::QueryService => self.wdqs.acquire().await,
            Service::Api => self.api.acquire().await,
        }
        .expect("the semaphore is never closed");
        let mut attempt = 0;
        let body = loop {
            attempt += 1;
            let response = self
                .client
                .get(url)
                .query(params)
                .header(
                    "Accept",
                    "application/json, application/sparql-results+json",
                )
                .send()
                .await?;
            let status = response.status();
            if status.is_success() {
                break response.text().await?;
            }
            let retry = matches!(status.as_u16(), 429 | 503 | 504) && attempt < ATTEMPTS;
            if !retry {
                return Err(FetchError::Status(status.as_u16(), request(url, params)));
            }
            let wait = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok()?.parse().ok())
                .map_or(BACKOFF, Duration::from_secs);
            tokio::time::sleep(wait.min(Duration::from_secs(60))).await;
        };
        if let Mode::Record(dir) = &self.mode {
            std::fs::create_dir_all(dir).map_err(FetchError::Record)?;
            std::fs::write(dir.join(format!("{key}.json")), &body).map_err(FetchError::Record)?;
        }
        Ok(serde_json::from_str(&body)?)
    }
}

/// The name of a recorded response: a hash of the request (FNV-1a, so it
/// is the same in every build).
fn key(url: &str, params: &[(&str, &str)]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in request(url, params).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn request(url: &str, params: &[(&str, &str)]) -> String {
    let query: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{url}?{}", query.join("&"))
}
