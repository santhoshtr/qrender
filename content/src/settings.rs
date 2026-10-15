//! What the fetcher needs to know from the composer's configuration: the
//! class layouts (to choose one and fetch its reverse statements), the
//! container properties, the qualifiers shown by a logo, and the concepts
//! that title the spreads. The `qrender` crate fills it from its layouts,
//! so this crate does not depend on `qrender`.

use serde::Serialize;

use crate::id::{Pid, Qid};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Settings {
    /// In match order; the last one matches every item.
    pub layouts: Vec<LayoutMatch>,
    /// The container properties, in order of preference.
    pub containers: Vec<Pid>,
    /// Qualifiers whose value is shown by its logo (P154).
    pub qualifier_logos: Vec<Pid>,
    /// The items whose labels and aliases name spreads and their sections.
    pub concepts: Vec<Qid>,
    /// Properties whose item values are events on a timeline: their own
    /// dates are fetched.
    pub timeline: Vec<Pid>,
    /// Properties whose item values are holidays: their days are fetched.
    pub calendar: Vec<Pid>,
    /// The web properties (sites and accounts): their formatter URLs are
    /// fetched.
    pub web: Vec<Pid>,
    /// The platform of a web property (X username: X): its logo is
    /// fetched.
    pub platforms: Vec<(Pid, Qid)>,
    /// An item property and the property of its words (motto, motto
    /// text): the names of the item values are fetched, to match them with
    /// the texts.
    pub texts: Vec<(Pid, Pid)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutMatch {
    pub name: String,
    /// P31 values that choose this layout; empty matches every item.
    pub match_p31: Vec<Qid>,
    /// Properties of other items that point to the item (works).
    pub reverse: Vec<Pid>,
}

impl Settings {
    /// A hash of the settings: part of the cache keys, so that a change of
    /// the configuration (a new concept, a new layout) does not read the
    /// parts fetched with the old one. FNV-1a, 64 bit: the same across
    /// Rust releases, so a key made by one build is valid for the next.
    pub fn fingerprint(&self) -> u64 {
        let json = serde_json::to_string(self).expect("settings serialize");
        json.bytes().fold(0xcbf2_9ce4_8422_2325, |hash: u64, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    /// The first layout whose classes the item has; else the last one.
    pub fn layout(&self, classes: &[Qid]) -> Option<&LayoutMatch> {
        self.layouts
            .iter()
            .find(|l| l.match_p31.iter().any(|c| classes.contains(c)))
            .or_else(|| self.layouts.iter().find(|l| l.match_p31.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_follows_the_settings() {
        let a = Settings::default();
        let mut b = Settings::default();
        assert_eq!(a.fingerprint(), b.fingerprint());
        // A new concept (a split title) is a new fingerprint.
        b.concepts.push("Q290".parse().unwrap());
        assert_ne!(a.fingerprint(), b.fingerprint());
        // FNV-1a of the JSON of the settings: fixed, so a key from one
        // build is valid in the next.
        assert_eq!(a.fingerprint(), 0xd5b2_378e_0bb2_46cb);
    }
}
