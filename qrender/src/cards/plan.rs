//! Presentation planning: pick each card's visual variant and the grid
//! size it fills. The variant is chosen from the card's own content (a
//! census of values, images, temporal spans) - never per property or
//! per archetype - so a spouse, an employer, and a citizenship with the
//! same data shape get the same treatment, on any item.
//!
//! The fill contract: every variant declares a fixed size and
//! guarantees to fill it - images stretch (object-fit: cover), lists
//! clip and scroll, text clamps. Boxes are honest by construction; no
//! stage tries to predict rendered height.

use serde::{Serialize, Serializer};

use super::{Card, CardKind, FactValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// Full-bleed single media value
    MediaFull,
    /// Horizontal scroll-snap strip of media
    GalleryStrip,
    /// One big number
    StatBlock,
    /// Current number + history bars (distinct years)
    Trend,
    /// A quantity on a config-declared scale
    Gauge,
    /// Static map tile
    MapPanel,
    /// Cross-property chronology
    Timeline,
    /// One or two items, at least one with an image: the picture fills
    /// the card and does the explaining
    Portrait,
    /// One or two plain values: a single compact line
    FactLine,
    /// Values with ended spans: what holds now emphasized, history as
    /// quiet lines beneath
    CurrentWithHistory,
    /// Three or more items, mostly with images: two cover tiles, the
    /// rest behind the ellipsis popover
    TileStrip,
    /// Item enumeration with few images: same cover treatment
    ChipList,
    /// Consolidated sibling time series: label, sparkline, current
    /// value per row (built by the density pass)
    IndicatorTable,
    /// Multi-property labeled rows (the grouped workhorse)
    #[default]
    FactsTable,
}

impl Variant {
    /// Cover-chips presentation: the card shows the two best chips;
    /// the full list opens in a popover. Templates branch on this.
    pub fn is_chip_cover(&self) -> bool {
        matches!(self, Variant::ChipList | Variant::TileStrip)
    }

