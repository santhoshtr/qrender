//! Wikimedia Commons: the metadata and assessments of files, and data
//! files (tabular data).

use indexmap::IndexMap;

use super::FetchError;
use super::http::{Http, Service};
use crate::geo;
use crate::html::{decode_entities, strip_tags};
use crate::model::{Cell, CommonsFile, Shape, Table};

const API: &str = "https://commons.wikimedia.org/w/api.php";
const BATCH: usize = 50;
/// Rows of a data file that a series can use.
const MAX_ROWS: usize = 1000;
/// The points of the outline of a geoshape at most: enough for a map on a
/// page.
const SHAPE_POINTS: usize = 1500;
const ASSESSMENTS: [&str; 3] = [
    "Featured pictures on Wikimedia Commons",
    "Quality images",
    "Valued images",
];

/// Size, type, author, licence and Commons assessments of each file.
pub async fn files(
    http: &Http,
    names: &[String],
) -> Result<IndexMap<String, CommonsFile>, FetchError> {
    let categories = ASSESSMENTS.map(|c| format!("Category:{c}")).join("|");
    let mut out = IndexMap::new();
    for batch in names.chunks(BATCH) {
        let titles = batch
            .iter()
            .map(|f| format!("File:{f}"))
            .collect::<Vec<_>>()
            .join("|");
        let json = http
            .json(
                Service::Api,
                API,
                &[
                    ("action", "query"),
                    ("format", "json"),
                    ("formatversion", "2"),
                    ("prop", "imageinfo|categories"),
                    ("titles", &titles),
                    ("iiprop", "size|mime|url|extmetadata"),
                    ("iiextmetadatafilter", "Artist|LicenseShortName"),
                    ("clcategories", &categories),
                    ("cllimit", "max"),
                ],
            )
            .await?;
        let query = &json["query"];
        // The title the API normalized -> the name that was asked for.
        let back: IndexMap<&str, &str> = query["normalized"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| Some((n["to"].as_str()?, n["from"].as_str()?)))
            .collect();
        for page in query["pages"].as_array().into_iter().flatten() {
            let Some(info) = page["imageinfo"].get(0) else {
                continue;
            };
            let title = page["title"].as_str().unwrap_or_default();
            let title = back.get(title).copied().unwrap_or(title);
            let name = title.strip_prefix("File:").unwrap_or(title).to_string();
            let meta = &info["extmetadata"];
            out.insert(
                name,
                CommonsFile {
                    width: info["width"].as_u64().map(|w| w as u32),
                    height: info["height"].as_u64().map(|h| h as u32),
                    bytes: info["size"].as_u64().unwrap_or(0),
                    url: info["url"]
                        .as_str()
                        .unwrap_or_default()
                        .split('?')
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    // Commons gives the author as HTML: the text only.
                    artist: decode_entities(&strip_tags(
                        meta["Artist"]["value"].as_str().unwrap_or_default(),
                    ))
                    .trim()
                    .to_string(),
                    license: meta["LicenseShortName"]["value"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    assessments: page["categories"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|c| {
                            c["title"]
                                .as_str()?
                                .strip_prefix("Category:")
                                .map(str::to_string)
                        })
                        .collect(),
                },
            );
        }
    }
    Ok(out)
}

/// A Commons data page ("Data:India.map"), from the URL that a value
/// holds: its title and its JSON.
async fn data_page(
    http: &Http,
    url: &str,
) -> Result<Option<(String, serde_json::Value)>, FetchError> {
    let Some(path) = url.rsplit_once("/data/main/").map(|(_, p)| p) else {
        return Ok(None);
    };
    let title = percent_encoding::percent_decode_str(path)
        .decode_utf8_lossy()
        .replace('+', " ");
    let json = http
        .json(
            Service::Api,
            API,
            &[
                ("action", "query"),
                ("format", "json"),
                ("formatversion", "2"),
                ("prop", "revisions"),
                ("titles", &title),
                ("rvprop", "content"),
                ("rvslots", "main"),
            ],
        )
        .await?;
    let Some(content) =
        json["query"]["pages"][0]["revisions"][0]["slots"]["main"]["content"].as_str()
    else {
        return Ok(None);
    };
    Ok(Some((title, serde_json::from_str(content)?)))
}

/// A Commons data file (a .tab page), from the URL that a TabularData
/// value holds: its fields, rows, licence and description.
pub async fn table(http: &Http, url: &str) -> Result<Option<Table>, FetchError> {
    let Some((title, doc)) = data_page(http, url).await? else {
        return Ok(None);
    };
    let fields = doc["schema"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["name"].as_str().map(str::to_string))
        .collect();
    let rows = doc["data"]
        .as_array()
        .into_iter()
        .flatten()
        .take(MAX_ROWS)
        .map(|row| {
            row.as_array()
                .into_iter()
                .flatten()
                .map(|c| serde_json::from_value(c.clone()).unwrap_or(Cell::Null))
                .collect()
        })
        .collect();
    Ok(Some(Table {
        title,
        fields,
        rows,
        license: doc["license"].as_str().unwrap_or_default().to_string(),
        description: serde_json::from_value(doc["description"].clone()).unwrap_or_default(),
    }))
}

/// The outline of a geoshape (a .map page), from the URL that a GeoShape
/// value holds: simplified to `SHAPE_POINTS` points, with its box, licence
/// and sources. None for a page without lines.
pub async fn shape(http: &Http, url: &str) -> Result<Option<Shape>, FetchError> {
    let Some((title, doc)) = data_page(http, url).await? else {
        return Ok(None);
    };
    let Some((lines, bbox)) = geo::outline(geo::lines(&doc["data"]), SHAPE_POINTS) else {
        return Ok(None);
    };
    Ok(Some(Shape {
        title,
        lines,
        bbox,
        license: doc["license"].as_str().unwrap_or_default().to_string(),
        sources: doc["sources"].as_str().unwrap_or_default().to_string(),
    }))
}
