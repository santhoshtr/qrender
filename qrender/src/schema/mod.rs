//! The composition schema: the data contract between the composer and the
//! localize step.

pub mod composition;
pub mod value;

pub use composition::{
    Block, COMPOSITION_SCHEMA, Composition, DESIGN, Dropped, Field, ProseConcepts, Silence,
    SilenceReason, Spread, Wanted,
};

pub use value::{
    Credit, MediaKind, Picture, PictureFacts, Provenance, QualifierPart, QualifierShown,
    StatementRef, ValueShown, round_to,
};
