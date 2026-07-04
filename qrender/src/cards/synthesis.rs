//! Build cards from a typed item, driven by groups.toml grouping and the
//! values' own types ("auto" synthesis): images become Image/Gallery
//! cards, coordinates a Map card, quantity series a chart card, item
//! references chip lists, URLs link lists; what remains is key-values.

use qjson::{Property, Rank, Value, WikidataItem};
use std::collections::HashSet;

use super::format::{display_value, format_time};
use super::{
    Card, CardKind, FactRow, FactValue, FactoidPage, GalleryImage, ItemChip, Layout, MediaKind,
    SeriesPoint, Tier, compose,
};
use crate::archetype::{self, ArchetypesConfig};
use crate::grouping::{GroupConfig, GroupingConfig};

const POINT_IN_TIME: &str = "P585";
const START_TIME: &str = "P580";
const END_TIME: &str = "P582";

pub fn synthesize(
    item: &WikidataItem,
    language: &str,
    config: &GroupingConfig,
    archetypes: &ArchetypesConfig,
    ignore_ids: bool,
) -> FactoidPage {
    let mut cards = Vec::new();

    let archetype = archetype::resolve(item, archetypes);
    let recipe = archetypes.archetypes.get(&archetype);
    let hero_facts = recipe
        .and_then(|r| r.hero.as_ref())
        .and_then(|h| compose::hero_facts(item, h));

    // The item's main image becomes the page hero: the preferred-rank
    // statement when one exists, else the first. When that leaves P18
    // nothing more to show, its standalone card would duplicate the
    // hero - skip it.
    let main_image = item.properties.get("P18");
    let hero = main_image
        .and_then(|p| {
            p.statements
                .iter()
                .find(|s| s.rank == Rank::Preferred)
                .or_else(|| p.statements.first())
        })
        .and_then(|s| {
            if let Value::CommonsMedia { file_name, .. } = &s.value {
                Some(gallery_image(
                    file_name,
                    item.label.as_deref().unwrap_or(&item.qid),
                ))
            } else {
                None
            }
        });
    let hero_consumes_p18 = hero.is_some()
        && main_image.is_some_and(|p| {
            let preferred = p
                .statements
                .iter()
                .filter(|s| s.rank == Rank::Preferred)
                .count();
            p.statements.len() == 1 || preferred == 1
        });

    // Same for the header emblem (person: P109 signature): a single
    // statement fully shown in the header needs no card of its own.
    let emblem_pid = recipe
        .and_then(|r| r.hero.as_ref())
        .and_then(|h| h.emblem.as_deref())
        .unwrap_or("");
    let emblem_consumed = hero_facts.as_ref().is_some_and(|f| f.emblem.is_some())
        && item
            .properties
            .get(emblem_pid)
            .is_some_and(|p| p.statements.len() == 1);

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
            .filter(|pid| !(emblem_consumed && *pid == emblem_pid))
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
        .filter(|p| !(emblem_consumed && p.pid == emblem_pid))
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

    // Stable sort: config `sort` reorders across the whole page (images
    // early, categories late); ties keep group order. DOM order is the
    // reading order - CSS never reorders.
    cards.sort_by_key(|card| card.layout.sort);
    // Footnote-tier cards split off into their own collapsed region;
    // the recipe's sections claim from the rest, and what no section
    // claims is the overflow bento grid.
    let (footnotes, standard): (Vec<Card>, Vec<Card>) =
        cards.into_iter().partition(|c| c.tier == Tier::Footnote);
    let (sections, overflow) = match recipe {
        Some(recipe) => compose::sections(item, standard, recipe),
        None => (Vec::new(), standard),
    };

    FactoidPage {
        qid: item.qid.clone(),
        label: item.label.clone(),
        description: item.description.clone(),
        language: language.to_string(),
        archetype,
        hero,
        hero_facts,
        sections,
        overflow,
        footnotes,
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
        _ => None,
    }
}

