//! Citations: the sources that the article text and the statements cite,
//! as typed data. The kinds and their fields follow the CSL and Zotero
//! item types, not the names of the templates of one wiki: a mapping
//! converts a template (`cite web`) or a Wikidata reference to a kind.

use std::fmt;
use std::str::FromStr;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::id::{IdError, Lang, Pid, Qid};

/// A citation and where it came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Cited {
    pub citation: Citation,
    pub origin: Origin,
}

/// A source that a text or a statement cites.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Citation {
    /// A page on a site.
    Web(Web),
    /// An article in a newspaper or on a news site.
    News(News),
    /// An article in a journal.
    Journal(Journal),
    /// An article in a magazine.
    Magazine(Magazine),
    Book(Book),
    /// A report of an institution.
    Report(Report),
    /// An entry in an encyclopedia or a dictionary.
    Entry(Entry),
    /// A database or a work that Wikidata names (P248 stated in), maybe
    /// with the identifier of the record in it.
    Record(Record),
    /// A short footnote: a work of the article's bibliography, at a place
    /// in it ("Faingold 2018, p. 8").
    Short(Short),
    /// Free text that no template made, as HTML. It is not parsed: no
    /// guess.
    Text {
        html: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Magazine {
    #[serde(flatten)]
    pub work: Work,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magazine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Report {
    #[serde(flatten)]
    pub work: Work,
    /// The institution that published it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
}

/// An entry: its title is the title of the work.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Entry {
    #[serde(flatten)]
    pub work: Work,
    /// The encyclopedia or the dictionary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encyclopedia: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

/// A short footnote. The work is in the works of the article, by its
/// anchor; the page can lack it (a broken short footnote).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Short {
    /// The anchor of the work in the article: "CITEREFFaingold2018".
    pub work: String,
    /// The words of the footnote's link: "Faingold 2018".
    pub label: String,
    /// The place in the work: "p. 8", "pp. 2–3".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// The fields that every work has.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Work {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<WebUrl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<Person>,
    /// The date of publication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<Date>,
    /// The date the source was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessed: Option<Date>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<Archive>,
    /// The language of the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Lang>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<Identifier>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Web {
    #[serde(flatten)]
    pub work: Work,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct News {
    #[serde(flatten)]
    pub work: Work,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newspaper: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Journal {
    #[serde(flatten)]
    pub work: Work,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// As written: "144–155".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Book {
    #[serde(flatten)]
    pub work: Work,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// The place of publication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    /// The pages cited, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Record {
    pub stated_in: Qid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<RecordId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<WebUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<Date>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessed: Option<Date>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<Archive>,
}

/// The identifier of a record in a database: an external identifier
/// property (P214 VIAF ID) and its value. The formatter URL of the
/// property gives its address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RecordId {
    pub pid: Pid,
    pub value: String,
}

/// An author: a name in parts, or a name as one text (an organization,
/// "mombasa.go.ke").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Person {
    Name {
        family: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        given: Option<String>,
    },
    Literal {
        name: String,
    },
}

/// A date. A date that the parser can read is a Wikidata time, so the
/// browser formats it as it formats a statement value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Date {
    Time {
        /// "2018-07-28T00:00:00Z".
        time: String,
        /// Wikidata precision: 9 year, 10 month, 11 day.
        precision: u8,
    },
    /// The text as the source wrote it: "c. 1990", "Spring 2004".
    Text { text: String },
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

impl Date {
    /// A date as a citation writes it: "2018-07-28", "2018-07", "2018",
    /// "28 July 2018", "July 28, 2018", "July 2018", with English month
    /// names, full or of three letters ("Jul", "Sept."). Another text
    /// stays as it is.
    pub fn parse(text: &str) -> Date {
        let text = text.trim();
        let words: Vec<&str> = text
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|w| !w.is_empty())
            .collect();
        let found = match words.as_slice() {
            [iso] => iso_date(iso),
            [day, month, year] if day.bytes().all(|b| b.is_ascii_digit()) => {
                date_of(year, Some(month), Some(day))
            }
            [month, day, year] => date_of(year, Some(month), Some(day)),
            [month, year] => date_of(year, Some(month), None),
            _ => None,
        };
        match found {
            Some((year, month, day)) => Date::Time {
                time: format!(
                    "{year:04}-{:02}-{:02}T00:00:00Z",
                    month.unwrap_or(1),
                    day.unwrap_or(1)
                ),
                precision: match (month, day) {
                    (Some(_), Some(_)) => 11,
                    (Some(_), None) => 10,
                    _ => 9,
                },
            },
            None => Date::Text {
                text: text.to_string(),
            },
        }
    }
}

