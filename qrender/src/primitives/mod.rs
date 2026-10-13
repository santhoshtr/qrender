//! The primitives: one module for each kind of block, with all of its
//! behaviour. Each module holds its block type and its `build` function
//! (the composition, from the content, with no labels). The localize step
//! and the renderers add their faces to the same modules.

pub mod absent;
pub mod credits;
pub mod fact;
pub mod identifiers;
pub mod kind;
pub mod map;
pub mod media;
pub mod network;
pub mod peers;
pub mod prose;
pub mod provenance;
pub mod scale;
pub mod score;
pub mod series;
pub mod sitelinks;
pub mod succession;
pub mod time;
pub mod title;
pub mod works;

/// Values in the given order, each marked as shown when it is one of the
/// first `shown`; the rest are behind "+N".
pub(crate) fn mark_shown(
    values: impl IntoIterator<Item = crate::schema::StatementRef>,
    shown: usize,
) -> Vec<crate::schema::StatementRef> {
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| crate::schema::StatementRef {
            shown: Some(i < shown),
            ..v
        })
        .collect()
}
