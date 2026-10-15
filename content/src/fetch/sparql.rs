//! Queries to the Wikidata Query Service and their row parsers. Every
//! identifier in a query is a validated `Qid` or `Pid`.

use indexmap::IndexMap;
use serde_json::Value as Json;

use super::FetchError;
use super::http::{Http, Service};
use crate::id::{Lang, Pid, Qid, StatementId};
use crate::model::{
    EventDates, HolidayDay, NormalQuantity, Qualifier, QualifierValue, QuantityRecord, Rank,
    ReverseWork, Statement, Value, ValueImage, file_name, year_of,
};

const ENDPOINT: &str = "https://query.wikidata.org/sparql";
const ENTITY: &str = "http://www.wikidata.org/entity/";

/// One result row: variable name -> binding.
struct Row(serde_json::Map<String, Json>);

impl Row {
    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name)?.get("value")?.as_str()
    }

    fn lang(&self, name: &str) -> Option<&str> {
        self.0.get(name)?.get("xml:lang")?.as_str()
    }

    /// The last part of a URI: "…/entity/Q42" gives "Q42", "…#WikibaseItem"
    /// gives "WikibaseItem".
    fn local(&self, name: &str) -> Option<&str> {
        let uri = self.get(name)?;
        uri.rsplit(['/', '#']).next()
    }

    fn qid(&self, name: &str) -> Option<Qid> {
        self.get(name)?.strip_prefix(ENTITY)?.parse().ok()
    }

    fn pid(&self, name: &str) -> Option<Pid> {
        self.local(name)?.parse().ok()
    }
}

async fn query(http: &Http, sparql: &str) -> Result<Vec<Row>, FetchError> {
    let json = http
        .json(
            Service::QueryService,
            ENDPOINT,
            &[("query", sparql), ("format", "json")],
        )
        .await?;
    let bindings = json["results"]["bindings"]
        .as_array()
        .ok_or(FetchError::Shape("SPARQL results"))?;
    Ok(bindings
        .iter()
        .filter_map(|b| b.as_object().cloned().map(Row))
        .collect())
}