type YearMonthDay = (u32, Option<u32>, Option<u32>);

/// "2018-07-28", "2018-07" or "2018".
fn iso_date(text: &str) -> Option<YearMonthDay> {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    let year = number(parts[0], 4)?;
    let month = parts.get(1).map(|m| number(m, 2)).unwrap_or(Some(0))?;
    let day = parts.get(2).map(|d| number(d, 2)).unwrap_or(Some(0))?;
    if parts.len() > 3 {
        return None;
    }
    valid(year, (month > 0).then_some(month), (day > 0).then_some(day))
}

/// A date of words: the month a name, the day and the year numbers.
fn date_of(year: &str, month: Option<&str>, day: Option<&str>) -> Option<YearMonthDay> {
    let year = (year.len() == 4).then(|| year.parse().ok()).flatten()?;
    let month = match month {
        Some(m) => Some(month_number(m)?),
        None => None,
    };
    let day = match day {
        Some(d) if (1..=2).contains(&d.len()) => Some(d.parse().ok()?),
        Some(_) => return None,
        None => None,
    };
    valid(year, month, day)
}

fn month_number(name: &str) -> Option<u32> {
    let name = name.trim_end_matches('.').to_lowercase();
    if name.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| *m == name || (name.len() <= 4 && m.starts_with(&name)))
        .map(|i| i as u32 + 1)
}

fn valid(year: u32, month: Option<u32>, day: Option<u32>) -> Option<YearMonthDay> {
    let month_ok = month.is_none_or(|m| (1..=12).contains(&m));
    let day_ok = day.is_none_or(|d| month.is_some() && (1..=31).contains(&d));
    (month_ok && day_ok).then_some((year, month, day))
}

/// A copy of the source in a web archive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Archive {
    pub url: WebUrl,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<Date>,
    /// The state of the original URL.
    #[serde(default, skip_serializing_if = "Original::is_live")]
    pub original: Original,
}

/// The state of the original URL of an archived source.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Original {
    /// Live, or not known.
    #[default]
    Live,
    /// The address gives no page: the archive is the source.
    Dead,
    /// The address now has other content (usurped, unfit): it gets no
    /// link.
    Unfit,
}

impl Original {
    fn is_live(&self) -> bool {
        *self == Self::Live
    }
}

/// A standard identifier of a work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum Identifier {
    Doi(String),
    Isbn(String),
    Issn(String),
    Oclc(String),
    Jstor(String),
    /// Semantic Scholar.
    S2cid(String),
    /// The Astrophysics Data System.
    Bibcode(String),
    /// The Handle System.
    Hdl(String),
}

/// Where a citation came from: what an edit changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "from", rename_all = "lowercase")]
pub enum Origin {
    /// A template in the article: its name and its parameters as
    /// wikitext, so an edit changes the same parameters (`access-date`,
    /// not `accessed`).
    Template {
        name: String,
        params: IndexMap<String, String>,
    },
    /// The COinS data of a citation that has no template data (a
    /// template made it inside another template).
    Coins,
    /// The free text of a note.
    Html,
    /// A reference of a Wikidata statement, by its hash.
    Wikidata { hash: String },
}

/// An `http` or `https` URL: only such a URL becomes a link. A
/// protocol-relative URL ("//example.org/") becomes `https`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WebUrl(String);

impl WebUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The host, in lower case, without user and port: what the favicon
    /// of a site is asked by.
    pub fn host(&self) -> String {
        let rest = &self.0[self.0.find("://").map_or(0, |i| i + 3)..];
        let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
        let host = authority.rsplit('@').next().unwrap_or(authority);
        // A port, but not the colons of an IPv6 address ("[::1]:8080").
        let host = match host.rfind(':') {
            Some(i) if !host[i..].contains(']') => &host[..i],
            _ => host,
        };
        host.to_lowercase()
    }
}

impl FromStr for WebUrl {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, IdError> {
        let error = || IdError {
            kind: "web URL",
            value: s.to_string(),
        };
        let s = s.trim();
        let url = if s.starts_with("//") {
            format!("https:{s}")
        } else {
            let scheme = s.find("://").map(|i| s[..i].to_ascii_lowercase());
            if !matches!(scheme.as_deref(), Some("http" | "https")) {
                return Err(error());
            }
            s.to_string()
        };
        if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(error());
        }
        let url = Self(url);
        if url.host().is_empty() {
            return Err(error());
        }
        Ok(url)
    }
}

