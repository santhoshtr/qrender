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

use super::Composer;
use crate::config::SpreadSpec;
use crate::schema::Spread;
use crate::vocab::SpreadKind;

/// The spread, or None when it has nothing to show.
pub fn build<'a>(c: &mut Composer<'a>, spec: &'a SpreadSpec) -> Option<Spread> {
    match spec.kind {
        SpreadKind::Identity => Some(identity::build(c, spec)),
        SpreadKind::Facts => facts::build(c, spec),
        SpreadKind::Scale => scale::build(c, spec),
        SpreadKind::History => history::build(c, spec),
        SpreadKind::Media => media::build(c, spec),
        SpreadKind::More => more::build(c, spec),
        SpreadKind::Evidence => Some(evidence::build(c, spec)),
    }
}
