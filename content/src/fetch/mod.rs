//! Fetch the content and the text of an item from Wikidata, Wikipedia and
//! Commons. Independent requests run at the same time; the transport
//! (`http.rs`) keeps the load on each service polite.

mod commons;
pub mod http;
mod sparql;
mod wikidata;
mod wikipedia;

use std::collections::BTreeSet;

use std::future::Future;

use futures::future::try_join_all;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::id::{EntityId, Lang, Pid, Qid};
use crate::model::{
    CONTENT_SCHEMA, Container, Content, Datatype, Peers, QualifierValue, Rank, Statement, Value,
};
use crate::settings::Settings;
use crate::text::{TEXT_SCHEMA, Text};
pub use http::{Http, Mode};

/// A peer set has this many members at least, and at most.
const PEERS_MIN: usize = 5;
const PEERS_MAX: usize = 150;
/// A property that this share of the peers has, and the item lacks, is
/// "missing".
const MISSING_SHARE: f64 = 0.6;
/// The reference properties and the age property whose labels the page
/// shows (stated in, reference URL, imported from, age).
const EXTRA_LABELS: [&str; 4] = ["P248", "P854", "P143", "P3629"];

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("HTTP {0} for {1}")]
    Status(u16, String),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected response: no {0}")]
    Shape(&'static str),
    #[error("not recorded: {0}")]
    NotRecorded(String),
    #[error("recording: {0}")]
    Record(std::io::Error),
    #[error("no item {0}")]
    NoItem(Qid),
}

/// The version of the fetcher. Raise it when a change of the fetcher
/// changes what it fetches (a new field, a new request): it is part of
/// every cache key, so the parts that an older fetcher stored are not read.
pub const FETCH_VERSION: u32 = 2;

/// Seconds a cached part lives: content and text change with edits;
/// a peer set changes slowly and many items share it.
pub const DAY: u64 = 24 * 60 * 60;
const PEERS_TTL: u64 = 7 * DAY;

/// Where fetched parts are kept between requests. A failure of the store
/// is a miss, never an error: the part is fetched again.
pub trait Store: Sync {
    fn get(&self, key: &str) -> impl Future<Output = Option<String>> + Send;
    fn set(&self, key: &str, value: String, ttl: u64) -> impl Future<Output = ()> + Send;
}

/// No store: everything is fetched.
pub struct NoStore;

impl Store for NoStore {
    async fn get(&self, _: &str) -> Option<String> {
        None
    }

    async fn set(&self, _: &str, _: String, _: u64) {}
}

/// A part from the store, or made and then stored.
pub async fn cached<S, T, F>(store: &S, key: &str, ttl: u64, make: F) -> Result<T, FetchError>
where
    S: Store,
    T: Serialize + DeserializeOwned,
    F: Future<Output = Result<T, FetchError>>,
{
    if let Some(json) = store.get(key).await
        && let Ok(value) = serde_json::from_str(&json)
    {
        return Ok(value);
    }
    let value = make.await?;
    if let Ok(json) = serde_json::to_string(&value) {
        store.set(key, json, ttl).await;
    }
    Ok(value)
}

pub struct Fetcher<S: Store = NoStore> {
    pub http: Http,
    pub settings: Settings,
    pub store: S,
}

impl<S: Store> Fetcher<S> {
    pub fn new(http: Http, settings: Settings, store: S) -> Self {
        Self {
            http,
            settings,
            store,
        }
    }

    /// The part of every cache key that says what made the part: the
    /// version of the fetcher and the fingerprint of its settings.
    fn made_by(&self) -> String {
        format!("f{FETCH_VERSION}.{:016x}", self.settings.fingerprint())
    }

    /// The content of an item, from the store or fetched.
    pub async fn content(&self, qid: Qid) -> Result<Content, FetchError> {
        cached(
            &self.store,
            &format!("{CONTENT_SCHEMA}:{}:{qid}", self.made_by()),
            DAY,
            self.fetch_content(qid),
        )
        .await
    }

