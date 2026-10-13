//! Transform SPARQL bindings into the typed model.
//!
//! Rows are grouped by the ?statement node URI. The Go tool grouped by
//! value label, which collapsed distinct statements sharing a label and
//! attached qualifiers to the wrong statement.

use percent_encoding::percent_decode_str;
use std::collections::HashMap;

use crate::model::{Property, Qualifier, Rank, Statement, Value, WikidataItem};
use crate::sparql::{Binding, SparqlValue};

const ENTITY_PREFIX: &str = "http://www.wikidata.org/entity/";
const FILEPATH_PREFIX: &str = "http://commons.wikimedia.org/wiki/Special:FilePath/";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const WKT_LITERAL: &str = "http://www.opengis.net/ont/geosparql#wktLiteral";

fn last_uri_segment(uri: &str) -> &str {
    uri.rsplit('/').next().unwrap_or(uri)
}

fn entity_id(uri: &str) -> Option<&str> {
    uri.strip_prefix(ENTITY_PREFIX)
}

fn commons_file_name(url: &str) -> String {
    let encoded = url.strip_prefix(FILEPATH_PREFIX).unwrap_or(url);
    percent_decode_str(encoded).decode_utf8_lossy().into_owned()
}

/// WKT coordinate literal: "Point(lon lat)", optionally prefixed with a
/// globe URI for non-Earth coordinates: "<http://...entity/Q405> Point(lon lat)"
fn parse_wkt_point(wkt: &str) -> Option<(f64, f64)> {
    let start = wkt.find("Point(")? + "Point(".len();
    let end = wkt[start..].find(')')? + start;
    let mut parts = wkt[start..end].split_whitespace();
    let lon: f64 = parts.next()?.parse().ok()?;
    let lat: f64 = parts.next()?.parse().ok()?;
    Some((lat, lon))
}

impl Rank {
    fn from_label(label: Option<&SparqlValue>) -> Rank {
        match label.map(|v| v.value.as_str()) {
            Some("Preferred") => Rank::Preferred,
            Some("Deprecated") => Rank::Deprecated,
            _ => Rank::Normal,
        }
    }
}

/// Parse a statement value using the property's wikibase datatype
/// (http://wikiba.se/ontology#Time, #Quantity, ...).
fn parse_statement_value(
    property_type: &str,
    value: &SparqlValue,
    label: Option<&SparqlValue>,
) -> Value {
    let display = label.map(|l| l.value.clone());
    match last_uri_segment(property_type) {
        "ontology#Time" | "Time" => Value::Time {
            iso: value.value.clone(),
            precision: None, // attached later from the value node row
        },
        "ontology#Quantity" | "Quantity" => match value.value.parse::<f64>() {
            Ok(amount) => Value::Quantity {
                amount,
                raw: value.value.clone(),
                unit_qid: None,   // attached later
                unit_label: None, // attached later
            },
            Err(_) => Value::Text {
                text: display.unwrap_or_else(|| value.value.clone()),
            },
        },
        "ontology#GlobeCoordinate" | "GlobeCoordinate" => match parse_wkt_point(&value.value) {
            Some((lat, lon)) => Value::Coordinate {
                lat,
                lon,
                raw: value.value.clone(),
            },
            None => Value::Text {
                text: value.value.clone(),
            },
        },
        "ontology#CommonsMedia" | "CommonsMedia" => Value::CommonsMedia {
            file_name: commons_file_name(&value.value),
            url: value.value.clone(),
        },
        "ontology#Url" | "Url" => Value::Url {
            url: value.value.clone(),
        },
        "ontology#GeoShape" | "GeoShape" => Value::GeoShape {
            url: value.value.clone(),
        },
        "ontology#ExternalId" | "ExternalId" => Value::ExternalId {
            id: value.value.clone(),
        },
        "ontology#Monolingualtext" | "Monolingualtext" => Value::MonolingualText {
            text: value.value.clone(),
            language: value.lang.clone().unwrap_or_default(),
        },
        "ontology#WikibaseItem" | "WikibaseItem" => match entity_id(&value.value) {
            Some(qid) => Value::ItemRef {
                qid: qid.to_string(),
                label: display.unwrap_or_else(|| qid.to_string()),
                image_url: None, // attached later
            },
            None => Value::Text {
                text: display.unwrap_or_else(|| value.value.clone()),
            },
        },
        // String, Math, TabularData, ...
        _ => {
            if value.value_type == "uri" {
                Value::Url {
                    url: value.value.clone(),
                }
            } else {
                Value::Text {
                    text: display.unwrap_or_else(|| value.value.clone()),
                }
            }
        }
    }
}

