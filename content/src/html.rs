//! Small HTML text helpers: the fetcher and the localize step both need
//! them.

/// The MathML elements of a formula that a page shows: MathML Core, and
/// `menclose`, which Wikipedia uses. `annotation` is not here: the TeX is
/// in the `alttext` of `math`.
pub const MATH_ELEMENTS: [&str; 30] = [
    "math",
    "semantics",
    "mrow",
    "mi",
    "mn",
    "mo",
    "ms",
    "mtext",
    "mspace",
    "mstyle",
    "mpadded",
    "mphantom",
    "merror",
    "mfrac",
    "msqrt",
    "mroot",
    "msub",
    "msup",
    "msubsup",
    "munder",
    "mover",
    "munderover",
    "mmultiscripts",
    "mprescripts",
    "none",
    "mtable",
    "mtr",
    "mtd",
    "maction",
    "menclose",
];

/// The attributes of a MathML element that change how it is drawn, and
/// the TeX of a formula (`alttext`). Not `href`, `class`, `style` or an
/// event attribute.
pub const MATH_ATTRIBUTES: [&str; 31] = [
    "alttext",
    "display",
    "mathvariant",
    "displaystyle",
    "scriptlevel",
    "mathsize",
    "stretchy",
    "symmetric",
    "fence",
    "separator",
    "largeop",
    "movablelimits",
    "lspace",
    "rspace",
    "minsize",
    "maxsize",
    "accent",
    "accentunder",
    "linethickness",
    "columnalign",
    "rowalign",
    "columnspacing",
    "rowspacing",
    "columnlines",
    "rowlines",
    "frame",
    "width",
    "height",
    "depth",
    "voffset",
    "notation",
];

/// Text for HTML: in an element or in a quoted attribute.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// The text of raw HTML: numeric character references and the named ones
/// that attribute values use. Another named reference stays as it is.
pub fn decode_entities(raw: &str) -> String {
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

/// The text of HTML: the tags removed. Entities stay; see `decode_entities`.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// The article that a link in the article HTML of `host` points to: its
/// title as the address has it (percent-encoded, with underscores). The
/// legacy parser writes "/wiki/Title"; Parsoid writes
/// "https://{host}/wiki/Title", and "./Title" for some links. None for a
/// link to another host, a red link (it has a query) and a link in the
/// page.
pub fn wiki_target<'a>(href: &'a str, host: &str) -> Option<&'a str> {
    let path = match href
        .strip_prefix("https:")
        .unwrap_or(href)
        .strip_prefix("//")
    {
        Some(rest) => rest.strip_prefix(host)?.strip_prefix("/wiki/")?,
        None => href
            .strip_prefix("/wiki/")
            .or_else(|| href.strip_prefix("./"))?,
    };
    if path.contains('?') {
        return None;
    }
    let title = path.split('#').next()?;
    (!title.is_empty()).then_some(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_of_html() {
        assert_eq!(
            decode_entities(&strip_tags("<b>R&amp;D</b> &lt;x&gt;")),
            "R&D <x>"
        );
    }

    #[test]
    fn links_to_articles_of_one_wiki() {
        let host = "en.wikipedia.org";
        for href in [
            "/wiki/Nairobi",
            "./Nairobi",
            "https://en.wikipedia.org/wiki/Nairobi#History",
            "//en.wikipedia.org/wiki/Nairobi",
        ] {
            assert_eq!(wiki_target(href, host), Some("Nairobi"), "{href}");
        }
        for href in [
            "https://fr.wikipedia.org/wiki/Nairobi",
            "./Nairobi_County?action=edit&redlink=1",
            "#cite_note-1",
            "https://www.knbs.or.ke/",
            "/w/index.php?title=X",
        ] {
            assert_eq!(wiki_target(href, host), None, "{href}");
        }
    }
}
