//! The Rust composition equals the v3 composition for every fixture item,
//! with these normalizations only:
//! - a single block in a slot equals a list of one block (Rust slots are
//!   always lists);
//! - a key with `null` equals an absent key;
//! - the `rule` key (Rust only) and the schema versions are not compared;
//! - numbers are equal to 1e-9.
//!
//! Delete this test with the v3 importer, when Rust is the reference.

use std::fs;
use std::path::Path;

use content::Content;
use qrender::config::Config;
use serde_json::{Map, Value};

const ITEMS: [&str; 6] = ["q3870", "q90", "q8023", "q117085614", "q405", "q139890532"];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

/// Drop nulls and `rule`; make every slot a list.
fn normalize(value: Value, in_slots: bool) -> Value {
    match value {
        Value::Object(o) => {
            let mut out = Map::new();
            for (k, v) in o {
                if v.is_null() || k == "rule" {
                    continue;
                }
                let v = if in_slots && v.is_object() {
                    Value::Array(vec![v])
                } else {
                    v
                };
                out.insert(k.clone(), normalize(v, k == "slots"));
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.into_iter().map(|v| normalize(v, false)).collect()),
        other => other,
    }
}

fn diff(path: &str, rust: &Value, v3: &Value, out: &mut Vec<String>) {
    match (rust, v3) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            if (a - b).abs() > 1e-9 * a.abs().max(1.0) {
                out.push(format!("{path}: rust {a} v3 {b}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                match b.get(k) {
                    Some(w) => diff(&format!("{path}.{k}"), v, w, out),
                    None => out.push(format!("{path}.{k}: only in rust")),
                }
            }
            for k in b.keys().filter(|k| !a.contains_key(*k)) {
                out.push(format!("{path}.{k}: only in v3 ({})", b[k]));
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                out.push(format!(
                    "{path}: rust has {} items, v3 {}",
                    a.len(),
                    b.len()
                ));
            }
            for (i, (v, w)) in a.iter().zip(b).enumerate() {
                diff(&format!("{path}[{i}]"), v, w, out);
            }
        }
        _ if rust == v3 => {}
        _ => out.push(format!("{path}: rust {rust} v3 {v3}")),
    }
}

#[test]
fn compositions_match_v3() {
    let mut failures = Vec::new();
    for qid in ITEMS {
        let read = |p: String| fs::read_to_string(root().join(p)).unwrap();
        let content: Content =
            serde_json::from_str(&read(format!("fixtures/content/{qid}.json"))).unwrap();
        let composition = qrender::compose(&content, Config::embedded());
        assert!(
            composition.unshown.is_empty(),
            "{qid}: unshown {:?}",
            composition.unshown
        );
        let mut rust = normalize(serde_json::to_value(&composition).unwrap(), false);
        let mut v3 = normalize(
            serde_json::from_str(&read(format!("prototype/v3/data/composition/{qid}.json")))
                .unwrap(),
            false,
        );
        for key in ["schema", "design"] {
            rust.as_object_mut().unwrap().remove(key);
            v3.as_object_mut().unwrap().remove(key);
        }
        let mut out = Vec::new();
        diff(qid, &rust, &v3, &mut out);
        failures.extend(out);
    }
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures
            .iter()
            .take(60)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
