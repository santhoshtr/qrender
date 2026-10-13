//! Snapshots of the composition of every fixture item. A rule change shows
//! here as a diff: review it with `cargo insta review`.

use std::fs;
use std::path::Path;

use content::Content;
use qrender::config::Config;

const ITEMS: [&str; 6] = ["q3870", "q90", "q8023", "q117085614", "q405", "q139890532"];

#[test]
fn compositions() {
    for qid in ITEMS {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../fixtures/content/{qid}.json"));
        let content: Content = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let composition = qrender::compose(&content, Config::embedded());
        // Nothing is lost: every statement is in some spread.
        assert!(
            composition.unshown.is_empty(),
            "{qid}: {:?}",
            composition.unshown
        );
        insta::assert_json_snapshot!(qid, composition);
    }
}
