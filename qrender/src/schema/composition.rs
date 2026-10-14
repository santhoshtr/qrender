//! The composition of one item (`composition/0.3`): its spreads, the slots
//! of each spread, the blocks in each slot. No labels, no geometry.

use content::{Qid, StatementId};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Serialize;

use super::StatementRef;
use crate::primitives::{
    absent, credits, fact, identifiers, kind, map, media, names, network, peers, pictures, prose,
    provenance, scale, score, series, sitelinks, sounds, split, succession, timeline, title, works,
};
use crate::vocab::{Dimension, Primitive, Slot, SpreadKind, Template};

pub const COMPOSITION_SCHEMA: &str = "composition/0.3";
pub const DESIGN: &str = "design/0.2";

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Composition {
    pub schema: &'static str,
    pub design: &'static str,
    pub qid: Qid,
    pub class_layout: String,
    pub spreads: Vec<Spread>,
    /// The spreads of the class layout that have nothing to show, and why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<Dropped>,
    /// Statements that no spread shows. Always empty: a test checks it.
    pub unshown: Vec<StatementId>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Spread {
    pub id: String,
    pub kind: SpreadKind,
    pub dimension: Dimension,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<Qid>,
    pub template: Template,
    /// In reading order.
    pub slots: IndexMap<Slot, Vec<Block>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prose: Option<ProseConcepts>,
    /// The "... of topic" items of this spread.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<StatementRef>,
    /// Primitives that had too little data, with the reason.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub silent: Vec<Silence>,
}

/// A spread with nothing to show. `silent`: the silences of its
/// primitives; empty when its properties have no statements.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Dropped {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub silent: Vec<Silence>,
}

/// The concepts whose article sections give the spread its prose.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProseConcepts {
    pub concepts: Vec<Qid>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Silence {
    pub primitive: Primitive,
    pub reason: SilenceReason,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SilenceReason {
    NoComparableQuantity,
}

/// A block: one primitive, from its own module.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "primitive", rename_all = "kebab-case")]
pub enum Block {
    Title(title::Title),
    Kind(kind::Kind),
    Prose(prose::Prose),
    Media(media::Media),
    Pictures(pictures::Pictures),
    Sounds(sounds::Sounds),
    Names(names::Names),
    Map(map::Map),
    Fact(fact::Fact),
    Series(series::Series),
    Split(split::Split),
    Scale(scale::Scale),
    Peers(peers::Peers),
    Network(network::Network),
    /// The single values of a spread: a network block in its gallery form.
    #[serde(rename = "network")]
    Gallery(network::Gallery),
    Succession(succession::Succession),
    Credits(credits::Credits),
    Score(score::Score),
    Works(works::Works),
    Timeline(timeline::Timeline),
    Absent(absent::Absent),
    Provenance(provenance::Provenance),
    Sitelinks(sitelinks::Sitelinks),
    Identifiers(identifiers::Identifiers),
}

impl Block {
    pub fn primitive(&self) -> Primitive {
        match self {
            Self::Title(_) => Primitive::Title,
            Self::Kind(_) => Primitive::Kind,
            Self::Prose(_) => Primitive::Prose,
            Self::Media(_) => Primitive::Media,
            Self::Pictures(_) => Primitive::Pictures,
            Self::Sounds(_) => Primitive::Sounds,
            Self::Names(_) => Primitive::Names,
            Self::Map(_) => Primitive::Map,
            Self::Fact(_) => Primitive::Fact,
            Self::Series(_) => Primitive::Series,
            Self::Split(_) => Primitive::Split,
            Self::Scale(_) => Primitive::Scale,
            Self::Peers(_) => Primitive::Peers,
            Self::Network(_) | Self::Gallery(_) => Primitive::Network,
            Self::Succession(_) => Primitive::Succession,
            Self::Credits(_) => Primitive::Credits,
            Self::Score(_) => Primitive::Score,
            Self::Works(_) => Primitive::Works,
            Self::Timeline(_) => Primitive::Timeline,
            Self::Absent(_) => Primitive::Absent,
            Self::Provenance(_) => Primitive::Provenance,
            Self::Sitelinks(_) => Primitive::Sitelinks,
            Self::Identifiers(_) => Primitive::Identifiers,
        }
    }
}
