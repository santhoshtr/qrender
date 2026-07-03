//! Build cards from a typed item, driven by groups.toml grouping and the
//! values' own types ("auto" synthesis): images become Image/Gallery
//! cards, coordinates a Map card, quantity series a chart card, item
//! references chip lists, URLs link lists; what remains is key-values.

use qjson::{Property, Rank, Value, WikidataItem};
use std::collections::HashSet;

use super::format::{display_value, format_time};
use super::{
    Card, CardKind, FactoidPage, GalleryImage, ItemChip, KeyValueEntry, LinkEntry, MediaKind,
    SeriesPoint,
};
use crate::grouping::GroupingConfig;

const POINT_IN_TIME: &str = "P585";

pub fn synthesize(
    item: &WikidataItem,
    language: &str,
    config: &GroupingConfig,
    ignore_ids: bool,
) -> FactoidPage {
    let mut cards = Vec::new();

    for (group_name, group_config) in config.sorted_groups() {
        if ignore_ids && group_name == "identifiers" {
            continue;
        }
        // pids may repeat across a group definition; keep first occurrence
        let mut seen = HashSet::new();
        let properties: Vec<&Property> = group_config
            .pids
            .iter()
            .filter(|pid| seen.insert(pid.as_str()))
            .filter_map(|pid| item.properties.get(pid))
            .collect();
        let mut group_cards = cards_for_group(&humanize(group_name), false, &properties);
        for card in &mut group_cards {
            card.icon = resolve_icon(card, group_config.icon.as_deref(), config);
        }
        cards.extend(group_cards);
    }

    // Ungrouped properties get their own auto card each, ordered by PID
    let grouped_pids: HashSet<&String> =
        config.groups.values().flat_map(|g| g.pids.iter()).collect();
    let mut leftover: Vec<&Property> = item
        .properties
        .values()
        .filter(|p| !grouped_pids.contains(&p.pid))
        .collect();
    leftover.sort_by_key(|p| p.pid.strip_prefix('P').and_then(|n| n.parse::<u32>().ok()));
    for property in leftover {
        if ignore_ids
            && property
                .statements
                .iter()
                .all(|s| matches!(s.value, Value::ExternalId { .. }))
        {
            continue;
        }
        let mut property_cards = cards_for_group(&property.label, true, &[property]);
        for card in &mut property_cards {
            card.icon = resolve_icon(card, None, config);
        }
        cards.extend(property_cards);
    }

    FactoidPage {
        qid: item.qid.clone(),
        label: item.label.clone(),
        description: item.description.clone(),
        language: language.to_string(),
        cards,
    }
}

fn humanize(group_name: &str) -> String {
    let mut title = group_name.replace('_', " ");
    if let Some(first) = title.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    title
}

/// Icon resolution: per-PID config wins, then the group icon, then a
/// default derived from the card kind. None is fine - the header falls
/// back to text.
fn resolve_icon(card: &Card, group_icon: Option<&str>, config: &GroupingConfig) -> Option<String> {
    for pid in &card.source_pids {
        if let Some(property_config) = config.properties.get(pid)
            && let Some(icon) = &property_config.icon
        {
            return Some(icon.clone());
        }
    }
    if let Some(icon) = group_icon {
        return Some(icon.to_string());
    }
    match card.kind {
        CardKind::Image { .. } | CardKind::Gallery { .. } => Some("photo_library".to_string()),
        CardKind::Map { .. } => Some("location_on".to_string()),
        CardKind::Links { .. } => Some("captive_portal".to_string()),
        _ => None,
    }
}

