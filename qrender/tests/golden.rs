//! Golden-output tests over checked-in qjson fixtures.
//! They pin the current rendering so the data-layer and pipeline
//! refactors can be verified against unchanged output.

use qrender::model::{WikidataItem, WikidataProperties};
use qrender::{RenderConfig, RenderFormatOptions, render_item};

fn load_item(json: &str) -> WikidataItem {
    let properties: WikidataProperties = serde_json::from_str(json).unwrap();
    WikidataItem { properties }
}

fn render_fixture(json: &str, format: RenderFormatOptions) -> String {
    let item = load_item(json);
    let config = RenderConfig::new(format, true, "en").unwrap();
    render_item(&item, &config).unwrap()
}

const Q42: &str = include_str!("fixtures/Q42.json"); // Douglas Adams (person)
const Q3870: &str = include_str!("fixtures/Q3870.json"); // Nairobi (population history)
const Q405: &str = include_str!("fixtures/Q405.json"); // Moon (quantities)

#[test]
fn q42_text() {
    insta::assert_snapshot!(render_fixture(Q42, RenderFormatOptions::Text));
}

#[test]
fn q42_markdown() {
    insta::assert_snapshot!(render_fixture(Q42, RenderFormatOptions::Markdown));
}

#[test]
fn q42_html() {
    insta::assert_snapshot!(render_fixture(Q42, RenderFormatOptions::HTML));
}

#[test]
fn q42_wikitext() {
    insta::assert_snapshot!(render_fixture(Q42, RenderFormatOptions::Wikitext));
}

#[test]
fn q3870_text() {
    insta::assert_snapshot!(render_fixture(Q3870, RenderFormatOptions::Text));
}

#[test]
fn q3870_markdown() {
    insta::assert_snapshot!(render_fixture(Q3870, RenderFormatOptions::Markdown));
}

#[test]
fn q405_text() {
    insta::assert_snapshot!(render_fixture(Q405, RenderFormatOptions::Text));
}
