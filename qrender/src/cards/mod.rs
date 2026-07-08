//! Card intermediate representation: a renderer-independent "facts about
//! an item" document. The factoid HTML renderer (and the JSON API) render
//! this; card kinds are derived from the typed values by `synthesize()`.

mod compose;
mod density;
mod format;
mod plan;
mod synthesis;

pub use plan::Variant;
pub use synthesis::synthesize;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FactoidPage {
    pub qid: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub language: String,
    /// Composition recipe this item resolved to ("person", "place",
    /// "generic", ...); selects sections and, for HTML, the accent theme
    pub archetype: String,
    /// The item's main image (P18), shown beside the title
    pub hero: Option<GalleryImage>,
    /// Archetype-selected facts rendered inside the header
    pub hero_facts: Option<HeroFacts>,
    /// Archetype-composed regions, in reading order
    pub sections: Vec<Section>,
    /// Cards no section claimed - the bento grid
    pub overflow: Vec<Card>,
    /// Footnote-tier cards, collapsed at the page end
    pub footnotes: Vec<Card>,
}

/// A titled region of a composed page. The name is a machine name;
/// visually the icon carries the meaning (no string to translate).
#[derive(Debug, Serialize)]
pub struct Section {
    pub name: String,
    pub icon: Option<String>,
    pub cards: Vec<Card>,
}

/// Header facts resolved from the archetype's hero config. All parts
/// are optional; whatever the data lacks simply doesn't render.
#[derive(Debug, Serialize)]
pub struct HeroFacts {
    /// e.g. "1952 – 2001" (years only, en dash)
    pub date_range: Option<String>,
    /// Joined value labels, e.g. "writer · screenwriter"
    pub tagline: Option<String>,
    /// A small image, e.g. the P109 signature
    pub emblem: Option<GalleryImage>,
}

impl FactoidPage {
    /// Every card in reading order: sections, then overflow, then
    /// footnotes. Textual backends and the sprite builder walk this.
    pub fn all_cards(&self) -> impl Iterator<Item = &Card> {
        self.sections
            .iter()
            .flat_map(|s| s.cards.iter())
            .chain(self.overflow.iter())
            .chain(self.footnotes.iter())
    }
}

/// Layout preference for the bento grid: column/row spans (in grid
/// units) and sort weight. Resolved by cascade: kind defaults with
/// content clamps, then group config, then per-PID config.
#[derive(Debug, Serialize, Clone, Copy)]
pub struct Layout {
    pub cols: u8,
    pub rows: u8,
    pub sort: i32,
    /// Tiles a cover-chips card shows; when more values exist the +N
    /// button follows (fourth cell, or the side rail when this is 1)
    pub cover_values: usize,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            cols: 2,
            rows: 2,
            sort: 1000,
            cover_values: 2,
        }
    }
}

/// Visual weight class. Standard cards fill the main grid; footnote
/// cards (Wikimedia-curation meta, flagged in groups.toml) collapse
/// into a details region at the page end.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Standard,
    Footnote,
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
    /// Visual treatment, chosen from the card's content census
    /// (plan.rs); the factoid page keys its layout off this
    pub variant: Variant,
    pub layout: Layout,
    pub tier: Tier,
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
        /// Commons map data URL (P3896 geoshape) from the same group,
        /// drawn as an outline by the interactive viewer
        geoshape: Option<String>,
    },
    /// Labeled rows of rich values - the workhorse card. A group's
    /// properties render as one scannable card, one row each; values
    /// are item chips (with thumbnails), links, or plain text.
    Facts {
        rows: Vec<FactRow>,
    },
    /// Cross-property chronology: dated statements (Time values and
    /// start-time/point-in-time qualifiers) merged and sorted
    Timeline {
        events: Vec<TimelineEvent>,
    },
    /// Sibling time-series consolidated by the density pass: one row
    /// per indicator (label, current value, sparkline history) instead
    /// of a wall of identical chart cards
    Indicators {
        indicators: Vec<Indicator>,
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
pub struct TimelineEvent {
    /// ISO timestamp, the sort key
    pub iso: String,
    /// Formatted for the value's precision, e.g. "1974"
    pub display: String,
    /// Localized property label, e.g. "award received"
    pub label: String,
    /// Value label for item-valued events, e.g. "Hugo Award"
    pub detail: Option<String>,
    /// Small thumbnail of the referenced item's image
    pub thumb_url: Option<String>,
}

/// One consolidated time series: what was a whole StatSeries card
/// before the density pass merged its region's siblings.
#[derive(Debug, Serialize)]
pub struct Indicator {
    /// Localized property label, e.g. "life expectancy"
    pub label: String,
    pub current: String,
    /// e.g. the year of the current value
    pub note: Option<String>,
    pub series: Vec<SeriesPoint>,
}

#[derive(Debug, Serialize)]
pub struct SeriesPoint {
    /// e.g. the year of the point-in-time qualifier
    pub label: String,
    pub value: f64,
    pub display: String,
}

/// One property inside a Facts card: localized label plus its values.
#[derive(Debug, Serialize)]
pub struct FactRow {
    pub label: String,
    pub values: Vec<FactValue>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FactValue {
    Item(ItemChip),
    Link {
        url: String,
    },
    Text {
        value: String,
        span: Option<TemporalSpan>,
        note: Option<String>,
    },
}

/// When a statement held: extracted from start-time/end-time/point-in-time
/// qualifiers. Displays at year granularity (qualifier time precision is
/// not available from WDQS) as "2018 – 2021", "2018 –", or "2021".
#[derive(Debug, Serialize, Clone)]
pub struct TemporalSpan {
    pub start: Option<String>,
    pub end: Option<String>,
    pub point: Option<String>,
}

impl TemporalSpan {
    pub fn display(&self) -> String {
        match (&self.start, &self.end, &self.point) {
            (Some(start), Some(end), _) if start == end => start.clone(),
            (Some(start), Some(end), _) => format!("{start} – {end}"),
            (Some(start), None, _) => format!("{start} –"),
            (None, Some(end), _) => format!("– {end}"),
            (None, None, Some(point)) => point.clone(),
            (None, None, None) => String::new(),
        }
    }

    /// The value no longer holds (an end time has passed... or at least
    /// been recorded). Drives current-vs-former presentation.
    pub fn ended(&self) -> bool {
        self.end.is_some()
    }
}

#[derive(Debug, Serialize)]
pub struct ItemChip {
    pub qid: String,
    pub label: String,
    pub image_url: Option<String>,
    /// Small thumbnail of the referenced item's P18, for visual chips
    pub thumb_url: Option<String>,
    /// When the statement held, e.g. "2018 – 2021" for a former spouse
    pub span: Option<TemporalSpan>,
    /// Summary of the remaining (non-temporal) qualifiers
    pub note: Option<String>,
    /// Preferred-rank statement: the value that holds now (the current
    /// country, the sitting mayor); rendered with emphasis
    pub current: bool,
}

impl ItemChip {
    /// The statement no longer holds (its span records an end time);
    /// rendered as quiet history
    pub fn ended(&self) -> bool {
        self.span.as_ref().is_some_and(TemporalSpan::ended)
    }
}
