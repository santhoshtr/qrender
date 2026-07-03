//! Golden snapshot of the factoid HTML body (head is skipped - it embeds
//! the Codex token stylesheets, which are vendored assets, not our output).

use qrender::cards::synthesize;
use qrender::factoid::render_page;
use qrender::grouping::load_grouping_config;

#[test]
fn q3870_factoid_body() {
    let response: qjson::sparql::SparqlResponse =
        serde_json::from_str(include_str!("../../qjson/tests/fixtures/Q3870.sparql.json"))
            .unwrap();
    let item = qjson::transform::transform("Q3870", &response.results.bindings);
    let page = synthesize(&item, "en", &load_grouping_config().unwrap(), true);
    let html = render_page(&page).unwrap();
    let body = html.split_once("</head>").expect("has a head").1;
    insta::assert_snapshot!(body);
}
