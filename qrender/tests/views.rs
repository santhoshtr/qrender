//! Snapshots of the view of every fixture page. A change of the labels, the
//! prose or the tables shows here as a diff: `cargo insta review`.

use std::fs;
use std::path::Path;

use content::{Content, Text};
use qrender::config::Config;

const PAGES: [&str; 12] = [
    "q3870.en",
    "q3870.gu",
    "q90.en",
    "q90.ml",
    "q8023.tcy",
    "q117085614.en",
    "q405.en",
    "q405.ml",
    "q139890532.en",
    "q139890532.ml",
    "q668.en",
    "q668.ml",
];

fn read<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn views() {
    for page in PAGES {
        let qid = page.split('.').next().unwrap();
        let content: Content = read(&format!("fixtures/content/{qid}.json"));
        let text: Text = read(&format!("fixtures/text/{page}.json"));
        let composition = qrender::compose(&content, Config::embedded());
        let view = qrender::localize(&composition, &text, Config::embedded());
        // The composition inside the view is in its own snapshot; here only
        // what the language adds.
        let mut json = serde_json::to_value(&view).unwrap();
        for spread in json["spreads"].as_array_mut().unwrap() {
            let o = spread.as_object_mut().unwrap();
            o.retain(|k, _| ["id", "title", "prose_text", "prose_links"].contains(&k.as_str()));
        }
        insta::assert_json_snapshot!(page, json);
    }
}
