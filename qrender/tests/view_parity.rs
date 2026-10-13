//! The Rust view gives what the v3 view gives, for every fixture page. The
//! shapes differ (the Rust view keeps labels in one table, D1), so this
//! compares what both have: the label of every identifier v3 labels, page
//! images, descriptions, the lead, section prose and links, coverage.
//!
//! Delete this test with `composition_parity.rs`, when Rust is the
//! reference.

use std::fs;
use std::path::Path;

use content::{Content, Text};
use qrender::config::Config;
use serde_json::{Value, json};

const PAGES: [&str; 10] = [
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
];

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)).unwrap()
}

fn walk(value: &Value, f: &mut impl FnMut(&serde_json::Map<String, Value>)) {
    match value {
        Value::Object(o) => {
            f(o);
            o.values().for_each(|v| walk(v, f));
        }
        Value::Array(a) => a.iter().for_each(|v| walk(v, f)),
        _ => {}
    }
}

/// The first block of a primitive in the v3 view.
fn block(view: &Value, primitive: &str) -> Value {
    let mut found = None;
    walk(view, &mut |o| {
        if found.is_none() && o.get("primitive").and_then(Value::as_str) == Some(primitive) {
            found = Some(Value::Object(o.clone()));
        }
    });
    found.unwrap_or_else(|| panic!("no {primitive} block"))
}

#[test]
fn views_match_v3() {
    let mut differences = Vec::new();
    for page in PAGES {
        let qid = page.split('.').next().unwrap();
        let content: Content =
            serde_json::from_str(&read(&format!("fixtures/content/{qid}.json"))).unwrap();
        let text: Text =
            serde_json::from_str(&read(&format!("fixtures/text/{page}.json"))).unwrap();
        let composition = qrender::compose(&content, Config::embedded());
        let rust = serde_json::to_value(qrender::localize(&composition, &text)).unwrap();
        let v3: Value =
            serde_json::from_str(&read(&format!("prototype/v3/data/view/{page}.json"))).unwrap();
        let mut eq = |what: String, a: &Value, b: &Value| {
            if a != b {
                differences.push(format!("{page} {what}:\n  rust {a}\n  v3   {b}"));
            }
        };

        eq("page_title".into(), &rust["page_title"], &v3["page_title"]);
        eq("dir".into(), &rust["dir"], &v3["dir"]);
        let coverage = &block(&v3, "provenance")["coverage"].clone();
        eq(
            "coverage".into(),
            &json!([rust["coverage"]["have"], rust["coverage"]["total"]]),
            &json!([coverage["have"], coverage["total"]]),
        );
        let lead = &block(&v3, "prose")["article"].clone();
        if !lead.is_null() {
            for key in ["lang", "local", "host", "url", "paragraphs", "more"] {
                eq(format!("lead.{key}"), &rust["lead"][key], &lead[key]);
            }
        }
        let ids = |v: &Value| {
            v.as_array()
                .unwrap()
                .iter()
                .map(|s| s["id"].clone())
                .collect::<Vec<_>>()
        };
        eq(
            "spreads".into(),
            &json!(ids(&rust["spreads"])),
            &json!(ids(&v3["spreads"])),
        );
        for (r, p) in rust["spreads"]
            .as_array()
            .unwrap()
            .iter()
            .zip(v3["spreads"].as_array().unwrap())
        {
            let id = p["id"].as_str().unwrap();
            eq(
                format!("{id}.title"),
                &r["title"]["text"],
                &p["title"]["text"],
            );
            eq(
                format!("{id}.prose_text"),
                &r["prose_text"],
                &p["prose_text"],
            );
            // v3 lists the topics with the sections; Rust has them in `topics`.
            let sections: Vec<_> = p["prose_links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|l| l.get("id").is_none())
                .cloned()
                .collect();
            eq(
                format!("{id}.prose_links"),
                &r["prose_links"],
                &json!(sections),
            );
        }
        eq(
            "unmatched".into(),
            &rust["unmatched_sections"],
            &v3["unmatched_sections"],
        );

        // Every label object of v3, against the table.
        let mut checked = 0;
        walk(&v3, &mut |o| {
            for label in o.values().filter_map(Value::as_object) {
                let (Some(id), Some(_)) = (
                    label.get("id").and_then(Value::as_str),
                    label.get("fallback"),
                ) else {
                    continue;
                };
                let r = &rust["labels"][id];
                let fallback = r.get("fallback").cloned().unwrap_or(json!(false));
                eq(
                    format!("label {id}"),
                    &json!([r["text"], r["lang"], fallback, r.get("add")]),
                    &json!([
                        label["text"],
                        label["lang"],
                        label["fallback"],
                        label.get("add")
                    ]),
                );
                checked += 1;
            }
        });
        assert!(checked > 0, "{page}: no labels checked");

        // Page images and descriptions on values.
        walk(&v3, &mut |o| {
            let Some(item) = o.get("item").and_then(Value::as_str) else {
                return;
            };
            if let Some(from) = o.get("image").and_then(|i| i.get("from")) {
                eq(
                    format!("page image {item}"),
                    &rust["page_images"][item]["from"],
                    from,
                );
            }
            if o.get("described") == Some(&json!(true))
                && let Some(d) = o.get("description")
            {
                eq(
                    format!("description {item}"),
                    &rust["labels"][item]["description"]["text"],
                    &d["text"],
                );
            }
        });
    }
    assert!(
        differences.is_empty(),
        "{} differences:\n{}",
        differences.len(),
        differences
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