/// Qualifier property types are not in the query; parse from the SPARQL
/// value's own datatype and shape.
fn parse_qualifier_value(value: &SparqlValue, label: Option<&SparqlValue>) -> Value {
    match value.datatype.as_deref() {
        Some(XSD_DATETIME) => Value::Time {
            iso: value.value.clone(),
            precision: None,
        },
        Some(XSD_DECIMAL) => match value.value.parse::<f64>() {
            Ok(amount) => Value::Quantity {
                amount,
                raw: value.value.clone(),
                unit_qid: None,
                unit_label: None,
            },
            Err(_) => Value::Text {
                text: value.value.clone(),
            },
        },
        Some(WKT_LITERAL) => match parse_wkt_point(&value.value) {
            Some((lat, lon)) => Value::Coordinate {
                lat,
                lon,
                raw: value.value.clone(),
            },
            None => Value::Text {
                text: value.value.clone(),
            },
        },
        _ => {
            if value.value_type == "uri" {
                if let Some(qid) = entity_id(&value.value) {
                    Value::ItemRef {
                        qid: qid.to_string(),
                        label: label
                            .map(|l| l.value.clone())
                            .unwrap_or_else(|| qid.to_string()),
                        image_url: None,
                    }
                } else if value.value.starts_with(FILEPATH_PREFIX) {
                    Value::CommonsMedia {
                        file_name: commons_file_name(&value.value),
                        url: value.value.clone(),
                    }
                } else {
                    Value::Url {
                        url: value.value.clone(),
                    }
                }
            } else if let Some(lang) = &value.lang {
                Value::MonolingualText {
                    text: value.value.clone(),
                    language: lang.clone(),
                }
            } else {
                Value::Text {
                    text: value.value.clone(),
                }
            }
        }
    }
}

