//! Layers 2 and 3: the composer, the localize step and the renderers. This
//! crate has no network and no I/O; it reads the content and the text that
//! the `content` crate gives it.

pub mod config;
pub mod vocab;
