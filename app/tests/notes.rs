//! The notes of a whole article, from its recorded Parsoid HTML: every
//! note gives a result, and a report counts how each was read. Citations
//! are wild; a change of the parse shows as a diff of the snapshots.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use content::fetch::article_notes;
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
        Citation::Book(_) => "book",
        Citation::Record(_) => "record",
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
/// citations of each kind, and the templates that no style has.
fn report(notes: &[&Note], html: &str) -> String {
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
            t.web.iter().chain(&t.news).chain(&t.journal).chain(&t.book)
        })
        .cloned()
        .collect();
    let unknown: BTreeMap<String, usize> = templates(html)
        .into_iter()
        .filter(|(name, _)| !known.contains(name))
        .collect();
    format!(
        "notes: {}\ngroups: {groups:?}\npaths of the references: {paths:?}\ncitations by kind: {kinds:?}\ntemplates in no style: {unknown:?}\n",
        notes.len()
    )
}

#[test]
fn every_note_of_nairobi() {
    let html = nairobi();
    let notes = article_notes(&html, &Config::embedded().citations);
    for (id, note) in &notes {
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
                href.starts_with("https://")
                    || href.starts_with("http://")
                    || href.starts_with("./"),
                "{id}: link {href:?}"
            );
        }
    }
    insta::assert_snapshot!(
        "q3870.en.report",
        report(&notes.values().collect::<Vec<_>>(), &html)
    );
    insta::assert_snapshot!(
        "q3870.en.notes.json",
        serde_json::to_string_pretty(&notes).unwrap()
    );
}