pub fn transform(qid: &str, bindings: &[Binding]) -> WikidataItem {
    let mut item = WikidataItem {
        qid: qid.to_string(),
        label: None,
        description: None,
        properties: HashMap::new(),
    };
    // statement URI -> (pid, index into that property's statements)
    let mut statement_index: HashMap<String, (String, usize)> = HashMap::new();
    // (statement URI, qualifier pid, raw qualifier value) seen, for dedup
    let mut seen_qualifiers: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for binding in bindings {
        if item.label.is_none()
            && let Some(l) = binding.get("itemLabel")
        {
            item.label = Some(l.value.clone());
        }
        if item.description.is_none()
            && let Some(d) = binding.get("itemDescription")
        {
            item.description = Some(d.value.clone());
        }

        let (Some(property), Some(property_label), Some(property_type)) = (
            binding.get("property"),
            binding.get("propertyLabel"),
            binding.get("propertyType"),
        ) else {
            continue;
        };
        let (Some(statement_node), Some(statement_value)) =
            (binding.get("statement"), binding.get("statementValue"))
        else {
            continue;
        };

        let pid = last_uri_segment(&property.value).to_string();
        let statement_uri = &statement_node.value;

        let property_entry = item
            .properties
            .entry(pid.clone())
            .or_insert_with(|| Property {
                pid: pid.clone(),
                label: property_label.value.clone(),
                statements: Vec::new(),
            });

        // First row for this statement creates it; later rows only add
        // qualifiers / value-node data.
        let statement_pos = match statement_index.get(statement_uri) {
            Some((_, pos)) => *pos,
            None => {
                let mut value = parse_statement_value(
                    &property_type.value,
                    statement_value,
                    binding.get("statementValueLabel"),
                );
                if let Value::ItemRef { image_url, .. } = &mut value {
                    *image_url = binding.get("statementValueImage").map(|v| v.value.clone());
                }
                property_entry.statements.push(Statement {
                    value,
                    rank: Rank::from_label(binding.get("statementRankLabel")),
                    qualifiers: Vec::new(),
                });
                let pos = property_entry.statements.len() - 1;
                statement_index.insert(statement_uri.clone(), (pid.clone(), pos));
                pos
            }
        };
        let statement = &mut property_entry.statements[statement_pos];

        // Value-node extras may arrive on any row of this statement.
        // Q199 ("1") marks a dimensionless quantity - not a unit worth showing.
        if let Some(unit) = binding.get("unitOfMeasure")
            && let Value::Quantity {
                unit_qid,
                unit_label,
                ..
            } = &mut statement.value
            && entity_id(&unit.value) != Some("Q199")
        {
            *unit_qid = entity_id(&unit.value).map(str::to_string);
            if unit_label.is_none() {
                *unit_label = binding.get("unitOfMeasureLabel").map(|v| v.value.clone());
            }
        }
        if let Some(p) = binding.get("timePrecision")
            && let Value::Time { precision, .. } = &mut statement.value
        {
            *precision = p.value.parse().ok();
        }

        if let (Some(qualifier_property), Some(qualifier_value)) = (
            binding.get("qualifierProperty"),
            binding.get("qualifierValue"),
        ) {
            let qualifier_pid = last_uri_segment(&qualifier_property.value).to_string();
            let key = (qualifier_pid.clone(), qualifier_value.value.clone());
            let seen = seen_qualifiers.entry(statement_uri.clone()).or_default();
            if !seen.contains(&key) {
                seen.push(key);
                statement.qualifiers.push(Qualifier {
                    pid: qualifier_pid,
                    label: binding
                        .get("qualifierPropertyLabel")
                        .map(|v| v.value.clone())
                        .unwrap_or_default(),
                    value: parse_qualifier_value(
                        qualifier_value,
                        binding.get("qualifierValueLabel"),
                    ),
                });
            }
        }
    }

    // Deprecated statements are wrong-by-assertion; nothing should render them
    for property in item.properties.values_mut() {
        property.statements.retain(|s| s.rank != Rank::Deprecated);
    }
    item.properties.retain(|_, p| !p.statements.is_empty());

    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparql::SparqlResponse;

    fn nairobi() -> WikidataItem {
        let response: SparqlResponse =
            serde_json::from_str(include_str!("../tests/fixtures/Q3870.sparql.json")).unwrap();
        transform("Q3870", &response.results.bindings)
    }

    #[test]
    fn item_header() {
        let item = nairobi();
        assert_eq!(item.label.as_deref(), Some("Nairobi"));
        assert_eq!(item.description.as_deref(), Some("capital city of Kenya"));
    }

    #[test]
    fn population_series_is_typed() {
        let item = nairobi();
        let population = &item.properties["P1082"];
        assert_eq!(population.statements.len(), 3);
        for statement in &population.statements {
            let Value::Quantity { amount, .. } = &statement.value else {
                panic!("population must be Quantity, got {:?}", statement.value);
            };
            assert!(*amount > 1_000_000.0);
            // each population statement carries a point-in-time qualifier
            assert!(
                statement
                    .qualifiers
                    .iter()
                    .any(|q| q.pid == "P585" && matches!(q.value, Value::Time { .. }))
            );
        }
    }

    #[test]
    fn coordinates_are_parsed() {
        let item = nairobi();
        let coordinates = &item.properties["P625"];
        let Value::Coordinate { lat, lon, .. } = &coordinates.statements[0].value else {
            panic!("P625 must be Coordinate");
        };
        assert!((lat - -1.286).abs() < 0.01);
        assert!((lon - 36.817).abs() < 0.01);
    }

    #[test]
    fn commons_media_file_name_is_decoded() {
        let item = nairobi();
        let image = &item.properties["P18"];
        let Value::CommonsMedia { file_name, .. } = &image.statements[0].value else {
            panic!("P18 must be CommonsMedia");
        };
        assert!(!file_name.contains("%20"), "must be percent-decoded");
        assert!(!file_name.contains("Special:FilePath"));
    }

    #[test]
    fn time_precision_is_attached() {
        let item = nairobi();
        let inception = &item.properties["P571"];
        let Value::Time { precision, .. } = &inception.statements[0].value else {
            panic!("P571 must be Time");
        };
        assert!(precision.is_some());
    }

    #[test]
    fn item_valued_statements_have_labels() {
        let item = nairobi();
        let country = &item.properties["P17"];
        let Value::ItemRef { qid, label, .. } = &country.statements[0].value else {
            panic!("P17 must be ItemRef");
        };
        assert_eq!(qid, "Q114");
        assert_eq!(label, "Kenya");
    }

    #[test]
    fn statements_with_same_label_stay_distinct() {
        // Regression for the Go bug: grouping by value label collapsed
        // distinct statements. Demonym "Nairobian" (en) and others share
        // labels across languages; ensure statement count matches the
        // distinct statement nodes, not distinct labels.
        let item = nairobi();
        let demonym = &item.properties["P1549"];
        assert!(demonym.statements.len() >= 5);
    }
}
