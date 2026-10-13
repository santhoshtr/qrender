//! The values of one property, made into blocks by the rule tables: the
//! split table divides them into groups, the block table gives each group
//! its block, the order table orders a set.

use content::{Datatype, Pid, Statement};

use super::Composer;
use super::facts::GroupFacts;
use crate::config::rules::{BlockMake, SplitMake, first};
use crate::primitives::fact::FactForm;
use crate::primitives::media::MediaRole;
use crate::primitives::network::GalleryValue;
use crate::primitives::{fact, map, media, network, score, series};
use crate::schema::Block;

/// What the rules made of a group of values. Short-lived: placement
/// consumes it at once, so the size of its variants does not matter.
#[allow(clippy::large_enum_variant)]
pub enum Made {
    /// A block, and the block rule that made it (None when a property rule
    /// made it: a succession, the credits, the works).
    Block {
        make: Option<BlockMake>,
        block: Block,
    },
    /// A small value: it joins the gallery of the spread. With the id of
    /// the block rule that made it.
    Single { value: GalleryValue, rule: String },
}

impl<'a> Composer<'a> {
    /// The blocks of the claimed statements of one property. `spread_pids`
    /// are the properties of the spread: a map takes its markers from them.
    pub fn blocks_for(
        &mut self,
        pid: Pid,
        statements: Vec<&'a Statement>,
        spread_pids: &[Pid],
    ) -> Vec<Made> {
        let rules = &self.config.rules;
        let mut groups = vec![(statements, false)];
        if groups[0].0[0].value.datatype() == Datatype::WikibaseItem {
            // Item values: current first, and the ones that ended apart.
            let current = self.current(groups.remove(0).0);
            let split = first(&rules.split, &GroupFacts::new(self, pid, &current, false));
            groups = match split.make {
                SplitMake::SplitEnded => {
                    let (ended, rest) = current.into_iter().partition(|s| Self::ended(s));
                    vec![(ended, true), (rest, false)]
                }
                SplitMake::Together => vec![(current, false)],
            };
        }
        let mut out = Vec::new();
        for (group, former) in groups {
            if group.is_empty() {
                continue;
            }
            let rule = first(&rules.block, &GroupFacts::new(self, pid, &group, former));
            out.extend(self.make(rule.make, &rule.id, pid, &group, spread_pids));
        }
        out
    }

    /// The blocks that one block rule makes of a group of values.
    fn make(
        &mut self,
        make: BlockMake,
        rule: &str,
        pid: Pid,
        group: &[&'a Statement],
        spread_pids: &[Pid],
    ) -> Vec<Made> {
        let thresholds = &self.config.layouts.thresholds;
        let id = rule;
        let rule = Some(rule);
        let block = match make {
            // The first value only, as in v3; the others are claimed but not
            // shown (to fix after parity).
            BlockMake::Media => Block::Media(media::build(self, group[0], MediaRole::Stack, rule)),
            BlockMake::Map => Block::Map(map::build(self, group[0], spread_pids, rule)),
            BlockMake::Series => match series::dated(self, pid, group, rule) {
                Some(series) => Block::Series(series),
                None => return Vec::new(),
            },
            BlockMake::FormerList => Block::Fact(fact::Fact {
                former: true,
                ..fact::build(self, pid, group, Some(FactForm::List), rule)
            }),
            BlockMake::ItemList => {
                let form = (group.len() > 2).then_some(FactForm::List);
                Block::Fact(fact::build(self, pid, group, form, rule))
            }
            BlockMake::Single => {
                let priority = self.priority(pid, group);
                return group
                    .iter()
                    .map(|s| Made::Single {
                        value: GalleryValue {
                            statement: self.statement(s),
                            priority,
                        },
                        rule: id.to_string(),
                    })
                    .collect();
            }
            BlockMake::Score => Block::Score(score::build(self, pid, group, rule)),
            BlockMake::Network => {
                let order = first(
                    &self.config.rules.order,
                    &GroupFacts::new(self, pid, group, false),
                );
                Block::Network(network::build(
                    self,
                    pid,
                    group,
                    order.order,
                    thresholds.set_max,
                    rule,
                ))
            }
            BlockMake::Fact => Block::Fact(fact::build(self, pid, group, None, rule)),
        };
        vec![Made::Block {
            make: Some(make),
            block,
        }]
    }
}
