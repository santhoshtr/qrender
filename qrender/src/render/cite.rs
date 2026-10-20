//! A citation, ready to show: its title line and its fields. The card of
//! the HTML page and the footnote of the Markdown both read it. Data only;
//! the markup is in `templates/parts/cite.html`.

use content::{Archive, Citation, Cited, Date, EntityId, Identifier, Original, Person, Work};
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use super::ui::favicon;
use crate::localize::View;
use crate::localize::format::Part;
use crate::localize::messages::format;
use crate::schema::StatementRef;

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
    /// Shown joined by " · ".
    Parts(Vec<Part>),
    /// A language code; the browser names it (`intl.js`).
    Language(String),
    /// A copy in a web archive: its date, its address, shown by the icon
    /// of the archive's site, and the state of the original.
    Archive {
        date: Option<Part>,
        href: String,
        host: String,
        favicon: String,
        state: Option<String>,
    },
}

/// The citations that most values of a list have (more than half, two at
/// least): the list shows them once, and those values do not; a value with
/// other citations keeps its own. 210 languages from one census are one
/// source, not 210.
pub fn shared(statements: &[StatementRef]) -> Option<&Vec<String>> {
    let mut counts: Vec<(&Vec<String>, usize)> = Vec::new();
    for s in statements.iter().filter(|s| !s.citations.is_empty()) {
        match counts.iter_mut().find(|(set, _)| **set == s.citations) {
            Some((_, n)) => *n += 1,
            None => counts.push((&s.citations, 1)),
        }
    }
    let (set, n) = counts.into_iter().max_by_key(|(_, n)| *n)?;
    (n >= 2 && 2 * n > statements.len()).then_some(set)
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
            vec![
                ("qrender-cite-website", vec![w.website.clone()]),
                ("qrender-cite-publisher", vec![w.publisher.clone()]),
            ],
        ),
        Citation::News(n) => work(
            view,
            "news",
            &n.work,
            vec![
                ("qrender-cite-newspaper", vec![n.newspaper.clone()]),
                ("qrender-cite-publisher", vec![n.publisher.clone()]),
            ],
        ),
        // The journal, its volume, issue and pages: one line.
        Citation::Journal(j) => work(
            view,
            "journal",
            &j.work,
            vec![(
                "qrender-cite-journal",
                vec![
                    j.journal.clone(),
                    volume(view, &j.volume),
                    issue(view, &j.issue),
                    pages(view, &j.pages),
                ],
            )],
        ),
        Citation::Magazine(m) => work(
            view,
            "magazine",
            &m.work,
            vec![(
                "qrender-cite-magazine",
                vec![
                    m.magazine.clone(),
                    volume(view, &m.volume),
                    issue(view, &m.issue),
                    pages(view, &m.pages),
                ],
            )],
        ),
        Citation::Book(b) => work(
            view,
            "book",
            &b.work,
            vec![
                (
                    "qrender-cite-publisher",
                    vec![b.publisher.clone(), b.place.clone()],
                ),
                ("qrender-cite-pages", vec![b.pages.clone()]),
            ],
        ),
        Citation::Report(r) => work(
            view,
            "report",
            &r.work,
            vec![
                ("qrender-cite-publisher", vec![r.publisher.clone()]),
                ("qrender-cite-pages", vec![r.pages.clone()]),
            ],
        ),
        Citation::Entry(e) => work(
            view,
            "entry",
            &e.work,
            vec![(
                "qrender-cite-encyclopedia",
                vec![e.encyclopedia.clone(), e.publisher.clone()],
            )],
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

/// A work: its title line, its authors, the lines of its kind, then the
/// dates, identifiers, language, and archive. A value that says the site
/// again (a publisher "nation.africa") or a value of a line before is not
/// shown twice.
fn work(
    view: &View,
    kind: &'static str,
    w: &Work,
    lines: Vec<(&'static str, Vec<Option<String>>)>,
) -> Card<'static> {
    let host = w.url.as_ref().map(|u| u.host());
    let mut seen: Vec<String> = host.iter().map(|h| same(h)).collect();
    let mut rows = Vec::new();
    let names: Vec<String> = w.authors.iter().map(person).collect();
    let lines = std::iter::once((
        "qrender-cite-authors",
        vec![(!names.is_empty()).then(|| names.join("; "))],
    ))
    .chain(lines);
    for (key, values) in lines {
        let shown = new_values(values, &mut seen);
        if !shown.is_empty() {
            rows.push(parts(key, shown.iter().map(|v| text(v)).collect()));
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

/// The values not shown yet: not empty, not the site, not a value of a
/// line before (without regard to case and to "www.").
fn new_values(values: Vec<Option<String>>, seen: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values.into_iter().flatten() {
        let key = same(&value);
        if !key.is_empty() && !seen.contains(&key) {
            seen.push(key);
            out.push(value);
        }
    }
    out
}

/// A value as it compares: "https://www.Nation.africa/" is "nation.africa".
fn same(value: &str) -> String {
    let value = value.trim().to_lowercase();
    let value = value
        .split_once("://")
        .map_or(value.as_str(), |(_, rest)| rest);
    let value = value.strip_prefix("www.").unwrap_or(value);
    value.trim_end_matches('/').to_string()
}

/// "vol. 70", "no. 2", "pp. 120–139" (or "p. 8"): short, beside the
/// journal.
fn volume(view: &View, v: &Option<String>) -> Option<String> {
    Some(format(view.msg("qrender-cite-volume-in"), &[v.as_ref()?]))
}

fn issue(view: &View, v: &Option<String>) -> Option<String> {
    Some(format(view.msg("qrender-cite-issue-in"), &[v.as_ref()?]))
}

fn pages(view: &View, v: &Option<String>) -> Option<String> {
    let v = v.as_ref()?;
    let key = if v.contains(['–', '-', ',']) {
        "qrender-cite-pages-in"
    } else {
        "qrender-cite-page-in"
    };
    Some(format(view.msg(key), &[v]))
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
    let date = date(view, "", &a.date).and_then(|row| match row.value {
        Value::Parts(mut p) => p.pop(),
        _ => None,
    });
    let state = match a.original {
        Original::Live => None,
        Original::Dead => Some("qrender-cite-original-dead"),
        Original::Unfit => Some("qrender-cite-original-unfit"),
    };
    let host = a.url.host();
    Some(Row {
        key: "qrender-cite-archived",
        value: Value::Archive {
            date,
            href: a.url.as_str().to_string(),
            favicon: favicon(&host),
            host: short_host(&host),
            state: state.map(|key| view.msg(key).to_string()),
        },
    })
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
    fn a_value_is_not_shown_twice() {
        let some = |v: &str| Some(v.to_string());
        // The site, as the host of the address.
        let mut seen = vec![same("www.Nation.africa")];
        assert_eq!(
            new_values(
                vec![some("nation.africa"), some("Nation Media Group"), None],
                &mut seen
            ),
            ["Nation Media Group"]
        );
        // A publisher that is the website of a line before.
        assert_eq!(
            new_values(vec![some("NATION MEDIA GROUP ")], &mut seen),
            Vec::<String>::new()
        );
        assert_eq!(same("https://www.Nation.africa/"), "nation.africa");
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
