//! The notes of an article, from its Parsoid HTML, with each reference as
//! typed citations. In a note, the citations come from, in order:
//! 1. a template of a citation style: its parameters are in the
//!    `data-mw` of the first node of the transclusion;
//! 2. COinS (`span.Z3988`): a citation that a template made inside
//!    another template (the infobox) has no template data of its own;
//! 3. else the note is one citation: its text, as HTML; or, when the text
//!    is only a link to a work of the article and a place ("Faingold
//!    2018, p. 8"), a short footnote. The works are read from their
//!    `<cite id>` as notes are (template, else COinS, else text).
//!
//! The input is wild: no note fails, and every note gives a citation.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use indexmap::IndexMap;
use percent_encoding::percent_decode_str;
use regex::Regex;
use scraper::{ElementRef, Html};
use serde_json::Value as Json;

use super::wikipedia::{class_has, write_children, write_element};
use crate::citation::{
    Archive, Book, Citation, Cited, Date, Identifier, Journal, News, Origin, Original, Person,
    Short, Web, WebUrl, Work,
};
use crate::html::{decode_entities, escape, strip_tags};
use crate::id::Lang;
use crate::settings::{AuthorParameters, ByKind, CitationStyle, Parameters};
use crate::text::Note;

/// The authors of a citation, at most.
const AUTHORS_MAX: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Web,
    News,
    Journal,
    Book,
}

/// A short footnote, cleaned: one link to a place in the page, then maybe
/// the place in the work, then maybe a full stop.
static SHORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r##"(?s)^<a href="#([^"]+)">(.*?)</a>(.*)$"##).unwrap());
/// What follows the link: ", p. 8", ", pp. 2–3", then maybe ".".
static AT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:,\s*(pp?\.\s*[^.]+?))?\s*\.?$").unwrap());

/// The notes of an article and the works that their short footnotes
/// cite.
pub struct ArticleNotes {
    /// By the id of its item in a list of notes.
    pub notes: IndexMap<String, Note>,
    /// By the anchor of the work.
    pub works: IndexMap<String, Cited>,
}

/// Every note of an article, from its Parsoid HTML, and the works that
/// they cite.
pub fn article_notes(html: &str, styles: &[CitationStyle]) -> ArticleNotes {
    let doc = Html::parse_fragment(html);
    let notes = notes(&doc, styles);
    let works = works(&doc, styles, &cited_works(notes.values()));
    ArticleNotes { notes, works }
}

/// The anchors of the works that short footnotes of these notes cite.
pub fn cited_works<'a>(notes: impl IntoIterator<Item = &'a Note>) -> BTreeSet<String> {
    notes
        .into_iter()
        .flat_map(Note::works)
        .map(str::to_string)
        .collect()
}

/// The works of these anchors: each `<cite id>` read as a citation of a
/// note is (its template, else its COinS, else its text). An anchor that
/// the page does not have is not there.
pub fn works(
    doc: &Html,
    styles: &[CitationStyle],
    anchors: &BTreeSet<String>,
) -> IndexMap<String, Cited> {
    let mut out = IndexMap::new();
    let cites = doc
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "cite");
    for cite in cites {
        let Some(id) = cite.value().attr("id").filter(|id| anchors.contains(*id)) else {
            continue;
        };
        if out.contains_key(id) {
            continue;
        }
        let work = template_of(cite, styles)
            .or_else(|| {
                let span = cite
                    .next_siblings()
                    .find_map(ElementRef::wrap)
                    .filter(|e| is_coins(*e))?;
                coins(span, styles)
            })
            .unwrap_or_else(|| Cited {
                citation: Citation::Text {
                    html: html_of(cite),
                },
                origin: Origin::Html,
            });
        out.insert(id.to_string(), work);
    }
    out
}

/// The citation of a `<cite>` from the template that made it: the first
/// node of its transclusion has the data (the `<cite>` itself, or a node
/// before it with the same `about`).
fn template_of(cite: ElementRef, styles: &[CitationStyle]) -> Option<Cited> {
    let about = cite.value().attr("about")?;
    let head = std::iter::once(cite)
        .chain(cite.prev_siblings().filter_map(ElementRef::wrap))
        .take_while(|e| e.value().attr("about") == Some(about))
        .filter_map(transclusion)
        .next()?;
    head.iter().find_map(|part| {
        let (style, kind) = styles
            .iter()
            .find_map(|s| Some((s, kind_of(&s.templates, &part.name)?)))?;
        Some(Cited {
            citation: from_params(kind, &style.parameters, &part.params),
            origin: Origin::Template {
                name: part.name.clone(),
                params: part.params.clone(),
            },
        })
    })
}

