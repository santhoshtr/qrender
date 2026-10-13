//! A history spread: the time axis. Below its threshold the spread stays,
//! empty, with the reason, when there is a dated point or a topic for it.

use indexmap::IndexMap;

use crate::compose::Composer;
use crate::config::SpreadSpec;
use crate::primitives::time;
use crate::schema::{Block, Silence, SilenceReason};
use crate::vocab::{Primitive, Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> super::Built {
    match time::build(c) {
        Ok(axis) => {
            let slots = IndexMap::from([(Slot::Axis, vec![Block::Time(axis)])]);
            Ok(c.spread(spec, slots, Template::Axis, Vec::new()))
        }
        Err(points) => {
            let topics = &c.config.layouts.topics;
            let has_topic = c
                .properties()
                .any(|(p, _)| topics.get(&p).is_some_and(|s| *s == spec.id));
            let silence = Silence {
                primitive: Primitive::Time,
                reason: SilenceReason::BelowThreshold,
                points: Some(points),
                needs: Some(c.config.layouts.thresholds.time_points),
            };
            if points == 0 && !has_topic {
                return Err(vec![silence]);
            }
            Ok(c.spread(spec, IndexMap::new(), Template::Axis, vec![silence]))
        }
    }
}
