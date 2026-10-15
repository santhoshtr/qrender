//! The spread builders: one for each spread kind. A builder is a composite:
//! it claims statements, asks the rule tables and calls the `build` of the
//! primitives. Only `facts` is driven by the rules alone; the others are
//! short and specialized.

mod evidence;
mod facts;
mod history;
mod identity;
mod media;
mod more;
mod scale;
mod web;

use super::Composer;
use crate::config::SpreadSpec;
use crate::schema::{Silence, Spread};
use crate::vocab::SpreadKind;

/// A spread, or why it has nothing to show: the silences of its
/// primitives (none when its properties have no statements).
pub type Built = Result<Spread, Vec<Silence>>;

pub fn build<'a>(c: &mut Composer<'a>, spec: &'a SpreadSpec) -> Built {
    match spec.kind {
        SpreadKind::Identity => Ok(identity::build(c, spec)),
        SpreadKind::Facts => facts::build(c, spec),
        SpreadKind::Scale => scale::build(c, spec),
        SpreadKind::History => history::build(c, spec),
        SpreadKind::Media => media::build(c, spec),
        SpreadKind::Web => web::build(c, spec),
        SpreadKind::More => more::build(c, spec),
        SpreadKind::Evidence => Ok(evidence::build(c, spec)),
    }
}
