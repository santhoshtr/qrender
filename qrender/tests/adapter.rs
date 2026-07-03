//! Renders through the full new data path — SPARQL fixture → qjson
//! transform → legacy adapter → renderer — and snapshots the result.
//! Compare with snapshots/golden__q3870_text.snap to review the
//! deliberate differences from the legacy qjson HTTP service output.

use qrender::{RenderConfig, RenderFormatOptions, data_loading::to_legacy, render_item};

#[test]
fn q3870_text_via_qjson_transform() {
    let response: qjson::sparql::SparqlResponse =
        serde_json::from_str(include_str!("../../qjson/tests/fixtures/Q3870.sparql.json"))
            .unwrap();
    let typed = qjson::transform::transform("Q3870", &response.results.bindings);
    let legacy = to_legacy(&typed);
    let config = RenderConfig::new(RenderFormatOptions::Text, true, "en").unwrap();
    insta::assert_snapshot!(render_item(&legacy, &config).unwrap());
}
