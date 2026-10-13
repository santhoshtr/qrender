//! What the fetcher needs to know from the composer's configuration: the
//! class layouts (to choose one and fetch its reverse statements), the
//! container properties, the qualifiers shown by a logo, and the concepts
//! that title the spreads. The `qrender` crate fills it from its layouts,
//! so this crate does not depend on `qrender`.

use crate::id::{Pid, Qid};

#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// In match order; the last one matches every item.
    pub layouts: Vec<LayoutMatch>,
    /// The container properties, in order of preference.
    pub containers: Vec<Pid>,
    /// Qualifiers whose value is shown by its logo (P154).
    pub qualifier_logos: Vec<Pid>,
    /// The items whose labels and aliases name spreads and their sections.
    pub concepts: Vec<Qid>,
}

#[derive(Debug, Clone)]
pub struct LayoutMatch {
    pub name: String,
    /// P31 values that choose this layout; empty matches every item.
    pub match_p31: Vec<Qid>,
    /// Properties of other items that point to the item (works).
    pub reverse: Vec<Pid>,
}

impl Settings {
    /// The first layout whose classes the item has; else the last one.
    pub fn layout(&self, classes: &[Qid]) -> Option<&LayoutMatch> {
        self.layouts
            .iter()
            .find(|l| l.match_p31.iter().any(|c| classes.contains(c)))
            .or_else(|| self.layouts.iter().find(|l| l.match_p31.is_empty()))
    }
}
