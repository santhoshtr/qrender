//! Card intermediate representation: a renderer-independent "facts about
//! an item" document. The factoid HTML renderer (and the JSON API) render
//! this; card kinds are derived from the typed values by `synthesize()`.

mod format;
mod synthesis;

pub use synthesis::synthesize;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FactoidPage {
    pub qid: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub language: String,
    pub cards: Vec<Card>,
}

#[derive(Debug, Serialize)]
pub struct Card {
    pub title: String,
    /// PIDs this card was built from, for provenance links back to Wikidata
    pub source_pids: Vec<String>,
    #[serde(flatten)]
    pub kind: CardKind,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CardKind {
    Image {
        image: GalleryImage,
    },
    Gallery {
        images: Vec<GalleryImage>,
    },
    Stat {
        value: String,
        note: Option<String>,
    },
    StatSeries {
        current: String,
        note: Option<String>,
        series: Vec<SeriesPoint>,
    },
    Map {
        lat: f64,
        lon: f64,
        label: String,
    },
    KeyValues {
        entries: Vec<KeyValueEntry>,
    },
    Links {
        entries: Vec<LinkEntry>,
    },
    ItemChips {
        items: Vec<ItemChip>,
    },
}

#[derive(Debug, Serialize)]
pub struct GalleryImage {
    pub file_name: String,
    /// Commons thumbnail (Special:FilePath with a width)
    pub thumb_url: String,
    /// Direct file URL (Special:FilePath), for audio/video sources
    pub file_url: String,
    /// Commons file description page, for attribution
    pub page_url: String,
    pub caption: String,
    pub media: MediaKind,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Audio,
    Video,
}

#[derive(Debug, Serialize)]
pub struct SeriesPoint {
    /// e.g. the year of the point-in-time qualifier
    pub label: String,
    pub value: f64,
    pub display: String,
}

#[derive(Debug, Serialize)]
pub struct KeyValueEntry {
    pub key: String,
    pub values: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LinkEntry {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ItemChip {
    pub qid: String,
    pub label: String,
    pub image_url: Option<String>,
    /// Qualifier summary, e.g. "start time: 1963"
    pub note: Option<String>,
}