fn values<T: std::fmt::Display>(ids: impl IntoIterator<Item = T>) -> String {
    ids.into_iter()
        .map(|id| format!("wd:{id}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A statement as the rows give it, before parsing.
struct Raw {
    id: String,
    pid: Option<Pid>,
    datatype: String,
    value: String,
    lang: Option<String>,
    rank: String,
    unit: Option<Qid>,
    precision: Option<u8>,
    image: Option<String>,
    logo: Option<String>,
    qualifiers: Vec<(String, String, Option<String>, Option<String>)>,
}

/// Every statement of the item, grouped by statement node, with
/// identifiers and raw values only: labels are text, fetched per language.
/// Deprecated statements are dropped. A value that does not parse is
/// dropped with a log line, not the item.
pub async fn statements(http: &Http, qid: Qid) -> Result<Vec<Statement>, FetchError> {
    let rows = query(
        http,
        &format!(
            "SELECT ?property ?propertyType ?statement ?value ?valueImage ?valueLogo ?rank
                    ?qualifierProperty ?qualifierType ?qualifierValue ?unit ?precision WHERE {{
                VALUES ?item {{ wd:{qid} }}
                ?item ?claim ?statement .
                ?property wikibase:claim ?claim ; wikibase:statementProperty ?ps ;
                          wikibase:propertyType ?propertyType ; wikibase:statementValue ?psv .
                ?statement ?ps ?value ; wikibase:rank ?rank .
                OPTIONAL {{ ?value wdt:P18 ?valueImage . }}
                OPTIONAL {{ ?value wdt:P154 ?valueLogo . }}
                OPTIONAL {{ ?statement ?pq ?qualifierValue .
                           ?qualifierProperty wikibase:qualifier ?pq ;
                                              wikibase:propertyType ?qualifierType . }}
                OPTIONAL {{ ?statement ?psv ?node . ?node wikibase:quantityUnit ?unit . }}
                OPTIONAL {{ ?statement ?psv ?tnode . ?tnode wikibase:timePrecision ?precision . }}
            }}"
        ),
    )
    .await?;
    let mut raw: IndexMap<String, Raw> = IndexMap::new();
    for r in &rows {
        let Some(node) = r.get("statement") else {
            continue;
        };
        let st = raw.entry(node.to_string()).or_insert_with(|| Raw {
            id: r.local("statement").unwrap_or_default().to_string(),
            pid: r.pid("property"),
            datatype: r.local("propertyType").unwrap_or_default().to_string(),
            value: r.get("value").unwrap_or_default().to_string(),
            lang: r.lang("value").map(str::to_string),
            rank: r
                .local("rank")
                .unwrap_or_default()
                .replace("Rank", "")
                .to_lowercase(),
            unit: r.qid("unit"),
            precision: r.get("precision").and_then(|p| p.parse().ok()),
            image: None,
            logo: None,
            qualifiers: Vec::new(),
        });
        if st.image.is_none() {
            st.image = r.get("valueImage").map(file_name);
        }
        if st.logo.is_none() {
            st.logo = r.get("valueLogo").map(file_name);
        }
        if let (Some(pid), Some(value)) = (r.local("qualifierProperty"), r.get("qualifierValue")) {
            let q = (
                pid.to_string(),
                value.to_string(),
                r.lang("qualifierValue").map(str::to_string),
                r.local("qualifierType").map(str::to_string),
            );
            if !st.qualifiers.contains(&q) {
                st.qualifiers.push(q);
            }
        }
    }
    let mut out = Vec::new();
    for st in raw.into_values() {
        let rank = match st.rank.as_str() {
            "preferred" => Rank::Preferred,
            "normal" => Rank::Normal,
            _ => continue, // deprecated
        };
        let (Ok(id), Some(pid)) = (st.id.parse::<StatementId>(), st.pid) else {
            eprintln!(
                "qrender: skipped a statement with an unknown id {:?}",
                st.id
            );
            continue;
        };
        let value = match Value::from_raw(
            &st.datatype,
            &st.value,
            st.unit,
            st.precision,
            st.lang.as_deref(),
        ) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("qrender: skipped {id}: {e}");
                continue;
            }
        };
        let qualifiers = st
            .qualifiers
            .iter()
            .filter_map(|(pid, value, lang, datatype)| {
                Some(Qualifier {
                    pid: pid.parse().ok()?,
                    value: match datatype {
                        Some(t) => QualifierValue::from_typed(t, value, lang.as_deref()),
                        None => QualifierValue::from_raw(value, lang.as_deref()),
                    },
                })
            })
            .collect();
        // The picture of a value: its image, else its logo (shown whole).
        let value_image = match (st.image, st.logo) {
            (Some(file), _) => Some(ValueImage { file, logo: false }),
            (None, Some(file)) => Some(ValueImage { file, logo: true }),
            (None, None) => None,
        };
        out.push(Statement {
            id,
            pid,
            rank,
            value,
            qualifiers,
            value_image,
        });
    }
    Ok(out)
}

/// For each statement, the property set of each of its references.
pub async fn references(
    http: &Http,
    qid: Qid,
) -> Result<IndexMap<StatementId, Vec<Vec<Pid>>>, FetchError> {
    let rows = query(
        http,
        &format!(
            "SELECT ?st ?ref ?rp WHERE {{
                wd:{qid} ?claim ?st . ?st prov:wasDerivedFrom ?ref . ?ref ?rp [] .
                FILTER(STRSTARTS(STR(?rp), \"http://www.wikidata.org/prop/reference/P\")) }}"
        ),
    )
    .await?;
    let mut by_statement: IndexMap<StatementId, IndexMap<String, Vec<Pid>>> = IndexMap::new();
    for r in &rows {
        let (Some(st), Some(reference), Some(pid)) = (
            r.local("st").and_then(|s| s.parse().ok()),
            r.get("ref"),
            r.pid("rp"),
        ) else {
            continue;
        };
        by_statement
            .entry(st)
            .or_default()
            .entry(reference.to_string())
            .or_default()
            .push(pid);
    }
    Ok(by_statement
        .into_iter()
        .map(|(st, refs)| (st, refs.into_values().collect()))
        .collect())
}

/// Normalized quantities (SI units) of some properties on some items, and
/// the picture of each item. A count has no unit, so its raw value is used.
pub async fn quantities(
    http: &Http,
    qids: &[Qid],
    pids: &[Pid],
) -> Result<IndexMap<Qid, QuantityRecord>, FetchError> {
    if qids.is_empty() || pids.is_empty() {
        return Ok(IndexMap::new());
    }
    let pid_values = pids
        .iter()
        .map(|p| format!("wd:{p}"))
        .collect::<Vec<_>>()
        .join(" ");
    let rows = query(
        http,
        &format!(
            "SELECT ?item ?img ?p ?amount ?unit ?rank WHERE {{
                VALUES ?item {{ {} }}
                VALUES ?p {{ {pid_values} }}
                ?p wikibase:claim ?claim ; wikibase:statementValue ?psv ;
                   wikibase:statementValueNormalized ?psn .
                ?item ?claim ?st . ?st wikibase:rank ?rank .
                FILTER(?rank != wikibase:DeprecatedRank)
                {{ ?st ?psn ?node . }} UNION {{ ?st ?psv ?node . ?node wikibase:quantityUnit wd:Q199 . }}
                ?node wikibase:quantityAmount ?amount ; wikibase:quantityUnit ?unit .
                OPTIONAL {{ ?item wdt:P18 ?img }} }}",
            values(qids)
        ),
    )
    .await?;
    let mut out: IndexMap<Qid, QuantityRecord> = IndexMap::new();
    for r in &rows {
        let (Some(item), Some(pid), Some(amount), Some(unit)) = (
            r.qid("item"),
            r.pid("p"),
            r.get("amount").and_then(|a| a.parse::<f64>().ok()),
            r.qid("unit"),
        ) else {
            continue;
        };
        let record = out.entry(item).or_insert_with(|| QuantityRecord {
            image: r.get("img").map(file_name),
            values: IndexMap::new(),
        });
        let preferred = r
            .get("rank")
            .is_some_and(|rank| rank.ends_with("PreferredRank"));
        let replace = match record.values.get(&pid) {
            None => true,
            Some(old) => preferred && !old.preferred,
        };
        if replace {
            record.values.insert(
                pid,
                NormalQuantity {
                    amount,
                    unit,
                    preferred,
                },
            );
        }
    }
    Ok(out)
}

/// The triple pattern of a peer set: members of a class, in a container
/// when there is one.
pub fn members_pattern(class: Qid, container: Option<(Pid, Qid)>) -> String {
    let within = container.map_or_else(String::new, |(p, q)| format!("?m wdt:{p} wd:{q} ."));
    format!("?m wdt:P31 wd:{class} . {within}")
}

/// How many entities match a pattern, counted up to `limit`. Rows, not
/// parsed items: an entity that is not an item (a lexeme can be an
/// "instance of human") still counts, so a large class stays large.
pub async fn count(http: &Http, pattern: &str, limit: usize) -> Result<usize, FetchError> {
    let rows = query(
        http,
        &format!("SELECT DISTINCT ?m WHERE {{ {pattern} }} LIMIT {limit}"),
    )
    .await?;
    Ok(rows.len())
}

/// The members of a peer set that are items, up to `limit`.
pub async fn members(http: &Http, pattern: &str, limit: usize) -> Result<Vec<Qid>, FetchError> {
    let rows = query(
        http,
        &format!("SELECT DISTINCT ?m WHERE {{ {pattern} }} LIMIT {limit}"),
    )
    .await?;
    Ok(rows.iter().filter_map(|r| r.qid("m")).collect())
}

/// For each property, how many members of a peer set have it. Identifiers
/// are not counted.
pub async fn frequency(http: &Http, pattern: &str) -> Result<IndexMap<Pid, u32>, FetchError> {
    let rows = query(
        http,
        &format!(
            "SELECT ?p (COUNT(DISTINCT ?m) AS ?n) WHERE {{
                {pattern}
                ?m ?claim [] . ?p wikibase:claim ?claim ; wikibase:propertyType ?t .
                FILTER(?t != wikibase:ExternalId) }} GROUP BY ?p"
        ),
    )
    .await?;
    Ok(rows
        .iter()
        .filter_map(|r| Some((r.pid("p")?, r.get("n")?.parse().ok()?)))
        .collect())
}

