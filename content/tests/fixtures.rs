//! Every fixture deserializes into the typed model, and the parsers know
//! every value in it.

use std::fs;
use std::path::{Path, PathBuf};

use content::{CONTENT_SCHEMA, Content, Datatype, TEXT_SCHEMA, Text, Value};

fn fixtures(kind: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures")
        .join(kind);
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    files
}

#[test]
fn content_fixtures() {
    let files = fixtures("content");
    assert_eq!(files.len(), 8);
    for path in files {
        let c: Content = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(c.schema, CONTENT_SCHEMA);
        assert!(!c.statements.is_empty(), "{}", path.display());
        // "Unknown value" (SomeValue) is a value of the model; an unknown
        // datatype (Other) is not.
        for s in &c.statements {
            assert!(
                !matches!(s.value, Value::Other { .. }),
                "{}: {} has {:?}",
                path.display(),
                s.id,
                s.value
            );
            assert_ne!(s.value.datatype(), Datatype::Other);
        }
    }
}

#[test]
fn text_fixtures() {
    let files = fixtures("text");
    assert_eq!(files.len(), 14);
    for path in files {
        let t: Text = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(t.schema, TEXT_SCHEMA);
        assert!(!t.labels.is_empty(), "{}", path.display());
    }
}
