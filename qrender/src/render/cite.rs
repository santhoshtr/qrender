//! A citation, ready to show: its title line and its fields. The card of
//! the HTML page and the footnote of the Markdown both read it. Data only;
//! the markup is in `templates/parts/cite.html`.

use content::{Archive, Citation, Cited, Date, EntityId, Identifier, Original, Person, Work};
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use super::ui::favicon;
use crate::localize::View;
use crate::localize::format::Part;

/// The characters of an identifier that its resolver's address encodes.
const ID: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

pub struct Card<'a> {
    /// The title of the work; else its address; else the name of the
    /// database (a record).
    pub title: String,
    /// The language of the title, when it is known.
    pub title_lang: Option<String>,
    pub href: Option<String>,
    /// The site, without "www.".
    pub host: Option<String>,
    /// The icon of the site.
    pub favicon: Option<String>,
    /// "web", "news", "journal", "book", "record", "text".
    pub kind: &'static str,
    pub rows: Vec<Row>,
    /// Free text: its HTML, cleaned by the localize step.
    pub html: Option<&'a str>,
}

/// A field: the message key of its name, and its value.
pub struct Row {
    pub key: &'static str,
    pub value: Value,
}

pub enum Value {
    Parts(Vec<Part>),
    /// A language code; the browser names it (`intl.js`).
    Language(String),
}

pub fn card<'a>(cited: &'a Cited, view: &'a View) -> Card<'a> {
    match &cited.citation {
        // The card of its work, with the place; without the work, its
        // words.
        Citation::Short(s) => {
            let at =
                s.at.as_ref()
                    .map(|at| parts("qrender-cite-at", vec![text(at)]));
            match view.works.get(&s.work) {
                Some(work) => {
                    let mut card = card(work, view);
                    card.rows.extend(at);
                    card
                }
                None => Card {
                    title: s.label.clone(),
                    title_lang: None,
                    href: None,
                    host: None,
                    favicon: None,
                    kind: "short",
                    rows: at.into_iter().collect(),
                    html: None,
                },
            }
        }
        Citation::Web(w) => work(
            view,
            "web",
            &w.work,
            &[
                ("qrender-cite-website", &w.website),
                ("qrender-cite-publisher", &w.publisher),
            ],
        ),
        Citation::News(n) => work(
            view,
            "news",
            &n.work,
            &[
                ("qrender-cite-newspaper", &n.newspaper),
                ("qrender-cite-publisher", &n.publisher),
            ],
        ),
        Citation::Journal(j) => work(
            view,
            "journal",
            &j.work,
            &[
                ("qrender-cite-journal", &j.journal),
                ("qrender-cite-volume", &j.volume),
                ("qrender-cite-issue", &j.issue),
                ("qrender-cite-pages", &j.pages),
            ],
        ),
        Citation::Book(b) => work(
            view,
            "book",
            &b.work,
            &[
                ("qrender-cite-publisher", &b.publisher),
                ("qrender-cite-place", &b.place),
                ("qrender-cite-pages", &b.pages),
            ],
        ),
        Citation::Record(r) => {
            let label = view.labels.get(&EntityId::Item(r.stated_in));
            let url = r.url.as_ref().map(|u| u.as_str().to_string());
            let mut rows = Vec::new();
            if let Some(id) = &r.id {
                rows.push(parts(
                    "qrender-cite-identifier",
                    vec![Part::Qualified {
                        pid: id.pid,
                        value: Box::new(text(&id.value)),
                    }],
                ));
            }
            if let Some(title) = &r.title {
                rows.push(parts("qrender-cite-title", vec![text(title)]));
            }
            rows.extend(date(view, "qrender-cite-published", &r.published));
            rows.extend(date(view, "qrender-cite-retrieved", &r.accessed));
            rows.extend(archived(view, &r.archive));
            let host = r.url.as_ref().map(|u| u.host());
            Card {
                title: label.map_or_else(|| r.stated_in.to_string(), |l| l.text.clone()),
                title_lang: label.and_then(|l| l.lang.as_ref()).map(ToString::to_string),
                // The record; else the database on Wikidata.
                href: url.or_else(|| label.map(|l| l.href.clone())),
                favicon: host.as_deref().map(favicon),
                host: host.map(|h| short_host(&h)),
                kind: "record",
                rows,
                html: None,
            }
        }
        Citation::Text { html } => Card {
            title: String::new(),
            title_lang: None,
            href: None,
            host: None,
            favicon: None,
            kind: "text",
            rows: Vec::new(),
            html: Some(html),
        },
    }
}

