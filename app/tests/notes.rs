//! The notes of whole articles, from their recorded Parsoid HTML: every
//! note gives a result, and a report counts how each was read. Citations
//! are wild; a change of the parse shows as a diff of the snapshots.
//! Nairobi (Q3870) and the United States (Q30, revision 1380971971: 723
//! notes, many short footnotes).

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use content::fetch::{ArticleNotes, article_notes};
use content::{Citation, Note, Origin};
use qrender::config::Config;
use serde_json::Value;

/// The Parsoid HTML of the English article of Nairobi: the largest
/// recorded parse with a list of notes.
fn nairobi() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/http/q3870");
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let json: Value =
                serde_json::from_str(&fs::read_to_string(entry.ok()?.path()).ok()?).ok()?;
            Some(json["parse"]["text"].as_str()?.to_string())
        })
        .filter(|html| html.contains("mw-references"))
        .max_by_key(String::len)
        .expect("the recorded article")
}

/// The Parsoid HTML of the English article of the United States,
/// compressed.
fn united_states() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/notes/q30.en.html.gz");
    let mut html = String::new();
    flate2::read::GzDecoder::new(fs::File::open(path).unwrap())
        .read_to_string(&mut html)
        .unwrap();
    html
}

fn origin(origin: &Origin) -> &'static str {
    match origin {
        Origin::Template { .. } => "template",
        Origin::Coins => "coins",
        Origin::Html => "html",
        Origin::Wikidata { .. } => "wikidata",
    }
}

fn kind(citation: &Citation) -> &'static str {
    match citation {
        Citation::Web(_) => "web",
        Citation::News(_) => "news",
        Citation::Journal(_) => "journal",
        Citation::Magazine(_) => "magazine",
        Citation::Book(_) => "book",
        Citation::Report(_) => "report",
        Citation::Entry(_) => "entry",
        Citation::Record(_) => "record",
        Citation::Short(_) => "short",
        Citation::Text { .. } => "text",
    }
}

/// The address of every link in some HTML.
fn hrefs(html: &str) -> Vec<&str> {
    html.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

/// The names of the templates in the lists of notes, normalized.
fn templates(html: &str) -> BTreeMap<String, usize> {
    let lists = html
        .split("class=\"mw-references")
        .skip(1)
        .map(|rest| &rest[..rest.find("</ol>").unwrap_or(rest.len())]);
    let mut out = BTreeMap::new();
    for list in lists {
        for rest in list.split("\"target\":{\"wt\":\"").skip(1) {
            let name = rest.split('"').next().unwrap_or_default();
            let name = name.replace("\\n", " ").replace('_', " ");
            let name = name
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            *out.entry(name).or_default() += 1;
        }
    }
    out
}

/// How the notes were read: the notes for each set of origins, the
/// citations of each kind, the short footnotes and their works, and the
/// templates that no style has.
fn report(
    notes: &[&Note],
    works: &indexmap::IndexMap<String, content::Cited>,
    html: &str,
) -> String {
    let mut paths: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();
    for note in notes {
        *groups
            .entry(note.group.clone().unwrap_or_else(|| "(references)".into()))
            .or_default() += 1;
        if note.group.is_some() {
            continue;
        }
        let mut origins: Vec<&str> = note.citations.iter().map(|c| origin(&c.origin)).collect();
        origins.dedup();
        *paths.entry(origins.join("+")).or_default() += 1;
        for cited in &note.citations {
            *kinds.entry(kind(&cited.citation)).or_default() += 1;
        }
    }
    let known: Vec<String> = Config::embedded()
        .citations
        .iter()
        .flat_map(|s| {
            let t = &s.templates;
            t.web
                .iter()
                .chain(&t.news)
                .chain(&t.journal)
                .chain(&t.book)
                .chain(&t.magazine)
                .chain(&t.report)
                .chain(&t.entry)
        })
        .cloned()
        .collect();
    let unknown: BTreeMap<String, usize> = templates(html)
        .into_iter()
        .filter(|(name, _)| !known.contains(name))
        .collect();
    let cited: std::collections::BTreeSet<&str> = notes.iter().flat_map(|n| n.works()).collect();
    let found = cited.iter().filter(|w| works.contains_key(**w)).count();
    let work_kinds: BTreeMap<&str, usize> = works.values().fold(BTreeMap::new(), |mut m, w| {
        *m.entry(kind(&w.citation)).or_default() += 1;
        m
    });
    format!(
        "notes: {}\ngroups: {groups:?}\npaths of the references: {paths:?}\ncitations by kind: {kinds:?}\nworks of short footnotes: {} cited, {found} found, by kind {work_kinds:?}\ntemplates in no style: {unknown:?}\n",
        notes.len(),
        cited.len()
    )
}

/// Every note gives a result, and every link in a note's HTML is to the
/// web, to an article (`./`), or to a place in the page (`#`: a short
/// footnote's work).
fn check(notes: &indexmap::IndexMap<String, Note>) {
    for (id, note) in notes {
        match &note.group {
            None => assert!(!note.citations.is_empty(), "{id}: no citation"),
            Some(_) => assert!(note.text.is_some(), "{id}: no text"),
        }
        let texts = note.citations.iter().filter_map(|c| match &c.citation {
            Citation::Text { html } => Some(html.as_str()),
            _ => None,
        });
        for href in texts.chain(note.text.as_deref()).flat_map(hrefs) {
            assert!(
                ["https://", "http://", "./", "#"]
                    .iter()
                    .any(|p| href.starts_with(p)),
                "{id}: link {href:?}"
            );
        }
    }
}

#[test]
fn every_note_of_nairobi() {
    let html = nairobi();
    let ArticleNotes { notes, works } = article_notes(&html, &Config::embedded().citations);
    check(&notes);
    insta::assert_snapshot!(
        "q3870.en.report",
        report(&notes.values().collect::<Vec<_>>(), &works, &html)
    );
    insta::assert_snapshot!(
        "q3870.en.notes.json",
        serde_json::to_string_pretty(&notes).unwrap()
    );
}

#[test]
fn every_note_of_the_united_states() {
    let html = united_states();
    let ArticleNotes { notes, works } = article_notes(&html, &Config::embedded().citations);
    check(&notes);
    insta::assert_snapshot!(
        "q30.en.report",
        report(&notes.values().collect::<Vec<_>>(), &works, &html)
    );
    // All the notes are too many for a snapshot: the first ten of each
    // path.
    let mut sample: BTreeMap<String, Vec<(&String, &Note)>> = BTreeMap::new();
    for (id, note) in &notes {
        let mut origins: Vec<&str> = note.citations.iter().map(|c| origin(&c.origin)).collect();
        origins.dedup();
        let path = note.group.clone().unwrap_or_else(|| origins.join("+"));
        let notes = sample.entry(path).or_default();
        if notes.len() < 10 {
            notes.push((id, note));
        }
    }
    insta::assert_snapshot!(
        "q30.en.sample.json",
        serde_json::to_string_pretty(&sample).unwrap()
    );
}