/// A short footnote: the cleaned HTML of a note that is one link to a
/// place in the page and maybe a place in the work.
fn short(html: &str) -> Option<Short> {
    let found = SHORT.captures(html)?;
    let rest = decode_entities(&strip_tags(&found[3])).replace('\u{a0}', " ");
    let at = AT.captures(rest.trim())?;
    let label = decode_entities(&strip_tags(&found[2])).replace('\u{a0}', " ");
    Some(Short {
        work: decode_entities(&found[1]),
        label: label.split_whitespace().collect::<Vec<_>>().join(" "),
        at: at
            .get(1)
            .map(|a| a.as_str().split_whitespace().collect::<Vec<_>>().join(" ")),
    })
}

/// Every note of the article, by the id of its item in a list of notes.
pub fn notes(doc: &Html, styles: &[CitationStyle]) -> IndexMap<String, Note> {
    let mut out = IndexMap::new();
    let lists = doc
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "ol" && class_has(*e, "mw-references"));
    for list in lists {
        let group = list
            .value()
            .attr("data-mw-group")
            .filter(|g| !g.is_empty())
            .map(str::to_string);
        for item in list.child_elements().filter(|e| e.value().name() == "li") {
            let Some(id) = item.value().attr("id") else {
                continue;
            };
            let body = item
                .descendants()
                .filter_map(ElementRef::wrap)
                .find(|e| class_has(*e, "mw-reference-text"));
            let Some(body) = body else {
                continue;
            };
            out.insert(id.to_string(), note(body, group.clone(), styles));
        }
    }
    out
}

fn note(body: ElementRef, group: Option<String>, styles: &[CitationStyle]) -> Note {
    if group.is_some() {
        // An explanatory note: text to read, not a source.
        return Note {
            group,
            citations: Vec::new(),
            text: Some(html_of(body)),
        };
    }
    let mut citations = citations(body, styles);
    if citations.is_empty() {
        let html = html_of(body);
        citations.push(Cited {
            citation: short(&html).map_or_else(|| Citation::Text { html }, Citation::Short),
            origin: Origin::Html,
        });
    }
    Note {
        group: None,
        citations,
        text: None,
    }
}

/// The citations of a note, in order. The text between them, when it has
/// words ("See also"), is a citation of its own.
fn citations(body: ElementRef, styles: &[CitationStyle]) -> Vec<Cited> {
    let children: Vec<_> = body.children().collect();
    let mut out = Vec::new();
    let mut loose = String::new();
    let mut i = 0;
    while i < children.len() {
        let Some(el) = ElementRef::wrap(children[i]) else {
            if let Some(text) = children[i].value().as_text() {
                loose.push_str(&escape(text));
            }
            i += 1;
            continue;
        };
        i += 1;
        if let (Some(about), Some(parts)) = (el.value().attr("about"), transclusion(el)) {
            // The transclusion: this node and the next ones of the same
            // `about`.
            let mut group = vec![el];
            while let Some(next) = children.get(i).and_then(|c| ElementRef::wrap(*c)) {
                if next.value().attr("about") != Some(about) {
                    break;
                }
                group.push(next);
                i += 1;
            }
            read_transclusion(&parts, &group, styles, &mut out);
        } else if is_coins(el) {
            out.extend(coins(el, styles));
        } else if el.value().name() == "cite"
            && el
                .next_siblings()
                .find_map(ElementRef::wrap)
                .is_some_and(is_coins)
        {
            // Its COinS reads it.
        } else {
            write_element(el, &mut loose);
        }
    }
    if !out.is_empty() && has_words(&loose) {
        out.push(Cited {
            citation: Citation::Text {
                html: loose.trim().to_string(),
            },
            origin: Origin::Html,
        });
    }
    out
}

/// A template in a transclusion: its name and its parameters, as
/// wikitext.
struct Part {
    name: String,
    params: IndexMap<String, String>,
}

