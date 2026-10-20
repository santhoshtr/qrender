//! Snapshots of the composition of every fixture item. A rule change shows
//! here as a diff: review it with `cargo insta review`.

use std::fs;
use std::path::Path;

use content::Content;
use qrender::config::Config;

const ITEMS: [&str; 8] = [
    "q3870",
    "q90",
    "q8023",
    "q117085614",
    "q405",
    "q139890532",
    "q668",
    "q2",
];

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
        // Every citation that a block cites is in the table, and the table
        // has no other.
        let json = serde_json::to_string(&composition.spreads).unwrap()
            + &serde_json::to_string(&composition.set_aside).unwrap();
        for hash in composition.citations.keys() {
            assert!(
                json.contains(&format!("\"{hash}\"")),
                "{qid}: {hash} not cited"
            );
        }
        let cited = json.split("\"citations\":[\"").skip(1).flat_map(|rest| {
            rest[..rest.find(']').unwrap()]
                .split(',')
                .map(|h| h.trim_matches('"').to_string())
        });
        for hash in cited {
            assert!(
                composition.citations.contains_key(&hash),
                "{qid}: {hash} not in the table"
            );
        }
        // Every property of the Earth has a place in its layout.
        if qid == "q2" {
            assert!(
                composition.spreads.iter().all(|s| s.id != "more"),
                "q2: a \"more\" spread"
            );
        }
        insta::assert_json_snapshot!(qid, composition);
    }
}
