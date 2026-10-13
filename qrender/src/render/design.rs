//! The design system: Codex tokens, the stylesheet, the page scripts.
//!
//! Each primitive module declares its own CSS and JavaScript (`CSS`, `JS`);
//! `assets` is the one table of them, and the compiler checks that it names
//! every primitive. The stylesheet is one file, served at a URL with its
//! version and cached by every page: `base.css`, the template partials, then
//! the CSS of each primitive in the order of `Primitive::ALL`. A page gets
//! the scripts of the primitives it shows.

use std::sync::LazyLock;

pub use crate::schema::DESIGN;

use crate::primitives::{
    fact, identifiers, kind, map, prose, provenance, scale, score, series, sitelinks, sounds,
    succession, time, title,
};
use crate::vocab::Primitive;

pub const TOKENS: &str = include_str!("../../design/tokens.css");

/// The CSS of the page grid and of the templates (the design system's own).
const BASE: [&str; 6] = [
    include_str!("../../design/base.css"),
    include_str!("../../design/templates/identity.css"),
    include_str!("../../design/templates/lead-beside.css"),
    include_str!("../../design/templates/columns.css"),
    include_str!("../../design/templates/feature-run.css"),
    include_str!("../../design/templates/collapsed.css"),
];

/// The scripts of every page: progressive enhancement only.
pub const PAGE_SCRIPTS: [&str; 2] = [
    include_str!("../../assets/invokers.js"),
    include_str!("../../assets/intl.js"),
];

/// The CSS and the JavaScript of a primitive, from its module.
#[derive(Debug, Clone, Copy, Default)]
pub struct Assets {
    pub css: Option<&'static str>,
    pub js: Option<&'static str>,
}

/// The assets of each primitive. A primitive without its own CSS uses the
/// shared parts of `base.css` (figures, runs, bars, value lines).
pub fn assets(primitive: Primitive) -> Assets {
    let css = |css| Assets {
        css: Some(css),
        js: None,
    };
    match primitive {
        Primitive::Title => css(title::CSS),
        Primitive::Kind => css(kind::CSS),
        Primitive::Prose => css(prose::CSS),
        Primitive::Sounds => Assets {
            css: Some(sounds::CSS),
            js: Some(sounds::JS),
        },
        Primitive::Map => css(map::CSS),
        Primitive::Fact => css(fact::CSS),
        Primitive::Series => css(series::CSS),
        Primitive::Scale => css(scale::CSS),
        Primitive::Succession => css(succession::CSS),
        Primitive::Score => css(score::CSS),
        Primitive::Time => css(time::CSS),
        Primitive::Provenance => css(provenance::CSS),
        Primitive::Sitelinks => css(sitelinks::CSS),
        Primitive::Identifiers => css(identifiers::CSS),
        Primitive::Media
        | Primitive::Pictures
        | Primitive::Peers
        | Primitive::Network
        | Primitive::Credits
        | Primitive::Works
        | Primitive::Absent => Assets::default(),
    }
}

/// The stylesheet: the design system's CSS, then each primitive's.
pub static STYLESHEET: LazyLock<String> = LazyLock::new(|| {
    BASE.into_iter()
        .chain(Primitive::ALL.into_iter().filter_map(|p| assets(p).css))
        .collect()
});

/// The scripts of a page: the page scripts, then the scripts of the
/// primitives it shows.
pub fn scripts(shown: &[Primitive]) -> String {
    PAGE_SCRIPTS
        .into_iter()
        .chain(
            Primitive::ALL
                .into_iter()
                .filter(|p| shown.contains(p))
                .filter_map(|p| assets(p).js),
        )
        .collect()
}
