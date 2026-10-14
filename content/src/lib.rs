//! Layer 1: the content of a Wikidata item and its text in one language.
//!
//! Two documents, because they change for different reasons:
//! - [`Content`], one for each item: statements with identifiers and typed
//!   values, references, the container, peers, Commons metadata. No text.
//! - [`Text`], one for each item and language: labels, descriptions, the
//!   article, page images.

#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "fetch")]
pub mod fetch;
pub mod html;
pub mod id;
pub mod model;
pub mod settings;
pub mod text;

pub use id::{EntityId, IdError, Lang, Pid, Qid, StatementId};
pub use model::{
    CONTENT_SCHEMA, Cell, CommonsFile, Container, Content, Datatype, NormalQuantity, Peers,
    Qualifier, QualifierValue, QuantityRecord, Range, Rank, ReverseWork, Statement, Table, Value,
    ValueError, ValueImage,
};
pub use settings::{LayoutMatch, Settings};
pub use text::{Article, Concept, LangText, PageImage, Section, TEXT_SCHEMA, Term, Text};