/// The value items that are classes: they have a subclass-of (P279)
/// statement. Their picture shows an example, not the value.
pub async fn classes_among(http: &Http, qids: &[Qid]) -> Result<Vec<Qid>, FetchError> {
    let mut out = Vec::new();
    for chunk in qids.chunks(200) {
        let rows = query(
            http,
            &format!(
                "SELECT DISTINCT ?v WHERE {{ VALUES ?v {{ {} }} ?v wdt:P279 [] . }}",
                values(chunk)
            ),
        )
        .await?;
        out.extend(rows.iter().filter_map(|r| r.qid("v")));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// The number of sitelinks of each item: the wikis that have a page about
/// it, a measure of how widely it is known. Items without a sitelink are
/// not in the map.
pub async fn sitelink_counts(http: &Http, qids: &[Qid]) -> Result<IndexMap<Qid, u32>, FetchError> {
    let mut out = IndexMap::new();
    for chunk in qids.chunks(200) {
        let rows = query(
            http,
            &format!(
                "SELECT ?v ?n WHERE {{ VALUES ?v {{ {} }} ?v wikibase:sitelinks ?n . }}",
                values(chunk)
            ),
        )
        .await?;
        for r in &rows {
            if let (Some(q), Some(n)) = (r.qid("v"), r.get("n").and_then(|n| n.parse().ok()))
                && n > 0
            {
                out.insert(q, n);
            }
        }
    }
    Ok(out)
}

/// Statements on other items that point to this one by one of `pids`: the
/// works of a person. Each with its picture and its earliest publication
/// or inception year.
pub async fn reverse(http: &Http, qid: Qid, pids: &[Pid]) -> Result<Vec<ReverseWork>, FetchError> {
    if pids.is_empty() {
        return Ok(Vec::new());
    }
    let props = pids
        .iter()
        .map(|p| format!("wdt:{p}"))
        .collect::<Vec<_>>()
        .join(" ");
    let rows = query(
        http,
        &format!(
            "SELECT ?work ?p ?img ?date WHERE {{
                VALUES ?p {{ {props} }}
                ?work ?p wd:{qid} .
                OPTIONAL {{ ?work wdt:P18 ?img }}
                OPTIONAL {{ ?work wdt:P577|wdt:P571 ?date }} }} LIMIT 500"
        ),
    )
    .await?;
    let mut works: IndexMap<Qid, ReverseWork> = IndexMap::new();
    for r in &rows {
        let (Some(item), Some(pid)) = (r.qid("work"), r.pid("p")) else {
            continue;
        };
        let w = works.entry(item).or_insert_with(|| ReverseWork {
            item,
            pids: Vec::new(),
            image: None,
            year: None,
        });
        if !w.pids.contains(&pid) {
            w.pids.push(pid);
        }
        if w.image.is_none() {
            w.image = r.get("img").map(file_name);
        }
        if let Some(year) = r.get("date").and_then(year_of) {
            w.year = Some(w.year.map_or(year, |y| y.min(year)));
        }
    }
    let mut works: Vec<_> = works.into_values().collect();
    works.sort_by_key(|w| (w.year.is_none(), w.year.unwrap_or(0)));
    Ok(works)
}

