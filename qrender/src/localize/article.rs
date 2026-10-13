//! Article prose from Wikipedia: cleaned, its links rewritten, glossed.
//!
//! One pass of an HTML rewriter does all of it, and it is also the
//! sanitizer: the renderer prints the result as HTML, so only these can
//! come out:
//! - `a`, with an `href` that this module builds;
//! - `b` and `i`, without attributes;
//! - `ruby`, `rt` and `span`, which this module adds for a gloss.
//!
//! Every other element is removed with its content (scripts, styles,
//! tables, references) or unwrapped (its text stays).

use content::{EntityId, Qid, Term};
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
            let alt = decode_entities(&el.get_attribute("alt").unwrap_or_default());
            el.replace(&alt, ContentType::Text);
        }
        "a" => link(el, links),
        "b" | "i" => strip_attributes(el),
        _ => el.remove_and_keep_content(),
    }
}

fn link(el: &mut Element, links: &Links) {
    let target = el
        .get_attribute("href")
        .and_then(|h| Some(h.strip_prefix("/wiki/")?.split('#').next()?.to_string()))
        .filter(|t| !t.is_empty());
    let Some(target) = target else {
        // Not a link to an article: the text stays, the link goes.
        el.remove_and_keep_content();
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

/// The text of raw HTML: numeric character references and the named ones
/// that attribute values use. Another named reference stays as it is.
pub(crate) fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let decoded = rest.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &rest[1..end];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => match name.strip_prefix('#') {
                    Some(hex) if hex.starts_with(['x', 'X']) => u32::from_str_radix(&hex[1..], 16)
                        .ok()
                        .and_then(char::from_u32),
                    Some(dec) => dec.parse().ok().and_then(char::from_u32),
                    None => None,
                },
            };
            c.map(|c| (c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn set_href(el: &mut Element, href: &str) {
    el.set_attribute("href", href).expect("a valid attribute");
}

fn strip_attributes(el: &mut Element) {
    let names: Vec<String> = el.attributes().iter().map(|a| a.name()).collect();
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
    fn entities() {
        assert_eq!(
            decode_entities("a &amp; b &lt;c&gt; &#233;&#x41; &copy; & x"),
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