fn cards_for_group(title: &str, title_is_localized: bool, properties: &[&Property]) -> Vec<Card> {
    let mut cards = Vec::new();
    let mut images: Vec<(String, GalleryImage)> = Vec::new(); // (pid, image)
    let mut links: Vec<(String, LinkEntry)> = Vec::new();
    let mut key_values: Vec<(String, KeyValueEntry)> = Vec::new();

    for property in properties {
        // A quantity property whose statements carry point-in-time
        // qualifiers is a time series (population, HDI, ...)
        if let Some(card) = as_stat_series(property) {
            cards.push(card);
            continue;
        }

        for statement in &property.statements {
            match &statement.value {
                Value::CommonsMedia { file_name, .. } => {
                    images.push((
                        property.pid.clone(),
                        gallery_image(file_name, &property.label),
                    ));
                }
                Value::Coordinate { lat, lon, .. } => {
                    cards.push(Card {
                        title: property.label.clone(),
                        localized_title: true,
                        icon: None,
                        source_pids: vec![property.pid.clone()],
                        kind: CardKind::Map {
                            lat: *lat,
                            lon: *lon,
                            label: property.label.clone(),
                        },
                    });
                }
                Value::Url { url } => {
                    links.push((
                        property.pid.clone(),
                        LinkEntry {
                            label: property.label.clone(),
                            url: url.clone(),
                        },
                    ));
                }
                _ => {}
            }
        }

        // Item references become a chip list card per property
        let chips: Vec<ItemChip> = property
            .statements
            .iter()
            .filter_map(|s| match &s.value {
                Value::ItemRef {
                    qid,
                    label,
                    image_url,
                } => Some(ItemChip {
                    qid: qid.clone(),
                    label: label.clone(),
                    image_url: image_url.clone(),
                    note: qualifier_note(s),
                }),
                _ => None,
            })
            .collect();
        if !chips.is_empty() {
            cards.push(Card {
                title: property.label.clone(),
                localized_title: true,
                icon: None,
                source_pids: vec![property.pid.clone()],
                kind: CardKind::ItemChips { items: chips },
            });
        }

        // Everything not consumed above becomes a key-value entry
        let values: Vec<String> = property
            .statements
            .iter()
            .filter(|s| {
                !matches!(
                    s.value,
                    Value::CommonsMedia { .. }
                        | Value::Coordinate { .. }
                        | Value::Url { .. }
                        | Value::ItemRef { .. }
                )
            })
            .map(|s| match qualifier_note(s) {
                Some(note) => format!("{} ({note})", display_value(&s.value)),
                None => display_value(&s.value),
            })
            .collect();
        if !values.is_empty() {
            key_values.push((
                property.pid.clone(),
                KeyValueEntry {
                    key: property.label.clone(),
                    values,
                },
            ));
        }
    }

    match images.len() {
        0 => {}
        1 => {
            let (pid, image) = images.remove(0);
            cards.push(Card {
                title: image.caption.clone(),
                localized_title: true,
                icon: None,
                source_pids: vec![pid],
                kind: CardKind::Image { image },
            });
        }
        _ => {
            let pids = dedup_pids(images.iter().map(|(pid, _)| pid.clone()));
            cards.push(Card {
                title: title.to_string(),
                localized_title: title_is_localized,
                icon: None,
                source_pids: pids,
                kind: CardKind::Gallery {
                    images: images.into_iter().map(|(_, image)| image).collect(),
                },
            });
        }
    }

    if !links.is_empty() {
        let pids = dedup_pids(links.iter().map(|(pid, _)| pid.clone()));
        cards.push(Card {
            title: title.to_string(),
            localized_title: title_is_localized,
            icon: None,
            source_pids: pids,
            kind: CardKind::Links {
                entries: links.into_iter().map(|(_, entry)| entry).collect(),
            },
        });
    }

    match key_values.len() {
        0 => {}
        // A lone single-valued property is a stat, not a one-row table
        1 if key_values[0].1.values.len() == 1 => {
            let (pid, entry) = key_values.remove(0);
            cards.push(Card {
                title: entry.key,
                localized_title: true,
                icon: None,
                source_pids: vec![pid],
                kind: CardKind::Stat {
                    value: entry.values.into_iter().next().unwrap(),
                    note: None,
                },
            });
        }
        _ => {
            let pids = dedup_pids(key_values.iter().map(|(pid, _)| pid.clone()));
            cards.push(Card {
                title: title.to_string(),
                localized_title: title_is_localized,
                icon: None,
                source_pids: pids,
                kind: CardKind::KeyValues {
                    entries: key_values.into_iter().map(|(_, entry)| entry).collect(),
                },
            });
        }
    }

    cards
}

fn dedup_pids(pids: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    pids.filter(|pid| seen.insert(pid.clone())).collect()
}

/// "label: value, label: value" summary of a statement's qualifiers.
/// Qualifiers often carry essential context (dates of office, ordinals),
/// so every backend gets them, not just the visual one.
fn qualifier_note(statement: &qjson::Statement) -> Option<String> {
    if statement.qualifiers.is_empty() {
        return None;
    }
    Some(
        statement
            .qualifiers
            .iter()
            .map(|q| format!("{}: {}", q.label, display_value(&q.value)))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

const THUMB_WIDTH: u32 = 640;

/// Path segment encoding for Commons file page URLs
const FILE_SEGMENT: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'?')
    .add(b'%');

fn media_kind(file_name: &str) -> MediaKind {
    let extension = file_name
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "ogg" | "oga" | "opus" | "mp3" | "flac" | "wav" | "mid" => MediaKind::Audio,
        "webm" | "ogv" | "mpg" | "mpeg" => MediaKind::Video,
        _ => MediaKind::Image,
    }
}

fn gallery_image(file_name: &str, caption: &str) -> GalleryImage {
    let encoded = percent_encoding::utf8_percent_encode(file_name, FILE_SEGMENT);
    GalleryImage {
        file_name: file_name.to_string(),
        thumb_url: format!(
            "https://commons.wikimedia.org/wiki/Special:FilePath/{encoded}?width={THUMB_WIDTH}"
        ),
        file_url: format!("https://commons.wikimedia.org/wiki/Special:FilePath/{encoded}"),
        page_url: format!("https://commons.wikimedia.org/wiki/File:{encoded}"),
        caption: caption.to_string(),
        media: media_kind(file_name),
    }
}