/// The logo (P154) of each item.
pub async fn logos(http: &Http, qids: &[Qid]) -> Result<IndexMap<Qid, String>, FetchError> {
    if qids.is_empty() {
        return Ok(IndexMap::new());
    }
    let rows = query(
        http,
        &format!(
            "SELECT ?item ?logo WHERE {{ VALUES ?item {{ {} }} ?item wdt:P154 ?logo }}",
            values(qids)
        ),
    )
    .await?;
    let mut out = IndexMap::new();
    for r in &rows {
        if let (Some(item), Some(logo)) = (r.qid("item"), r.get("logo")) {
            out.entry(item).or_insert_with(|| file_name(logo));
        }
    }
    Ok(out)
}

/// The own dates of event items: point in time (P585), start time (P580),
/// end time (P582); the first value of each. Items without a date are not
/// in the map.
pub async fn event_dates(
    http: &Http,
    qids: &[Qid],
) -> Result<IndexMap<Qid, EventDates>, FetchError> {
    if qids.is_empty() {
        return Ok(IndexMap::new());
    }
    let rows = query(
        http,
        &format!(
            "SELECT ?item ?point ?start ?end WHERE {{ VALUES ?item {{ {} }} \
             OPTIONAL {{ ?item wdt:P585 ?point }} OPTIONAL {{ ?item wdt:P580 ?start }} \
             OPTIONAL {{ ?item wdt:P582 ?end }} }}",
            values(qids)
        ),
    )
    .await?;
    let mut out: IndexMap<Qid, EventDates> = IndexMap::new();
    for r in &rows {
        let Some(item) = r.qid("item") else {
            continue;
        };
        let dates = out.entry(item).or_default();
        for (key, date) in [
            ("point", &mut dates.point_in_time),
            ("start", &mut dates.start_time),
            ("end", &mut dates.end_time),
        ] {
            if date.is_none() {
                *date = r.get(key).map(str::to_string);
            }
        }
    }
    out.retain(|_, d| *d != EventDates::default());
    Ok(out)
}

