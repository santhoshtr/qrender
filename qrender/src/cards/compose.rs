//! Composition: apply an archetype recipe to the synthesized card pool.
//! Sections claim cards by source PID (first section wins), a timeline
//! card is synthesized from dated statements across properties, and
//! hero facts surface in the page header. Nothing is dropped: unclaimed
//! cards remain for the overflow grid, under-threshold features simply
//! don't render, and a recipe-less archetype composes to pure overflow.

use qjson::{Rank, Value, WikidataItem};

use super::format::format_time;
use super::synthesis::chip_thumb_url;
use super::{Card, CardKind, HeroFacts, Layout, Section, Tier, TimelineEvent};
use crate::archetype::{ArchetypeConfig, HeroConfig, TimelineConfig};

const START_TIME: &str = "P580";
const POINT_IN_TIME: &str = "P585";

/// Distribute standard-tier cards into the recipe's sections; the rest
/// stay in the overflow grid.
pub(super) fn sections(
    item: &WikidataItem,
    cards: Vec<Card>,
    recipe: &ArchetypeConfig,
) -> (Vec<Section>, Vec<Card>) {
    let mut pool = cards;
    let mut sections = Vec::new();
    for section_config in &recipe.sections {
        let (mut claimed, rest): (Vec<Card>, Vec<Card>) = pool.into_iter().partition(|card| {
            card.source_pids
                .iter()
                .any(|pid| section_config.pids.contains(pid))
        });
        pool = rest;
        if let Some(timeline_config) = &section_config.timeline
            && let Some(card) = timeline_card(item, timeline_config)
        {
            claimed.insert(0, card);
        }
        if !claimed.is_empty() {
            sections.push(Section {
                name: section_config.name.clone(),
                icon: section_config.icon.clone(),
                cards: claimed,
            });
        }
    }
    (sections, pool)
}

/// Header facts from the hero config. Every part is optional - absent
/// data leaves the header exactly as it is without the recipe.
pub(super) fn hero_facts(item: &WikidataItem, config: &HeroConfig) -> Option<HeroFacts> {
    let year = |pid: &String| {
        let statement = item.properties.get(pid)?.statements.first()?;
        match &statement.value {
            Value::Time { iso, .. } => Some(format_time(iso, Some(9))),
            _ => None,
        }
    };
    let (start, end) = match config.dates.as_slice() {
        [start_pid, end_pid] => (year(start_pid), year(end_pid)),
        [start_pid] => (year(start_pid), None),
        _ => (None, None),
    };
    let date_range = match (start, end) {
        (Some(start), Some(end)) => Some(format!("{start} – {end}")),
        (Some(start), None) => Some(format!("{start} –")),
        (None, Some(end)) => Some(format!("– {end}")),
        (None, None) => None,
    };

    let tagline = config.tagline.as_ref().and_then(|pid| {
        // preferred-rank statements first: "writer" before hobby gigs
        let mut statements: Vec<&qjson::Statement> =
            item.properties.get(pid)?.statements.iter().collect();
        statements.sort_by_key(|s| s.rank != Rank::Preferred);
        let labels: Vec<&str> = statements
            .iter()
            .filter_map(|s| match &s.value {
                Value::ItemRef { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .take(4)
            .collect();
        (!labels.is_empty()).then(|| labels.join(" · "))
    });

    let emblem = config.emblem.as_ref().and_then(|pid| {
        let property = item.properties.get(pid)?;
        let statement = property.statements.first()?;
        match &statement.value {
            Value::CommonsMedia { file_name, .. } => {
                Some(super::synthesis::gallery_image(file_name, &property.label))
            }
            _ => None,
        }
    });

    if date_range.is_none() && tagline.is_none() && emblem.is_none() {
        return None;
    }
    Some(HeroFacts {
        date_range,
        tagline,
        emblem,
    })
}

/// One chronology from every dated statement of the configured
/// properties: Time values directly (birth, death), item values via a
/// start-time or point-in-time qualifier (awards, occupations). Below
/// min_events no card renders - no half-empty showpieces.
fn timeline_card(item: &WikidataItem, config: &TimelineConfig) -> Option<Card> {
    let mut events: Vec<TimelineEvent> = Vec::new();
    let mut source_pids: Vec<String> = Vec::new();
    for pid in &config.pids {
        let Some(property) = item.properties.get(pid) else {
            continue;
        };
        let before = events.len();
        for statement in &property.statements {
            // Years only: keeps the date rail compact; the full date
            // still shows on the property's own card. (Qualifier
            // precision is never fetched anyway - see transform.rs.)
            let event = match &statement.value {
                Value::Time { iso, .. } => Some(TimelineEvent {
                    iso: iso.clone(),
                    display: format_time(iso, Some(9)),
                    label: property.label.clone(),
                    detail: None,
                    thumb_url: None,
                }),
                Value::ItemRef {
                    label, image_url, ..
                } => event_time(statement).map(|iso| TimelineEvent {
                    display: format_time(&iso, Some(9)),
                    iso,
                    label: property.label.clone(),
                    detail: Some(label.clone()),
                    thumb_url: image_url.as_deref().map(chip_thumb_url),
                }),
                _ => None,
            };
            events.extend(event);
        }
        if events.len() > before {
            source_pids.push(pid.clone());
        }
    }
    if events.len() < config.min_events {
        return None;
    }
    events.sort_by(|a, b| a.iso.cmp(&b.iso));

    let kind = CardKind::Timeline { events };
    let (cols, rows) = super::synthesis::kind_layout(&kind);
    Some(Card {
        // machine name: the event icon carries the meaning visually
        title: "timeline".to_string(),
        localized_title: false,
        icon: Some("event".to_string()),
        source_pids,
        layout: Layout {
            cols,
            rows,
            ..Layout::default()
        },
        tier: Tier::Standard,
        kind,
    })
}

fn event_time(statement: &qjson::Statement) -> Option<String> {
    statement.qualifiers.iter().find_map(|q| match &q.value {
        Value::Time { iso, .. } if q.pid == START_TIME || q.pid == POINT_IN_TIME => {
            Some(iso.clone())
        }
        _ => None,
    })
}
