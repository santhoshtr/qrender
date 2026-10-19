//! Wikipedia: an article from one Parsoid parse of the whole page (its
//! lead, its sections, the first paragraph of a section, the whole body
//! of each section: the outline), the items its links point to; page
//! images.

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

/// An article from one Parsoid parse of the whole page: its lead, its
/// top-level sections, the sections whose title names a concept and the
/// first paragraph of each, and the item of each article that this text
/// links to. `outline`: also the sections with their content, for a page
/// that shows them.
pub async fn article(
    http: &Http,
    host: &str,
    title: &str,
    concepts: &IndexMap<Qid, Concept>,
    outline: bool,
) -> Result<Article, FetchError> {
    let text = parse(
        http,
        host,
        &[
            ("page", title),
            ("prop", "text"),
            ("parsoid", "1"),
            ("disableeditsection", "1"),
            ("disablelimitreport", "1"),
        ],
    )
    .await?;
    let parsed = read(text["text"].as_str().unwrap_or_default());
    let sections: Vec<Section> = parsed
        .outline
        .iter()
        .map(|s| Section {
            title: s.title.clone(),
            anchor: s.anchor.clone(),
        })
        .collect();
    let concept_sections = match_sections(&sections, concepts);
    let section_text: IndexMap<String, String> = sections
        .iter()
        .filter(|s| concept_sections.values().flatten().any(|a| *a == s.anchor))
        .filter_map(|s| Some((s.anchor.clone(), parsed.first.get(&s.anchor)?.clone())))
        .collect();
    let outline = if outline { parsed.outline } else { Vec::new() };
    let mut texts: Vec<String> = parsed
        .lead
        .iter()
        .chain(section_text.values())
        .cloned()
        .collect();
    collect_blocks(&outline, &mut texts);
    Ok(Article {
        title: title.to_string(),
        links: links(http, host, &texts).await?,
        paragraphs: parsed.lead,
        sections,
        concept_sections,
        section_text,
        outline,
    })
}

/// The sections whose title is a label or an alias of a concept, in the
/// language of the article. No other matching: a title that names no
/// concept stays unmatched.
fn match_sections(
    sections: &[Section],
    concepts: &IndexMap<Qid, Concept>,
) -> IndexMap<Qid, Vec<String>> {
    let mut out = IndexMap::new();
    for (qid, concept) in concepts {
        let wanted: Vec<String> = concept.names.iter().map(|n| n.to_lowercase()).collect();
        let hits: Vec<String> = sections
            .iter()
            .filter(|s| wanted.contains(&s.title.to_lowercase()))
            .map(|s| s.anchor.clone())
            .collect();
        if !hits.is_empty() {
            out.insert(*qid, hits);
        }
    }
    out
}

fn collect_blocks(sections: &[OutlineSection], out: &mut Vec<String>) {
    for s in sections {
        out.extend(s.blocks.iter().cloned());
        collect_blocks(&s.sections, out);
    }
}

/// What a page reads from the Parsoid HTML of an article.
struct Parsed {
    /// The lead: the paragraphs of section 0.
    lead: Vec<String>,
    /// The sections with a heading, and their content.
    outline: Vec<OutlineSection>,
    /// The first paragraph of each section, by its anchor.
    first: IndexMap<String, String>,
}

fn read(html: &str) -> Parsed {
    let doc = Html::parse_fragment(html);
    let mut lead = Vec::new();
    let mut first = IndexMap::new();
    let sections = doc
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "section");
    for el in sections {
        if el.value().attr("data-mw-section-id") == Some("0") {
            lead = paragraphs(el).collect();
        } else if let Some(heading) = el.child_elements().find_map(heading)
            && let Some(anchor) = heading.value().attr("id")
            && let Some(paragraph) = paragraphs(el).next()
        {
            first.insert(anchor.to_string(), paragraph);
        }
    }
    Parsed {
        lead,
        outline: outline_of(&doc),
        first,
    }
}

/// The paragraphs in an element, its subsections too, in order: each as
/// its content, cleaned. Not a paragraph in a table, an empty one, or one
/// with a message of the wiki parser ("Cite error: …"). Parsoid marks a
/// paragraph of templates only as empty (`mw-empty-elt`).
fn paragraphs(el: ElementRef) -> impl Iterator<Item = String> {
    el.descendants()
        .filter_map(ElementRef::wrap)
        .filter(|p| p.value().name() == "p")
        .filter(move |p| !within(*p, el, |a| a.value().name() == "table"))
        .filter(|p| !class_has(*p, "mw-empty-elt"))
        .filter(|p| !has_error(*p))
        .map(|p| {
            let mut out = String::new();
            write_children(p, &mut out);
            out
        })
        .filter(|p| !strip_tags(p).trim().is_empty())
}

