//! Names: the names of an item in many languages (the monolingual-text
//! properties of a names spread: official name, short name, demonym; and
//! the item properties whose values name their languages: named after),
//! one block with one choice of language. The composition has every value;
//! the renderer groups them by language for the page language
//! (`groups`), and the qr-names element (`assets/names.js`) shows the
//! group of the chosen language.

use content::{EntityId, Pid, Qid, QualifierValue, Rank, Statement, Value};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::schema::{Block, QualifierPart, StatementRef, ValueShown};
use crate::vocab::Dimension;
use crate::wikidata::LANGUAGE;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Names {
    /// The properties, in the order of the spread.
    pub pids: Vec<Pid>,
    /// Every value, by property; the preferred values of a property first.
    pub values: Vec<StatementRef>,
    /// The language code of each language of the item values (their
    /// "language of work or name" qualifiers): a value is in the group of
    /// each of its languages.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub codes: IndexMap<Qid, String>,
}

/// HTML: the template binding. The markup is templates/primitives/names.html.
#[derive(askama::Template)]
#[template(path = "primitives/names.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Names,
}

/// The CSS of this primitive (design/primitives/names.css).
pub const CSS: &str = include_str!("../../design/primitives/names.css");

/// The script of this primitive: the qr-names element (assets/names.js).
pub const JS: &str = include_str!("../../assets/names.js");

/// The language items of a statement (its "language of work or name"
/// qualifiers) that have a language code.
fn languages<'a>(c: &'a Composer, s: &'a Statement) -> impl Iterator<Item = Qid> + 'a {
    s.qualifiers.iter().filter_map(|q| match q.value {
        QualifierValue::Entity {
            entity: EntityId::Item(item),
        } if q.pid == *LANGUAGE && c.content.language_codes.contains_key(&item) => Some(item),
        _ => None,
    })
}

/// The block of the names of a names spread, at the place of its first
/// property whose values are all monolingual texts, or all items with a
/// language that has a code. It takes the values of all such properties of
/// the spread.
pub fn at(c: &mut Composer, pid: Pid, spec: &SpreadSpec, pids: &[Pid]) -> Option<Block> {
    if spec.dimension != Dimension::Names {
        return None;
    }
    let named = |s: &Statement| match s.value {
        Value::Monolingualtext { .. } => true,
        Value::WikibaseItem { .. } => languages(c, s).next().is_some(),
        _ => false,
    };
    let texts: Vec<Pid> = pids
        .iter()
        .copied()
        .filter(|p| {
            let free = c.free(*p);
            !free.is_empty()
                && (free
                    .iter()
                    .all(|s| matches!(s.value, Value::Monolingualtext { .. }))
                    || free.iter().all(|s| named(s)))
        })
        .collect();
    if texts.first() != Some(&pid) {
        return None;
    }
    let mut values = Vec::new();
    let mut codes = IndexMap::new();
    for p in &texts {
        let mut statements = c.free(*p);
        // Stable: the preferred values first, else the order of the data.
        statements.sort_by_key(|s| s.rank != Rank::Preferred);
        for s in c.take(statements) {
            for item in languages(c, s) {
                codes.insert(item, c.content.language_codes[&item].clone());
            }
            values.push(c.statement(s));
        }
    }
    Some(Block::Names(Names {
        pids: texts,
        values,
        codes,
    }))
}

/// The values of one language.
pub struct Group<'a> {
    pub lang: &'a str,
    pub rows: Vec<Row<'a>>,
}

/// The values of one property in a group.
pub struct Row<'a> {
    pub pid: Pid,
    pub values: Vec<Text<'a>>,
    /// The language of the values when it is not the language of the group:
    /// a fallback, by the chain.
    pub fallback: Option<&'a str>,
}

/// A value in one language: a text, or an item that names the language.
pub struct Text<'a> {
    /// The text; None for an item.
    pub text: Option<&'a str>,
    pub lang: &'a str,
    pub statement: &'a StatementRef,
}

/// The groups of languages for a page in `page`. The page language comes
/// first, always; then `mul`, `en`, then the other languages, the language
/// with the most properties first, then by code. An item value is in the
/// group of each of its languages. A row without a value in
/// the language of its group has the values by the chain "the language of
/// the group, the page language, mul, en", marked as a fallback; without
/// any, the row is not in the group.
pub fn groups<'a>(b: &'a Names, page: &'a str) -> Vec<Group<'a>> {
    let texts: Vec<Text<'a>> = b
        .values
        .iter()
        .flat_map(|s| match &s.value {
            ValueShown::Text { text, lang } => vec![Text {
                text: Some(text),
                lang,
                statement: s,
            }],
            _ => s
                .qualifiers
                .iter()
                .filter_map(|q| match q.value {
                    QualifierPart::Item {
                        item: EntityId::Item(item),
                    } if q.pid == *LANGUAGE => b.codes.get(&item),
                    _ => None,
                })
                .map(|lang| Text {
                    text: None,
                    lang,
                    statement: s,
                })
                .collect(),
        })
        .collect();
    let has = |pid: Pid, lang: &str| {
        texts
            .iter()
            .any(|t| t.statement.pid == pid && t.lang == lang)
    };
    let mut langs: Vec<&'a str> = texts.iter().map(|t| t.lang).collect();
    langs.sort_unstable();
    langs.dedup();
    let first = |lang: &str| [page, "mul", "en"].iter().position(|l| *l == lang);
    let properties = |lang: &str| b.pids.iter().filter(|p| has(**p, lang)).count();
    langs.sort_by_key(|lang| {
        (
            first(lang).unwrap_or(3),
            usize::MAX - properties(lang),
            *lang,
        )
    });
    if langs.first() != Some(&page) {
        langs.insert(0, page);
    }
    langs
        .into_iter()
        .map(|lang| Group {
            lang,
            rows: b
                .pids
                .iter()
                .filter_map(|pid| {
                    let found = [lang, page, "mul", "en"]
                        .into_iter()
                        .find(|l| has(*pid, l))?;
                    Some(Row {
                        pid: *pid,
                        values: texts
                            .iter()
                            .filter(|t| t.statement.pid == *pid && t.lang == found)
                            .map(|t| Text { ..*t })
                            .collect(),
                        fallback: (found != lang).then_some(found),
                    })
                })
                .collect(),
        })
        .collect()
}

