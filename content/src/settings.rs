//! What the fetcher needs to know from the composer's configuration: the
//! class layouts (to choose one and fetch its reverse statements), the
//! container properties, the qualifiers shown by a logo, and the concepts
//! that title the spreads; and the citation templates (how a note of an
//! article becomes a typed citation). The `qrender` crate fills it from
//! its configuration, so this crate does not depend on `qrender`.

use serde::{Deserialize, Serialize};

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
    /// The "... of topic" properties: the Wikipedia articles of their item
    /// values are fetched.
    pub topics: Vec<Pid>,
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
    /// The families of citation templates. A template that is in none is
    /// read from its COinS data.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<CitationStyle>,
}

/// A family of citation templates that share their parameters (Citation
/// Style 1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationStyle {
    pub name: String,
    /// The template names of each kind.
    pub templates: ByKind,
    /// The classes of the `<cite>` that the templates of each kind make
    /// (`citation web`): the kind of a citation that has only COinS data.
    pub classes: ByKind,
    pub parameters: Parameters,
}

/// Names for each kind of citation that a template makes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByKind {
    #[serde(default)]
    pub web: Vec<String>,
    #[serde(default)]
    pub news: Vec<String>,
    #[serde(default)]
    pub journal: Vec<String>,
    #[serde(default)]
    pub book: Vec<String>,
    #[serde(default)]
    pub magazine: Vec<String>,
    #[serde(default)]
    pub report: Vec<String>,
    /// An entry in an encyclopedia or a dictionary.
    #[serde(default)]
    pub entry: Vec<String>,
}

/// The parameters of each field, the first one that has a value first.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub title: Vec<String>,
    pub url: Vec<String>,
    /// The work that contains the source: a site, a newspaper, a journal.
    pub container: Vec<String>,
    pub publisher: Vec<String>,
    pub date: Vec<String>,
    pub accessed: Vec<String>,
    pub language: Vec<String>,
    pub volume: Vec<String>,
    pub issue: Vec<String>,
    pub pages: Vec<String>,
    pub place: Vec<String>,
    pub authors: AuthorParameters,
    pub archive: ArchiveParameters,
    pub ids: IdParameters,
}

/// The parameters of the authors. "{n}" in a name is the number of the
/// author: empty or 1 for the first, 2 for the second, and so on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorParameters {
    pub family: Vec<String>,
    pub given: Vec<String>,
    /// A name as one text.
    pub name: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveParameters {
    pub url: Vec<String>,
    pub date: Vec<String>,
    /// The state of the original URL: live, dead, usurped, unfit.
    pub status: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdParameters {
    pub doi: Vec<String>,
    pub isbn: Vec<String>,
    pub issn: Vec<String>,
    pub oclc: Vec<String>,
    pub jstor: Vec<String>,
    pub s2cid: Vec<String>,
    pub bibcode: Vec<String>,
    pub hdl: Vec<String>,
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
        assert_eq!(a.fingerprint(), 0xad09_f105_b0b7_ee5d);
    }
}