/// An ancestor of `el`, below `top`, is such an element.
fn within(el: ElementRef, top: ElementRef, test: impl Fn(ElementRef) -> bool) -> bool {
    el.ancestors()
        .take_while(|a| a.id() != top.id())
        .filter_map(ElementRef::wrap)
        .any(test)
}

fn class_has(el: ElementRef, name: &str) -> bool {
    el.value()
        .attr("class")
        .is_some_and(|c| c.split_whitespace().any(|w| w == name))
}

/// A message of the wiki parser in a paragraph, outside its references.
fn has_error(p: ElementRef) -> bool {
    let reference = |e: ElementRef| e.value().name() == "sup" && class_has(e, "reference");
    p.descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| class_has(*e, "error") || class_has(*e, "mw-ext-cite-error"))
        .any(|e| !within(e, p, reference))
}

/// The sections of Parsoid HTML. Parsoid wraps each section in
/// `<section>`, nested by level, with its heading first (in a
/// `div.mw-heading`, or bare). Section 0, the lead, has no heading. The
/// parse API puts them all in a `div.mw-parser-output`.
fn outline_of(doc: &Html) -> Vec<OutlineSection> {
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
    write_children(el, out);
    out.push_str("</");
    out.push_str(name);
    out.push('>');
}

fn write_children(el: ElementRef, out: &mut String) {
    for child in el.children() {
        if let Some(child) = ElementRef::wrap(child) {
            write_element(child, out);
        } else if let Some(text) = child.value().as_text() {
            out.push_str(&escape(text));
        }
    }
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

    /// A lead, as Parsoid gives it, around some paragraphs.
    fn lead_of(paragraphs: &str) -> Vec<String> {
        read(&format!(
            "<div class=\"mw-parser-output\"><section data-mw-section-id=\"0\">{paragraphs}</section></div>"
        ))
        .lead
    }

    #[test]
    fn paragraphs_without_tables_references_and_errors() {
        let html = concat!(
            "<table><tr><td><p>in a table</p></td></tr></table>",
            "<p>Nairobi<sup class=\"reference\"><a>[1]</a></sup> is a city.</p>",
            "<p> </p>",
            "<p><span class=\"error mw-ext-cite-error\">Cite error: ...</span></p>",
            "<style>.x{}</style><p>Second.</p>",
            "<div class=\"x\"><p>In a box.</p></div>"
        );
        assert_eq!(
            lead_of(html),
            ["Nairobi is a city.", "Second.", "In a box."]
        );
    }

    #[test]
    fn an_error_in_a_reference_keeps_the_paragraph() {
        let html = concat!(
            "<p>Text<sup class=\"mw-ref reference\"><a href=\"#cite_note-1\">",
            "<span class=\"mw-ext-cite-error\">[1]</span></a></sup>.</p>"
        );
        assert_eq!(lead_of(html), ["Text."]);
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
            "<div class=\"div-col\"><ul><li>A</li></ul></div>",
            "<section data-mw-section-id=\"4\"><h3 id=\"Famous\">Famous</h3>",
            "<p>Later.</p></section></section>",
            "</div>",
        );
        let early = OutlineSection {
            title: "Early years".into(),
            anchor: "Early_years".into(),
            level: 3,
            blocks: vec!["<ul><li>One &amp; two</li></ul>".into()],
            sections: vec![],
        };
        let parsed = read(html);
        assert_eq!(parsed.lead, ["The lead."]);
        // A section without a paragraph of its own: the first one of its
        // subsections.
        assert_eq!(
            parsed.first,
            IndexMap::from([
                (
                    "History".to_string(),
                    "Founded in <a href=\"https://en.wikipedia.org/wiki/1899\">1899</a>."
                        .to_string()
                ),
                ("People".to_string(), "Later.".to_string()),
                ("Famous".to_string(), "Later.".to_string()),
            ])
        );
        assert_eq!(
            parsed.outline,
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
                    sections: vec![OutlineSection {
                        title: "Famous".into(),
                        anchor: "Famous".into(),
                        level: 3,
                        blocks: vec!["<p>Later.</p>".into()],
                        sections: vec![],
                    }],
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
            lead_of(html),
            [concat!(
                "<b>Kenya</b>, is a ",
                "<a href=\"https://en.wikipedia.org/wiki/Country\">country</a>."
            )]
        );
    }
}
