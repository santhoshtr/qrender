//! The slots and the template of a facts spread, by the placement and
//! template tables.

use indexmap::IndexMap;

use super::Composer;
use super::blocks::Made;
use super::facts::{PlacementFacts, TemplateFacts};
use crate::config::rules::first;
use crate::primitives::network;
use crate::schema::{Block, MediaKind};
use crate::vocab::{Slot, Template};

impl Composer<'_> {
    /// The first block of a lead primitive leads the spread (the hero
    /// slot). The others go to the slot of their placement rule. A stack
    /// with a block as tall as the lead (a second office) keeps only such
    /// blocks; the others go under the lead. Slot order is reading order:
    /// hero, stack, gallery, sheet, strips.
    pub fn place(&self, made: Vec<Made>) -> (IndexMap<Slot, Vec<Block>>, Template) {
        let rules = &self.config.rules;
        let mut placed: IndexMap<Slot, Vec<Block>> = IndexMap::new();
        let mut singles = Vec::new();
        let mut single_rule = None;
        for m in made {
            match m {
                Made::Single { value, rule } => {
                    singles.push(value);
                    single_rule = Some(rule);
                }
                Made::Block { make, block } => {
                    let facts = PlacementFacts {
                        make,
                        primitive: Some(block.primitive()),
                        audio: matches!(&block, Block::Media(m) if m.picture.kind == MediaKind::Audio),
                        strip: block.is_strip(),
                    };
                    placed
                        .entry(first(&rules.placement, &facts).slot)
                        .or_default()
                        .push(block);
                }
            }
        }
        let stack = placed.shift_remove(&Slot::Stack).unwrap_or_default();
        let lead_at = stack
            .iter()
            .position(|b| rules.lead.primitives.contains(&b.primitive()));
        let mut stack = stack;
        let lead = lead_at.map(|i| stack.remove(i));
        let template = first(
            &rules.template,
            &TemplateFacts {
                lead: lead.as_ref().map(Block::primitive),
            },
        )
        .template;

        let mut hero: Vec<Block> = lead.into_iter().collect();
        let beside = |b: &Block| rules.lead.beside.contains(&b.primitive());
        if !hero.is_empty() && stack.iter().any(beside) {
            let (kept, under): (Vec<Block>, Vec<Block>) = stack.into_iter().partition(beside);
            hero.extend(under);
            stack = kept;
        }

        let mut slots = IndexMap::new();
        if !hero.is_empty() {
            slots.insert(Slot::Hero, hero);
        }
        if !stack.is_empty() {
            slots.insert(Slot::Stack, stack);
        }
        // The single pictures of different properties, of the item and of
        // its values: a slot of their own, before the sheet.
        let mut gallery = placed.shift_remove(&Slot::Gallery).unwrap_or_default();
        if !singles.is_empty() {
            gallery.push(Block::Gallery(network::gallery(
                singles,
                single_rule.as_deref(),
            )));
        }
        if !gallery.is_empty() {
            slots.insert(Slot::Gallery, gallery);
        }
        // The sheet, then any other slot a placement rule named. The strips
        // last: each takes the full width, under the columns.
        let strips = placed.shift_remove(&Slot::Strips);
        slots.extend(placed);
        if let Some(strips) = strips {
            slots.insert(Slot::Strips, strips);
        }
        (slots, template)
    }
}
