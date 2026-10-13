//! The closed vocabulary: every name that the configuration and the
//! composition can use. A name that is not here is a load error.
//!
//! Templates and slots belong to the design system: the composer can only
//! name what the CSS has.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How the composer builds a spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
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

/// The dimension of the data of a spread. The design system gives each
/// one its colour; colour means nothing else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Dimension {
    Place,
    Measures,
    People,
    Time,
    Names,
    Recognition,
    Media,
    /// The page head, the rest, the evidence: no colour of their own.
    #[default]
    Neutral,
}

impl Dimension {
    /// The name the design system uses (`data-dimension`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Measures => "measures",
            Self::People => "people",
            Self::Time => "time",
            Self::Names => "names",
            Self::Recognition => "recognition",
            Self::Media => "media",
            Self::Neutral => "neutral",
        }
    }
}

/// What a property is for, when its datatype does not say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Primitive {
    Title,
    Kind,
    Prose,
    Media,
    /// Single pictures that follow each other in a slot: one run.
    Pictures,
    /// The recordings of one property (pronunciations): one player.
    Sounds,
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

impl Primitive {
    /// Every primitive, in the order of their CSS in the stylesheet and of
    /// their scripts in a page.
    pub const ALL: [Primitive; 21] = [
        Self::Title,
        Self::Kind,
        Self::Prose,
        Self::Media,
        Self::Pictures,
        Self::Sounds,
        Self::Map,
        Self::Fact,
        Self::Series,
        Self::Scale,
        Self::Peers,
        Self::Network,
        Self::Succession,
        Self::Credits,
        Self::Score,
        Self::Works,
        Self::Time,
        Self::Absent,
        Self::Provenance,
        Self::Sitelinks,
        Self::Identifiers,
    ];
}

/// A named arrangement of slots, defined by the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
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

impl Template {
    /// The name the design system uses (`t-<name>`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::LeadBeside => "lead-beside",
            Self::HeroPlusStack => "hero-plus-stack",
            Self::Columns => "columns",
            Self::Axis => "axis",
            Self::FeatureRun => "feature-run",
            Self::Sheet => "sheet",
            Self::Collapsed => "collapsed",
        }
    }
}

impl Slot {
    /// The name the design system uses (`data-slot`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Band => "band",
            Self::Lead => "lead",
            Self::Hero => "hero",
            Self::Kind => "kind",
            Self::Stack => "stack",
            Self::Gallery => "gallery",
            Self::Sheet => "sheet",
            Self::Axis => "axis",
            Self::Feature => "feature",
            Self::Run => "run",
            Self::Audio => "audio",
            Self::Summary => "summary",
            Self::Web => "web",
            Self::Projects => "projects",
            Self::Identifiers => "identifiers",
            Self::Absent => "absent",
        }
    }
}

/// How the item is compared with its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContainerMode {
    /// The item against a parent body or taxon.
    Scale,
    /// The item as a part of a whole (a city of its country).
    Share,
}

/// The end of a range that a qualifier value marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RangeEnd {
    Min,
    Max,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_primitives_are_listed() {
        // The variant names, from the generated schema of the enum.
        let schema = serde_json::to_string(&schemars::schema_for!(Primitive)).unwrap();
        let mut names: Vec<String> =
            serde_json::from_str::<serde_json::Value>(&schema).unwrap()["oneOf"]
                .as_array()
                .map(|all| {
                    // Documented variants have a "const" each; the others share
                    // one "enum".
                    all.iter()
                        .flat_map(|v| {
                            let named = v["enum"].as_array().cloned().unwrap_or_default();
                            named.into_iter().chain(v.get("const").cloned())
                        })
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
        names.sort();
        let mut listed: Vec<String> = Primitive::ALL
            .iter()
            .map(|p| {
                serde_json::to_value(p)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        listed.sort();
        assert!(!names.is_empty());
        assert_eq!(listed, names, "Primitive::ALL must list every variant once");
    }

    #[test]
    fn role_set() {
        let mut roles = RoleSet::default();
        roles.insert(Role::Office);
        roles.insert(Role::Event);
        assert!(roles.has(Role::Office) && roles.has(Role::Event));
        assert!(!roles.has(Role::Relation));
    }
}