/// The day of each holiday item (P837), with its month: "part of" (P361)
/// of a calendar day, else "month of the year" (P2922) of a rule. The
/// first day of each holiday.
pub async fn holiday_days(
    http: &Http,
    qids: &[Qid],
) -> Result<IndexMap<Qid, HolidayDay>, FetchError> {
    if qids.is_empty() {
        return Ok(IndexMap::new());
    }
    let rows = query(
        http,
        &format!(
            "SELECT ?item ?day ?part ?month WHERE {{ VALUES ?item {{ {} }} \
             ?item wdt:P837 ?day . OPTIONAL {{ ?day wdt:P361 ?part }} \
             OPTIONAL {{ ?day wdt:P2922 ?month }} }}",
            values(qids)
        ),
    )
    .await?;
    let mut out = IndexMap::new();
    for r in &rows {
        if let (Some(item), Some(day)) = (r.qid("item"), r.qid("day")) {
            let rule = r.qid("month");
            out.entry(item).or_insert(HolidayDay {
                day,
                month: rule.or(r.qid("part")),
                fixed: rule.is_none(),
            });
        }
    }
    Ok(out)
}

/// The language code (P424) of each language item. An item with more than
/// one code: the shortest valid code, then the first in the order of
/// codes. Items without a valid code are not in the map.
pub async fn language_codes(
    http: &Http,
    qids: &[Qid],
) -> Result<IndexMap<Qid, String>, FetchError> {
    if qids.is_empty() {
        return Ok(IndexMap::new());
    }
    let rows = query(
        http,
        &format!(
            "SELECT ?item ?code WHERE {{ VALUES ?item {{ {} }} ?item wdt:P424 ?code }}",
            values(qids)
        ),
    )
    .await?;
    let mut out: IndexMap<Qid, String> = IndexMap::new();
    for r in &rows {
        let (Some(item), Some(code)) = (r.qid("item"), r.get("code")) else {
            continue;
        };
        if code.parse::<Lang>().is_err() {
            continue;
        }
        let shorter = |old: &String| (code.len(), code) < (old.len(), old.as_str());
        if out.get(&item).is_none_or(shorter) {
            out.insert(item, code.to_string());
        }
    }
    Ok(out)
}

/// The formatter URL (P1630) of each property: "https://x.com/$1". A
/// property with more than one: the shortest, then the first in order.
pub async fn formatters(http: &Http, pids: &[Pid]) -> Result<IndexMap<Pid, String>, FetchError> {
    if pids.is_empty() {
        return Ok(IndexMap::new());
    }
    let rows = query(
        http,
        &format!(
            "SELECT ?property ?url WHERE {{ VALUES ?property {{ {} }} ?property wdt:P1630 ?url }}",
            values(pids)
        ),
    )
    .await?;
    let mut out: IndexMap<Pid, String> = IndexMap::new();
    for r in &rows {
        let (Some(pid), Some(url)) = (r.pid("property"), r.get("url")) else {
            continue;
        };
        let shorter = |old: &String| (url.len(), url) < (old.len(), old.as_str());
        if out.get(&pid).is_none_or(shorter) {
            out.insert(pid, url.to_string());
        }
    }
    Ok(out)
}

/// The labels and the aliases of each item in some languages: (language,
/// name). Every language is a validated `Lang`.
pub async fn names(
    http: &Http,
    qids: &[Qid],
    langs: &[Lang],
) -> Result<IndexMap<Qid, Vec<(String, String)>>, FetchError> {
    if qids.is_empty() || langs.is_empty() {
        return Ok(IndexMap::new());
    }
    let langs = langs
        .iter()
        .map(|l| format!("\"{l}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let rows = query(
        http,
        &format!(
            "SELECT ?item ?name WHERE {{ VALUES ?item {{ {} }}
                ?item rdfs:label|skos:altLabel ?name . FILTER(LANG(?name) IN ({langs})) }}",
            values(qids)
        ),
    )
    .await?;
    let mut out: IndexMap<Qid, Vec<(String, String)>> = IndexMap::new();
    for r in &rows {
        let (Some(item), Some(name), Some(lang)) = (r.qid("item"), r.get("name"), r.lang("name"))
        else {
            continue;
        };
        out.entry(item)
            .or_default()
            .push((lang.to_string(), name.to_string()));
    }
    Ok(out)
}
