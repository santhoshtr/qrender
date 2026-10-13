//! Layers 2 and 3: the composer, the localize step and the renderers. This
//! crate has no network and no I/O; it reads the content and the text that
//! the `content` crate gives it.

pub mod compose;
pub mod config;
pub mod localize;
pub mod primitives;
pub mod render;
pub mod schema;
pub mod vocab;
pub mod wikidata;

use content::Content;

use crate::config::Config;
use crate::schema::Composition;

pub use localize::localize;

/// The composition of an item: the same for every language.
pub fn compose(content: &Content, config: &Config) -> Composition {
    compose::Composer::new(content, config).compose()
}
