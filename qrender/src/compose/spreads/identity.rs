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
use crate::vocab::{Slot, Template};
use crate::wikidata::{IMAGE, PAGE_BANNER};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> Spread {
    let images = c.free(*IMAGE);
    let hero = images
        .iter()
        .find(|s| s.rank == Rank::Preferred)
        .or(images.first())
        .copied();
    let mut slots = IndexMap::new();
    slots.insert(
        Slot::Head,
        vec![Block::Title(Title {
            item: c.content.qid,
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
