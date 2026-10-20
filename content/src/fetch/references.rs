//! The references of Wikidata statements, as typed citations. A reference
//! that names a work (P248 stated in) is a record of it; else one with a
//! URL (P854) is a web page; else (P143 imported from, P813 retrieved
//! alone) it cites no source and is not a citation. The composer still
//! sees the property set of every reference, for the provenance.

use indexmap::IndexMap;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use super::sparql::Snak;
use crate::citation::{
    Archive, Citation, Cited, Date, Origin, Person, Record, RecordId, Web, WebUrl, Work,
};
use crate::id::{Pid, Qid, StatementId};

const ENTITY: &str = "http://www.wikidata.org/entity/";

// The reference properties of Wikidata.
const STATED_IN: u64 = 248;
const REFERENCE_URL: u64 = 854;
const TITLE: u64 = 1476;
const PUBLICATION_DATE: u64 = 577;
const RETRIEVED: u64 = 813;
const ARCHIVE_URL: u64 = 1065;
const ARCHIVE_DATE: u64 = 2960;
const AUTHOR_NAME: u64 = 2093;

/// The characters of an identifier that its URL encodes.
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

/// What the content keeps of the references of the statements.
#[derive(Debug, Default)]
pub struct References {
    /// For each statement, the property set of each of its references.
    pub sets: IndexMap<StatementId, Vec<Vec<Pid>>>,
    /// For each statement, the hashes of its references that cite a
    /// source.
    pub cited: IndexMap<StatementId, Vec<String>>,
    /// Those references, by hash: a reference that many statements share
    /// is here once.
    pub citations: IndexMap<String, Cited>,
}

pub fn read(by_statement: IndexMap<StatementId, IndexMap<String, Vec<Snak>>>) -> References {
    let mut out = References::default();
    for (st, refs) in by_statement {
        let mut sets = Vec::new();
        let mut cited = Vec::new();
        for (hash, snaks) in refs {
            sets.push(snaks.iter().map(|s| s.pid).collect());
            if !out.citations.contains_key(&hash) {
                let Some(citation) = citation(&snaks) else {
                    continue;
                };
                out.citations.insert(
                    hash.clone(),
                    Cited {
                        citation,
                        origin: Origin::Wikidata { hash: hash.clone() },
                    },
                );
            }
            cited.push(hash);
        }
        out.sets.insert(st.clone(), sets);
        if !cited.is_empty() {
            out.cited.insert(st, cited);
        }
    }
    out
}

/// The citation of a reference: a record of the work it names, else the
/// web page of its URL. None when it names no source.
fn citation(snaks: &[Snak]) -> Option<Citation> {
    let first = |n: u64| snaks.iter().find(|s| s.pid.number() == n);
    let date = |n: u64| first(n).map(time);
    let url = first(REFERENCE_URL).and_then(|s| s.value.parse::<WebUrl>().ok());
    let title = first(TITLE)
        .map(|s| s.value.trim().to_string())
        .filter(|t| !t.is_empty());
    let archive = first(ARCHIVE_URL)
        .and_then(|s| s.value.parse::<WebUrl>().ok())
        .map(|url| Archive {
            url,
            date: date(ARCHIVE_DATE),
            original: Default::default(),
        });
    if let Some(stated_in) = first(STATED_IN).and_then(|s| item(&s.value)) {
        let id = snaks.iter().find(|s| s.datatype == "ExternalId");
        let url = url.or_else(|| {
            let id = id?;
            let value = utf8_percent_encode(&id.value, ID).to_string();
            id.formatter.as_ref()?.replace("$1", &value).parse().ok()
        });
        return Some(Citation::Record(Record {
            stated_in,
            id: id.map(|s| RecordId {
                pid: s.pid,
                value: s.value.clone(),
            }),
            url,
            title,
            published: date(PUBLICATION_DATE),
            accessed: date(RETRIEVED),
            archive,
        }));
    }
    let url = url?;
    Some(Citation::Web(Web {
        work: Work {
            language: first(TITLE).and_then(|s| s.lang.as_deref()?.parse().ok()),
            title,
            url: Some(url),
            authors: snaks
                .iter()
                .filter(|s| s.pid.number() == AUTHOR_NAME)
                .map(|s| Person::Literal {
                    name: s.value.trim().to_string(),
                })
                .collect(),
            date: date(PUBLICATION_DATE),
            accessed: date(RETRIEVED),
            archive,
            ids: Vec::new(),
        },
        website: None,
        publisher: None,
    }))
}

fn item(uri: &str) -> Option<Qid> {
    uri.strip_prefix(ENTITY)?.parse().ok()
}

