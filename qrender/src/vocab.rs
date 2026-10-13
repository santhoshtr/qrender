//! The closed vocabulary: every name that the configuration and the
//! composition can use. A name that is not here is a load error.
//!
//! Templates and slots belong to the design system: the composer can only
//! name what the CSS has.

use serde::{Deserialize, Serialize};

/// How the composer builds a spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpreadKind {
    /// The page head: title and description, banner, lead, picture, kinds.
    Identity,
    /// The listed properties, each by the shape of its data and its role.
    Facts,
    /// Share of the container, peers, series of the listed properties.
    Scale,
    /// A time axis of events and history spans.
    History,
    /// The Commons files that no spread took.
    Media,
    /// Everything left, collapsed.
    More,
    /// Web presence, curation, sources, identifiers, provenance.
    Evidence,
}

/// What a property is for, when its datatype does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Event,
    HistorySpan,
    Series,
    Crew,
    Score,
    Described,
    Billing,
    Relation,
    Tabular,
    ExtremePoint,
    Web,
    /// A holder property with the property of its office (P6 -> P1313).
    Office,
}

/// The roles of one property.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoleSet(u16);

impl RoleSet {
    pub fn insert(&mut self, role: Role) {
        self.0 |= 1 << role as u16;
    }

    pub fn has(self, role: Role) -> bool {
        self.0 & (1 << role as u16) != 0
    }
}

/// The kinds of composition block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Primitive {
    Title,
    Kind,
    Prose,
    Media,
    Map,
    Fact,
    Series,
    Scale,
    Peers,
    Network,
    Succession,
    Credits,
    Score,
    Works,
    Time,
    Absent,
    Provenance,
    Sitelinks,
    Identifiers,
}

/// A named arrangement of slots, defined by the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Template {
    Identity,
    LeadBeside,
    HeroPlusStack,
    Columns,
    Axis,
    FeatureRun,
    Sheet,
    Collapsed,
}

/// A named place in a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Slot {
    Head,
    Band,
    Lead,
    Hero,
    Kind,
    Stack,
    Gallery,
    Sheet,
    Axis,
    Feature,
    Run,
    Audio,
    Summary,
    Web,
    Projects,
    Identifiers,
    Absent,
}

/// How the item is compared with its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContainerMode {
    /// The item against a parent body or taxon.
    Scale,
    /// The item as a part of a whole (a city of its country).
    Share,
}

/// The end of a range that a qualifier value marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RangeEnd {
    Min,
    Max,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_set() {
        let mut roles = RoleSet::default();
        roles.insert(Role::Office);
        roles.insert(Role::Event);
        assert!(roles.has(Role::Office) && roles.has(Role::Event));
        assert!(!roles.has(Role::Relation));
    }
}
