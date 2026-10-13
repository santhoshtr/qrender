//! Layer 1: the content of a Wikidata item and its text in one language.
//!
//! Two documents, because they change for different reasons:
//! - [`Content`], one for each item: statements with identifiers and typed
//!   values, references, the container, peers, Commons metadata. No text.
//! - [`Text`], one for each item and language: labels, descriptions, the
//!   article, page images.

pub mod id;
pub mod model;
pub mod text;

pub use id::{EntityId, IdError, Lang, Pid, Qid, StatementId};
pub use model::{
    CONTENT_SCHEMA, Cell, CommonsFile, Container, Content, Datatype, NormalQuantity, Peers,
    Qualifier, QualifierValue, QuantityRecord, Rank, ReverseWork, Statement, Table, Value,
    ValueError, ValueImage,
};
pub use text::{Article, Concept, LangText, PageImage, Section, TEXT_SCHEMA, Term, Text};
