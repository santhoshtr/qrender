//! Article prose from Wikipedia: cleaned, its links rewritten, glossed.
//!
//! One pass of an HTML rewriter does all of it, and it is also the
//! sanitizer: the renderer prints the result as HTML, so only these can
//! come out:
//! - `a`, with an `href` that this module builds (in the text of a
//!   citation, also the `http` and `https` links as they are);
//! - `b` and `i`, without attributes;
//! - `sup` with the number of a note on the page: `<sup data-note="3">`;
//! - the blocks of a section, without attributes: `p`, `ul`, `ol`, `li`,
//!   `dl`, `dt`, `dd`, `blockquote`;
//! - `ruby`, `rt` and `span`, which this module adds for a gloss;
//! - a formula: the MathML elements of `content::html::MATH_ELEMENTS`,
//!   with the attributes of `MATH_ATTRIBUTES` only.
//!
//! Every other element is removed with its content (scripts, styles,
//! tables, references) or unwrapped (its text stays).

use std::cell::RefCell;

use content::html::{MATH_ATTRIBUTES, MATH_ELEMENTS};
use content::{EntityId, Note, Qid, Term};
use indexmap::IndexMap;
use lol_html::html_content::{ContentType, Element};
use lol_html::{RewriteStrSettings, doc_comments, element, rewrite_str};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

use super::format::entity_url;

/// Characters that Python's `urllib.parse.quote` leaves as they are.
const PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'_')
    .remove(b'.')
    .remove(b'-')
    .remove(b'~')
    .remove(b'/');

/// What a rewrite needs to know.
pub struct Links<'a> {
    /// Page title -> item, for the links of the article.
    pub items: &'a IndexMap<String, Qid>,
    /// The language of the article.
    pub source: &'a str,
    /// The language of the page.
    pub page: &'a str,
    pub labels: &'a IndexMap<EntityId, Term>,
    /// The numbers of the notes. None: marks are removed.
    pub notes: Option<&'a Numbering<'a>>,
    /// Links to other sites stay (the text of a citation).
    pub external: bool,
}

/// The number of each note on a page, given at its first mark: in reading
/// order. The numbers only join a mark to its note.
pub struct Numbering<'a> {
    notes: &'a IndexMap<String, Note>,
    numbers: RefCell<IndexMap<String, usize>>,
}

impl<'a> Numbering<'a> {
    pub fn new(notes: &'a IndexMap<String, Note>) -> Self {
        Self {
            notes,
            numbers: RefCell::default(),
        }
    }

    /// The number of a note; None when the article does not have it.
    pub fn number(&self, id: &str) -> Option<usize> {
        if !self.notes.contains_key(id) {
            return None;
        }
        let mut numbers = self.numbers.borrow_mut();
        let next = numbers.len() + 1;
        Some(*numbers.entry(id.to_string()).or_insert(next))
    }

    /// The note of a number.
    pub fn note(&self, number: usize) -> Option<(&'a str, &'a Note)> {
        let numbers = self.numbers.borrow();
        let (id, _) = numbers.get_index(number.checked_sub(1)?)?;
        let (id, note) = self.notes.get_key_value(id)?;
        Some((id.as_str(), note))
    }
}

/// The address of an article.
pub fn article_url(lang: &str, title: &str) -> String {
    let path = utf8_percent_encode(&title.replace(' ', "_"), PATH).to_string();
    format!("https://{lang}.wikipedia.org/wiki/{path}")
}