    /// Kebab-case name used by both the JSON API and the factoid
    /// page's data-variant attribute.
    pub fn as_str(&self) -> &'static str {
        match self {
            Variant::MediaFull => "media-full",
            Variant::GalleryStrip => "gallery-strip",
            Variant::StatBlock => "stat-block",
            Variant::Trend => "trend",
            Variant::Gauge => "gauge",
            Variant::MapPanel => "map-panel",
            Variant::Timeline => "timeline",
            Variant::Portrait => "portrait",
            Variant::FactLine => "fact-line",
            Variant::CurrentWithHistory => "current-history",
            Variant::TileStrip => "tile-strip",
            Variant::ChipList => "chip-list",
            Variant::IndicatorTable => "indicator-table",
            Variant::FactsTable => "facts-table",
        }
    }
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Variant {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// What a Facts card actually holds - the input to variant selection.
struct Census {
    values: usize,
    with_image: usize,
    ended: usize,
}

fn census(values: &[FactValue]) -> Census {
    let mut c = Census {
        values: values.len(),
        with_image: 0,
        ended: 0,
    };
    for value in values {
        let (image, span) = match value {
            FactValue::Item(chip) => (chip.thumb_url.is_some(), chip.span.as_ref()),
            FactValue::Text { span, .. } => (false, span.as_ref()),
            FactValue::Link { .. } => (false, None),
        };
        c.with_image += image as usize;
        c.ended += span.is_some_and(|s| s.ended()) as usize;
    }
    c
}

fn select(kind: &CardKind) -> Variant {
    match kind {
        CardKind::Image { .. } => Variant::MediaFull,
        CardKind::Gallery { .. } => Variant::GalleryStrip,
        CardKind::Stat { .. } => Variant::StatBlock,
        CardKind::StatSeries { .. } => Variant::Trend,
        CardKind::Meter { .. } => Variant::Gauge,
        CardKind::Map { .. } => Variant::MapPanel,
        CardKind::Timeline { .. } => Variant::Timeline,
        CardKind::Indicators { .. } => Variant::IndicatorTable,
        CardKind::Facts { rows } => {
            let [row] = rows.as_slice() else {
                return Variant::FactsTable;
            };
            let c = census(&row.values);
            if c.values <= 2 && c.with_image >= 1 {
                Variant::Portrait
            } else if c.values <= 2 {
                Variant::FactLine
            } else if c.with_image * 5 >= c.values * 3 {
                // >= 60% of the values have a picture
                Variant::TileStrip
            } else if c.ended >= 1 {
                Variant::CurrentWithHistory
            } else {
                Variant::ChipList
            }
        }
    }
}

/// Fixed size per variant, in grid units. Content only picks between a
/// variant's few designated steps - it never derives free-form spans.
fn size(variant: Variant, kind: &CardKind) -> (u8, u8) {
    match (variant, kind) {
        (Variant::MediaFull, _) => (2, 2),
        (Variant::GalleryStrip, CardKind::Gallery { images }) => {
            (if images.len() >= 4 { 6 } else { 4 }, 2)
        }
        (Variant::StatBlock, CardKind::Stat { value, .. }) => {
            (2, if value.len() > 32 { 2 } else { 1 })
        }
        (Variant::Trend, CardKind::StatSeries { series, .. }) => {
            (2, if series.len() > 6 { 3 } else { 2 })
        }
        (Variant::Gauge, _) => (2, 1),
        (Variant::MapPanel, _) => (1, 1),
        (Variant::Timeline, CardKind::Timeline { events }) => {
            (2, if events.len() > 6 { 4 } else { 3 })
        }
        (Variant::Portrait, CardKind::Facts { rows }) => {
            if rows[0].values.len() == 1 {
                (2, 2)
            } else {
                (3, 2)
            }
        }
        (Variant::FactLine, _) => (2, 1),
        (Variant::CurrentWithHistory, CardKind::Facts { rows }) => {
            (2, if rows[0].values.len() <= 3 { 2 } else { 3 })
        }
        (Variant::TileStrip, _) => (3, 2),
        (Variant::IndicatorTable, CardKind::Indicators { indicators }) => {
            (4, if indicators.len() > 7 { 3 } else { 2 })
        }
        (Variant::ChipList, _) => (3, 2),
        (Variant::FactsTable, CardKind::Facts { rows }) => {
            let values: usize = rows.iter().map(|r| r.values.len()).sum();
            (2, (1 + values.div_ceil(3) as u8).clamp(2, 4))
        }
        // size() is only called with the kind select() saw
        _ => (2, 2),
    }
}

/// Image tiles need real resolution; round chip thumbs don't.
const TILE_THUMB_WIDTH: u32 = 400;

pub(super) fn apply(card: &mut Card) {
    card.variant = select(&card.kind);
    let (cols, rows) = size(card.variant, &card.kind);
    card.layout.cols = cols;
    card.layout.rows = rows;

    // Cover-chips cards show only the two best values up front:
    // preferred rank first, then values with a picture; stable
    // otherwise. All backends see the same order.
    if card.variant.is_chip_cover()
        && let CardKind::Facts { rows } = &mut card.kind
    {
        rows[0].values.sort_by_key(|value| match value {
            FactValue::Item(chip) => (!chip.current, chip.thumb_url.is_none()),
            _ => (true, true),
        });
    }

    // Tile variants render the picture as the card body, not as a
    // 48px chip bead - request tile-resolution thumbnails.
    if matches!(
        card.variant,
        Variant::Portrait | Variant::TileStrip | Variant::ChipList
    ) && let CardKind::Facts { rows } = &mut card.kind
    {
        for row in rows {
            for value in &mut row.values {
                if let FactValue::Item(chip) = value
                    && let Some(image_url) = &chip.image_url
                {
                    chip.thumb_url = Some(super::synthesis::thumb_url(image_url, TILE_THUMB_WIDTH));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{FactRow, ItemChip, TemporalSpan};

    fn chip(image: bool, ended: bool) -> FactValue {
        FactValue::Item(ItemChip {
            qid: "Q1".to_string(),
            label: "x".to_string(),
            image_url: image.then(|| "http://commons.wikimedia.org/x.jpg".to_string()),
            thumb_url: image.then(|| "https://commons.wikimedia.org/x.jpg?width=96".to_string()),
            span: ended.then(|| TemporalSpan {
                start: Some("2018".to_string()),
                end: Some("2021".to_string()),
                point: None,
            }),
            note: None,
            current: false,
        })
    }

    fn facts(values: Vec<FactValue>) -> CardKind {
        CardKind::Facts {
            rows: vec![FactRow {
                label: "p".to_string(),
                values,
            }],
        }
    }

    #[test]
    fn one_item_with_image_is_a_portrait() {
        assert_eq!(select(&facts(vec![chip(true, false)])), Variant::Portrait);
    }

    #[test]
    fn one_item_without_image_is_a_fact_line() {
        assert_eq!(select(&facts(vec![chip(false, false)])), Variant::FactLine);
    }

    #[test]
    fn image_rich_enumerations_are_tile_strips() {
        let values = vec![chip(true, false), chip(true, false), chip(true, true)];
        assert_eq!(select(&facts(values)), Variant::TileStrip);
    }

    #[test]
    fn spans_without_images_are_current_with_history() {
        let values = vec![chip(false, false), chip(false, true), chip(false, true)];
        assert_eq!(select(&facts(values)), Variant::CurrentWithHistory);
    }

    #[test]
    fn plain_enumerations_are_chip_lists() {
        let values = (0..5).map(|_| chip(false, false)).collect();
        assert_eq!(select(&facts(values)), Variant::ChipList);
    }

    #[test]
    fn grouped_rows_are_a_facts_table() {
        let kind = CardKind::Facts {
            rows: vec![
                FactRow {
                    label: "a".to_string(),
                    values: vec![chip(false, false)],
                },
                FactRow {
                    label: "b".to_string(),
                    values: vec![chip(false, false)],
                },
            ],
        };
        assert_eq!(select(&kind), Variant::FactsTable);
    }
}
