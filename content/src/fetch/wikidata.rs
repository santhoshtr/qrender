//! wbgetentities on Wikidata: sitelinks, labels and descriptions,
//! concept names, property datatypes, range constraints.

use futures::future::try_join_all;
use indexmap::IndexMap;
use serde_json::Value as Json;

use super::FetchError;
use super::http::{Http, Service};
use crate::id::{EntityId, Lang, Pid, Qid};
use crate::model::Range;
use crate::text::{Concept, LangText, Term};

const API: &str = "https://www.wikidata.org/w/api.php";
const BATCH: usize = 50;

async fn entities(
    http: &Http,
    ids: &[String],
    props: &str,
    languages: Option<&str>,
) -> Result<Json, FetchError> {
    let ids = ids.join("|");
    let mut params = vec![
        ("action", "wbgetentities"),
        ("format", "json"),
        ("formatversion", "2"),
        ("ids", ids.as_str()),
        ("props", props),
    ];
    if let Some(languages) = languages {
        params.push(("languages", languages));
    }
    let json = http.json(Service::Api, API, &params).await?;
    Ok(json["entities"].clone())
}

/// Site id ("enwiki") -> page title.
pub async fn sitelinks(http: &Http, qid: Qid) -> Result<IndexMap<String, String>, FetchError> {
    let all = entities(http, &[qid.to_string()], "sitelinks", None).await?;
    let links = all[qid.to_string()]["sitelinks"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    Ok(links
        .into_iter()
        .filter_map(|(site, link)| Some((site, link["title"].as_str()?.to_string())))
        .collect())
}

/// The Wikipedia article of each item: its title in the wiki of the page
/// language, else in the English one; `lang` is the language of the wiki.
pub async fn articles(
    http: &Http,
    qids: &[Qid],
    lang: &Lang,
) -> Result<IndexMap<Qid, LangText>, FetchError> {
    let batches = qids.chunks(BATCH).map(|batch| {
        let batch: Vec<String> = batch.iter().map(ToString::to_string).collect();
        async move { entities(http, &batch, "sitelinks", None).await }
    });
    let mut out = IndexMap::new();
    for all in try_join_all(batches).await? {
        for (id, entity) in all.as_object().into_iter().flatten() {
            let Ok(qid) = id.parse::<Qid>() else {
                continue;
            };
            let found = [lang.as_str(), "en"].into_iter().find_map(|code| {
                Some(LangText {
                    text: entity["sitelinks"][format!("{code}wiki")]["title"]
                        .as_str()?
                        .to_string(),
                    lang: code.parse().ok()?,
                })
            });
            if let Some(article) = found {
                out.insert(qid, article);
            }
        }
    }
    Ok(out)
}

/// The language chain of labels: the page language, `mul`, English.
pub fn chain(lang: &Lang) -> Vec<String> {
    let mut chain = vec![lang.to_string()];
    for code in ["mul", "en"] {
        if !chain.iter().any(|c| c == code) {
            chain.push(code.to_string());
        }
    }
    chain
}

/// The first text of a terms object in the chain, with its language.
fn first(terms: &Json, chain: &[String]) -> Option<LangText> {
    chain.iter().find_map(|code| {
        Some(LangText {
            text: terms.get(code)?.get("value")?.as_str()?.to_string(),
            lang: code.parse().ok()?,
        })
    })
}

/// The label and the description of every entity, by the chain.
pub async fn terms(
    http: &Http,
    ids: &[EntityId],
    lang: &Lang,
) -> Result<IndexMap<EntityId, Term>, FetchError> {
    let chain = chain(lang);
    let languages = chain.join("|");
    let batches = ids.chunks(BATCH).map(|batch| {
        let batch: Vec<String> = batch.iter().map(ToString::to_string).collect();
        let languages = languages.clone();
        async move { entities(http, &batch, "labels|descriptions", Some(&languages)).await }
    });
    let mut out = IndexMap::new();
    for all in try_join_all(batches).await? {
        for (id, entity) in all.as_object().into_iter().flatten() {
            let Ok(id) = id.parse::<EntityId>() else {
                continue;
            };
            out.insert(
                id,
                Term {
                    label: first(&entity["labels"], &chain),
                    description: first(&entity["descriptions"], &chain),
                },
            );
        }
    }
    Ok(out)
}

/// The label of each concept by the chain, and its names in one language
/// (label and aliases): the section titles it matches.
pub async fn concepts(
    http: &Http,
    qids: &[Qid],
    lang: &Lang,
) -> Result<IndexMap<Qid, Concept>, FetchError> {
    let chain = chain(lang);
    let languages = chain.join("|");
    let batches = qids.chunks(BATCH).map(|batch| {
        let batch: Vec<String> = batch.iter().map(ToString::to_string).collect();
        let languages = languages.clone();
        async move { entities(http, &batch, "labels|aliases", Some(&languages)).await }
    });
    let mut out = IndexMap::new();
    for all in try_join_all(batches).await? {
        for (id, entity) in all.as_object().into_iter().flatten() {
            let Ok(qid) = id.parse::<Qid>() else {
                continue;
            };
            let mut names: Vec<String> = entity["labels"][lang.as_str()]["value"]
                .as_str()
                .map(str::to_string)
                .into_iter()
                .chain(
                    entity["aliases"][lang.as_str()]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|a| a["value"].as_str().map(str::to_string)),
                )
                .collect();
            names.sort();
            names.dedup();
            out.insert(
                qid,
                Concept {
                    label: first(&entity["labels"], &chain),
                    names,
                },
            );
        }
    }
    Ok(out)
}

/// The datatype of each property, as Wikibase names it ("wikibase-item").
pub async fn datatypes(http: &Http, pids: &[Pid]) -> Result<IndexMap<Pid, String>, FetchError> {
    let mut out = IndexMap::new();
    for batch in pids.chunks(BATCH) {
        let ids: Vec<String> = batch.iter().map(ToString::to_string).collect();
        let all = entities(http, &ids, "datatype", None).await?;
        for pid in batch {
            if let Some(t) = all[pid.to_string()]["datatype"].as_str() {
                out.insert(*pid, t.to_string());
            }
        }
    }
    Ok(out)
}

/// Property constraint (P2302): range constraint (Q21510860), with its
/// minimum value (P2313) and maximum value (P2312).
const CONSTRAINT: &str = "P2302";
const RANGE_CONSTRAINT: &str = "Q21510860";
const MINIMUM: &str = "P2313";
const MAXIMUM: &str = "P2312";

/// The range constraint of each property that has one with both ends.
pub async fn ranges(http: &Http, pids: &[Pid]) -> Result<IndexMap<Pid, Range>, FetchError> {
    let mut out = IndexMap::new();
    for batch in pids.chunks(BATCH) {
        let ids: Vec<String> = batch.iter().map(ToString::to_string).collect();
        let all = entities(http, &ids, "claims", None).await?;
        for pid in batch {
            let constraints = all[pid.to_string()]["claims"][CONSTRAINT].as_array();
            let range = constraints.into_iter().flatten().find_map(|c| {
                if c["mainsnak"]["datavalue"]["value"]["id"] != RANGE_CONSTRAINT {
                    return None;
                }
                let end = |p: &str| -> Option<f64> {
                    c["qualifiers"][p][0]["datavalue"]["value"]["amount"]
                        .as_str()?
                        .trim_start_matches('+')
                        .parse()
                        .ok()
                };
                Some(Range {
                    min: end(MINIMUM)?,
                    max: end(MAXIMUM)?,
                })
            });
            if let Some(range) = range {
                out.insert(*pid, range);
            }
        }
    }
    Ok(out)
}