    /// The text of an item in one language, from the store or fetched.
    pub async fn text(&self, content: &Content, lang: &Lang) -> Result<Text, FetchError> {
        let key = format!("{TEXT_SCHEMA}:{}:{}:{lang}", self.made_by(), content.qid);
        cached(&self.store, &key, DAY, self.fetch_text(content, lang)).await
    }

    /// The content of an item: the same for every language.
    async fn fetch_content(&self, qid: Qid) -> Result<Content, FetchError> {
        let http = &self.http;
        let statements = sparql::statements(http, qid).await?;
        if statements.is_empty() {
            return Err(FetchError::NoItem(qid));
        }
        let classes: Vec<Qid> = statements
            .iter()
            .filter(|s| s.pid.number() == 31)
            .filter_map(|s| s.value.item())
            .collect();
        let container = self.container(&statements);
        let quantity_pids: Vec<Pid> = unique(
            statements
                .iter()
                .filter(|s| s.value.datatype() == Datatype::Quantity)
                .map(|s| s.pid),
        );
        let layout = self.settings.layout(&classes);
        let reverse_pids = layout.map(|l| l.reverse.clone()).unwrap_or_default();
        let value_items: Vec<Qid> = unique(statements.iter().filter_map(|s| s.value.item()));
        let logo_items: Vec<Qid> = unique(
            statements
                .iter()
                .flat_map(|s| &s.qualifiers)
                .filter_map(
                    |q| match (&q.value, self.settings.qualifier_logos.contains(&q.pid)) {
                        (
                            QualifierValue::Entity {
                                entity: EntityId::Item(item),
                            },
                            true,
                        ) => Some(*item),
                        _ => None,
                    },
                ),
        );
        let tabular_urls: Vec<String> = statements
            .iter()
            .filter_map(|s| match &s.value {
                Value::TabularData { page } => Some(page.clone()),
                _ => None,
            })
            .collect();
        let targets: Vec<Qid> = std::iter::once(qid).chain(container.qid).collect();

        let event_items: Vec<Qid> = unique(
            statements
                .iter()
                .filter(|s| self.settings.timeline.contains(&s.pid))
                .filter_map(|s| s.value.item()),
        );
        let (
            references,
            quantities,
            peers,
            value_classes,
            reverse,
            logos,
            sitelinks,
            tables,
            ranges,
            event_dates,
        ) = futures::try_join!(
            sparql::references(http, qid),
            sparql::quantities(http, &targets, &quantity_pids),
            self.peers(&statements, &classes, &container, &quantity_pids),
            sparql::classes_among(http, &value_items),
            sparql::reverse(http, qid, &reverse_pids),
            sparql::logos(http, &logo_items),
            wikidata::sitelinks(http, qid),
            try_join_all(tabular_urls.iter().map(|url| async move {
                Ok::<_, FetchError>((url.clone(), commons::table(http, url).await?))
            })),
            wikidata::ranges(http, &quantity_pids),
            sparql::event_dates(http, &event_items),
        )?;

        let mut media: Vec<String> = statements
            .iter()
            .filter_map(|s| match &s.value {
                Value::CommonsMedia { file } => Some(file.clone()),
                _ => None,
            })
            .chain(
                statements
                    .iter()
                    .filter_map(|s| s.value_image.as_ref().map(|i| i.file.clone())),
            )
            .chain(reverse.iter().filter_map(|r| r.image.clone()))
            .chain(logos.values().cloned())
            .collect();
        media.sort();
        media.dedup();
        let commons = commons::files(http, &media).await?;

        Ok(Content {
            schema: CONTENT_SCHEMA.to_string(),
            qid,
            statements,
            references,
            container,
            quantities,
            peers,
            commons,
            value_classes,
            sitelinks,
            class_layout: layout
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "generic".to_string()),
            reverse,
            tabular: tables
                .into_iter()
                .filter_map(|(url, t)| Some((url, t?)))
                .collect(),
            logos,
            ranges,
            event_dates,
        })
    }

    /// The container: the first container property with an item value, and
    /// its current value (preferred, else not ended, else the latest start).
    fn container(&self, statements: &[Statement]) -> Container {
        for pid in &self.settings.containers {
            let mut values: Vec<&Statement> = statements
                .iter()
                .filter(|s| s.pid == *pid && s.value.item().is_some())
                .collect();
            if values.is_empty() {
                continue;
            }
            let year = |s: &Statement, p: u64| {
                s.qualifiers
                    .iter()
                    .find(|q| q.pid.number() == p)
                    .and_then(|q| q.value.year())
            };
            values.sort_by_key(|s| {
                (
                    s.rank != Rank::Preferred,
                    year(s, 582).is_some(),
                    -year(s, 580).unwrap_or(-1_000_000),
                )
            });
            return Container {
                pid: Some(*pid),
                qid: values[0].value.item(),
            };
        }
        Container::default()
    }

    /// The peer set, from the store or found. Items with the same classes
    /// and container share it (the cities of one country).
    async fn peers(
        &self,
        statements: &[Statement],
        classes: &[Qid],
        container: &Container,
        quantity_pids: &[Pid],
    ) -> Result<Option<Peers>, FetchError> {
        let mut sorted = classes.to_vec();
        sorted.sort();
        let key = format!(
            "peers/0.1:{}:{}:{}:{}",
            self.made_by(),
            sorted
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            container.pid.map(|p| p.to_string()).unwrap_or_default(),
            container.qid.map(|q| q.to_string()).unwrap_or_default(),
        );
        let found = cached(
            &self.store,
            &key,
            PEERS_TTL,
            self.find_peers(classes, container, quantity_pids),
        )
        .await?;
        // What the item lacks depends on the item, not on the set.
        let Some(mut peers) = found else {
            return Ok(None);
        };
        let have: BTreeSet<Pid> = statements.iter().map(|s| s.pid).collect();
        peers.missing.retain(|p, _| !have.contains(p));
        Ok(Some(peers))
    }

    /// The most specific useful peer set: class and container together
    /// when that gives 5 to 150 members, else the smallest class that does.
    /// `missing` holds every property that most members have; `peers`
    /// keeps those the item lacks.
    async fn find_peers(
        &self,
        classes: &[Qid],
        container: &Container,
        quantity_pids: &[Pid],
    ) -> Result<Option<Peers>, FetchError> {
        let http = &self.http;
        let within = container.pid.zip(container.qid);
        // (without the container, size, class, container): the first is best.
        let options = try_join_all(classes.iter().map(|class| async move {
            if within.is_some() {
                let pattern = sparql::members_pattern(*class, within);
                let n = sparql::count(http, &pattern, PEERS_MAX + 1).await?;
                if (PEERS_MIN..=PEERS_MAX).contains(&n) {
                    return Ok::<_, FetchError>(Some((false, n, class.to_string(), within)));
                }
            }
            let n =
                sparql::count(http, &sparql::members_pattern(*class, None), PEERS_MAX + 1).await?;
            Ok((PEERS_MIN..=PEERS_MAX)
                .contains(&n)
                .then(|| (true, n, class.to_string(), None)))
        }))
        .await?;
        let Some((_, _, class, within)) = options.into_iter().flatten().min_by(|a, b| a.cmp(b))
        else {
            return Ok(None);
        };
        let class: Qid = class.parse().expect("a class id");
        let pattern = sparql::members_pattern(class, within);
        let (members, frequency) = futures::try_join!(
            sparql::members(http, &pattern, PEERS_MAX + 1),
            sparql::frequency(http, &pattern),
        )?;
        let enough = (0.3 * members.len() as f64).max(5.0);
        let shared: Vec<Pid> = quantity_pids
            .iter()
            .filter(|p| f64::from(frequency.get(*p).copied().unwrap_or(0)) >= enough)
            .copied()
            .collect();
        let missing: Vec<Pid> = frequency
            .iter()
            .filter(|(_, n)| f64::from(**n) >= MISSING_SHARE * members.len() as f64)
            .map(|(p, _)| *p)
            .collect();
        let (quantities, missing) = futures::try_join!(
            sparql::quantities(http, &members, &shared),
            wikidata::datatypes(http, &missing),
        )?;
        Ok(Some(Peers {
            class,
            container_pid: within.map(|(p, _)| p),
            container: within.map(|(_, q)| q),
            members,
            frequency,
            quantities,
            missing,
        }))
    }

    /// The text of an item in one language: labels, concepts, the article
    /// (and the English one when the page language has none), page images.
    async fn fetch_text(&self, content: &Content, lang: &Lang) -> Result<Text, FetchError> {
        let http = &self.http;
        let ids: Vec<EntityId> = identifiers(content).into_iter().collect();
        let unpictured: Vec<Qid> = unique(
            content
                .statements
                .iter()
                .filter(|s| s.value_image.is_none())
                .filter_map(|s| s.value.item()),
        );
        let en: Lang = "en".parse().expect("a language code");
        let (labels, concepts, page_images) = futures::try_join!(
            wikidata::terms(http, &ids, lang),
            wikidata::concepts(http, &self.settings.concepts, lang),
            async { Ok(wikipedia::page_images(http, &unpictured, lang).await) },
        )?;
        let article = match content.sitelinks.get(&format!("{lang}wiki")) {
            Some(title) => {
                let host = format!("{lang}.wikipedia.org");
                let mut article = wikipedia::lead(http, &host, title).await?;
                wikipedia::match_sections(http, &host, &mut article, &concepts).await?;
                Some(article)
            }
            None => None,
        };
        let fallback_article = match content.sitelinks.get("enwiki") {
            Some(title) if lang.as_str() != "en" => {
                let host = "en.wikipedia.org";
                let english = wikidata::concepts(http, &self.settings.concepts, &en).await?;
                let mut article = wikipedia::lead(http, host, title).await?;
                wikipedia::match_sections(http, host, &mut article, &english).await?;
                Some(article)
            }
            _ => None,
        };
        Ok(Text {
            schema: TEXT_SCHEMA.to_string(),
            lang: lang.clone(),
            labels,
            concepts,
            article,
            fallback_article,
            page_images,
        })
    }
}

