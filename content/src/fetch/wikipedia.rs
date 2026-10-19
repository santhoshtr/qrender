//! Wikipedia: the lead of an article, its sections, the first paragraph
//! of a section, the whole body of a section (the outline), the items its
//! links point to; page images.

use std::sync::LazyLock;

use futures::future::join_all;
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value as Json;

use super::FetchError;
use super::http::{Http, Service};
use scraper::{ElementRef, Html};

use crate::html::{escape, strip_tags, wiki_target};
use crate::id::{Lang, Qid};
use crate::text::{Article, Concept, OutlineSection, PageImage, Section};

const BATCH: usize = 50;
const DISPLAY_IMAGE: &str = "https://wiki-display-image.toolforge.org/json/wikidata/";

static TABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<table.*?</table>").unwrap());
static REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<sup[^>]*class="[^"]*reference[^"]*".*?</sup>"#).unwrap());
static STYLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<style.*?</style>").unwrap());
/// A paragraph: `<p>` (legacy parser) or `<p id="…">` (Parsoid).
static PARAGRAPH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<p(\s[^>]*)?>(.*?)</p>").unwrap());
static HREF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"href="([^"]*)""#).unwrap());

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
/// removed; empty paragraphs and parser error messages left out. Parsoid
/// marks a paragraph of templates only as empty (`mw-empty-elt`).
fn paragraphs(html: &str) -> Vec<String> {
    let html = TABLE.replace_all(html, "");
    let html = REFERENCE.replace_all(&html, "");
    let html = STYLE.replace_all(&html, "");
    PARAGRAPH
        .captures_iter(&html)
        .filter(|c| {
            !c.get(1)
                .is_some_and(|a| a.as_str().contains("mw-empty-elt"))
        })
        .map(|c| c[2].to_string())
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
        outline: Vec::new(),
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

/// The sections of an article with their content (not the lead), from
/// its Parsoid HTML, and the items of the articles that they link to.
pub async fn outline(http: &Http, host: &str, article: &mut Article) -> Result<(), FetchError> {
    let text = parse(
        http,
        host,
        &[
            ("page", &article.title),
            ("prop", "text"),
            ("parsoid", "1"),
            ("disableeditsection", "1"),
            ("disablelimitreport", "1"),
        ],
    )
    .await?;
    let outline = outline_of(text["text"].as_str().unwrap_or_default());
    let mut blocks = Vec::new();
    collect_blocks(&outline, &mut blocks);
    let more = links(http, host, &blocks).await?;
    article.links.extend(more);
    article.outline = outline;
    Ok(())
}

fn collect_blocks(sections: &[OutlineSection], out: &mut Vec<String>) {
    for s in sections {
        out.extend(s.blocks.iter().cloned());
        collect_blocks(&s.sections, out);
    }
}

/// The sections of Parsoid HTML. Parsoid wraps each section in
/// `<section>`, nested by level, with its heading first (in a
/// `div.mw-heading`, or bare). Section 0, the lead, has no heading. The
/// parse API puts them all in a `div.mw-parser-output`.
fn outline_of(html: &str) -> Vec<OutlineSection> {
    let doc = Html::parse_fragment(html);
    let wrapper = |e: &ElementRef| {
        e.value().name() == "div"
            && e.value()
                .attr("class")
                .is_some_and(|c| c.contains("mw-parser-output"))
    };
    let mut out = Vec::new();
    for el in doc.root_element().child_elements() {
        let top: Vec<ElementRef> = if wrapper(&el) {
            el.child_elements().collect()
        } else {
            vec![el]
        };
        for el in top.into_iter().filter(|e| e.value().name() == "section") {
            section(el, &mut out, &mut Vec::new());
        }
    }
    out
}

/// One `<section>`: a section with a heading goes in `out`; without one
/// (the lead, or a section that a template made), its blocks go in
/// `blocks` and its subsections in `out`.
fn section(el: ElementRef, out: &mut Vec<OutlineSection>, blocks: &mut Vec<String>) {
    let Some(heading) = el.child_elements().find_map(heading) else {
        for child in el.child_elements() {
            match child.value().name() {
                "section" => section(child, out, blocks),
                _ => block(child, blocks),
            }
        }
        return;
    };
    let mut node = OutlineSection {
        title: heading.text().collect::<String>().trim().to_string(),
        anchor: heading.value().attr("id").unwrap_or_default().to_string(),
        level: heading.value().name()[1..].parse().unwrap_or(2),
        blocks: Vec::new(),
        sections: Vec::new(),
    };
    for child in el.child_elements() {
        match child.value().name() {
            "section" => section(child, &mut node.sections, &mut node.blocks),
            _ => block(child, &mut node.blocks),
        }
    }
    out.push(node);
}

/// The heading of a section: `h2`–`h6`, bare or in a `div.mw-heading`.
fn heading(el: ElementRef) -> Option<ElementRef> {
    let is_heading = |e: &ElementRef| matches!(e.value().name(), "h2" | "h3" | "h4" | "h5" | "h6");
    if is_heading(&el) {
        return Some(el);
    }
    let class = el.value().attr("class").unwrap_or_default();
    (el.value().name() == "div" && class.contains("mw-heading"))
        .then(|| el.child_elements().find(is_heading))
        .flatten()
}

/// A block of text that a reader reads: a paragraph with text, a list, a
/// quotation; the lists of a multi-column list. Not a heading, a hatnote
/// ("Main article: …"), a table, a figure or a navigation box.
fn block(el: ElementRef, out: &mut Vec<String>) {
    let class = el.value().attr("class").unwrap_or_default();
    match el.value().name() {
        "p" if !class.contains("mw-empty-elt") => {
            let html = clean(el);
            if !strip_tags(&html).trim().is_empty() && !class.contains("error") {
                out.push(html);
            }
        }
        "ul" | "ol" | "dl" | "blockquote" => out.push(clean(el)),
        "div" if class.contains("div-col") => {
            for child in el.child_elements() {
                block(child, out);
            }
        }
        _ => {}
    }
}

/// An element as HTML without references, styles and metadata, and with no
/// attribute but `href` and `alt`. The localize step cleans it again; this
/// keeps the stored text small (Parsoid's attributes are most of it).
fn clean(el: ElementRef) -> String {
    let mut out = String::new();
    write_element(el, &mut out);
    out
}

fn write_element(el: ElementRef, out: &mut String) {
    let e = el.value();
    let name = e.name();
    let class = e.attr("class").unwrap_or_default();
    let reference = name == "sup" && class.contains("reference");
    // The other forms of a coordinate, which the CSS of the wiki hides.
    let hidden = class.contains("geo-nondefault") || class.contains("geo-multi-punct");
    if reference
        || hidden
        || matches!(
            name,
            "style" | "script" | "link" | "meta" | "table" | "figure"
        )
    {
        return;
    }
    out.push('<');
    out.push_str(name);
    for attribute in ["href", "alt"] {
        if let Some(value) = e.attr(attribute) {
            out.push_str(&format!(" {attribute}=\"{}\"", escape(value)));
        }
    }
    out.push('>');
    if matches!(name, "img" | "br" | "wbr") {
        return;
    }
    for child in el.children() {
        if let Some(child) = ElementRef::wrap(child) {
            write_element(child, out);
        } else if let Some(text) = child.value().as_text() {
            out.push_str(&escape(text));
        }
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
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
        .flat_map(|p| {
            HREF.captures_iter(p)
                .filter_map(|c| Some(wiki_target(c.get(1)?.as_str(), host)?.to_string()))
        })
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

    #[test]
    fn the_outline_of_parsoid_html() {
        let html = concat!(
            "<div class=\"mw-content-ltr mw-parser-output\" lang=\"en\">",
            "<section data-mw-section-id=\"0\"><p>The lead.</p></section>",
            "<section data-mw-section-id=\"1\"><div class=\"mw-heading mw-heading2\">",
            "<h2 id=\"History\">History</h2></div>",
            "<div role=\"note\" class=\"hatnote\">Main article: X</div>",
            "<p id=\"mwAB\" data-mw='{\"a\":\">\"}'>Founded in <a rel=\"mw:WikiLink\" ",
            "href=\"https://en.wikipedia.org/wiki/1899\" title=\"1899\">1899</a>",
            "<sup class=\"mw-ref reference\"><a href=\"#cite_note-1\">[1]</a></sup>.",
            "<style>.x{}</style></p>",
            "<p class=\"mw-empty-elt\"></p><table><tr><td>a table</td></tr></table>",
            "<section data-mw-section-id=\"2\"><h3 id=\"Early_years\">Early years</h3>",
            "<ul><li>One &amp; two</li></ul></section>",
            "<section data-mw-section-id=\"-1\"><p>From a template.</p></section>",
            "</section>",
            "<section data-mw-section-id=\"3\"><h2 id=\"People\">People</h2>",
            "<div class=\"div-col\"><ul><li>A</li></ul></div></section>",
            "</div>",
        );
        let early = OutlineSection {
            title: "Early years".into(),
            anchor: "Early_years".into(),
            level: 3,
            blocks: vec!["<ul><li>One &amp; two</li></ul>".into()],
            sections: vec![],
        };
        assert_eq!(
            outline_of(html),
            [
                OutlineSection {
                    title: "History".into(),
                    anchor: "History".into(),
                    level: 2,
                    blocks: vec![
                        concat!(
                            "<p>Founded in <a href=\"https://en.wikipedia.org/wiki/1899\">",
                            "1899</a>.</p>"
                        )
                        .into(),
                        "<p>From a template.</p>".into(),
                    ],
                    sections: vec![early],
                },
                OutlineSection {
                    title: "People".into(),
                    anchor: "People".into(),
                    level: 2,
                    blocks: vec!["<ul><li>A</li></ul>".into()],
                    sections: vec![],
                },
            ]
        );
    }

    #[test]
    fn parsoid_paragraphs() {
        let html = concat!(
            "<p class=\"mw-empty-elt\" id=\"mwCA\"><span typeof=\"mw:Transclusion\"></span>",
            "<link rel=\"mw:PageProp/Category\" href=\"./Category:X\"/></p>\n",
            "<p id=\"mwTQ\"><b id=\"mwTg\">Kenya</b>,<sup about=\"#mwt91\" ",
            "class=\"mw-ref reference\" id=\"cite_ref-12\"><a href=\"#cite_note-12\">",
            "<span class=\"mw-reflink-text\">[a]</span></a></sup> is a ",
            "<a rel=\"mw:WikiLink\" href=\"https://en.wikipedia.org/wiki/Country\">country</a>.</p>",
            "<pre>not a paragraph</pre>"
        );
        assert_eq!(
            paragraphs(html),
            [concat!(
                "<b id=\"mwTg\">Kenya</b>, is a <a rel=\"mw:WikiLink\" ",
                "href=\"https://en.wikipedia.org/wiki/Country\">country</a>."
            )]
        );
    }
}