/// A paragraph of article HTML, cleaned and with its links rewritten:
/// - in the page language, a link goes to the article in that language;
/// - in another language (the fallback article), a link goes to the item
///   on Wikidata, with the item's label in the page language as a gloss.
pub fn rewrite(paragraph: &str, links: &Links) -> String {
    let settings = RewriteStrSettings::new()
        .append_element_content_handler(element!("*", |el| {
            clean(el, links);
            Ok(())
        }))
        .append_document_content_handler(doc_comments!(|c| {
            c.remove();
            Ok(())
        }));
    rewrite_str(paragraph, settings)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn clean(el: &mut Element, links: &Links) {
    let tag = el.tag_name();
    match tag.as_str() {
        "script" | "style" | "table" => el.remove(),
        "sup" if el.get_attribute("data-note").is_some() => {
            let id =
                content::html::decode_entities(&el.get_attribute("data-note").unwrap_or_default());
            match links.notes.and_then(|n| n.number(&id)) {
                Some(number) => {
                    strip_attributes(el);
                    el.set_attribute("data-note", &number.to_string())
                        .expect("a valid attribute");
                    el.set_inner_content("", ContentType::Text);
                }
                None => el.remove(),
            }
        }
        "sup"
            if el
                .get_attribute("class")
                .is_some_and(|c| c.contains("reference")) =>
        {
            el.remove()
        }
        // A picture in the text (an icon, a flag) leaves its alternative text.
        "img" => {
            // The attribute is raw HTML text: decode it, and the rewriter
            // escapes it again as text.
            let alt = content::html::decode_entities(&el.get_attribute("alt").unwrap_or_default());
            el.replace(&alt, ContentType::Text);
        }
        "a" => link(el, links),
        "b" | "i" | "p" | "ul" | "ol" | "li" | "dl" | "dt" | "dd" | "blockquote" => {
            strip_attributes(el)
        }
        name if MATH_ELEMENTS.contains(&name) => keep_attributes(el, &MATH_ATTRIBUTES),
        // The TeX of a formula, which its `alttext` has.
        "annotation" | "annotation-xml" => el.remove(),
        _ => el.remove_and_keep_content(),
    }
}

fn link(el: &mut Element, links: &Links) {
    let host = format!("{}.wikipedia.org", links.source);
    let target = el
        .get_attribute("href")
        .and_then(|h| Some(content::html::wiki_target(&h, &host)?.to_string()));
    let Some(target) = target else {
        let href = el
            .get_attribute("href")
            .map(|h| content::html::decode_entities(&h));
        match href.and_then(|h| h.parse::<content::WebUrl>().ok()) {
            // A source on another site, in the text of a citation.
            Some(url) if links.external => {
                strip_attributes(el);
                // `set_attribute` escapes the quotes only.
                set_href(el, &url.as_str().replace('&', "&amp;"));
            }
            // Not a link to an article: the text stays, the link goes.
            _ => el.remove_and_keep_content(),
        }
        return;
    };
    strip_attributes(el);
    if links.source == links.page {
        // The path as MediaWiki wrote it, under a fixed prefix.
        set_href(
            el,
            &format!("https://{}.wikipedia.org/wiki/{target}", links.page),
        );
        return;
    }
    let title = percent_decode_str(&target)
        .decode_utf8_lossy()
        .replace('_', " ");
    let Some(item) = links.items.get(&title) else {
        el.remove_and_keep_content();
        return;
    };
    set_href(el, &entity_url(EntityId::Item(*item)));
    let gloss = links
        .labels
        .get(&EntityId::Item(*item))
        .and_then(|t| t.label.as_ref())
        .filter(|l| l.lang.as_str() == links.page);
    if let Some(gloss) = gloss {
        el.before("<ruby>", ContentType::Html);
        el.after("</span></rt></ruby>", ContentType::Html);
        el.after(&gloss.text, ContentType::Text);
        el.after("<rt><span>", ContentType::Html);
    }
}

fn set_href(el: &mut Element, href: &str) {
    el.set_attribute("href", href).expect("a valid attribute");
}

fn strip_attributes(el: &mut Element) {
    keep_attributes(el, &[]);
}

fn keep_attributes(el: &mut Element, keep: &[&str]) {
    let names: Vec<String> = el
        .attributes()
        .iter()
        .map(|a| a.name())
        .filter(|n| !keep.contains(&n.as_str()))
        .collect();
    for name in names {
        el.remove_attribute(&name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use content::LangText;

    fn links<'a>(
        items: &'a IndexMap<String, Qid>,
        labels: &'a IndexMap<EntityId, Term>,
        source: &'a str,
        page: &'a str,
    ) -> Links<'a> {
        Links {
            items,
            source,
            page,
            labels,
            notes: None,
            external: false,
        }
    }

    #[test]
    fn local_article_links_stay_in_the_wiki() {
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let l = links(&items, &labels, "en", "en");
        let html = r#"<a href="/wiki/Nairobi_River" title="Nairobi River" class="mw-redirect">river</a> and <b class="x">bold</b>"#;
        assert_eq!(
            rewrite(html, &l),
            r#"<a href="https://en.wikipedia.org/wiki/Nairobi_River">river</a> and <b>bold</b>"#
        );
    }

    #[test]
    fn parsoid_links() {
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let l = links(&items, &labels, "en", "en");
        let html = concat!(
            r#"<a rel="mw:WikiLink" href="https://en.wikipedia.org/wiki/East_Africa" id="mwX">East Africa</a>, "#,
            r#"<a rel="mw:WikiLink/Interwiki" href="https://fr.wikipedia.org/wiki/Kenya">fr</a>, "#,
            r#"<a rel="mw:WikiLink" href="./Nyeri_County?action=edit&amp;redlink=1" class="new">Nyeri</a>"#
        );
        assert_eq!(
            rewrite(html, &l),
            r#"<a href="https://en.wikipedia.org/wiki/East_Africa">East Africa</a>, fr, Nyeri"#
        );
    }

    #[test]
    fn fallback_article_links_go_to_items_with_a_gloss() {
        let q114: Qid = "Q114".parse().unwrap();
        let items = IndexMap::from([
            ("Kenya".to_string(), q114),
            ("Africa".to_string(), "Q15".parse().unwrap()),
        ]);
        let labels = IndexMap::from([(
            EntityId::Item(q114),
            Term {
                label: Some(LangText {
                    text: "കെനിയ".into(),
                    lang: "ml".parse().unwrap(),
                }),
                description: None,
            },
        )]);
        let l = links(&items, &labels, "en", "ml");
        let html = r#"capital of <a href="/wiki/Kenya">Kenya</a>, in <a href="/wiki/Africa">Africa</a>, <a href="/wiki/Unknown">here</a>"#;
        assert_eq!(
            rewrite(html, &l),
            concat!(
                r#"capital of <ruby><a href="https://www.wikidata.org/wiki/Q114">Kenya</a>"#,
                r#"<rt><span>കെനിയ</span></rt></ruby>, in "#,
                r#"<a href="https://www.wikidata.org/wiki/Q15">Africa</a>, here"#
            )
        );
    }

    #[test]
    fn everything_else_is_removed() {
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let l = links(&items, &labels, "en", "en");
        let html = concat!(
            r##"<span class="nowrap">Text</span><sup class="reference"><a href="#cite">[1]</a></sup>"##,
            r#"<script>alert(1)</script><style>p{}</style><!-- note -->"#,
            r#" <img src="x.png" alt="flag &amp; more" onerror="alert(1)">"#,
            r#" <a href="javascript:alert(1)">js</a> <a href="https://evil.example/">out</a>"#,
            r#" <i onclick="alert(1)">it</i><br><table><tr><td>t</td></tr></table>"#
        );
        let out = rewrite(html, &l);
        assert_eq!(out, "Text flag &amp; more js out <i>it</i>");
        for bad in [
            "script",
            "onerror",
            "onclick",
            "javascript",
            "evil",
            "<img",
            "<span",
            "<sup",
        ] {
            assert!(!out.contains(bad), "{bad} in {out}");
        }
    }

    #[test]
    fn a_formula_keeps_its_mathml_only() {
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let l = links(&items, &labels, "en", "en");
        let html = concat!(
            r#"<math alttext="x" display="block" href="javascript:alert(1)" onclick="alert(1)">"#,
            r#"<semantics><mi mathvariant="normal" style="color:red">x</mi>"#,
            r#"<mo><a href="javascript:alert(1)">+</a></mo><mi><img src="x" onerror="alert(1)" alt="y"></mi>"#,
            r#"<mtext><script>alert(1)</script>z</mtext>"#,
            r#"<annotation encoding="application/x-tex">x</annotation></semantics></math>"#
        );
        assert_eq!(
            rewrite(html, &l),
            concat!(
                r#"<math alttext="x" display="block"><semantics><mi mathvariant="normal">x</mi>"#,
                r#"<mo>+</mo><mi>y</mi><mtext>z</mtext></semantics></math>"#
            )
        );
    }

    #[test]
    fn marks_of_notes_get_numbers_in_order() {
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let note = Note {
            group: None,
            citations: Vec::new(),
            text: None,
        };
        let notes: IndexMap<String, Note> = ["cite_note-1", "cite_note-Nation-2"]
            .into_iter()
            .map(|id| (id.to_string(), note.clone()))
            .collect();
        let numbering = Numbering::new(&notes);
        let l = Links {
            notes: Some(&numbering),
            ..links(&items, &labels, "en", "en")
        };
        let mark = |id: &str| format!("<sup data-note=\"{id}\"></sup>");
        // The second note first: it is 1. A note again keeps its number; a
        // note that the article does not have loses its mark.
        let first = format!(
            "A{} b{}.",
            mark("cite_note-Nation-2"),
            mark("cite_note-gone")
        );
        let second = format!("C{}{}.", mark("cite_note-1"), mark("cite_note-Nation-2"));
        assert_eq!(rewrite(&first, &l), "A<sup data-note=\"1\"></sup> b.");
        assert_eq!(
            rewrite(&second, &l),
            "C<sup data-note=\"2\"></sup><sup data-note=\"1\"></sup>."
        );
        assert_eq!(
            numbering.note(1).map(|(id, _)| id),
            Some("cite_note-Nation-2")
        );
        assert_eq!(numbering.note(2).map(|(id, _)| id), Some("cite_note-1"));
        assert!(numbering.note(3).is_none() && numbering.note(0).is_none());
        // Without numbers, a mark goes.
        let plain = links(&items, &labels, "en", "en");
        assert_eq!(rewrite(&first, &plain), "A b.");
    }

    #[test]
    fn links_to_other_sites_stay_only_in_a_citation() {
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let html = concat!(
            r#"<a href="https://www.knbs.or.ke/x?a=1&amp;b=2" class="external">KNBS</a> "#,
            r#"<a href="//example.org/a">rel</a> <a href="javascript:alert(1)">js</a>"#
        );
        let citation = Links {
            external: true,
            ..links(&items, &labels, "en", "en")
        };
        assert_eq!(
            rewrite(html, &citation),
            concat!(
                r#"<a href="https://www.knbs.or.ke/x?a=1&amp;b=2">KNBS</a> "#,
                r#"<a href="https://example.org/a">rel</a> js"#
            )
        );
        assert_eq!(
            rewrite(html, &links(&items, &labels, "en", "en")),
            "KNBS rel js"
        );
    }

    #[test]
    fn entities() {
        assert_eq!(
            content::html::decode_entities("a &amp; b &lt;c&gt; &#233;&#x41; &copy; & x"),
            "a & b <c> éA &copy; & x"
        );
        // A picture whose text is markup: it stays text.
        let (items, labels) = (IndexMap::new(), IndexMap::new());
        let l = links(&items, &labels, "en", "en");
        assert_eq!(
            rewrite(r#"<img alt="&lt;script&gt;x">"#, &l),
            "&lt;script&gt;x"
        );
    }

    #[test]
    fn article_urls_are_quoted_like_python() {
        assert_eq!(
            article_url("en", "Nairobi"),
            "https://en.wikipedia.org/wiki/Nairobi"
        );
        assert_eq!(
            article_url("ml", "കൊക്കച്ചാൽ"),
            "https://ml.wikipedia.org/wiki/%E0%B4%95%E0%B5%8A%E0%B4%95%E0%B5%8D%E0%B4%95%E0%B4%9A%E0%B5%8D%E0%B4%9A%E0%B4%BE%E0%B5%BD"
        );
        assert_eq!(
            article_url("en", "One Battle (film)"),
            "https://en.wikipedia.org/wiki/One_Battle_%28film%29"
        );
    }
}
