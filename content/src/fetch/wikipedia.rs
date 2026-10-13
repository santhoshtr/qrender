//! Wikipedia: the lead of an article, its sections, the first paragraph
//! of a section, the items its links point to; page images.

use std::sync::LazyLock;

use futures::future::join_all;
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value as Json;

use super::FetchError;
use super::http::{Http, Service};
use crate::html::strip_tags;
use crate::id::{Lang, Qid};
use crate::text::{Article, Concept, PageImage, Section};

const BATCH: usize = 50;
const DISPLAY_IMAGE: &str = "https://wiki-display-image.toolforge.org/json/wikidata/";

static TABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<table.*?</table>").unwrap());
static REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<sup[^>]*class="[^"]*reference[^"]*".*?</sup>"#).unwrap());
static STYLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<style.*?</style>").unwrap());
static PARAGRAPH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<p>(.*?)</p>").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r##"href="/wiki/([^"#?]+)"##).unwrap());

fn api(host: &str) -> String {
    format!("https://{host}/w/api.php")
}

async fn parse(http: &Http, host: &str, params: &[(&str, &str)]) -> Result<Json, FetchError> {
    let mut all = vec![
        ("action", "parse"),
        ("format", "json"),
        ("formatversion", "2"),
        ("redirects", "1"),
    ];
    all.extend_from_slice(params);
    let json = http.json(Service::Api, &api(host), &all).await?;
    Ok(json["parse"].clone())
}

/// The paragraphs of rendered HTML: tables, styles and reference marks
/// removed; empty paragraphs and parser error messages left out.
fn paragraphs(html: &str) -> Vec<String> {
    let html = TABLE.replace_all(html, "");
    let html = REFERENCE.replace_all(&html, "");
    let html = STYLE.replace_all(&html, "");
    PARAGRAPH
        .captures_iter(&html)
        .map(|c| c[1].to_string())
        .filter(|p| !strip_tags(p).trim().is_empty())
        // "Cite error: ..." and other messages of the wiki parser.
        .filter(|p| !p.contains("class=\"error") && !p.contains("mw-ext-cite-error"))
        .collect()
}

/// The lead of an article, the item of each article it links to, and the
/// titles of its top-level sections.
pub async fn lead(http: &Http, host: &str, title: &str) -> Result<Article, FetchError> {
    let text = parse(
        http,
        host,
        &[
            ("page", title),
            ("section", "0"),
            ("prop", "text"),
            ("disableeditsection", "1"),
            ("disablelimitreport", "1"),
        ],
    )
    .await?;
    let paragraphs = paragraphs(text["text"].as_str().unwrap_or_default());
    let sections = parse(http, host, &[("page", title), ("prop", "sections")]).await?;
    let sections = sections["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["toclevel"] == 1)
        .map(|s| Section {
            title: strip_tags(s["line"].as_str().unwrap_or_default()),
            anchor: s["anchor"].as_str().unwrap_or_default().to_string(),
            index: s["index"].as_str().unwrap_or_default().to_string(),
        })
        .collect();
    Ok(Article {
        title: title.to_string(),
        links: links(http, host, &paragraphs).await?,
        paragraphs,
        sections,
        concept_sections: IndexMap::new(),
        section_text: IndexMap::new(),
    })
}

/// The first paragraph of one section of an article.
async fn section_text(
    http: &Http,
    host: &str,
    title: &str,
    index: &str,
) -> Result<Option<String>, FetchError> {
    let text = parse(
        http,
        host,
        &[
            ("page", title),
            ("section", index),
            ("prop", "text"),
            ("disableeditsection", "1"),
            ("disablelimitreport", "1"),
        ],
    )
    .await?;
    Ok(paragraphs(text["text"].as_str().unwrap_or_default())
        .into_iter()
        .next())
}

/// The sections whose title is a label or an alias of a concept, in the
/// language of the article; then the first paragraph of each, and the
/// items of their links. No other matching: a title that names no concept
/// stays unmatched.
pub async fn match_sections(
    http: &Http,
    host: &str,
    article: &mut Article,
    concepts: &IndexMap<Qid, Concept>,
) -> Result<(), FetchError> {
    for (qid, concept) in concepts {
        let wanted: Vec<String> = concept.names.iter().map(|n| n.to_lowercase()).collect();
        let hits: Vec<String> = article
            .sections
            .iter()
            .filter(|s| wanted.contains(&s.title.to_lowercase()))
            .map(|s| s.anchor.clone())
            .collect();
        if !hits.is_empty() {
            article.concept_sections.insert(*qid, hits);
        }
    }
    let matched: Vec<&Section> = article
        .sections
        .iter()
        .filter(|s| {
            article
                .concept_sections
                .values()
                .flatten()
                .any(|a| *a == s.anchor)
        })
        .collect();
    let texts = join_all(matched.iter().map(|s| async {
        (
            s.anchor.clone(),
            section_text(http, host, &article.title, &s.index).await,
        )
    }))
    .await;
    let mut found = Vec::new();
    for (anchor, text) in texts {
        if let Some(text) = text? {
            found.push(text.clone());
            article.section_text.insert(anchor, text);
        }
    }
    let more = links(http, host, &found).await?;
    article.links.extend(more);
    Ok(())
}

/// The Wikidata item of every article that the paragraphs link to, by
/// title, following redirects and normalization back to the linked title.
async fn links(
    http: &Http,
    host: &str,
    paragraphs: &[String],
) -> Result<IndexMap<String, Qid>, FetchError> {
    let mut titles: Vec<String> = paragraphs
        .iter()
        .flat_map(|p| LINK.captures_iter(p).map(|c| c[1].to_string()))
        .map(|t| {
            percent_encoding::percent_decode_str(&t)
                .decode_utf8_lossy()
                .replace('_', " ")
        })
        .filter(|t| !t.contains(':'))
        .collect();
    titles.sort();
    titles.dedup();
    let mut by_title = IndexMap::new();
    for batch in titles.chunks(BATCH) {
        let joined = batch.join("|");
        let json = http
            .json(
                Service::Api,
                &api(host),
                &[
                    ("action", "query"),
                    ("format", "json"),
                    ("formatversion", "2"),
                    ("prop", "pageprops"),
                    ("ppprop", "wikibase_item"),
                    ("redirects", "1"),
                    ("titles", &joined),
                ],
            )
            .await?;
        let query = &json["query"];
        let mut back: IndexMap<String, String> = IndexMap::new();
        for key in ["redirects", "normalized"] {
            for r in query[key].as_array().into_iter().flatten() {
                if let (Some(to), Some(from)) = (r["to"].as_str(), r["from"].as_str()) {
                    back.insert(to.to_string(), from.to_string());
                }
            }
        }
        for page in query["pages"].as_array().into_iter().flatten() {
            let Some(qid) = page["pageprops"]["wikibase_item"]
                .as_str()
                .and_then(|q| q.parse::<Qid>().ok())
            else {
                continue;
            };
            let mut title = page["title"].as_str().map(str::to_string);
            // The page, and every title that led to it.
            let mut seen = 0;
            while let Some(t) = title {
                by_title.insert(t.clone(), qid);
                title = back.get(&t).cloned();
                seen += 1;
                if seen > 10 {
                    break;
                }
            }
        }
    }
    Ok(by_title)
}

/// The page image of each item from its article: the page language first,
/// then English. A failed request is no image.
pub async fn page_images(http: &Http, qids: &[Qid], lang: &Lang) -> IndexMap<Qid, PageImage> {
    let mut codes = vec![lang.to_string()];
    if lang.as_str() != "en" {
        codes.push("en".to_string());
    }
    let found = join_all(qids.iter().map(|qid| {
        let codes = codes.clone();
        async move {
            for code in &codes {
                let url = format!("{DISPLAY_IMAGE}{qid}");
                let Ok(json) = http
                    .json(
                        Service::Api,
                        &url,
                        &[("lang", code.as_str()), ("fallback", "none")],
                    )
                    .await
                else {
                    continue;
                };
                let Some(file) = json["file"].as_str() else {
                    continue;
                };
                return Some((
                    *qid,
                    PageImage {
                        file: file.strip_prefix("File:").unwrap_or(file).to_string(),
                        width: json["original"]["width"].as_u64().unwrap_or(0) as u32,
                        height: json["original"]["height"].as_u64().unwrap_or(0) as u32,
                        artist: json["artist"].as_str().unwrap_or_default().to_string(),
                        license: json["license"].as_str().unwrap_or_default().to_string(),
                        source: format!("{code}.wikipedia.org"),
                    },
                ));
            }
            None
        }
    }))
    .await;
    found.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_without_tables_references_and_errors() {
        let html = concat!(
            "<table><tr><td><p>in a table</p></td></tr></table>",
            "<p>Nairobi<sup class=\"reference\"><a>[1]</a></sup> is a city.</p>",
            "<p> </p>",
            "<p><span class=\"error mw-ext-cite-error\">Cite error: ...</span></p>",
            "<style>.x{}</style><p>Second.</p>"
        );
        assert_eq!(paragraphs(html), ["Nairobi is a city.", "Second."]);
    }
}
