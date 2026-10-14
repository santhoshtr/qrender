//! The JSON Schemas of the composition and the view, generated from the
//! Rust types, are checked in at `schema/`, so other programs (the Python
//! prototype, an app) can validate against them. After an intended change
//! of the types: `UPDATE_SCHEMA=1 cargo test -p qrender --test schema`,
//! and review the diff.

use std::fs;
use std::path::Path;

use content::{Content, Text};
use qrender::config::Config;
use qrender::localize::{VIEW_SCHEMA, View};
use qrender::schema::{COMPOSITION_SCHEMA, Composition};
use schemars::JsonSchema;
use serde_json::Value;

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

/// The schema of what serde writes: a field that is skipped when it is
/// empty is optional.
fn schema<T: JsonSchema>() -> Value {
    let schema = schemars::generate::SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>();
    serde_json::to_value(schema).unwrap()
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn check_in(version: &str, schema: &Value) {
    let generated = serde_json::to_string_pretty(schema).unwrap() + "\n";
    let path = root().join(format!("schema/{}.json", version.replace('/', "-")));
    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &generated).unwrap();
    }
    let checked_in = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        checked_in == generated,
        "{} is not the schema of the types; run with UPDATE_SCHEMA=1",
        path.display()
    );
}

fn validate(schema: &Value, name: &str, json: &Value) {
    let validator = jsonschema::validator_for(schema).unwrap();
    let errors: Vec<_> = validator
        .iter_errors(json)
        .take(5)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{name}:\n{}", errors.join("\n"));
}

fn read<T: serde::de::DeserializeOwned>(path: &str) -> T {
    serde_json::from_str(&fs::read_to_string(root().join(path)).unwrap()).unwrap()
}

#[test]
fn schemas_are_up_to_date() {
    check_in(COMPOSITION_SCHEMA, &schema::<Composition>());
    check_in(VIEW_SCHEMA, &schema::<View>());
}

/// What serde writes is what the schema says: every fixture composition
/// and view validates against its schema.
#[test]
fn fixtures_validate() {
    let (composition_schema, view_schema) = (schema::<Composition>(), schema::<View>());
    for page in PAGES {
        let qid = page.split('.').next().unwrap();
        let content: Content = read(&format!("fixtures/content/{qid}.json"));
        let text: Text = read(&format!("fixtures/text/{page}.json"));
        let composition = qrender::compose(&content, Config::embedded());
        validate(
            &composition_schema,
            qid,
            &serde_json::to_value(&composition).unwrap(),
        );
        let view = qrender::localize(&composition, &text, Config::embedded());
        validate(&view_schema, page, &serde_json::to_value(&view).unwrap());
    }
}
