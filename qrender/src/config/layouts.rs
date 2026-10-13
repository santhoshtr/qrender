//! The types of `layouts.toml`: class layouts, property roles, thresholds
//! and the other judgment that the composer cannot derive from the data.

use content::{Pid, Qid};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::vocab::{ContainerMode, RangeEnd, Role, SpreadKind};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layouts {
    /// In match order: the first layout whose `match_p31` has a P31 value of
    /// the item is its layout. `generic` matches every item.
    pub class_layouts: IndexMap<String, ClassLayout>,
    pub roles: Roles,
    pub thresholds: Thresholds,
    /// The container properties, in order of preference, and how the item
    /// is compared with the container.
    pub containers: IndexMap<Pid, ContainerMode>,
    pub compare: Compare,
    pub media: Media,
    pub evidence: Evidence,
    /// "... of topic" properties, and the spread that takes each one.
    pub topics: IndexMap<Pid, String>,
    pub range_ends: IndexMap<Qid, RangeEnd>,
    pub provenance: Provenance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassLayout {
    pub match_p31: Vec<Qid>,
    pub spreads: Vec<SpreadSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadSpec {
    pub id: String,
    pub kind: SpreadKind,
    /// The item whose label titles the spread.
    pub concept: Option<Qid>,
    /// Items whose label or alias names an article section for the spread.
    #[serde(default)]
    pub prose: Vec<Qid>,
    #[serde(default)]
    pub pids: Vec<Pid>,
    /// Properties of other items that point to this one (works by a person).
    #[serde(default)]
    pub reverse: Vec<Pid>,
    /// Also take any item property whose values are mostly pictured.
    #[serde(default)]
    pub pictured: bool,
}

/// The property lists of each role, and the office table.
#[derive(Debug, Deserialize)]
pub struct Roles {
    /// Holder property -> the property that names the office.
    pub office: IndexMap<Pid, Pid>,
    #[serde(flatten)]
    pub lists: IndexMap<Role, Vec<Pid>>,
}

impl Roles {
    pub fn list(&self, role: Role) -> &[Pid] {
        self.lists.get(&role).map_or(&[], Vec::as_slice)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    /// Fewer distinct years than this: no time axis.
    pub time_points: usize,
    /// Fewer peers with a value than this: no peer lineup.
    pub peers: usize,
    /// A set of values is pictured when this share of them has pictures.
    pub run_pictured: f64,
    /// Pictures shown in a set before "+N".
    pub set_max: usize,
    /// A property that this share of the peers has is "absent" when the
    /// item lacks it.
    pub absence: f64,
    /// Fewer peers than this: no absence is shown.
    pub absence_min_peers: usize,
    /// Classes shown in the identity chain.
    pub chain_max: usize,
    /// Values shown in a long list before "+N".
    pub list_shown: usize,
    /// Predecessors shown before "+N".
    pub succession_shown: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compare {
    /// Quantities that describe the thing itself, so that a comparison with
    /// the container or the peers means something.
    pub pids: Vec<Pid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Media {
    /// Photographs of the subject itself, in order of preference.
    pub views: Vec<Pid>,
    pub min_megapixels: f64,
    /// Qualifiers whose value is shown by its logo.
    pub qualifier_logos: Vec<Pid>,
    /// Commons assessment category -> weight for the feature picture.
    pub assessments: IndexMap<String, u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Curation metadata: real data, but not what a reader reads.
    pub pids: Vec<Pid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Reference properties that are not a source (P143 imported from).
    pub not_a_source: Vec<Pid>,
}