/// Markdown: each property with every value and its language.
pub fn markdown(b: &Names, md: &mut crate::render::markdown::Markdown) {
    for pid in &b.pids {
        let values: Vec<String> = b
            .values
            .iter()
            .filter(|s| s.pid == *pid)
            .map(|s| md.value(s))
            .collect();
        md.items(&md.property(*pid), &values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(n: u32, pid: &str, text: &str, lang: &str) -> StatementRef {
        StatementRef {
            statement: format!("Q1-{n:04x}").parse().unwrap(),
            pid: pid.parse().unwrap(),
            value: ValueShown::Text {
                text: text.into(),
                lang: lang.into(),
            },
            rank: Rank::Normal,
            provenance: crate::schema::Provenance::None,
            citations: Vec::new(),
            qualifiers: Vec::new(),
            texts: Vec::new(),
            image: None,
            class: false,
            described: false,
            shown: None,
        }
    }

    /// An item value of "named after" in the languages of `langs`.
    fn named(n: u32, item: &str, langs: &[&str]) -> StatementRef {
        StatementRef {
            value: ValueShown::Item {
                item: Some(item.parse().unwrap()),
            },
            qualifiers: langs
                .iter()
                .map(|l| crate::schema::QualifierShown {
                    pid: *LANGUAGE,
                    value: QualifierPart::Item {
                        item: EntityId::Item(l.parse().unwrap()),
                    },
                })
                .collect(),
            ..text(n, "P138", "", "")
        }
    }

    fn names() -> Names {
        Names {
            pids: ["P1448", "P1813", "P1705", "P138"]
                .iter()
                .map(|p| p.parse().unwrap())
                .collect(),
            values: vec![
                text(1, "P1448", "Republic of India", "en"),
                text(2, "P1448", "भारत गणराज्य", "hi"),
                text(3, "P1813", "India", "en"),
                text(4, "P1705", "भारत", "hi"),
                text(5, "P1705", "இந்தியா", "ta"),
                // Named after the Indus in English, Bharata in Hindi; the
                // language Q9 has no code.
                named(6, "Q7348", &["Q1860", "Q9"]),
                named(7, "Q2", &["Q1568"]),
            ],
            codes: [("Q1860", "en"), ("Q1568", "hi")]
                .iter()
                .map(|(q, c)| (q.parse().unwrap(), c.to_string()))
                .collect(),
        }
    }

    /// Each group as "lang: pid=text[fallback]".
    fn shown(groups: &[Group]) -> Vec<String> {
        groups
            .iter()
            .map(|g| {
                let rows: Vec<String> = g
                    .rows
                    .iter()
                    .map(|r| {
                        let texts: Vec<String> = r
                            .values
                            .iter()
                            .map(|t| match t.text {
                                Some(text) => text.to_string(),
                                None => t.statement.value.item().unwrap().to_string(),
                            })
                            .collect();
                        let mark = r.fallback.map(|l| format!("[{l}]")).unwrap_or_default();
                        format!("{}={}{mark}", r.pid, texts.join("/"))
                    })
                    .collect();
                format!("{}: {}", g.lang, rows.join(" "))
            })
            .collect()
    }

    #[test]
    fn page_language_first_with_fallbacks() {
        let b = names();
        assert_eq!(
            shown(&groups(&b, "en")),
            [
                // No native label in English: no row.
                "en: P1448=Republic of India P1813=India P138=Q7348",
                // Hindi has three properties, Tamil one.
                "hi: P1448=भारत गणराज्य P1813=India[en] P1705=भारत P138=Q2",
                "ta: P1448=Republic of India[en] P1813=India[en] P1705=இந்தியா P138=Q7348[en]",
            ]
        );
    }

    #[test]
    fn a_page_language_without_values_has_a_group() {
        let b = names();
        let groups = groups(&b, "ml");
        assert_eq!(
            shown(&groups)[0],
            "ml: P1448=Republic of India[en] P1813=India[en] P138=Q7348[en]"
        );
        assert_eq!(groups[1].lang, "en");
    }
}