/// Kind-derived layout defaults, clamped by content: what a card *is*
/// and how much it holds suggest its visual weight.
pub(super) fn kind_layout(kind: &CardKind) -> (u8, u8) {
    match kind {
        CardKind::Stat { value, .. } => (2, if value.len() > 16 { 2 } else { 1 }),
        CardKind::StatSeries { series, .. } => (2, if series.len() > 6 { 3 } else { 2 }),
        CardKind::Image { .. } => (2, 2),
        CardKind::Gallery { images } => (if images.len() >= 4 { 6 } else { 4 }, 2),
        CardKind::Map { .. } => (2, 2),
        CardKind::Facts { rows } => {
            let values: usize = rows.iter().map(|r| r.values.len()).sum();
            (2, (1 + values.div_ceil(3) as u8).clamp(2, 4))
        }
        CardKind::Timeline { events } => (2, if events.len() > 6 { 4 } else { 3 }),
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
    let mut rows: Vec<(String, FactRow)> = Vec::new(); // (pid, row)

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

        // When any media statement is preferred, the others are
        // superseded variants (old flags, alternate crops) - show only
        // the preferred ones instead of near-duplicates.
        let has_preferred_media = property
            .statements
            .iter()
            .any(|s| s.rank == Rank::Preferred && matches!(s.value, Value::CommonsMedia { .. }));

        // Media and coordinates make visual cards; everything else
        // becomes one labeled row so a group reads as one card, not a
        // scatter of single-property fragments.
        let mut values: Vec<FactValue> = Vec::new();
        for statement in ordered_statements(property) {
            match &statement.value {
                Value::CommonsMedia { file_name, .. } => {
                    if has_preferred_media && statement.rank != Rank::Preferred {
                        continue;
                    }
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
                    values.push(FactValue::Link { url: url.clone() });
                }
                Value::ItemRef {
                    qid,
                    label,
                    image_url,
                } => {
                    values.push(FactValue::Item(ItemChip {
                        qid: qid.clone(),
                        label: label.clone(),
                        image_url: image_url.clone(),
                        thumb_url: image_url.as_deref().map(chip_thumb_url),
                        note: qualifier_note(statement),
                        current: statement.rank == Rank::Preferred,
                    }));
                }
                other => {
                    values.push(FactValue::Text {
                        value: display_value(other),
                        note: qualifier_note(statement),
                    });
                }
            }
        }
        if !values.is_empty() {
            rows.push((
                property.pid.clone(),
                FactRow {
                    label: property.label.clone(),
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

    match rows.len() {
        0 => {}
        1 => {
            let (pid, row) = rows.remove(0);
            // A lone plain value is a stat, not a one-row table;
            // anything richer keeps its row under the property title.
            if row.values.len() == 1 && matches!(row.values[0], FactValue::Text { .. }) {
                let Some(FactValue::Text { value, note }) = row.values.into_iter().next() else {
                    unreachable!()
                };
                cards.push(Card {
                    title: row.label,
                    layout: Layout::default(),
                    tier: Tier::Standard,
                    localized_title: true,
                    icon: None,
                    source_pids: vec![pid],
                    kind: CardKind::Stat { value, note },
                });
            } else {
                let mut row = row;
                feature_chip_thumbs(&mut row);
                cards.push(Card {
                    title: row.label.clone(),
                    layout: Layout::default(),
                    tier: Tier::Standard,
                    localized_title: true,
                    icon: None,
                    source_pids: vec![pid],
                    kind: CardKind::Facts { rows: vec![row] },
                });
            }
        }
        _ => {
            let pids = dedup_pids(rows.iter().map(|(pid, _)| pid.clone()));
            cards.push(Card {
                title: title.to_string(),
                layout: Layout::default(),
                tier: Tier::Standard,
                localized_title: title_is_localized,
                icon: None,
                source_pids: pids,
                kind: CardKind::Facts {
                    rows: rows.into_iter().map(|(_, row)| row).collect(),
                },
            });
        }
    }

    cards
}

/// Display order for a property's statements: what holds now first
/// (preferred rank, then statements without an end-time qualifier),
/// then history chronologically by start time. Keeps "country: France"
/// ahead of wartime occupations without dropping them.
fn ordered_statements(property: &Property) -> Vec<&qjson::Statement> {
    let mut statements: Vec<&qjson::Statement> = property.statements.iter().collect();
    statements.sort_by_cached_key(|s| {
        let ended = s.qualifiers.iter().any(|q| q.pid == END_TIME);
        let start = s
            .qualifiers
            .iter()
            .find_map(|q| match &q.value {
                Value::Time { iso, .. } if q.pid == START_TIME => Some(iso.clone()),
                _ => None,
            })
            .unwrap_or_default();
        (s.rank != Rank::Preferred, ended, start)
    });
    statements
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
const FEATURE_THUMB_WIDTH: u32 = 400; // chips rendered as image tiles

/// image_url arrives as an http FilePath URL from SPARQL; normalize to
/// https and request a chip-sized thumbnail.
pub(super) fn chip_thumb_url(image_url: &str) -> String {
    thumb_url(image_url, CHIP_THUMB_WIDTH)
}

fn thumb_url(image_url: &str, width: u32) -> String {
    let https = image_url.replacen("http://", "https://", 1);
    format!("{https}?width={width}")
}

/// A card that is just one or two linked items renders them as image
/// tiles (the picture fills half the card), so the thumbs need real
/// resolution. The matching layout rule lives in factoid.css.
fn feature_chip_thumbs(row: &mut FactRow) {
    if row.values.len() > 2 {
        return;
    }
    for value in &mut row.values {
        if let FactValue::Item(chip) = value
            && let Some(image_url) = &chip.image_url
        {
            chip.thumb_url = Some(thumb_url(image_url, FEATURE_THUMB_WIDTH));
        }
    }
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

pub(super) fn gallery_image(file_name: &str, caption: &str) -> GalleryImage {
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
    use crate::archetype::load_archetypes_config;
    use crate::grouping::load_grouping_config;

    fn nairobi_page() -> FactoidPage {
        let response: qjson::sparql::SparqlResponse = serde_json::from_str(include_str!(
            "../../../qjson/tests/fixtures/Q3870.sparql.json"
        ))
        .unwrap();
        let item = qjson::transform::transform("Q3870", &response.results.bindings);
        synthesize(
            &item,
            "en",
            &load_grouping_config().unwrap(),
            &load_archetypes_config().unwrap(),
            true,
        )
    }

    fn q42_page() -> FactoidPage {
        let response: qjson::sparql::SparqlResponse = serde_json::from_str(include_str!(
            "../../../qjson/tests/fixtures/Q42.sparql.json"
        ))
        .unwrap();
        let item = qjson::transform::transform("Q42", &response.results.bindings);
        synthesize(
            &item,
            "en",
            &load_grouping_config().unwrap(),
            &load_archetypes_config().unwrap(),
            true,
        )
    }

    fn find(page: &FactoidPage, predicate: impl Fn(&&Card) -> bool) -> Option<&Card> {
        page.all_cards().find(|c| predicate(c))
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
        let hero = page.hero.as_ref().expect("Nairobi has a P18");
        assert!(hero.thumb_url.contains("width=640"));
        assert_eq!(hero.caption, "Nairobi");
        // P18 has a single statement consumed by the hero - no duplicate card
        assert!(
            page.all_cards()
                .all(|c| c.source_pids != ["P18".to_string()])
        );
    }

    #[test]
    fn grouped_properties_form_one_facts_card() {
        let page = nairobi_page();
        // the located_in group (country + admin entity) is ONE card
        // with a labeled row per property - grouping means grouping
        let card =
            find(&page, |c| c.source_pids.contains(&"P17".to_string())).expect("located_in card");
        assert!(card.source_pids.contains(&"P131".to_string()));
        let CardKind::Facts { rows } = &card.kind else {
            panic!("located_in must be Facts, got {:?}", card.kind);
        };
        let country = rows
            .iter()
            .find(|r| r.label == "country")
            .expect("country row");
        let FactValue::Item(chip) = &country.values[0] else {
            panic!("country value must be an item");
        };
        assert_eq!(chip.label, "Kenya");
        assert_eq!(chip.qid, "Q114");
        let thumb = chip.thumb_url.as_deref().expect("Kenya has an image");
        assert!(thumb.starts_with("https://"));
        assert!(thumb.contains("width=96"));
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
        let page = synthesize(
            &item,
            "en",
            &load_grouping_config().unwrap(),
            &load_archetypes_config().unwrap(),
            true,
        );
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
    fn curation_meta_lands_in_footnotes() {
        let page = nairobi_page();
        // categories group (P910 et al.) and ungrouped curation PIDs
        // (P1343 described by source) are footnotes; content is not
        assert!(
            page.footnotes
                .iter()
                .any(|c| c.source_pids.contains(&"P910".to_string())),
            "categories card must be a footnote"
        );
        assert!(
            page.footnotes.iter().any(|c| c.source_pids == ["P1343"]),
            "described-by card must be a footnote"
        );
        assert!(
            !page.footnotes.iter().any(|c| c.source_pids == ["P1082"]),
            "population is content, not a footnote"
        );
        // the regions and the tier flag agree
        assert!(page.footnotes.iter().all(|c| c.tier == Tier::Footnote));
        assert!(page.overflow.iter().all(|c| c.tier == Tier::Standard));
    }

    #[test]
    fn person_hero_facts_from_recipe() {
        let page = q42_page();
        let facts = page.hero_facts.as_ref().expect("Q42 has hero facts");
        assert_eq!(facts.date_range.as_deref(), Some("1952 – 2001"));
        assert!(
            facts
                .tagline
                .as_deref()
                .is_some_and(|t| t.contains("writer")),
            "tagline from occupations: {:?}",
            facts.tagline
        );
        // the P109 signature is consumed by the header - no card of its own
        assert!(facts.emblem.is_some());
        assert!(page.all_cards().all(|c| c.source_pids != ["P109"]));
    }

    #[test]
    fn person_timeline_opens_career_section() {
        let page = q42_page();
        let career = page
            .sections
            .iter()
            .find(|s| s.name == "career")
            .expect("career section");
        let CardKind::Timeline { events } = &career.cards[0].kind else {
            panic!(
                "career leads with a timeline, got {:?}",
                career.cards[0].kind
            );
        };
        assert!(events.len() >= 4);
        assert!(events.windows(2).all(|w| w[0].iso <= w[1].iso));
        // birth opens the chronology; awards carry their item as detail
        assert!(events[0].iso.starts_with("1952"));
        assert!(
            events
                .iter()
                .any(|e| e.detail.is_some() && e.label == "award received")
        );
    }

    #[test]
    fn person_sections_claim_cards() {
        let page = q42_page();
        let names: Vec<&str> = page.sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["life", "career", "legacy"]);
        // claimed properties left the overflow grid
        assert!(
            page.overflow
                .iter()
                .all(|c| !c.source_pids.iter().any(|p| p == "P569" || p == "P166"))
        );
    }

    #[test]
    fn place_sections_claim_cards() {
        let page = nairobi_page();
        let names: Vec<&str> = page.sections.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"geography"), "sections: {names:?}");
        // population (people section) claimed its StatSeries
        let people = page.sections.iter().find(|s| s.name == "people").unwrap();
        assert!(people.cards.iter().any(|c| c.source_pids == ["P1082"]));
        // Nairobi has too few dated events - no timeline renders
        assert!(
            page.all_cards()
                .all(|c| !matches!(c.kind, CardKind::Timeline { .. }))
        );
    }

    #[test]
    fn preferred_media_supersedes_variants() {
        // Two flag images, one preferred: only the preferred renders
        // (the other is a superseded variant, not a second flag)
        let media = |file: &str, rank: qjson::Rank| qjson::Statement {
            value: qjson::Value::CommonsMedia {
                file_name: file.to_string(),
                url: format!("http://commons.wikimedia.org/wiki/Special:FilePath/{file}"),
            },
            rank,
            qualifiers: vec![],
        };
        let item = qjson::WikidataItem {
            qid: "Q1".to_string(),
            label: Some("Testland".to_string()),
            description: None,
            properties: std::collections::HashMap::from([(
                "P41".to_string(),
                qjson::Property {
                    pid: "P41".to_string(),
                    label: "flag image".to_string(),
                    statements: vec![
                        media("Old flag.svg", qjson::Rank::Normal),
                        media("Flag.svg", qjson::Rank::Preferred),
                    ],
                },
            )]),
        };
        let page = synthesize(
            &item,
            "en",
            &load_grouping_config().unwrap(),
            &load_archetypes_config().unwrap(),
            true,
        );
        let card = find(&page, |c| c.source_pids == ["P41"]).expect("flag card");
        let CardKind::Image { image } = &card.kind else {
            panic!(
                "one preferred flag must be a single Image, got {:?}",
                card.kind
            );
        };
        assert_eq!(image.file_name, "Flag.svg");
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
