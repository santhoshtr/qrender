//! `Composer::statement` gives what v3 gives for every statement that a v3
//! composition shows. Primitives add keys to a statement (`shown`,
//! `priority`, the keys of a picture), so only the keys that Rust writes are
//! compared.

use std::fs;
use std::path::Path;

use content::Content;
use qrender::compose::Composer;
use qrender::config::Config;
use serde_json::Value;

/// The keys of `Composer.statement()` in v3.
const STATEMENT_KEYS: &[&str] = &[
    "statement",
    "pid",
    "rank",
    "provenance",
    "qualifiers",
    "image",
    "class",
    "described",
    "item",
    "amount",
    "unit",
    "time",
    "precision",
    "text",
    "lang",
    "file",
    "lat",
    "lon",
    "globe",
    "url",
    "string",
];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

/// Every object in the v3 composition that `Composer.statement()` made.
fn statement_objects<'v>(value: &'v Value, out: &mut Vec<&'v Value>) {
    match value {
        Value::Object(o) => {
            if o.contains_key("statement") && o.contains_key("rank") && o.contains_key("provenance")
            {
                out.push(value);
            }
            o.values().for_each(|v| statement_objects(v, out));
        }
        Value::Array(a) => a.iter().for_each(|v| statement_objects(v, out)),
        _ => {}
    }
}

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= 1e-9 * x.abs().max(1.0)
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(v, w)| same(v, w))
        }
        _ => a == b,
    }
}

#[test]
fn statements_match_v3() {
    let config = Config::embedded();
    let mut checked = 0;
    for qid in ["q3870", "q90", "q8023", "q117085614", "q405", "q139890532"] {
        let read = |p: &str| fs::read_to_string(root().join(p)).unwrap();
        let content: Content =
            serde_json::from_str(&read(&format!("fixtures/content/{qid}.json"))).unwrap();
        let v3: Value =
            serde_json::from_str(&read(&format!("prototype/v3/data/composition/{qid}.json")))
                .unwrap();
        let composer = Composer::new(&content, config);
        let mut objects = Vec::new();
        statement_objects(&v3, &mut objects);
        assert!(!objects.is_empty());
        for python in objects {
            let id = python["statement"].as_str().unwrap();
            let statement = content
                .statements
                .iter()
                .find(|s| s.id.as_str() == id)
                .unwrap_or_else(|| panic!("{qid}: {id} not in content"));
            let rust = serde_json::to_value(composer.statement(statement)).unwrap();
            for (key, value) in rust.as_object().unwrap() {
                // A media block replaces "file" and merges its picture; its
                // own keys are checked in R2.
                if python.get("primitive").is_some_and(|p| p == "media") && key == "image" {
                    continue;
                }
                assert!(
                    python.get(key).is_some_and(|p| same(value, p)),
                    "{qid} {id} {key}:\n rust   {value}\n python {}",
                    python.get(key).unwrap_or(&Value::Null)
                );
            }
            // The other direction: a statement key that v3 writes with a
            // value, Rust writes too.
            let is_media = python.get("primitive").is_some_and(|p| p == "media");
            for key in STATEMENT_KEYS {
                let has_value = python
                    .get(key)
                    .is_some_and(|v| !v.is_null() && *v != Value::Bool(false));
                if has_value && !(is_media && matches!(*key, "image" | "file" | "url")) {
                    assert!(rust.get(key).is_some(), "{qid} {id}: rust has no {key}");
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 500, "only {checked} statements checked");
    println!("{checked} statements match v3");
}
