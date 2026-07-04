//! Build cards from a typed item, driven by groups.toml grouping and the
//! values' own types ("auto" synthesis): images become Image/Gallery
//! cards, coordinates a Map card, quantity series a chart card, item
//! references chip lists, URLs link lists; what remains is key-values.

use qjson::{Property, Rank, Value, WikidataItem};
use std::collections::HashSet;

use super::format::{display_value, format_time};
use super::{
    Card, CardKind, FactoidPage, GalleryImage, ItemChip, KeyValueEntry, Layout, LinkEntry,
    MediaKind, SeriesPoint, Tier,
};
use crate::grouping::{GroupConfig, GroupingConfig};

const POINT_IN_TIME: &str = "P585";

pub fn synthesize(
    item: &WikidataItem,
    language: &str,
    config: &GroupingConfig,
    ignore_ids: bool,
) -> FactoidPage {
    let mut cards = Vec::new();

    // The item's main image becomes the page hero. When P18 has just that
    // one statement, its standalone card would duplicate the hero - skip it.
    let main_image = item.properties.get("P18");
    let hero = main_image.and_then(|p| p.statements.first()).and_then(|s| {
        if let Value::CommonsMedia { file_name, .. } = &s.value {
            Some(gallery_image(
                file_name,
                item.label.as_deref().unwrap_or(&item.qid),
            ))
        } else {
            None
        }
    });
    let hero_consumes_p18 = hero.is_some() && main_image.is_some_and(|p| p.statements.len() == 1);

    for (group_name, group_config) in config.sorted_groups() {
        if ignore_ids && group_name == "identifiers" {
            continue;
        }
        // pids may repeat across a group definition; keep first occurrence
        let mut seen = HashSet::new();
        let properties: Vec<&Property> = group_config
            .pids
            .iter()
            .filter(|pid| seen.insert(pid.as_str()) && !config.is_ignored(pid))
            .filter(|pid| !(hero_consumes_p18 && *pid == "P18"))
            .filter_map(|pid| item.properties.get(pid))
            .collect();
        let mut group_cards = cards_for_group(&humanize(group_name), false, &properties, config);
        for card in &mut group_cards {
            card.icon = resolve_icon(card, group_config.icon.as_deref(), config);
            card.layout = resolve_layout(card, Some(group_config), config);
            card.tier = resolve_tier(card, Some(group_config), config);
        }
        cards.extend(group_cards);
    }

    // Ungrouped properties get their own auto card each, ordered by PID
    let grouped_pids: HashSet<&String> =
        config.groups.values().flat_map(|g| g.pids.iter()).collect();
    let mut leftover: Vec<&Property> = item
        .properties
        .values()
        .filter(|p| !grouped_pids.contains(&p.pid) && !config.is_ignored(&p.pid))
        .filter(|p| !(hero_consumes_p18 && p.pid == "P18"))
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
        let mut property_cards = cards_for_group(&property.label, true, &[property], config);
        for card in &mut property_cards {
            card.icon = resolve_icon(card, None, config);
            card.layout = resolve_layout(card, None, config);
            card.tier = resolve_tier(card, None, config);
        }
        cards.extend(property_cards);
    }

    // Stable sort: footnote-tier cards sink below everything (all
    // backends benefit - meta noise ends up last in text output too),
    // then config `sort` reorders across the whole page (images early,
    // categories late); ties keep group order. DOM order is the
    // reading order - CSS never reorders.
    cards.sort_by_key(|card| (card.tier == Tier::Footnote, card.layout.sort));

    FactoidPage {
        qid: item.qid.clone(),
        label: item.label.clone(),
        description: item.description.clone(),
        language: language.to_string(),
        hero,
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

/// Kind-derived layout defaults, clamped by content: what a card *is*
/// and how much it holds suggest its visual weight.
fn kind_layout(kind: &CardKind) -> (u8, u8) {
    match kind {
        CardKind::Stat { value, .. } => (2, if value.len() > 16 { 2 } else { 1 }),
        CardKind::StatSeries { series, .. } => (2, if series.len() > 6 { 3 } else { 2 }),
        CardKind::Image { .. } => (2, 2),
        CardKind::Gallery { images } => (if images.len() >= 4 { 6 } else { 4 }, 2),
        CardKind::Map { .. } => (2, 2),
        CardKind::KeyValues { entries } => {
            let values: usize = entries.iter().map(|e| e.values.len()).sum();
            (2, (1 + values.div_ceil(3) as u8).clamp(2, 4))
        }
        CardKind::ItemChips { items } => (if items.len() >= 4 { 4 } else { 2 }, 1),
        CardKind::Links { .. } => (2, 1),
        CardKind::Meter { .. } => (2, 1),
    }
}

/// A card is a footnote when its group or any source property is
/// flagged as Wikimedia-curation meta in groups.toml.
fn resolve_tier(card: &Card, group_config: Option<&GroupConfig>, config: &GroupingConfig) -> Tier {
    let property_footnote = card
        .source_pids
        .iter()
        .any(|pid| config.properties.get(pid).is_some_and(|p| p.footnote));
    if property_footnote || group_config.is_some_and(|g| g.footnote) {
        Tier::Footnote
    } else {
        Tier::Standard
    }
}

/// Layout cascade: kind defaults (content-aware), then group config,
/// then per-PID config.
fn resolve_layout(
    card: &Card,
    group_config: Option<&GroupConfig>,
    config: &GroupingConfig,
) -> Layout {
    let (cols, rows) = kind_layout(&card.kind);
    let mut layout = Layout {
        cols,
        rows,
        ..Layout::default()
    };
    if let Some(group) = group_config {
        layout.cols = group.cols.unwrap_or(layout.cols);
        layout.rows = group.rows.unwrap_or(layout.rows);
        layout.sort = group.sort.unwrap_or(layout.sort);
    }
    for pid in &card.source_pids {
        if let Some(property_config) = config.properties.get(pid) {
            layout.cols = property_config.cols.unwrap_or(layout.cols);
            layout.rows = property_config.rows.unwrap_or(layout.rows);
            layout.sort = property_config.sort.unwrap_or(layout.sort);
            break;
        }
    }
    layout
}

fn cards_for_group(
    title: &str,
    title_is_localized: bool,
    properties: &[&Property],
    config: &GroupingConfig,
) -> Vec<Card> {
    let mut cards = Vec::new();
    let mut images: Vec<(String, GalleryImage)> = Vec::new(); // (pid, image)
    let mut links: Vec<(String, LinkEntry)> = Vec::new();
    let mut key_values: Vec<(String, KeyValueEntry)> = Vec::new();

    for property in properties {
        // Config-declared gauges (HDI etc.) win over series detection
        if let Some(meter_config) = config.properties.get(&property.pid).and_then(|p| p.meter)
            && let Some(card) = as_meter(property, &meter_config)
        {
            cards.push(card);
            continue;
        }
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
                        layout: Layout::default(),
                        tier: Tier::Standard,
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
                    thumb_url: image_url.as_deref().map(chip_thumb_url),
                    note: qualifier_note(s),
                }),
                _ => None,
            })
            .collect();
        if !chips.is_empty() {
            cards.push(Card {
                title: property.label.clone(),
                layout: Layout::default(),
                tier: Tier::Standard,
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
                layout: Layout::default(),
                tier: Tier::Standard,
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
                layout: Layout::default(),
                tier: Tier::Standard,
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
            layout: Layout::default(),
            tier: Tier::Standard,
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
                layout: Layout::default(),
                tier: Tier::Standard,
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
                layout: Layout::default(),
                tier: Tier::Standard,
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
const CHIP_THUMB_WIDTH: u32 = 96; // 2x for the 48px chip thumb

/// image_url arrives as an http FilePath URL from SPARQL; normalize to
/// https and request a chip-sized thumbnail.
fn chip_thumb_url(image_url: &str) -> String {
    let https = image_url.replacen("http://", "https://", 1);
    format!("{https}?width={CHIP_THUMB_WIDTH}")
}

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

/// Pick the statement holding the "current" quantity: preferred rank
/// first, else the latest by point-in-time qualifier, else the first.
fn current_quantity(property: &Property) -> Option<(&qjson::Statement, Option<String>)> {
    let statement = property
        .statements
        .iter()
        .find(|s| s.rank == Rank::Preferred)
        .or_else(|| {
            property
                .statements
                .iter()
                .max_by_key(|s| statement_time(s).map(|(iso, _)| iso))
        })
        .or_else(|| property.statements.first())?;
    let note = statement_time(statement).map(|(iso, _)| format_time(&iso, Some(9)));
    Some((statement, note))
}

fn statement_time(statement: &qjson::Statement) -> Option<(String, Option<u8>)> {
    statement.qualifiers.iter().find_map(|q| match &q.value {
        Value::Time { iso, precision } if q.pid == POINT_IN_TIME => Some((iso.clone(), *precision)),
        _ => None,
    })
}

/// A quantity on a config-declared scale becomes a gauge card.
fn as_meter(property: &Property, meter: &crate::grouping::MeterConfig) -> Option<Card> {
    let (statement, note) = current_quantity(property)?;
    let Value::Quantity { amount, .. } = &statement.value else {
        return None;
    };
    Some(Card {
        title: property.label.clone(),
        layout: Layout::default(),
        tier: Tier::Standard,
        localized_title: true,
        icon: None,
        source_pids: vec![property.pid.clone()],
        kind: CardKind::Meter {
            value: *amount,
            display: display_value(&statement.value),
            note,
            min: meter.min,
            max: meter.max,
            low: meter.low,
            high: meter.high,
            optimum: meter.optimum,
        },
    })
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
        layout: Layout::default(),
        tier: Tier::Standard,
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
        let CardKind::StatSeries {
            current, series, ..
        } = &card.kind
        else {
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
    fn hero_comes_from_main_image() {
        let page = nairobi_page();
        let hero = page.hero.expect("Nairobi has a P18");
        assert!(hero.thumb_url.contains("width=640"));
        assert_eq!(hero.caption, "Nairobi");
        // P18 has a single statement consumed by the hero - no duplicate card
        assert!(
            page.cards
                .iter()
                .all(|c| c.source_pids != ["P18".to_string()])
        );
    }

    #[test]
    fn chips_carry_thumbnails() {
        let page = nairobi_page();
        let card = page
            .cards
            .iter()
            .find(|c| c.source_pids == ["P17"])
            .expect("country card");
        let CardKind::ItemChips { items } = &card.kind else {
            panic!("country must be ItemChips");
        };
        let thumb = items[0].thumb_url.as_deref().expect("Kenya has an image");
        assert!(thumb.starts_with("https://"));
        assert!(thumb.contains("width=96"));
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
    fn configured_scale_becomes_a_meter() {
        // Synthetic HDI property: P1081 has meter config in groups.toml
        let item = qjson::WikidataItem {
            qid: "Q1".to_string(),
            label: Some("Testland".to_string()),
            description: None,
            properties: std::collections::HashMap::from([(
                "P1081".to_string(),
                qjson::Property {
                    pid: "P1081".to_string(),
                    label: "Human Development Index".to_string(),
                    statements: vec![qjson::Statement {
                        value: qjson::Value::Quantity {
                            amount: 0.601,
                            raw: "0.601".to_string(),
                            unit_qid: None,
                            unit_label: None,
                        },
                        rank: qjson::Rank::Preferred,
                        qualifiers: vec![qjson::Qualifier {
                            pid: "P585".to_string(),
                            label: "point in time".to_string(),
                            value: qjson::Value::Time {
                                iso: "2021-01-01T00:00:00Z".to_string(),
                                precision: Some(9),
                            },
                        }],
                    }],
                },
            )]),
        };
        let page = synthesize(&item, "en", &load_grouping_config().unwrap(), true);
        let card = find(&page, |c| c.source_pids == ["P1081"]).expect("meter card");
        let CardKind::Meter {
            value, note, max, ..
        } = &card.kind
        else {
            panic!("HDI must be a Meter, got {:?}", card.kind);
        };
        assert_eq!(*value, 0.601);
        assert_eq!(*max, 1.0);
        assert_eq!(note.as_deref(), Some("2021"));
    }

    #[test]
    fn curation_meta_is_footnote_tier() {
        let page = nairobi_page();
        // categories group (P910 et al.) and ungrouped curation PIDs
        // (P1343 described by source) are footnotes; content is not
        let categories =
            find(&page, |c| c.source_pids.contains(&"P910".to_string())).expect("categories card");
        assert_eq!(categories.tier, Tier::Footnote);
        let sources = find(&page, |c| c.source_pids == ["P1343"]).expect("described-by card");
        assert_eq!(sources.tier, Tier::Footnote);
        let population = find(&page, |c| c.source_pids == ["P1082"]).expect("population card");
        assert_eq!(population.tier, Tier::Standard);
        // footnotes sink: once the first appears, everything after is one
        let first = page
            .cards
            .iter()
            .position(|c| c.tier == Tier::Footnote)
            .unwrap();
        assert!(page.cards[first..].iter().all(|c| c.tier == Tier::Footnote));
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
