//! The design system: Codex tokens, and the stylesheet joined from its
//! partials at compile time, in a fixed order (base, templates,
//! primitives). Served at a URL with its version, so a new design is never
//! hidden by a cached old one.

pub use crate::schema::DESIGN;

pub const TOKENS: &str = include_str!("../../design/tokens.css");

pub const STYLESHEET: &str = concat!(
    include_str!("../../design/base.css"),
    include_str!("../../design/templates/identity.css"),
    include_str!("../../design/templates/lead-beside.css"),
    include_str!("../../design/templates/columns.css"),
    include_str!("../../design/templates/feature-run.css"),
    include_str!("../../design/templates/collapsed.css"),
    include_str!("../../design/primitives/title.css"),
    include_str!("../../design/primitives/kind.css"),
    include_str!("../../design/primitives/prose.css"),
    include_str!("../../design/primitives/scale.css"),
    include_str!("../../design/primitives/series.css"),
    include_str!("../../design/primitives/map.css"),
    include_str!("../../design/primitives/time.css"),
    include_str!("../../design/primitives/fact.css"),
    include_str!("../../design/primitives/score.css"),
    include_str!("../../design/primitives/succession.css"),
    include_str!("../../design/primitives/sitelinks.css"),
    include_str!("../../design/primitives/provenance.css"),
    include_str!("../../design/primitives/identifiers.css"),
);

/// The inline scripts of a page: progressive enhancement only.
pub const SCRIPTS: &str = concat!(
    include_str!("../../assets/invokers.js"),
    include_str!("../../assets/intl.js"),
);
