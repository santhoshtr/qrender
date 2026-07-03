//! Golden snapshots for the IR-based text/markdown/wikitext/html backends,
//! rendered from checked-in WDQS fixtures.

use qrender::cards::synthesize;
use qrender::grouping::load_grouping_config;
use qrender::textual;

fn page(qid: &str, fixture: &str) -> qrender::cards::FactoidPage {
    let response: qjson::sparql::SparqlResponse = serde_json::from_str(fixture).unwrap();
    let item = qjson::transform::transform(qid, &response.results.bindings);
    synthesize(&item, "en", &load_grouping_config().unwrap(), true)
}

const Q3870: &str = include_str!("../../qjson/tests/fixtures/Q3870.sparql.json"); // Nairobi
const Q42: &str = include_str!("../../qjson/tests/fixtures/Q42.sparql.json"); // Douglas Adams

#[test]
fn q3870_text() {
    insta::assert_snapshot!(textual::render_text(&page("Q3870", Q3870)));
}

#[test]
fn q3870_markdown() {
    insta::assert_snapshot!(textual::render_markdown(&page("Q3870", Q3870)));
}

#[test]
fn q3870_wikitext() {
    insta::assert_snapshot!(textual::render_wikitext(&page("Q3870", Q3870)));
}

#[test]
fn q3870_html() {
    insta::assert_snapshot!(textual::render_html(&page("Q3870", Q3870)));
}

#[test]
fn q42_text() {
    insta::assert_snapshot!(textual::render_text(&page("Q42", Q42)));
}

#[test]
fn q42_markdown() {
    insta::assert_snapshot!(textual::render_markdown(&page("Q42", Q42)));
}
