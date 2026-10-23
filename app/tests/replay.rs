//! The fetcher, from recorded responses (fixtures/http/): no network. A
//! test of the row parsers and the order of the steps, from the requests
//! to a page. Record again with:
//!   qrender fetch --qid Q3870 --lang en --record fixtures/http/q3870 --out /tmp/q
//!   qrender fetch --qid Q3870 --lang gu --record fixtures/http/q3870 --out /tmp/q

use std::collections::BTreeSet;
use std::path::Path;

use content::fetch::{Fetcher, Http, Mode, NoStore};
use content::{Lang, Qid};
use qrender::config::Config;
use qrender::render::markdown;
use serde_json::json;

fn fetcher() -> Fetcher<NoStore> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/http/q3870");
    let http = Http::new(app::DEFAULT_USER_AGENT, Mode::Replay(dir)).unwrap();
    Fetcher::new(http, Config::embedded().fetch_settings(), NoStore)
}

#[tokio::test]
async fn nairobi_from_recorded_responses() {
    let fetcher = fetcher();
    let qid: Qid = "Q3870".parse().unwrap();
    let content = fetcher.content(qid).await.unwrap();
    assert_eq!(content.statements.len(), 153);
    assert_eq!(content.class_layout, "settlement");
    assert_eq!(content.container.qid, Some("Q114".parse().unwrap()));
    let peers = content.peers.as_ref().unwrap();
    assert_eq!(peers.class, "Q1549591".parse().unwrap());
    assert!(!content.commons.is_empty() && !content.references.is_empty());
    // The references that cite a source, once each; every statement's
    // citation is in the table.
    assert!(!content.citations.is_empty());
    assert!(
        content
            .statement_citations
            .values()
            .flatten()
            .all(|h| content.citations.contains_key(h))
    );
    insta::assert_snapshot!(
        "q3870.citations.json",
        serde_json::to_string_pretty(&content.citations).unwrap()
    );

    let composition = qrender::compose(&content, Config::embedded());
    assert!(composition.unshown.is_empty());

    for lang in ["en", "gu"] {
        let lang: Lang = lang.parse().unwrap();
        let text = fetcher.text(&content, &lang).await.unwrap();
        let article = text
            .article
            .as_ref()
            .or(text.fallback_article.as_ref())
            .unwrap();
        // The parser error message of the lead is left out.
        assert!(
            article
                .lead
                .iter()
                .flat_map(|b| b.texts())
                .all(|t| !t.contains("Cite error"))
        );
        assert!(!article.concept_sections.is_empty());
        // Every mark in the stored text has its note, and every note kept
        // is marked.
        let mut texts: Vec<&str> = article
            .lead
            .iter()
            .flat_map(|b| b.texts())
            .chain(article.section_text.values().map(String::as_str))
            .collect();
        let mut sections: Vec<&content::OutlineSection> = article.outline.iter().collect();
        while let Some(s) = sections.pop() {
            texts.extend(s.blocks.iter().flat_map(|b| b.texts()));
            sections.extend(&s.sections);
        }
        texts.extend(article.notes.values().filter_map(|n| n.text.as_deref()));
        let marks: BTreeSet<&str> = texts
            .iter()
            .flat_map(|t| t.split("data-note=\"").skip(1))
            .filter_map(|rest| rest.split('"').next())
            .collect();
        let notes: BTreeSet<&str> = article.notes.keys().map(String::as_str).collect();
        assert_eq!(marks, notes);
        assert!(!notes.is_empty());
        let view = qrender::localize(&composition, &text, Config::embedded());
        insta::assert_snapshot!(format!("q3870.{lang}.md"), markdown::page(&view));
        // The article prose as the page shows it, after the localize step:
        // a change of how the fetcher reads the article shows here.
        let spreads: Vec<_> = view
            .spreads
            .iter()
            .map(|s| json!([s.prose_text, s.prose_links, s.sections]))
            .collect();
        let prose = json!({
            "lead": view.lead,
            "spreads": spreads,
            "unmatched_sections": view.unmatched_sections,
            "more_sections": view.more_sections,
        });
        insta::assert_snapshot!(
            format!("q3870.{lang}.prose.json"),
            serde_json::to_string_pretty(&prose).unwrap()
        );
    }
}

#[tokio::test]
async fn an_unrecorded_request_is_an_error() {
    let fetcher = fetcher();
    assert!(fetcher.content("Q1".parse().unwrap()).await.is_err());
}
