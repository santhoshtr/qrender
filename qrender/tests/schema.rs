//! The JSON Schema of the composition, generated from the Rust types, is
//! checked in at `schema/`, so other programs (the Python prototype, an
//! app) can validate against it. After an intended change of the types:
//! `UPDATE_SCHEMA=1 cargo test -p qrender --test schema`, and review the diff.

use std::fs;
use std::path::Path;

use qrender::schema::{COMPOSITION_SCHEMA, Composition};

/// The schema of what serde writes: a field that is skipped when it is
/// empty is optional.
fn schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<Composition>()
}

#[test]
fn composition_schema_is_up_to_date() {
    let schema = schema();
    let generated = serde_json::to_string_pretty(&schema).unwrap() + "\n";
    let name = COMPOSITION_SCHEMA.replace('/', "-");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../schema/{name}.json"));
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

/// What serde writes is what the schema says: every fixture composition
/// validates against the generated schema.
#[test]
fn compositions_validate() {
    let schema = serde_json::to_value(schema()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for qid in ["q3870", "q90", "q8023", "q117085614", "q405", "q139890532"] {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../fixtures/content/{qid}.json"));
        let content: content::Content =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let composition = qrender::compose(&content, qrender::config::Config::embedded());
        let json = serde_json::to_value(&composition).unwrap();
        let errors: Vec<_> = validator
            .iter_errors(&json)
            .take(5)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{qid}:\n{}", errors.join("\n"));
    }
}
