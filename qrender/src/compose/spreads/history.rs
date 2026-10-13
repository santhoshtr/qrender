//! A history spread: the time axis. Below its threshold the spread stays,
//! empty, with the reason, when there is a dated point or a topic for it.

use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::time;
use crate::schema::{Block, Silence, SilenceReason, Spread};
use crate::vocab::{Primitive, Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> Option<Spread> {
    match time::build(c) {
        Ok(axis) => {
            let slots = IndexMap::from([(Slot::Axis, vec![Block::Time(axis)])]);
            Some(c.spread(spec, slots, Template::Axis, Vec::new()))
        }
        Err(points) => {
            let topics = &c.config.layouts.topics;
            let has_topic = c
                .properties()
                .any(|(p, _)| topics.get(&p).is_some_and(|s| *s == spec.id));
            if points == 0 && !has_topic {
                return None;
            }
            let silence = Silence {
                primitive: Primitive::Time,
                reason: SilenceReason::BelowThreshold,
                points: Some(points),
                needs: Some(c.config.layouts.thresholds.time_points),
            };
            Some(c.spread(spec, IndexMap::new(), Template::Axis, vec![silence]))
        }
    }
}
