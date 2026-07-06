//! Page-level density: a dense item (a country) produces dozens of
//! individually fine cards that aggregate into an unscannable wall.
//! Per region (each section, the overflow grid, the footnotes),
//! consolidate runs of same-shape cards:
//!
//! - three or more time-series cards merge into one indicator table
//!   (label + sparkline + current value per row) - fifteen identical
//!   bar charts bury the story one table tells;
//! - two or more map cards merge into one labeled coordinate list
//!   (the extreme-point properties P1332-P1335 are four near-identical
//!   country tiles as separate cards).
//!
//! Merging never crosses a region boundary, so a section's editorial
//! placement survives, and nothing is dropped - every series and every
//! coordinate is still on the page.

use super::{Card, CardKind, FactRow, FactValue, FactoidPage, Indicator, Layout, Tier, plan};

const MIN_TRENDS: usize = 3;
const MIN_MAPS: usize = 2;

pub(super) fn consolidate(page: &mut FactoidPage) {
    for section in &mut page.sections {
        consolidate_region(&mut section.cards);
    }
    consolidate_region(&mut page.overflow);
    consolidate_region(&mut page.footnotes);
}

fn consolidate_region(cards: &mut Vec<Card>) {
    merge_trends(cards);
    merge_maps(cards);
}

/// Replace the region's StatSeries cards with one Indicators card at
/// the position of the first, keeping everything else in order.
fn merge_trends(cards: &mut Vec<Card>) {
    let trends = cards
        .iter()
        .filter(|c| matches!(c.kind, CardKind::StatSeries { .. }))
        .count();
    if trends < MIN_TRENDS {
        return;
    }

    let mut indicators = Vec::new();
    let mut source_pids = Vec::new();
    let mut tier = Tier::Standard;
    let mut first_position = None;
    let mut kept = Vec::with_capacity(cards.len() - trends + 1);
    for card in cards.drain(..) {
        let CardKind::StatSeries {
            current,
            note,
            series,
        } = card.kind
        else {
            kept.push(card);
            continue;
        };
        first_position.get_or_insert(kept.len());
        indicators.push(Indicator {
            label: card.title,
            current,
            note,
            series,
        });
        source_pids.extend(card.source_pids);
        tier = card.tier;
    }

    let mut merged = Card {
        // machine name: the icon carries the meaning, like group titles
        title: "indicators".to_string(),
        localized_title: false,
        icon: Some("pace".to_string()),
        source_pids,
        variant: plan::Variant::default(),
        layout: Layout::default(),
        tier,
        kind: CardKind::Indicators { indicators },
    };
    plan::apply(&mut merged);
    kept.insert(first_position.unwrap_or(kept.len()), merged);
    *cards = kept;
}

/// Replace the region's Map cards with one labeled coordinate list.
/// Static map tiles cannot mark multiple points, so N near-identical
/// tiles explain less than one compact list does.
fn merge_maps(cards: &mut Vec<Card>) {
    let maps = cards
        .iter()
        .filter(|c| matches!(c.kind, CardKind::Map { .. }))
        .count();
    if maps < MIN_MAPS {
        return;
    }

    let mut rows = Vec::new();
    let mut source_pids = Vec::new();
    let mut tier = Tier::Standard;
    let mut first_position = None;
    let mut kept = Vec::with_capacity(cards.len() - maps + 1);
    for card in cards.drain(..) {
        let CardKind::Map { lat, lon, .. } = card.kind else {
            kept.push(card);
            continue;
        };
        first_position.get_or_insert(kept.len());
        rows.push(FactRow {
            label: card.title,
            values: vec![FactValue::Text {
                value: format!("{lat}, {lon}"),
                span: None,
                note: None,
            }],
        });
        source_pids.extend(card.source_pids);
        tier = card.tier;
    }

    let mut merged = Card {
        title: "coordinates".to_string(),
        localized_title: false,
        icon: Some("location_on".to_string()),
        source_pids,
        variant: plan::Variant::default(),
        layout: Layout::default(),
        tier,
        kind: CardKind::Facts { rows },
    };
    plan::apply(&mut merged);
    kept.insert(first_position.unwrap_or(kept.len()), merged);
    *cards = kept;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::SeriesPoint;

    fn trend(title: &str, pid: &str) -> Card {
        Card {
            title: title.to_string(),
            localized_title: true,
            icon: None,
            source_pids: vec![pid.to_string()],
            variant: plan::Variant::default(),
            layout: Layout::default(),
            tier: Tier::Standard,
            kind: CardKind::StatSeries {
                current: "1".to_string(),
                note: None,
                series: vec![
                    SeriesPoint {
                        label: "2011".to_string(),
                        value: 1.0,
                        display: "1".to_string(),
                    },
                    SeriesPoint {
                        label: "2016".to_string(),
                        value: 2.0,
                        display: "2".to_string(),
                    },
                ],
            },
        }
    }

    fn map(title: &str, pid: &str) -> Card {
        Card {
            title: title.to_string(),
            localized_title: true,
            icon: None,
            source_pids: vec![pid.to_string()],
            variant: plan::Variant::default(),
            layout: Layout::default(),
            tier: Tier::Standard,
            kind: CardKind::Map {
                lat: 1.0,
                lon: 2.0,
                label: title.to_string(),
            },
        }
    }

    fn stat() -> Card {
        Card {
            title: "x".to_string(),
            localized_title: true,
            icon: None,
            source_pids: vec!["P1".to_string()],
            variant: plan::Variant::default(),
            layout: Layout::default(),
            tier: Tier::Standard,
            kind: CardKind::Stat {
                value: "v".to_string(),
                note: None,
            },
        }
    }

    #[test]
    fn three_trends_merge_into_an_indicator_table() {
        let mut cards = vec![
            stat(),
            trend("birth rate", "P8763"),
            trend("death rate", "P10091"),
            trend("fertility rate", "P4841"),
        ];
        consolidate_region(&mut cards);
        assert_eq!(cards.len(), 2);
        // merged card sits where the first trend was
        let CardKind::Indicators { indicators } = &cards[1].kind else {
            panic!("expected indicators, got {:?}", cards[1].kind);
        };
        assert_eq!(indicators.len(), 3);
        assert_eq!(indicators[0].label, "birth rate");
        assert_eq!(cards[1].variant, plan::Variant::IndicatorTable);
        assert_eq!(cards[1].source_pids, ["P8763", "P10091", "P4841"]);
    }

    #[test]
    fn two_trends_stay_separate() {
        let mut cards = vec![trend("a", "P1"), trend("b", "P2")];
        consolidate_region(&mut cards);
        assert_eq!(cards.len(), 2);
    }

    #[test]
    fn sibling_maps_merge_into_a_coordinate_list() {
        let mut cards = vec![
            map("coordinates of northernmost point", "P1332"),
            stat(),
            map("coordinates of southernmost point", "P1333"),
        ];
        consolidate_region(&mut cards);
        assert_eq!(cards.len(), 2);
        let CardKind::Facts { rows } = &cards[0].kind else {
            panic!("expected facts, got {:?}", cards[0].kind);
        };
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "coordinates of northernmost point");
    }

    #[test]
    fn a_lone_map_is_untouched() {
        let mut cards = vec![map("coordinate location", "P625"), stat()];
        consolidate_region(&mut cards);
        assert!(matches!(cards[0].kind, CardKind::Map { .. }));
    }
}