/// The templates of a transclusion, from the `data-mw` of its first node.
/// None when the node is not one, or its data is not JSON.
fn transclusion(el: ElementRef) -> Option<Vec<Part>> {
    let types = el.value().attr("typeof")?;
    if !types.split_whitespace().any(|t| t == "mw:Transclusion") {
        return None;
    }
    let data: Json = serde_json::from_str(el.value().attr("data-mw")?).ok()?;
    let parts = data["parts"].as_array()?;
    Some(
        parts
            .iter()
            .filter_map(|p| {
                let template = p.get("template")?;
                Some(Part {
                    name: template["target"]["wt"].as_str()?.trim().to_string(),
                    params: template["params"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .filter_map(|(k, v)| Some((k.clone(), v["wt"].as_str()?.to_string())))
                        .collect(),
                })
            })
            .collect(),
    )
}

/// The citations of a transclusion: each template of a style, else the
/// COinS that the templates made.
fn read_transclusion(
    parts: &[Part],
    group: &[ElementRef],
    styles: &[CitationStyle],
    out: &mut Vec<Cited>,
) {
    let before = out.len();
    for part in parts {
        let Some((style, kind)) = styles
            .iter()
            .find_map(|s| Some((s, kind_of(&s.templates, &part.name)?)))
        else {
            continue;
        };
        out.push(Cited {
            citation: from_params(kind, &style.parameters, &part.params),
            origin: Origin::Template {
                name: part.name.clone(),
                params: part.params.clone(),
            },
        });
    }
    if out.len() == before {
        let spans = group
            .iter()
            .flat_map(|el| el.descendants().filter_map(ElementRef::wrap))
            .filter(|e| is_coins(*e));
        out.extend(spans.filter_map(|span| coins(span, styles)));
    }
}

/// A name as MediaWiki compares template names, and without regard to
/// case: "Cite_web " is "cite web".
fn normalize(name: &str) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn kind_of(by: &ByKind, name: &str) -> Option<Kind> {
    let name = normalize(name);
    [
        (Kind::Web, &by.web),
        (Kind::News, &by.news),
        (Kind::Journal, &by.journal),
        (Kind::Book, &by.book),
    ]
    .into_iter()
    .find(|(_, names)| names.iter().any(|n| normalize(n) == name))
    .map(|(kind, _)| kind)
}

// ------------------------------------------------------------ templates

fn from_params(kind: Kind, p: &Parameters, params: &IndexMap<String, String>) -> Citation {
    let get = |names: &[String]| names.iter().find_map(|n| plain(params.get(n)?));
    let work = Work {
        title: get(&p.title),
        url: get(&p.url).and_then(|u| u.parse().ok()),
        authors: authors(&p.authors, params),
        date: get(&p.date).map(|d| Date::parse(&d)),
        accessed: get(&p.accessed).map(|d| Date::parse(&d)),
        archive: archive(p, params),
        language: get(&p.language).and_then(|l| language(&l)),
        ids: ids(p, params),
    };
    let container = get(&p.container);
    let publisher = get(&p.publisher);
    match kind {
        Kind::Web => Citation::Web(Web {
            work,
            website: container,
            publisher,
        }),
        Kind::News => Citation::News(News {
            work,
            newspaper: container,
            publisher,
        }),
        Kind::Journal => Citation::Journal(Journal {
            work,
            journal: container,
            volume: get(&p.volume),
            issue: get(&p.issue),
            pages: get(&p.pages),
        }),
        Kind::Book => Citation::Book(Book {
            work,
            publisher,
            place: get(&p.place),
            pages: get(&p.pages),
        }),
    }
}

/// The authors, by number: a family name with a given name, or a name as
/// one text. The first number without an author ends the list.
fn authors(a: &AuthorParameters, params: &IndexMap<String, String>) -> Vec<Person> {
    let mut out = Vec::new();
    for n in 1..=AUTHORS_MAX {
        let get = |names: &[String]| {
            names
                .iter()
                .flat_map(|name| numbered(name, n))
                .find_map(|k| plain(params.get(&k)?))
        };
        let person = match (get(&a.family), get(&a.given), get(&a.name)) {
            (Some(family), Some(given), _) => Person::Name {
                family,
                given: Some(given),
            },
            (Some(name), None, _) | (None, _, Some(name)) => Person::Literal { name },
            _ => break,
        };
        out.push(person);
    }
    out
}

/// The parameter names of author `n`: "last" and "last1" for the first.
fn numbered(name: &str, n: usize) -> Vec<String> {
    match (n, name.contains("{n}")) {
        (1, _) => vec![name.replace("{n}", ""), name.replace("{n}", "1")],
        (_, true) => vec![name.replace("{n}", &n.to_string())],
        (_, false) => Vec::new(),
    }
}

fn archive(p: &Parameters, params: &IndexMap<String, String>) -> Option<Archive> {
    let get = |names: &[String]| names.iter().find_map(|n| plain(params.get(n)?));
    let a = &p.archive;
    Some(Archive {
        url: get(&a.url)?.parse().ok()?,
        date: get(&a.date).map(|d| Date::parse(&d)),
        // Without a state, an archived source is dead (as CS1 shows it).
        original: match get(&a.status).map(|s| s.to_lowercase()).as_deref() {
            Some("live") => Original::Live,
            Some("usurped" | "unfit") => Original::Unfit,
            _ => Original::Dead,
        },
    })
}

fn ids(p: &Parameters, params: &IndexMap<String, String>) -> Vec<Identifier> {
    let get = |names: &[String]| names.iter().find_map(|n| plain(params.get(n)?));
    let i = &p.ids;
    [
        (get(&i.doi), Identifier::Doi as fn(String) -> Identifier),
        (get(&i.isbn), Identifier::Isbn),
        (get(&i.issn), Identifier::Issn),
        (get(&i.oclc), Identifier::Oclc),
        (get(&i.jstor), Identifier::Jstor),
        (get(&i.s2cid), Identifier::S2cid),
        (get(&i.bibcode), Identifier::Bibcode),
        (get(&i.hdl), Identifier::Hdl),
    ]
    .into_iter()
    .filter_map(|(value, id)| Some(id(value?)))
    .collect()
}

/// A language code ("en", "en-US", "EN"); not a name ("English").
fn language(text: &str) -> Option<Lang> {
    let code = text.split([',', ';', ' ']).next()?.trim().to_lowercase();
    let primary = code.split('-').next()?;
    if !(2..=3).contains(&primary.len()) {
        return None;
    }
    code.parse().ok()
}

/// The text of a wikitext value: comments, templates, tags and quote
/// marks removed; a link as its text; entities decoded; spaces collapsed.
/// `{{!}}` is a "|". None when no text is left.
pub fn plain(wikitext: &str) -> Option<String> {
    let text = remove_between(wikitext, "<!--", "-->");
    let text = remove_templates(&text.replace("{{!}}", "|"));
    let text = wiki_links(&text);
    let text = external_links(&text);
    let text = text.replace("'''", "").replace("''", "");
    let text = decode_entities(&strip_tags(&text));
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

/// The text without the parts from `open` to `close`; an unclosed part to
/// the end.
fn remove_between(text: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find(close) {
            Some(end) => &rest[start + end + close.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// The text without templates, nested ones too.
fn remove_templates(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut rest = text;
    while !rest.is_empty() {
        if rest.starts_with("{{") {
            depth += 1;
            rest = &rest[2..];
        } else if depth > 0 && rest.starts_with("}}") {
            depth -= 1;
            rest = &rest[2..];
        } else {
            let c = rest.chars().next().expect("not empty");
            if depth == 0 {
                out.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// `[[Target|text]]` as "text", `[[Target]]` as "Target".
fn wiki_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);
        let inner = &rest[start + 2..];
        let Some(end) = inner.find("]]") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let link = &inner[..end];
        out.push_str(link.rsplit('|').next().unwrap_or(link));
        rest = &inner[end + 2..];
    }
    out.push_str(rest);
    out
}

/// `[https://example.org text]` as "text"; a bare `[https://example.org]`
/// as nothing.
fn external_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let inner = &rest[start + 1..];
        let is_link = ["http://", "https://", "//"].iter().any(|p| {
            inner
                .get(..p.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(p))
        });
        match inner.find(']').filter(|_| is_link) {
            Some(end) => {
                out.push_str(inner[..end].split_once(' ').map_or("", |(_, t)| t));
                rest = &inner[end + 1..];
            }
            None => {
                out.push('[');
                rest = inner;
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- COinS

fn is_coins(el: ElementRef) -> bool {
    el.value().name() == "span" && class_has(el, "Z3988")
}

/// A citation from COinS: the OpenURL key-value text in the title of a
/// `span.Z3988`. The kind is from the `<cite>` before it (its class), else
/// from the genre. None without a title and a URL.
fn coins(span: ElementRef, styles: &[CitationStyle]) -> Option<Cited> {
    let pairs: Vec<(String, String)> = span
        .value()
        .attr("title")?
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            let value = percent_decode_str(&value.replace('+', " "))
                .decode_utf8_lossy()
                .into_owned();
            Some((key.to_string(), value))
        })
        .collect();
    let get = |key: &str| {
        pairs
            .iter()
            .find(|(k, v)| k == key && !v.trim().is_empty())
            .map(|(_, v)| v.trim().to_string())
    };
    let all = |key: &'static str| {
        pairs
            .iter()
            .filter(move |(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let cite = span
        .prev_siblings()
        .find_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "cite");
    let kind = cite
        .and_then(|c| {
            let class = c.value().attr("class")?;
            class
                .split_whitespace()
                .find_map(|word| styles.iter().find_map(|s| kind_of(&s.classes, word)))
        })
        .unwrap_or(match get("rft.genre").as_deref() {
            Some("book" | "bookitem") => Kind::Book,
            Some("article") => Kind::Journal,
            _ => Kind::Web,
        });
    let mut authors = Vec::new();
    if let Some(family) = get("rft.aulast") {
        authors.push(Person::Name {
            family,
            given: get("rft.aufirst"),
        });
    }
    authors.extend(
        all("rft.au")
            .filter(|a| !a.trim().is_empty())
            .map(|a| Person::Literal {
                name: a.trim().to_string(),
            }),
    );
    let mut ids = Vec::new();
    let mut url = None;
    for id in all("rft_id") {
        let prefixed = [
            ("info:doi/", Identifier::Doi as fn(String) -> Identifier),
            ("info:oclcnum/", Identifier::Oclc),
            ("info:bibcode/", Identifier::Bibcode),
            ("info:hdl/", Identifier::Hdl),
        ];
        if let Some(found) = prefixed
            .iter()
            .find_map(|(prefix, make)| Some(make(id.strip_prefix(prefix)?.to_string())))
        {
            ids.push(found);
        } else if url.is_none() {
            url = id.parse::<WebUrl>().ok();
        }
    }
    ids.extend(get("rft.isbn").map(Identifier::Isbn));
    ids.extend(
        get("rft.issn")
            .or_else(|| get("rft.eissn"))
            .map(Identifier::Issn),
    );
    let (first, second) = (get("rft.atitle"), get("rft.btitle"));
    let title = match kind {
        Kind::Book => second.or(first),
        _ => first.or(second),
    };
    if title.is_none() && url.is_none() {
        return None;
    }
    let work = Work {
        title,
        url,
        authors,
        date: get("rft.date").map(|d| Date::parse(&d)),
        ids,
        ..Work::default()
    };
    let pages = get("rft.pages").or_else(|| {
        let start = get("rft.spage")?;
        Some(match get("rft.epage") {
            Some(end) => format!("{start}–{end}"),
            None => start,
        })
    });
    let citation = match kind {
        Kind::Web => Citation::Web(Web {
            work,
            website: get("rft.jtitle"),
            publisher: get("rft.pub"),
        }),
        Kind::News => Citation::News(News {
            work,
            newspaper: get("rft.jtitle"),
            publisher: get("rft.pub"),
        }),
        Kind::Journal => Citation::Journal(Journal {
            work,
            journal: get("rft.jtitle"),
            volume: get("rft.volume"),
            issue: get("rft.issue"),
            pages,
        }),
        Kind::Book => Citation::Book(Book {
            work,
            publisher: get("rft.pub"),
            place: get("rft.place"),
            pages,
        }),
    };
    Some(Cited {
        citation,
        origin: Origin::Coins,
    })
}

// ----------------------------------------------------------------- text

/// The content of an element as cleaned HTML, with the marks of notes.
fn html_of(el: ElementRef) -> String {
    let mut out = String::new();
    write_children(el, &mut out);
    out.trim().to_string()
}

fn has_words(html: &str) -> bool {
    decode_entities(&strip_tags(html))
        .chars()
        .any(char::is_alphanumeric)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    /// The styles of the configuration, as the fetcher gets them.
    fn styles() -> Vec<CitationStyle> {
        #[derive(Deserialize)]
        struct File {
            style: Vec<CitationStyle>,
        }
        let file: File = toml::from_str(include_str!("../../../qrender/citations.toml")).unwrap();
        file.style
    }

    fn read(html: &str) -> IndexMap<String, Note> {
        article_notes(html, &styles()).notes
    }

    /// The citations of the only note of some HTML.
    fn only(html: &str) -> Vec<Cited> {
        let notes = read(html);
        assert_eq!(notes.len(), 1, "{notes:?}");
        notes[0].citations.clone()
    }

    /// A note around the content of a reference.
    fn note(content: &str) -> String {
        format!(
            "<ol class=\"mw-references\"><li id=\"cite_note-1\"><span class=\"mw-reference-text\">{content}</span></li></ol>"
        )
    }

    fn time(t: &str) -> Option<Date> {
        Some(Date::parse(t))
    }

    fn url(s: &str) -> WebUrl {
        s.parse().unwrap()
    }

    /// cite_note-10 of the English article of Nairobi.
    const TEMPLATE: &str = include_str!("../../../fixtures/notes/template.html");

    fn mombasa() -> Citation {
        Citation::Web(Web {
            work: Work {
                title: Some("History of Mombasa".into()),
                url: Some(url("http://www.mombasa.go.ke/history-of-mombasa/")),
                authors: vec![Person::Literal {
                    name: "mombasa.go.ke".into(),
                }],
                date: time("2018-07-28"),
                accessed: time("14 June 2019"),
                archive: Some(Archive {
                    url: url(
                        "https://web.archive.org/web/20210126151126/http://www.mombasa.go.ke/history-of-mombasa/",
                    ),
                    date: time("26 January 2021"),
                    original: Original::Dead,
                }),
                ..Work::default()
            },
            website: None,
            publisher: Some("Mombasa County".into()),
        })
    }

    #[test]
    fn a_template_on_the_deduplicated_style() {
        let notes = read(TEMPLATE);
        assert_eq!(notes.keys().collect::<Vec<_>>(), ["cite_note-10"]);
        let cited = &notes["cite_note-10"].citations;
        assert_eq!(cited.len(), 1);
        assert_eq!(cited[0].citation, mombasa());
        let Origin::Template { name, params } = &cited[0].origin else {
            panic!("{:?}", cited[0].origin);
        };
        assert_eq!(name, "cite web");
        assert_eq!(params.len(), 9);
        assert_eq!(params["access-date"], "14 June 2019");
    }

    #[test]
    fn a_template_on_the_style() {
        // The first citation of a page has the whole style sheet in place
        // of the link to it.
        let start = TEMPLATE
            .find("<link rel=\"mw-deduplicated-inline-style\"")
            .unwrap();
        let end = start + TEMPLATE[start..].find("/>").unwrap() + 2;
        let link = &TEMPLATE[start..end];
        let style = link
            .replace("<link rel=\"mw-deduplicated-inline-style\" href=\"mw-data:TemplateStyles:r1333433106\"", "<style")
            .replace("/>", ">.cite{}</style>");
        let html = TEMPLATE.replace(link, &style);
        assert_eq!(only(&html)[0].citation, mombasa());
    }

    #[test]
    fn a_journal() {
        let cited = only(include_str!("../../../fixtures/notes/journal.html"));
        let Citation::Journal(journal) = &cited[0].citation else {
            panic!("{:?}", cited[0].citation);
        };
        assert_eq!(
            journal.work.authors,
            [
                Person::Name {
                    family: "Martin".into(),
                    given: Some("A. M.".into())
                },
                Person::Name {
                    family: "Bezemer".into(),
                    given: Some("P. M.".into())
                },
            ]
        );
        assert_eq!(journal.work.date, time("2020"));
        assert_eq!(journal.journal.as_deref(), Some("Planning Perspectives"));
        assert_eq!(journal.volume.as_deref(), Some("35"));
        assert_eq!(journal.issue.as_deref(), Some("4"));
        assert_eq!(journal.pages.as_deref(), Some("609–634"));
        assert_eq!(
            journal.work.ids,
            [
                Identifier::Doi("10.1080/02665433.2019.1602785".into()),
                Identifier::S2cid("150702513".into()),
                Identifier::Bibcode("2020PlPer..35..609M".into()),
                Identifier::Hdl("11370/e9e3efbb-1753-41d6-a17a-c08d0f68f771".into()),
            ]
        );
        // An archive without a state: the original is dead, as CS1 says.
        assert_eq!(
            journal.work.archive.as_ref().unwrap().original,
            Original::Dead
        );
    }

    #[test]
    fn only_coins() {
        // cite_note-1: the infobox made this citation, so it has no
        // template data.
        let cited = only(include_str!("../../../fixtures/notes/coins.html"));
        assert_eq!(cited.len(), 1);
        assert_eq!(cited[0].origin, Origin::Coins);
        assert_eq!(
            cited[0].citation,
            Citation::Web(Web {
                work: Work {
                    title: Some("Nairobi County".into()),
                    url: Some(url(
                        "http://countytrak.infotrakresearch.com/nairobi-county/"
                    )),
                    ..Work::default()
                },
                website: Some("Infotrak".into()),
                publisher: None,
            })
        );
    }

    #[test]
    fn free_text() {
        let cited = only(include_str!("../../../fixtures/notes/text.html"));
        assert_eq!(
            cited,
            [Cited {
                citation: Citation::Text {
                    html: "Roger S. Greenway, Timothy M. Monsma, <i>Cities: missions' new frontier</i>, (Baker Book House: 1989), p.163.".into()
                },
                origin: Origin::Html,
            }]
        );
    }

    #[test]
    fn an_explanatory_note_with_a_reference() {
        let notes = read(include_str!("../../../fixtures/notes/lower-alpha.html"));
        let note = &notes["cite_note-2"];
        assert_eq!(note.group.as_deref(), Some("lower-alpha"));
        assert!(note.citations.is_empty());
        let text = note.text.as_deref().unwrap();
        assert!(text.starts_with("Some sources indicate Nairobi has an area of 703.9"));
        assert!(
            text.ends_with("mi)<sup data-note=\"cite_note-1\"></sup>"),
            "{text}"
        );
    }

    /// The transclusion of the template note, with another `about`.
    fn transclusion(about: &str) -> String {
        let body = TEMPLATE.find("<link rel=").unwrap();
        let end = TEMPLATE.rfind("</span></li>").unwrap();
        TEMPLATE[body..end].replace("#mwt66", about)
    }

    #[test]
    fn two_citations_and_text_in_one_note() {
        let html = note(&format!(
            "{}; {} See also the census.",
            transclusion("#mwt1"),
            transclusion("#mwt2")
        ));
        let cited = only(&html);
        assert_eq!(cited.len(), 3);
        assert_eq!(cited[0].citation, mombasa());
        assert_eq!(cited[1].citation, mombasa());
        assert_eq!(
            cited[2].citation,
            Citation::Text {
                html: ";  See also the census.".into()
            }
        );
    }

    #[test]
    fn data_that_is_not_json_goes_to_coins() {
        let html = TEMPLATE.replacen("data-mw='{", "data-mw='{{", 1);
        let cited = only(&html);
        assert_eq!(cited.len(), 1);
        assert_eq!(cited[0].origin, Origin::Coins);
        // The COinS of this cite web says "book" (`rft_val_fmt`); the class
        // of the <cite> decides: web.
        let Citation::Web(web) = &cited[0].citation else {
            panic!("{:?}", cited[0].citation);
        };
        assert_eq!(web.work.title.as_deref(), Some("History of Mombasa"));
        assert_eq!(web.publisher.as_deref(), Some("Mombasa County"));
    }

    #[test]
    fn a_transclusion_without_data_is_text() {
        let cited = only(&note(
            "<span about=\"#mwt9\" typeof=\"mw:Transclusion\">A book, 1999.</span>",
        ));
        assert_eq!(
            cited[0].citation,
            Citation::Text {
                html: "<span>A book, 1999.</span>".into()
            }
        );
    }

    /// A note with one CS1 template of these parameters.
    fn cite(template: &str, params: &str) -> String {
        note(&format!(
            "<link about=\"#mwt1\" typeof=\"mw:Transclusion\" data-mw='{{\"parts\":[{{\"template\":{{\"target\":{{\"wt\":\"{template}\"}},\"params\":{params}}}}}]}}'/>"
        ))
    }

    #[test]
    fn hostile_parameters() {
        let cited = only(&cite(
            "Cite_Web\\n ",
            r#"{"title":{"wt":""},"url":{"wt":"//example.org/a <!-- old -->"},
                "date":{"wt":"c. 1990"},"language":{"wt":"English"},
                "archive-url":{"wt":"javascript:alert(1)"},
                "author":{"wt":"<!--Not stated-->"},
                "website":{"wt":"[[National Geographic Society|National Geographic]]"}}"#,
        ));
        assert_eq!(
            cited[0].citation,
            Citation::Web(Web {
                work: Work {
                    url: Some(url("https://example.org/a")),
                    date: Some(Date::Text {
                        text: "c. 1990".into()
                    }),
                    ..Work::default()
                },
                website: Some("National Geographic".into()),
                publisher: None,
            })
        );
    }

    #[test]
    fn a_template_of_no_style_without_coins_is_text() {
        let html = cite("cite magazine", r#"{"title":{"wt":"A"}}"#)
            .replace("</span></li>", "Rendered text.</span></li>");
        assert_eq!(
            only(&html),
            [Cited {
                citation: Citation::Text {
                    html: "Rendered text.".into()
                },
                origin: Origin::Html,
            }]
        );
    }

    #[test]
    fn coins_of_an_unknown_genre() {
        let cited = only(&note(
            "<span class=\"Z3988\" title=\"rft.genre=thing&amp;rft.btitle=A+B&amp;rft_id=info%3Adoi%2F10.1%2Fx&amp;rft_id=https%3A%2F%2Fexample.org%2F\"></span>",
        ));
        assert_eq!(
            cited[0].citation,
            Citation::Web(Web {
                work: Work {
                    title: Some("A B".into()),
                    url: Some(url("https://example.org/")),
                    ids: vec![Identifier::Doi("10.1/x".into())],
                    ..Work::default()
                },
                ..Web::default()
            })
        );
    }

    #[test]
    fn empty_and_broken_notes() {
        let html = concat!(
            "<ol class=\"mw-references\">",
            "<li id=\"cite_note-1\"><span class=\"mw-reference-text\"></span></li>",
            "<li id=\"cite_note-2\">no text span</li>",
            "<li>no id</li>",
            "</ol>"
        );
        let notes = read(html);
        assert_eq!(notes.keys().collect::<Vec<_>>(), ["cite_note-1"]);
        assert_eq!(
            notes["cite_note-1"].citations[0].citation,
            Citation::Text { html: "".into() }
        );
    }

    #[test]
    fn short_footnotes_and_their_works() {
        // Three notes of the English article of the United States, and the
        // entries of its bibliography (COinS only: the list is in a
        // template).
        let found = article_notes(
            include_str!("../../../fixtures/notes/q30-short.html"),
            &styles(),
        );
        let citation = |id: &str| found.notes[id].citations[0].citation.clone();
        assert_eq!(
            citation("cite_note-FOOTNOTESider2007226-52"),
            Citation::Short(Short {
                work: "CITEREFSider2007".into(),
                label: "Sider 2007".into(),
                at: Some("p. 226".into()),
            })
        );
        // A quotation after the place: not only a short footnote.
        assert!(matches!(citation("cite_note-9"), Citation::Text { .. }));
        // The message for editors ("sfn error: multiple targets") is
        // hidden, as the wiki hides it.
        let Citation::Short(ryan) = citation("cite_note-FOOTNOTERyan2000-270") else {
            panic!("{:?}", citation("cite_note-FOOTNOTERyan2000-270"));
        };
        assert_eq!(ryan.work, "CITEREFRyan2000");
        // The works of the short footnotes, from their COinS; not the one
        // that only a text cites.
        assert_eq!(
            found.works.keys().collect::<Vec<_>>(),
            ["CITEREFSider2007", "CITEREFRyan2000"]
        );
        let Citation::Book(sider) = &found.works["CITEREFSider2007"].citation else {
            panic!("{:?}", found.works["CITEREFSider2007"]);
        };
        assert!(sider.work.title.is_some());
        assert_eq!(found.works["CITEREFSider2007"].origin, Origin::Coins);
    }

    #[test]
    fn what_a_short_footnote_is() {
        let link = r##"<a href="#CITEREFMintz1996">Mintz 1996</a>"##;
        let short_of = |html: &str| short(html).map(|s| (s.work, s.label, s.at));
        assert_eq!(
            short_of(&format!("{link}.")),
            Some(("CITEREFMintz1996".into(), "Mintz 1996".into(), None))
        );
        assert_eq!(
            short_of(&format!("{link}, pp.<span>\u{a0}</span>2–3.")).and_then(|s| s.2),
            Some("pp. 2–3".into())
        );
        assert_eq!(
            short_of(&format!("{link}, p. 134")).and_then(|s| s.2),
            Some("p. 134".into())
        );
        // Not a short footnote: a link to another page, two links, words
        // after the place.
        assert_eq!(
            short_of(r#"<a href="https://example.org/">Mintz 1996</a>."#),
            None
        );
        assert_eq!(short_of(&format!("{link}; {link}.")), None);
        assert_eq!(short_of(&format!(r#"{link}, p. 8. "A quotation.""#)), None);
    }

    #[test]
    fn plain_text_of_wikitext() {
        for (wikitext, text) in [
            ("[[Merriam-Webster]]", Some("Merriam-Webster")),
            (
                "[[National Geographic Society|National Geographic]]",
                Some("National Geographic"),
            ),
            (
                "Nairobi, Kenya {{!}} History Timeline",
                Some("Nairobi, Kenya | History Timeline"),
            ),
            (
                "{{lang|sw|Jiji {{nowrap|kuu}}}} la Nairobi",
                Some("la Nairobi"),
            ),
            ("<!-- Staff writer(s); no by-line. -->", None),
            ("[https://archive.org/details/x/page/22 22]–5", Some("22–5")),
            ("[https://example.org]", None),
            ("IRIN &#124; KENYA &amp; more", Some("IRIN | KENYA & more")),
            ("''Italic'' and '''bold'''", Some("Italic and bold")),
            ("a<br />b  \n c", Some("ab c")),
            ("{{cite", None),
            ("[[unclosed", Some("[[unclosed")),
            ("[not a link]", Some("[not a link]")),
        ] {
            assert_eq!(plain(wikitext).as_deref(), text, "{wikitext:?}");
        }
    }

    #[test]
    fn languages() {
        assert_eq!(language("en-US").unwrap().as_str(), "en-us");
        assert_eq!(language("EN").unwrap().as_str(), "en");
        assert_eq!(language("en, fr").unwrap().as_str(), "en");
        assert!(language("English").is_none());
        assert!(language("").is_none());
    }
}
