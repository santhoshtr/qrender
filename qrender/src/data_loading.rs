//! Data loading via the qjson crate (in-process SPARQL fetch + cache),
//! adapted to the legacy model shape the Handlebars templates consume.

use crate::error::QRenderError;
use crate::model::{Property, Qualifier, Statement, StatementType, WikidataItem};

pub async fn fetch_typed(qid: &str, language: &str) -> Result<qjson::WikidataItem, QRenderError> {
    Ok(qjson::Client::new().get_item(qid, language).await?)
}

pub async fn fetch_wikidata_item(
    qid: &str,
    language: &str,
) -> Result<WikidataItem, QRenderError> {
    Ok(to_legacy(&fetch_typed(qid, language).await?))
}

/// Map the typed qjson model onto the flat legacy structs. Rendering-visible
/// fields (value, qid, image_url, qualifier label/value) reproduce what the
/// qjson HTTP service returned.
pub fn to_legacy(item: &qjson::WikidataItem) -> WikidataItem {
    let properties = item
        .properties
        .iter()
        .map(|(pid, property)| {
            (
                pid.clone(),
                Property {
                    pid: property.pid.clone(),
                    wd_label: property.label.clone(),
                    statements: property.statements.iter().map(to_legacy_statement).collect(),
                },
            )
        })
        .collect();
    WikidataItem { properties }
}

fn statement_type(value: &qjson::Value) -> StatementType {
    match value {
        qjson::Value::ItemRef { .. }
        | qjson::Value::Url { .. }
        | qjson::Value::CommonsMedia { .. } => StatementType::Uri,
        _ => StatementType::Literal,
    }
}

fn datatype_name(value: &qjson::Value) -> String {
    match value {
        qjson::Value::Text { .. } => "string",
        qjson::Value::MonolingualText { .. } => "monolingualtext",
        qjson::Value::ItemRef { .. } => "wikibase-item",
        qjson::Value::Time { .. } => "time",
        qjson::Value::Quantity { .. } => "quantity",
        qjson::Value::Coordinate { .. } => "globe-coordinate",
        qjson::Value::CommonsMedia { .. } => "commonsMedia",
        qjson::Value::Url { .. } => "url",
        qjson::Value::ExternalId { .. } => "external-id",
    }
    .to_string()
}

fn to_legacy_statement(statement: &qjson::Statement) -> Statement {
    let value = &statement.value;
    let (unit_of_measure, unit_of_measure_label) = match value {
        qjson::Value::Quantity {
            unit_qid,
            unit_label,
            ..
        } => (
            unit_qid
                .as_ref()
                .map(|qid| format!("http://www.wikidata.org/entity/{qid}")),
            unit_label.clone(),
        ),
        _ => (None, None),
    };
    let qualifiers: Vec<Qualifier> = statement
        .qualifiers
        .iter()
        .map(|qualifier| Qualifier {
            datatype: datatype_name(&qualifier.value),
            _type: match statement_type(&qualifier.value) {
                StatementType::Uri => "uri".to_string(),
                StatementType::Literal => "literal".to_string(),
            },
            pid: qualifier.pid.clone(),
            value: qualifier.value.display().to_string(),
            label: qualifier.label.clone(),
        })
        .collect();

    Statement {
        statement_type: statement_type(value),
        datatype: datatype_name(value),
        value: value.display().to_string(),
        rank: format!("{:?}", statement.rank),
        unit_of_measure,
        unit_of_measure_label,
        qualifiers: if qualifiers.is_empty() {
            None
        } else {
            Some(qualifiers)
        },
        qid: match value {
            qjson::Value::ItemRef { qid, .. } => Some(qid.clone()),
            _ => None,
        },
        image_url: match value {
            qjson::Value::ItemRef { image_url, .. } => image_url.clone(),
            _ => None,
        },
    }
}
