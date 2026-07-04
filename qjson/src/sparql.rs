//! SPARQL query construction and the WDQS HTTP client.

use serde::Deserialize;
use std::collections::HashMap;

use crate::error::QjsonError;

pub const WIKIDATA_ENDPOINT: &str = "https://query.wikidata.org/sparql";
const USER_AGENT: &str =
    "qrender/0.1 (https://github.com/santhoshtr/qrender; santhosh.thottingal@gmail.com)";

// SPARQL JSON results (https://www.w3.org/TR/sparql11-results-json/)
#[derive(Debug, Deserialize)]
pub struct SparqlResponse {
    pub results: SparqlResults,
}

#[derive(Debug, Deserialize)]
pub struct SparqlResults {
    pub bindings: Vec<Binding>,
}

pub type Binding = HashMap<String, SparqlValue>;

#[derive(Debug, Deserialize)]
pub struct SparqlValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: String,
    pub datatype: Option<String>,
    #[serde(rename = "xml:lang")]
    pub lang: Option<String>,
}

/// QIDs and language codes are interpolated into the SPARQL query, so they
/// must be validated first (the Go tool interpolated `lang` unvalidated).
pub fn validate_qid(qid: &str) -> Result<(), QjsonError> {
    let ok = qid.len() > 1 && qid.starts_with('Q') && qid[1..].bytes().all(|b| b.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(QjsonError::InvalidQid(qid.to_string()))
    }
}

pub fn validate_language(language: &str) -> Result<(), QjsonError> {
    let ok = !language.is_empty()
        && language.len() <= 32
        && language
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(QjsonError::InvalidLanguage(language.to_string()))
    }
}

/// One query fetches every statement of the item with labels resolved by
/// the label service. The fallback chain includes "mul": Wikidata is
/// migrating language-independent labels to the mul language code, and
/// items like Q42 no longer have an en rdfs:label at all.
/// Ported from the Go tool, additionally selecting:
/// - ?statement       statement node URI, the grouping key
/// - ?propertyType    wikibase datatype, drives typed value parsing
/// - ?timePrecision   Wikidata time precision from the value node
/// - ?itemLabel/?itemDescription for the page header
pub fn build_query(qid: &str, language: &str) -> String {
    format!(
        r#"
    SELECT
        ?itemLabel
        ?itemDescription
        ?property
        ?propertyLabel
        ?propertyType
        ?statement
        ?statementValue
        ?statementValueLabel
        ?statementValueImage
        ?qualifierProperty
        ?qualifierPropertyLabel
        ?qualifierValue
        ?qualifierValueLabel
        ?unitOfMeasure
        ?unitOfMeasureLabel
        ?timePrecision
        ?statementRankLabel
    WHERE {{
        VALUES ?item {{wd:{qid}}}

        ?item ?propertyPredicate ?statement .
        ?statement ?statementPropertyPredicate ?statementValue .

        ?property wikibase:claim ?propertyPredicate .
        ?property wikibase:statementProperty ?statementPropertyPredicate .
        ?property wikibase:propertyType ?propertyType .

        ?statement wikibase:rank ?statementRank .
        BIND(
            IF(?statementRank = wikibase:NormalRank, "Normal",
                IF(?statementRank = wikibase:PreferredRank, "Preferred",
                    IF(?statementRank = wikibase:DeprecatedRank, "Deprecated", "Unknown")
                )
            ) AS ?statementRankLabel
        )

        OPTIONAL {{
            ?statementValue wdt:P18 ?statementValueImage .
        }}

        OPTIONAL {{
            ?statement ?qualifierPredicate ?qualifierValue .
            ?qualifierProperty wikibase:qualifier ?qualifierPredicate .
        }}

        ?property wikibase:statementValue ?statementValueNodePredicate .
        OPTIONAL {{
            ?statement ?statementValueNodePredicate ?valueNode .
            ?valueNode wikibase:quantityUnit ?unitOfMeasure .
        }}
        OPTIONAL {{
            ?statement ?statementValueNodePredicate ?timeValueNode .
            ?timeValueNode wikibase:timePrecision ?timePrecision .
        }}

        SERVICE wikibase:label {{
            bd:serviceParam wikibase:language "{language}, mul, en" .
            ?item rdfs:label ?itemLabel .
            ?item schema:description ?itemDescription .
            ?property rdfs:label ?propertyLabel .
            ?statementValue rdfs:label ?statementValueLabel .
            ?qualifierProperty rdfs:label ?qualifierPropertyLabel .
            ?qualifierValue rdfs:label ?qualifierValueLabel .
            ?unitOfMeasure rdfs:label ?unitOfMeasureLabel .
        }}
    }}
    ORDER BY ?property ?statement ?qualifierProperty ?qualifierValue
    "#
    )
}

pub async fn fetch_bindings(qid: &str, language: &str) -> Result<Vec<Binding>, QjsonError> {
    validate_qid(qid)?;
    validate_language(language)?;

    let query = build_query(qid, language);
    let client = reqwest::Client::new();
    let response = client
        .get(WIKIDATA_ENDPOINT)
        .query(&[("query", query.as_str())])
        .header(reqwest::header::ACCEPT, "application/sparql-results+json")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(QjsonError::WdqsStatus(response.status().as_u16()));
    }

    let sparql_response: SparqlResponse = response.json().await?;
    Ok(sparql_response.results.bindings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qid_validation() {
        assert!(validate_qid("Q42").is_ok());
        assert!(validate_qid("Q").is_err());
        assert!(validate_qid("P42").is_err());
        assert!(validate_qid("Q42}").is_err());
    }

    #[test]
    fn language_validation() {
        assert!(validate_language("en").is_ok());
        assert!(validate_language("zh-hans").is_ok());
        assert!(validate_language("").is_err());
        assert!(validate_language("en\" . ?x ?y ?z").is_err());
    }
}