/// A work: its title line, its authors, the fields of its kind, then the
/// dates, identifiers, language, and archive.
fn work(
    view: &View,
    kind: &'static str,
    w: &Work,
    fields: &[(&'static str, &Option<String>)],
) -> Card<'static> {
    let mut rows = Vec::new();
    if !w.authors.is_empty() {
        let names: Vec<String> = w.authors.iter().map(person).collect();
        rows.push(parts("qrender-cite-authors", vec![text(&names.join("; "))]));
    }
    for (key, value) in fields {
        if let Some(value) = value {
            rows.push(parts(key, vec![text(value)]));
        }
    }
    rows.extend(date(view, "qrender-cite-published", &w.date));
    if !w.ids.is_empty() {
        rows.push(parts(
            "qrender-cite-identifiers",
            w.ids.iter().map(identifier).collect(),
        ));
    }
    if let Some(lang) = &w.language {
        rows.push(Row {
            key: "qrender-language",
            value: Value::Language(lang.to_string()),
        });
    }
    rows.extend(date(view, "qrender-cite-retrieved", &w.accessed));
    rows.extend(archived(view, &w.archive));
    let host = w.url.as_ref().map(|u| u.host());
    // The original; the archive when the original is gone or reused.
    let href = match &w.archive {
        Some(a) if a.original != Original::Live => Some(&a.url),
        _ => w.url.as_ref().or(w.archive.as_ref().map(|a| &a.url)),
    };
    // No title: the address without its scheme, decoded to read.
    let title = w.title.clone().or_else(|| {
        let url = w.url.as_ref()?.as_str();
        let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
        Some(percent_decode_str(rest).decode_utf8_lossy().into_owned())
    });
    Card {
        title: title.unwrap_or_default(),
        title_lang: w.language.as_ref().map(ToString::to_string),
        href: href.map(|u| u.as_str().to_string()),
        favicon: host.as_deref().map(favicon),
        host: host.map(|h| short_host(&h)),
        kind,
        rows,
        html: None,
    }
}

fn parts(key: &'static str, parts: Vec<Part>) -> Row {
    Row {
        key,
        value: Value::Parts(parts),
    }
}

fn text(text: &str) -> Part {
    Part::Text {
        text: text.to_string(),
        lang: None,
    }
}

/// A name as a bibliography writes it: "Family, Given".
fn person(p: &Person) -> String {
    match p {
        Person::Name {
            family,
            given: Some(given),
        } => format!("{family}, {given}"),
        Person::Name {
            family,
            given: None,
        } => family.clone(),
        Person::Literal { name } => name.clone(),
    }
}

fn date(view: &View, key: &'static str, date: &Option<Date>) -> Option<Row> {
    let part = match date.as_ref()? {
        Date::Time { time, precision } => view.time(time, Some(*precision)),
        Date::Text { text: t } => text(t),
    };
    Some(parts(key, vec![part]))
}

/// The date of the copy, a link to it, and the state of the original.
fn archived(view: &View, archive: &Option<Archive>) -> Option<Row> {
    let a = archive.as_ref()?;
    let mut out: Vec<Part> = date(view, "", &a.date)
        .map(|row| match row.value {
            Value::Parts(p) => p,
            Value::Language(_) => Vec::new(),
        })
        .unwrap_or_default();
    out.push(Part::Link {
        text: short_host(&a.url.host()),
        href: a.url.as_str().to_string(),
    });
    let state = match a.original {
        Original::Live => None,
        Original::Dead => Some("qrender-cite-original-dead"),
        Original::Unfit => Some("qrender-cite-original-unfit"),
    };
    if let Some(key) = state {
        out.push(text(view.messages.get(key).map_or("", String::as_str)));
    }
    Some(parts("qrender-cite-archived", out))
}

/// A standard identifier: its name and value, linked to its resolver.
fn identifier(id: &Identifier) -> Part {
    let (name, value, base) = match id {
        Identifier::Doi(v) => ("DOI", v, "https://doi.org/"),
        Identifier::Isbn(v) => (
            "ISBN",
            v,
            "https://www.wikidata.org/wiki/Special:BookSources/",
        ),
        Identifier::Issn(v) => ("ISSN", v, "https://portal.issn.org/resource/ISSN/"),
        Identifier::Oclc(v) => ("OCLC", v, "https://search.worldcat.org/title/"),
        Identifier::Jstor(v) => ("JSTOR", v, "https://www.jstor.org/stable/"),
        Identifier::S2cid(v) => ("S2CID", v, "https://api.semanticscholar.org/CorpusID:"),
        Identifier::Bibcode(v) => ("Bibcode", v, "https://ui.adsabs.harvard.edu/abs/"),
        Identifier::Hdl(v) => ("hdl", v, "https://hdl.handle.net/"),
    };
    Part::Link {
        text: format!("{name} {value}"),
        href: format!("{base}{}", utf8_percent_encode(value, ID)),
    }
}

fn short_host(host: &str) -> String {
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_link_to_their_resolvers() {
        let link = |id| match identifier(&id) {
            Part::Link { text, href } => (text, href),
            other => panic!("{other:?}"),
        };
        assert_eq!(
            link(Identifier::Doi("10.4000/eastafrica.521".into())),
            (
                "DOI 10.4000/eastafrica.521".into(),
                "https://doi.org/10.4000/eastafrica.521".into()
            )
        );
        assert_eq!(
            link(Identifier::Isbn("978-3-319-30750-3".into())).1,
            "https://www.wikidata.org/wiki/Special:BookSources/978-3-319-30750-3"
        );
        // A character that would end the address is encoded.
        assert_eq!(
            link(Identifier::Hdl("11370/a b#c".into())).1,
            "https://hdl.handle.net/11370/a%20b%23c"
        );
    }

    #[test]
    fn names_as_a_bibliography_writes_them() {
        let name = |family: &str, given: Option<&str>| Person::Name {
            family: family.into(),
            given: given.map(Into::into),
        };
        assert_eq!(person(&name("Rashid", Some("Mahbub"))), "Rashid, Mahbub");
        assert_eq!(person(&name("Martin", None)), "Martin");
        assert_eq!(
            person(&Person::Literal {
                name: "OECD/SWAC".into()
            }),
            "OECD/SWAC"
        );
        assert_eq!(short_host("www.britannica.com"), "britannica.com");
    }
}
