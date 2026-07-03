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
    /// The item's main image (P18), shown beside the title
    pub hero: Option<GalleryImage>,
    pub cards: Vec<Card>,
}

/// Layout preference for the bento grid: column/row spans (in grid
/// units) and sort weight. Resolved by cascade: kind defaults with
/// content clamps, then group config, then per-PID config.
#[derive(Debug, Serialize, Clone, Copy)]
pub struct Layout {
    pub cols: u8,
    pub rows: u8,
    pub sort: i32,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            cols: 2,
            rows: 2,
            sort: 1000,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Card {
    pub title: String,
    /// True when the title is a label-service-localized property label.
    /// False for machine group names, which the factoid page hides
    /// visually when an icon carries the meaning instead.
    pub localized_title: bool,
    /// Symbol name from assets/icons/ (see icons::lookup)
    pub icon: Option<String>,
    /// PIDs this card was built from, for provenance links back to Wikidata
    pub source_pids: Vec<String>,
    pub layout: Layout,
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
    /// A quantity on a known scale, e.g. HDI - rendered as a gauge
    Meter {
        value: f64,
        display: String,
        note: Option<String>,
        min: f64,
        max: f64,
        low: Option<f64>,
        high: Option<f64>,
        optimum: Option<f64>,
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
    /// Small thumbnail of the referenced item's P18, for visual chips
    pub thumb_url: Option<String>,
    /// Qualifier summary, e.g. "start time: 1963"
    pub note: Option<String>,
}