/// Every identifier whose label a page can show.
fn identifiers(content: &Content) -> BTreeSet<EntityId> {
    let mut ids = BTreeSet::from([EntityId::Item(content.qid)]);
    for s in &content.statements {
        ids.insert(s.pid.into());
        match &s.value {
            Value::WikibaseItem { item } => {
                ids.insert((*item).into());
            }
            Value::Quantity { unit: Some(u), .. } => {
                ids.insert((*u).into());
            }
            _ => {}
        }
        for q in &s.qualifiers {
            ids.insert(q.pid.into());
            if let QualifierValue::Entity { entity } = q.value {
                ids.insert(entity);
            }
        }
    }
    if let Some(peers) = &content.peers {
        ids.extend(peers.members.iter().map(|q| EntityId::Item(*q)));
        ids.extend(peers.missing.keys().map(|p| EntityId::Property(*p)));
        ids.insert(peers.class.into());
    }
    if let Some(q) = content.container.qid {
        ids.insert(q.into());
    }
    for record in content.quantities.values() {
        ids.extend(record.values.values().map(|v| EntityId::Item(v.unit)));
    }
    ids.extend(
        EXTRA_LABELS
            .iter()
            .filter_map(|p| p.parse::<EntityId>().ok()),
    );
    for work in &content.reverse {
        ids.insert(work.item.into());
        ids.extend(work.pids.iter().map(|p| EntityId::Property(*p)));
    }
    ids
}

fn unique<T: Ord + Copy>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let set: BTreeSet<T> = items.into_iter().collect();
    set.into_iter().collect()
}
