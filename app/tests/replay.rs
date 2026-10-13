//! The fetcher, from recorded responses (fixtures/http/): no network. A
//! test of the row parsers and the order of the steps, from the requests
//! to a page. Record again with:
//!   qrender fetch --qid Q3870 --lang en --record fixtures/http/q3870 --out /tmp/q
//!   qrender fetch --qid Q3870 --lang gu --record fixtures/http/q3870 --out /tmp/q

use std::path::Path;

use content::fetch::{Fetcher, Http, Mode, NoStore};
use content::{Lang, Qid};
use qrender::config::Config;
use qrender::render::markdown;

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
        assert!(article.paragraphs.iter().all(|p| !p.contains("Cite error")));
        assert!(!article.concept_sections.is_empty());
        let view = qrender::localize(&composition, &text);
        insta::assert_snapshot!(format!("q3870.{lang}.md"), markdown::page(&view));
    }
}

#[tokio::test]
async fn an_unrecorded_request_is_an_error() {
    let fetcher = fetcher();
    assert!(fetcher.content("Q1".parse().unwrap()).await.is_err());
}
