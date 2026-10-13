//! One-time import of the v3 prototype data (`prototype/v3/data`) into the
//! typed fixtures (`fixtures/`). Delete this file when the Rust composer
//! has reached parity with v3; after that, fixtures come from the fetcher.
//!
//! Run: cargo test -p content --test import_v3 -- --ignored

use std::fs;
use std::path::{Path, PathBuf};

use content::{
    CONTENT_SCHEMA, CommonsFile, Concept, Container, Content, LangText, Peers, Qid, Qualifier,
    QualifierValue, QuantityRecord, Rank, ReverseWork, Statement, TEXT_SCHEMA, Table, Term, Text,
    Value, ValueImage,
};
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Deserialize)]
struct RawContent {
    qid: Qid,
    statements: Vec<RawStatement>,
    references: IndexMap<String, Vec<Vec<String>>>,
    container: Container,
    quantities: IndexMap<Qid, QuantityRecord>,
    peers: Option<Peers>,
    commons: IndexMap<String, CommonsFile>,
    value_classes: Vec<Qid>,
    sitelinks: IndexMap<String, String>,
    class_layout: String,
    reverse: Vec<ReverseWork>,
    tabular: IndexMap<String, Table>,
    #[serde(default)]
    logos: IndexMap<Qid, String>,
}

#[derive(Deserialize)]
struct RawStatement {
    id: String,
    pid: String,
    #[serde(rename = "type")]
    datatype: String,
    value: String,
    value_lang: Option<String>,
    rank: Rank,
    unit: Option<Qid>,
    precision: Option<u8>,
    value_image: Option<String>,
    value_image_from: Option<String>,
    qualifiers: Vec<RawQualifier>,
}

#[derive(Deserialize)]
struct RawQualifier {
    pid: String,
    value: String,
    value_lang: Option<String>,
}

#[derive(Deserialize)]
struct RawText {
    lang: String,
    labels: IndexMap<String, RawTerm>,
    concepts: IndexMap<Qid, RawConcept>,
    article: Option<content::Article>,
    fallback_article: Option<content::Article>,
    page_images: IndexMap<Qid, content::PageImage>,
}

#[derive(Deserialize)]
struct RawTerm {
    label: Option<String>,
    lang: Option<String>,
    description: Option<String>,
    description_lang: Option<String>,
}

#[derive(Deserialize)]
struct RawConcept {
    label: String,
    lang: Option<String>,
    names: Vec<String>,
}

fn lang_text(text: Option<String>, lang: Option<String>) -> Option<LangText> {
    Some(LangText {
        text: text?,
        lang: lang?.parse().expect("language code"),
    })
}

fn statement(raw: RawStatement) -> Statement {
    let value = Value::from_raw(
        &raw.datatype,
        &raw.value,
        raw.unit,
        raw.precision,
        raw.value_lang.as_deref(),
    )
    .unwrap_or_else(|e| panic!("statement {}: {e}", raw.id));
    Statement {
        id: raw.id.parse().expect("statement id"),
        pid: raw.pid.parse().expect("property id"),
        rank: raw.rank,
        value,
        qualifiers: raw
            .qualifiers
            .into_iter()
            .map(|q| Qualifier {
                pid: q.pid.parse().expect("qualifier property id"),
                value: QualifierValue::from_raw(&q.value, q.value_lang.as_deref()),
            })
            .collect(),
        value_image: raw.value_image.map(|file| ValueImage {
            file,
            logo: raw.value_image_from.as_deref() == Some("P154"),
        }),
    }
}

fn convert_content(raw: RawContent) -> Content {
    Content {
        schema: CONTENT_SCHEMA.to_string(),
        qid: raw.qid,
        statements: raw.statements.into_iter().map(statement).collect(),
        references: raw
            .references
            .into_iter()
            .map(|(id, sets)| {
                let sets = sets
                    .into_iter()
                    .map(|set| set.iter().map(|p| p.parse().expect("pid")).collect())
                    .collect();
                (id.parse().expect("statement id"), sets)
            })
            .collect(),
        container: raw.container,
        quantities: raw.quantities,
        peers: raw.peers,
        commons: raw.commons,
        value_classes: raw.value_classes,
        sitelinks: raw.sitelinks,
        class_layout: raw.class_layout,
        reverse: raw.reverse,
        tabular: raw.tabular,
        logos: raw.logos,
    }
}

fn convert_text(raw: RawText) -> Text {
    Text {
        schema: TEXT_SCHEMA.to_string(),
        lang: raw.lang.parse().expect("language code"),
        labels: raw
            .labels
            .into_iter()
            .map(|(id, t)| {
                let term = Term {
                    label: lang_text(t.label, t.lang),
                    description: lang_text(t.description, t.description_lang),
                };
                (id.parse().expect("entity id"), term)
            })
            .collect(),
        concepts: raw
            .concepts
            .into_iter()
            .map(|(q, c)| {
                let concept = Concept {
                    // Without a label, v3 wrote the identifier and no language.
                    label: lang_text(Some(c.label), c.lang),
                    names: c.names,
                };
                (q, concept)
            })
            .collect(),
        article: raw.article,
        fallback_article: raw.fallback_article,
        page_images: raw.page_images,
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

fn write(path: PathBuf, value: &impl serde::Serialize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut json = serde_json::to_string_pretty(value).unwrap();
    json.push('\n');
    fs::write(&path, json).unwrap();
    println!("wrote {}", path.display());
}

#[test]
#[ignore = "one-time import; writes fixtures/"]
fn import_v3() {
    let v3 = root().join("prototype/v3/data");
    let out = root().join("fixtures");
    for path in json_files(&v3.join("content")) {
        let raw: RawContent = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        write(
            out.join("content").join(path.file_name().unwrap()),
            &convert_content(raw),
        );
    }
    for path in json_files(&v3.join("text")) {
        let raw: RawText = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        write(
            out.join("text").join(path.file_name().unwrap()),
            &convert_text(raw),
        );
    }
}
