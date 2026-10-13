//! The composition schema: the data contract between the composer and the
//! localize step.

pub mod value;

pub use value::{
    Credit, MediaKind, Picture, Provenance, QualifierPart, QualifierShown, StatementRef,
    ValueShown, round_to,
};