/// Quantity statements with point-in-time qualifiers form a series card:
/// the preferred (or latest) value shown big, history as chart points.
fn as_stat_series(property: &Property) -> Option<Card> {
    let mut points: Vec<(String, SeriesPoint, Rank)> = Vec::new(); // (iso, point, rank)
    for statement in &property.statements {
        let Value::Quantity { amount, .. } = &statement.value else {
            return None;
        };
        let time = statement.qualifiers.iter().find_map(|q| match &q.value {
            Value::Time { iso, precision } if q.pid == POINT_IN_TIME => {
                Some((iso.clone(), *precision))
            }
            _ => None,
        })?;
        points.push((
            time.0.clone(),
            SeriesPoint {
                label: format_time(&time.0, Some(9)),
                value: *amount,
                display: display_value(&statement.value),
            },
            statement.rank,
        ));
    }
    if points.len() < 2 {
        return None;
    }
    points.sort_by(|a, b| a.0.cmp(&b.0));

    let current = points
        .iter()
        .find(|(_, _, rank)| *rank == Rank::Preferred)
        .unwrap_or_else(|| points.last().unwrap());
    let (current_display, current_label) = (current.1.display.clone(), current.1.label.clone());

    Some(Card {
        title: property.label.clone(),
        localized_title: true,
        icon: None,
        source_pids: vec![property.pid.clone()],
        kind: CardKind::StatSeries {
            current: current_display,
            note: Some(current_label),
            series: points.into_iter().map(|(_, point, _)| point).collect(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grouping::load_grouping_config;

    fn nairobi_page() -> FactoidPage {
        let response: qjson::sparql::SparqlResponse = serde_json::from_str(include_str!(
            "../../../qjson/tests/fixtures/Q3870.sparql.json"
        ))
        .unwrap();
        let item = qjson::transform::transform("Q3870", &response.results.bindings);
        synthesize(&item, "en", &load_grouping_config().unwrap(), true)
    }

    fn find<'a>(page: &'a FactoidPage, predicate: impl Fn(&&Card) -> bool) -> Option<&'a Card> {
        page.cards.iter().find(|c| predicate(c))
    }

    #[test]
    fn header_is_populated() {
        let page = nairobi_page();
        assert_eq!(page.label.as_deref(), Some("Nairobi"));
        assert_eq!(page.description.as_deref(), Some("capital city of Kenya"));
    }

    #[test]
    fn population_becomes_stat_series() {
        let page = nairobi_page();
        let card = find(&page, |c| c.source_pids == ["P1082"]).expect("population card");
        let CardKind::StatSeries { current, series, .. } = &card.kind else {
            panic!("population must be a StatSeries, got {:?}", card.kind);
        };
        assert_eq!(series.len(), 3);
        // sorted by time: 2009, 2010(11?), 2016 - latest value is current
        assert_eq!(current, "5545000");
        assert!(series.windows(2).all(|w| w[0].label <= w[1].label));
    }

    #[test]
    fn coordinates_become_a_map() {
        let page = nairobi_page();
        let card = find(&page, |c| matches!(c.kind, CardKind::Map { .. })).expect("map card");
        let CardKind::Map { lat, lon, .. } = card.kind else {
            unreachable!()
        };
        assert!((lat - -1.286).abs() < 0.01);
        assert!((lon - 36.817).abs() < 0.01);
    }

    #[test]
    fn images_group_into_gallery() {
        let page = nairobi_page();
        let gallery = find(&page, |c| matches!(c.kind, CardKind::Gallery { .. }));
        assert!(gallery.is_some(), "articleimage group must form a gallery");
    }

    #[test]
    fn country_becomes_item_chips() {
        let page = nairobi_page();
        let card = find(&page, |c| c.source_pids == ["P17"]).expect("country card");
        let CardKind::ItemChips { items } = &card.kind else {
            panic!("country must be ItemChips");
        };
        assert_eq!(items[0].label, "Kenya");
        assert_eq!(items[0].qid, "Q114");
    }

    #[test]
    fn identifiers_are_suppressed() {
        let page = nairobi_page();
        // Freebase ID etc. are ExternalId-valued and must not surface
        assert!(
            find(&page, |c| c
                .source_pids
                .iter()
                .any(|p| p == "P646" || p == "P8093"))
            .is_none()
        );
    }

    #[test]
    fn inception_time_is_formatted() {
        let page = nairobi_page();
        let card = find(&page, |c| c.source_pids.contains(&"P571".to_string()))
            .expect("inception appears on some card");
        // inception 1899, year precision: no ISO noise on the card
        let text = serde_json::to_string(&card).unwrap();
        assert!(text.contains("1899"));
        assert!(!text.contains("1899-01-01T"));
    }
}