impl TryFrom<String> for WebUrl {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, IdError> {
        s.parse()
    }
}

impl From<WebUrl> for String {
    fn from(url: WebUrl) -> Self {
        url.0
    }
}

impl fmt::Display for WebUrl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl schemars::JsonSchema for WebUrl {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "WebUrl".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "pattern": "^[Hh][Tt][Tt][Pp][Ss]?://" })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn url(s: &str) -> WebUrl {
        s.parse().unwrap()
    }

    #[test]
    fn web_urls() {
        assert_eq!(
            url("https://example.org/a").as_str(),
            "https://example.org/a"
        );
        assert_eq!(url(" HTTP://example.org ").as_str(), "HTTP://example.org");
        assert_eq!(url("//example.org/a").as_str(), "https://example.org/a");
        for bad in [
            "javascript:alert(1)",
            "ftp://example.org/",
            "example.org",
            "https://",
            "https:///path",
            "https://exa mple.org/",
            "",
        ] {
            assert!(bad.parse::<WebUrl>().is_err(), "{bad}");
        }
        assert!(serde_json::from_value::<WebUrl>(json!("javascript:x")).is_err());
    }

    #[test]
    fn dates() {
        let time = |t: &str, precision| Date::Time {
            time: format!("{t}T00:00:00Z"),
            precision,
        };
        let text = |t: &str| Date::Text { text: t.into() };
        for (written, date) in [
            ("2018-07-28", time("2018-07-28", 11)),
            ("2018-07", time("2018-07-01", 10)),
            ("2018", time("2018-01-01", 9)),
            ("28 July 2018", time("2018-07-28", 11)),
            ("July 28, 2018", time("2018-07-28", 11)),
            ("September 8, 2026", time("2026-09-08", 11)),
            (" 2 Feb 2004 ", time("2004-02-02", 11)),
            ("Sept. 2019", time("2019-09-01", 10)),
            ("July 2018", time("2018-07-01", 10)),
            ("c. 1990", text("c. 1990")),
            ("Spring 2004", text("Spring 2004")),
            ("2018-13-01", text("2018-13-01")),
            ("32 July 2018", text("32 July 2018")),
            ("Ju 2018", text("Ju 2018")),
            ("18 July", text("18 July")),
            ("2018–2019", text("2018–2019")),
            ("", text("")),
        ] {
            assert_eq!(Date::parse(written), date, "{written:?}");
        }
    }

    #[test]
    fn hosts() {
        assert_eq!(
            url("https://www.Mombasa.go.ke/history/").host(),
            "www.mombasa.go.ke"
        );
        assert_eq!(
            url("http://user@example.org:8080?q=1").host(),
            "example.org"
        );
        assert_eq!(url("https://[::1]:8080/").host(), "[::1]");
        assert_eq!(url("https://example.org#top").host(), "example.org");
    }

    #[test]
    fn a_web_citation_from_a_template() {
        // cite_note-10 of the English article of Nairobi.
        let cited = Cited {
            citation: Citation::Web(Web {
                work: Work {
                    title: Some("History of Mombasa".into()),
                    url: Some(url("http://www.mombasa.go.ke/history-of-mombasa/")),
                    authors: vec![Person::Literal {
                        name: "mombasa.go.ke".into(),
                    }],
                    date: Some(Date::Time {
                        time: "2018-07-28T00:00:00Z".into(),
                        precision: 11,
                    }),
                    accessed: Some(Date::Time {
                        time: "2019-06-14T00:00:00Z".into(),
                        precision: 11,
                    }),
                    archive: Some(Archive {
                        url: url(
                            "https://web.archive.org/web/20210126151126/http://www.mombasa.go.ke/history-of-mombasa/",
                        ),
                        date: Some(Date::Time {
                            time: "2021-01-26T00:00:00Z".into(),
                            precision: 11,
                        }),
                        original: Original::Dead,
                    }),
                    ..Work::default()
                },
                publisher: Some("Mombasa County".into()),
                ..Web::default()
            }),
            origin: Origin::Template {
                name: "cite web".into(),
                params: [("title", "History of Mombasa"), ("url-status", "dead")]
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .into_iter()
                    .collect(),
            },
        };
        let value = serde_json::to_value(&cited).unwrap();
        assert_eq!(
            value,
            json!({
                "citation": {
                    "kind": "web",
                    "title": "History of Mombasa",
                    "url": "http://www.mombasa.go.ke/history-of-mombasa/",
                    "authors": [{"name": "mombasa.go.ke"}],
                    "date": {"time": "2018-07-28T00:00:00Z", "precision": 11},
                    "accessed": {"time": "2019-06-14T00:00:00Z", "precision": 11},
                    "archive": {
                        "url": "https://web.archive.org/web/20210126151126/http://www.mombasa.go.ke/history-of-mombasa/",
                        "date": {"time": "2021-01-26T00:00:00Z", "precision": 11},
                        "original": "dead"
                    },
                    "publisher": "Mombasa County"
                },
                "origin": {
                    "from": "template",
                    "name": "cite web",
                    "params": {"title": "History of Mombasa", "url-status": "dead"}
                }
            })
        );
        assert_eq!(serde_json::from_value::<Cited>(value).unwrap(), cited);
    }

    #[test]
    fn a_journal_citation() {
        let citation = Citation::Journal(Journal {
            work: Work {
                title: Some("Zagreb".into()),
                authors: vec![Person::Name {
                    family: "Zlatar".into(),
                    given: Some("Jelena".into()),
                }],
                date: Some(Date::Text {
                    text: "Spring 2014".into(),
                }),
                language: Some("en".parse().unwrap()),
                ids: vec![Identifier::Doi("10.1016/j.cities.2014.03.004".into())],
                ..Work::default()
            },
            journal: Some("Cities".into()),
            volume: Some("39".into()),
            pages: Some("144–155".into()),
            ..Journal::default()
        });
        let value = serde_json::to_value(&citation).unwrap();
        assert_eq!(
            value,
            json!({
                "kind": "journal",
                "title": "Zagreb",
                "authors": [{"family": "Zlatar", "given": "Jelena"}],
                "date": {"text": "Spring 2014"},
                "language": "en",
                "ids": [{"type": "doi", "id": "10.1016/j.cities.2014.03.004"}],
                "journal": "Cities",
                "volume": "39",
                "pages": "144–155"
            })
        );
        assert_eq!(serde_json::from_value::<Citation>(value).unwrap(), citation);
    }

    #[test]
    fn a_wikidata_record_and_free_text() {
        // The reference of P17 (country) of Nairobi: archINFORM location
        // ID 8464, stated in archINFORM, retrieved 2018-08-06.
        let cited = Cited {
            citation: Citation::Record(Record {
                stated_in: "Q265049".parse().unwrap(),
                id: Some(RecordId {
                    pid: "P5573".parse().unwrap(),
                    value: "8464".into(),
                }),
                url: None,
                title: None,
                published: None,
                accessed: Some(Date::Time {
                    time: "2018-08-06T00:00:00Z".into(),
                    precision: 11,
                }),
                archive: None,
            }),
            origin: Origin::Wikidata {
                hash: "7b276f86e9588d62c2b6e9176bc2edd62d63e623".into(),
            },
        };
        let value = serde_json::to_value(&cited).unwrap();
        assert_eq!(
            value,
            json!({
                "citation": {
                    "kind": "record",
                    "stated_in": "Q265049",
                    "id": {"pid": "P5573", "value": "8464"},
                    "accessed": {"time": "2018-08-06T00:00:00Z", "precision": 11}
                },
                "origin": {"from": "wikidata", "hash": "7b276f86e9588d62c2b6e9176bc2edd62d63e623"}
            })
        );
        assert_eq!(serde_json::from_value::<Cited>(value).unwrap(), cited);

        let text = Citation::Text {
            html: "Sana Aiyar, <i>Indians in Kenya</i>, 2015, p.42".into(),
        };
        let value = serde_json::to_value(&text).unwrap();
        assert_eq!(
            value,
            json!({"kind": "text", "html": "Sana Aiyar, <i>Indians in Kenya</i>, 2015, p.42"})
        );
        assert_eq!(serde_json::from_value::<Citation>(value).unwrap(), text);
    }

    #[test]
    fn an_unsafe_url_does_not_load() {
        let value = json!({"kind": "web", "url": "javascript:alert(1)"});
        assert!(serde_json::from_value::<Citation>(value).is_err());
    }
}