/// A time value, as the query service gives it ("2018-08-06T00:00:00Z"),
/// with its precision (day when the query gives none).
fn time(snak: &Snak) -> Date {
    Date::Time {
        time: snak.value.clone(),
        precision: snak.precision.unwrap_or(11),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snak(pid: &str, datatype: &str, value: &str) -> Snak {
        Snak {
            pid: pid.parse().unwrap(),
            datatype: datatype.into(),
            value: value.into(),
            lang: None,
            precision: None,
            formatter: None,
        }
    }

    fn stated_in(q: &str) -> Snak {
        snak("P248", "WikibaseItem", &format!("{ENTITY}{q}"))
    }

    fn retrieved(day: &str) -> Snak {
        Snak {
            precision: Some(11),
            ..snak("P813", "Time", &format!("{day}T00:00:00Z"))
        }
    }

    fn day(day: &str) -> Option<Date> {
        Some(Date::Time {
            time: format!("{day}T00:00:00Z"),
            precision: 11,
        })
    }

    fn record(q: &str) -> Record {
        Record {
            stated_in: q.parse().unwrap(),
            id: None,
            url: None,
            title: None,
            published: None,
            accessed: None,
            archive: None,
        }
    }

    fn url(s: &str) -> WebUrl {
        s.parse().unwrap()
    }

    #[test]
    fn the_shapes_of_nairobi() {
        // P143 imported from, alone: not a source.
        let imported = [snak("P143", "WikibaseItem", &format!("{ENTITY}Q328"))];
        assert_eq!(citation(&imported), None);
        // P248 stated in (ISO 3166-2).
        assert_eq!(
            citation(&[stated_in("Q14005")]),
            Some(Citation::Record(record("Q14005")))
        );
        // P248, P813 retrieved.
        assert_eq!(
            citation(&[stated_in("Q14005"), retrieved("2018-08-06")]),
            Some(Citation::Record(Record {
                accessed: day("2018-08-06"),
                ..record("Q14005")
            }))
        );
        // P248, an identifier (P214 VIAF ID), P813: the URL by the formatter.
        let viaf = Snak {
            formatter: Some("https://viaf.org/viaf/$1/".into()),
            ..snak("P214", "ExternalId", "264835402")
        };
        assert_eq!(
            citation(&[stated_in("Q54919"), viaf, retrieved("2015-08-02")]),
            Some(Citation::Record(Record {
                id: Some(RecordId {
                    pid: "P214".parse().unwrap(),
                    value: "264835402".into()
                }),
                url: Some(url("https://viaf.org/viaf/264835402/")),
                accessed: day("2015-08-02"),
                ..record("Q54919")
            }))
        );
        // P248, P1065 archive URL, P2960 archive date.
        let archived = [
            stated_in("Q3063249"),
            snak(
                "P1065",
                "Url",
                "https://web.archive.org/web/20010208080459/http://www.itl.nist.gov/fipspubs/fip10-4.htm",
            ),
            Snak {
                precision: Some(11),
                ..snak("P2960", "Time", "2001-02-08T00:00:00Z")
            },
        ];
        let Some(Citation::Record(r)) = citation(&archived) else {
            panic!();
        };
        let archive = r.archive.unwrap();
        assert_eq!(archive.date, day("2001-02-08"));
        assert_eq!(archive.original, Default::default());
        // P854 reference URL, alone.
        let page = "https://web.archive.org/web/20090314131819/http://www.cbs.go.ke/sectoral/population/projections.html";
        assert_eq!(
            citation(&[snak("P854", "Url", page)]),
            Some(Citation::Web(Web {
                work: Work {
                    url: Some(url(page)),
                    ..Work::default()
                },
                ..Web::default()
            }))
        );
        // P854, P1476 title (English), P813.
        let title = Snak {
            lang: Some("en".into()),
            ..snak(
                "P1476",
                "Monolingualtext",
                "Nairobi City and the city of Raleigh, North Carolina ink twining agreement",
            )
        };
        let Some(Citation::Web(web)) = citation(&[
            snak(
                "P854",
                "Url",
                "https://nairobi.go.ke/nairobi-city-city-raleigh-north-carolina-ink-twining-agreement/",
            ),
            title,
            retrieved("2023-11-01"),
        ]) else {
            panic!();
        };
        assert!(web.work.title.unwrap().starts_with("Nairobi City and"));
        assert_eq!(web.work.language.unwrap().as_str(), "en");
        assert_eq!(web.work.accessed, day("2023-11-01"));
    }

    #[test]
    fn a_url_that_is_not_web_is_not_a_page() {
        assert_eq!(
            citation(&[snak("P854", "Url", "javascript:alert(1)")]),
            None
        );
        // With a work, the record stays, without the URL.
        assert_eq!(
            citation(&[stated_in("Q1"), snak("P854", "Url", "javascript:alert(1)")]),
            Some(Citation::Record(record("Q1")))
        );
    }

    #[test]
    fn a_shared_reference_is_stored_once() {
        let st = |s: &str| s.parse::<StatementId>().unwrap();
        let shared = vec![stated_in("Q14005")];
        let imported = vec![snak("P143", "WikibaseItem", &format!("{ENTITY}Q328"))];
        let by_statement = IndexMap::from([
            (
                st("Q3870-1"),
                IndexMap::from([
                    ("aa".to_string(), shared.clone()),
                    ("bb".to_string(), imported.clone()),
                ]),
            ),
            (st("Q3870-2"), IndexMap::from([("aa".to_string(), shared)])),
            (
                st("Q3870-3"),
                IndexMap::from([("bb".to_string(), imported)]),
            ),
        ]);
        let refs = read(by_statement);
        let p = |s: &str| s.parse::<Pid>().unwrap();
        // Every reference keeps its property set, for the provenance.
        assert_eq!(
            refs.sets[&st("Q3870-1")],
            [vec![p("P248")], vec![p("P143")]]
        );
        assert_eq!(refs.sets[&st("Q3870-3")], [vec![p("P143")]]);
        // Only the source is a citation, once.
        assert_eq!(refs.citations.keys().collect::<Vec<_>>(), ["aa"]);
        assert_eq!(
            refs.citations["aa"].origin,
            Origin::Wikidata { hash: "aa".into() }
        );
        assert_eq!(refs.cited[&st("Q3870-1")], ["aa"]);
        assert_eq!(refs.cited[&st("Q3870-2")], ["aa"]);
        assert!(!refs.cited.contains_key(&st("Q3870-3")));
    }
}
