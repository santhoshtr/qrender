//! The design system: Codex tokens, the stylesheet, the page scripts.
//!
//! Each primitive module declares its own CSS and JavaScript (`CSS`, `JS`);
//! `assets` is the one table of them, and the compiler checks that it names
//! every primitive. The stylesheet is one file, served at a URL with its
//! version and cached by every page: the Codex tokens, `base.css`, the
//! template partials, then the CSS of each primitive in the order of
//! `Primitive::ALL`. A page gets
//! the scripts of the primitives it shows. A library that a script loads
//! (ECharts, Leaflet) is served by the same route, not put in the page.

use std::sync::LazyLock;

pub use crate::schema::DESIGN;

use crate::primitives::{
    earlier, fact, identifiers, kind, links, map, names, prose, provenance, scale, score, series,
    sitelinks, sounds, split, succession, timeline, title,
};
use crate::vocab::Primitive;

const TOKENS: &str = include_str!("../../design/tokens.css");

/// Apache ECharts (the "common" build), for the qr-chart element. The
/// server serves it by the design route; `chart.js` loads it.
pub const ECHARTS: &str = include_str!("../../assets/vendor/echarts/echarts.common.min.js");

/// Leaflet, for the qr-map element (the map of a dialog). The server
/// serves it by the design route; `map.js` loads it.
pub const LEAFLET_JS: &str = include_str!("../../assets/vendor/leaflet/leaflet.js");
pub const LEAFLET_CSS: &str = include_str!("../../assets/vendor/leaflet/leaflet.css");

/// The icon of the translation action (a Codex icon), inline in the page:
/// in the colour of its button (`currentColor`), hidden from screen
/// readers (the button has its words), without its English title.
pub static LANGUAGE_ICON: LazyLock<String> = LazyLock::new(|| {
    let svg = include_str!("../../assets/icons/language.svg");
    let start = svg.find("<svg").expect("an svg element");
    let svg = &svg[start..];
    let svg = match (svg.find("<title>"), svg.find("</title>")) {
        (Some(a), Some(b)) => format!("{}{}", &svg[..a], &svg[b + "</title>".len()..]),
        _ => svg.to_string(),
    };
    svg.replacen("<svg", r#"<svg aria-hidden="true" fill="currentColor""#, 1)
        .trim()
        .to_string()
});

/// The CSS of the page grid and of the templates (the design system's own).
const BASE: [&str; 6] = [
    include_str!("../../design/base.css"),
    include_str!("../../design/templates/identity.css"),
    include_str!("../../design/templates/lead-beside.css"),
    include_str!("../../design/templates/columns.css"),
    include_str!("../../design/templates/feature-run.css"),
    include_str!("../../design/templates/collapsed.css"),
];

/// The scripts of every page: progressive enhancement only. The lightbox,
/// 3D model and map scripts are here, not with a primitive: the media,
/// pictures and other blocks show the files of the item, and a coordinate
/// pin (a map) can be in any block. The translation of the article's
/// sections is in spreads and at the end of the page.
pub const PAGE_SCRIPTS: [&str; 7] = [
    include_str!("../../assets/invokers.js"),
    include_str!("../../assets/intl.js"),
    include_str!("../../assets/play.js"),
    include_str!("../../assets/lightbox.js"),
    include_str!("../../assets/model.js"),
    include_str!("../../assets/map.js"),
    include_str!("../../assets/translate.js"),
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
        Primitive::Names => Assets {
            css: Some(names::CSS),
            js: Some(names::JS),
        },
        Primitive::Map => css(map::CSS),
        Primitive::Earlier => css(earlier::CSS),
        Primitive::Split => css(split::CSS),
        Primitive::Fact => css(fact::CSS),
        Primitive::Series => Assets {
            css: Some(series::CSS),
            js: Some(series::JS),
        },
        Primitive::Scale => css(scale::CSS),
        Primitive::Succession => css(succession::CSS),
        Primitive::Score => css(score::CSS),
        Primitive::Timeline => css(timeline::CSS),
        Primitive::Links => css(links::CSS),
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

/// The stylesheet: the tokens, the design system's CSS, then each
/// primitive's.
pub static STYLESHEET: LazyLock<String> = LazyLock::new(|| {
    std::iter::once(TOKENS)
        .chain(BASE)
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
