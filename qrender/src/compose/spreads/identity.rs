//! The page head, the same for every class: the title and description,
//! the banner, the lead beside the picture, the kinds under it.

use content::Rank;
use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::media::MediaRole;
use crate::primitives::title::Title;
use crate::primitives::{kind, media, prose};
use crate::schema::{Block, Spread};
use crate::vocab::{Role, Slot, Template};
use crate::wikidata::{IMAGE, LOGO, PAGE_BANNER};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> Spread {
    // The image of the item; without one, its logo.
    let mut images = c.free(*IMAGE);
    if images.is_empty() {
        images = c.free(*LOGO);
    }
    let hero = images
        .iter()
        .find(|s| s.rank == Rank::Preferred)
        .or(images.first())
        .copied();
    // The character after the title: the only value, or the only
    // preferred one. Else no choice is made: the values stay a fact.
    let glyph = c
        .config
        .layouts
        .roles
        .list(Role::Glyph)
        .iter()
        .find_map(|pid| {
            let glyphs = c.free(*pid);
            let preferred: Vec<_> = glyphs
                .iter()
                .filter(|s| s.rank == Rank::Preferred)
                .copied()
                .collect();
            match (glyphs.as_slice(), preferred.as_slice()) {
                ([only], _) | (_, [only]) => Some(*only),
                _ => None,
            }
        });
    let glyph = glyph.map(|s| {
        c.take([s]);
        c.statement(s)
    });
    let mut slots = IndexMap::new();
    slots.insert(
        Slot::Head,
        vec![Block::Title(Title {
            item: c.content.qid,
            glyph,
        })],
    );
    // The first banner; the others go to the media spread.
    let banners = c.free(*PAGE_BANNER);
    if let Some(banner) = c.take(banners.into_iter().take(1)).first() {
        let block = media::build(c, banner, MediaRole::Banner, None);
        slots.insert(Slot::Band, vec![Block::Media(block)]);
    }
    let kinds = kind::build(c);
    slots.insert(
        Slot::Lead,
        vec![Block::Prose(prose::lead(&c.content.sitelinks))],
    );
    if let Some(hero) = hero {
        c.take([hero]);
        let block = media::build(c, hero, MediaRole::Identity, None);
        slots.insert(Slot::Hero, vec![Block::Media(block)]);
    }
    slots.insert(Slot::Kind, vec![Block::Kind(kinds)]);
    c.spread(spec, slots, Template::Identity, Vec::new())
}
