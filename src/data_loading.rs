use crate::{config::USER_AGENT, model::WikidataItem};

pub async fn fetch_wikidata_item(
    qid: &str,
    language: &str,
) -> Result<WikidataItem, Box<dyn std::error::Error>> {
    // Construct the URL for the qjson lookup. API Example: https://qjson.toolforge.org/Q405.json
    let api_url = format!("https://qjson.toolforge.org/{}.json?lang={}", qid, language);
    // Make a GET request to the API
    let client = reqwest::Client::new();
    let response = client
        .get(&api_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .unwrap();

    let json_text = response.text().await?;
    let wikidata_properties = serde_json::from_str(&json_text)?;

    Ok(WikidataItem {
        properties: wikidata_properties,
    })
}
