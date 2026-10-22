//! Layer 1: the content of a Wikidata item and its text in one language.
//!
//! Two documents, because they change for different reasons:
//! - [`Content`], one for each item: statements with identifiers and typed
//!   values, references, the container, peers, Commons metadata. No text.
//! - [`Text`], one for each item and language: labels, descriptions, the
//!   article, page images.

#[cfg(feature = "cache")]
pub mod cache;
pub mod citation;
#[cfg(feature = "fetch")]
pub mod fetch;
pub mod geo;
pub mod html;
pub mod id;
pub mod model;
pub mod settings;
pub mod text;

pub use citation::{
    Archive, Book, Citation, Cited, Date, Entry, Identifier, Journal, Magazine, News, Origin,
    Original, Person, Record, RecordId, Report, Short, Web, WebUrl, Work,
};
pub use id::{EntityId, IdError, Lang, Pid, Qid, StatementId};
pub use model::{
    CONTENT_SCHEMA, Cell, CommonsFile, Container, Content, Datatype, EventDates, HolidayDay,
    NormalQuantity, Peers, Qualifier, QualifierValue, QuantityRecord, Range, Rank, ReverseWork,
    Shape, Statement, Table, Value, ValueError, ValueImage,
};
pub use settings::{CitationStyle, LayoutMatch, Settings};
pub use text::{
    Article, ArticleFigure, ArticleImage, Concept, LangText, Note, OutlineBlock, OutlineSection,
    PageImage, Section, TEXT_SCHEMA, Term, Text,
};
