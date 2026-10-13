//! The media spread: the Commons files that no other spread took. The best
//! photograph is the feature; the rest is a run, photographs first.

use content::{Datatype, Statement};
use indexmap::IndexMap;

use crate::compose::Composer;
use crate::compose::picture::media_kind;
use crate::config::SpreadSpec;
use crate::primitives::media::{self, MediaRole};
use crate::schema::{Block, MediaKind};
use crate::vocab::{Slot, Template};

pub fn build(c: &mut Composer, spec: &SpreadSpec) -> super::Built {
    let files: Vec<&Statement> = c
        .content
        .statements
        .iter()
        .filter(|s| s.value.datatype() == Datatype::CommonsMedia && !c.is_claimed(s))
        .collect();
    let files = c.take(files);
    if files.is_empty() {
        return Err(Vec::new());
    }
    // The first file with the best score.
    let feature = files
        .iter()
        .filter_map(|s| Some((c.feature_score(s)?, *s)))
        .reduce(|best, x| if x.0 > best.0 { x } else { best })
        .map(|(_, s)| s);
    let kind = |s: &Statement| match &s.value {
        content::Value::CommonsMedia { file } => media_kind(file),
        _ => MediaKind::Photo,
    };
    let mut run: Vec<_> = files
        .iter()
        .filter(|s| !feature.is_some_and(|f| std::ptr::eq(f, **s)) && kind(s) != MediaKind::Audio)
        .map(|s| media::build(c, s, MediaRole::Run, None))
        .collect();
    let audio: Vec<_> = files
        .iter()
        .filter(|s| kind(s) == MediaKind::Audio)
        .map(|s| Block::Media(media::build(c, s, MediaRole::Audio, None)))
        .collect();
    let order = |k: MediaKind| match k {
        MediaKind::Photo => 0,
        MediaKind::Video => 1,
        MediaKind::Model3d => 2,
        MediaKind::Drawing => 3,
        MediaKind::Audio => 4,
    };
    run.sort_by(|a, b| {
        (order(a.picture.kind), -a.picture.megapixels.unwrap_or(0.0))
            .partial_cmp(&(order(b.picture.kind), -b.picture.megapixels.unwrap_or(0.0)))
            .expect("megapixels are numbers")
    });
    let mut slots = IndexMap::new();
    if let Some(feature) = feature {
        let block = media::build(c, feature, MediaRole::Feature, None);
        slots.insert(Slot::Feature, vec![Block::Media(block)]);
    }
    if !run.is_empty() {
        slots.insert(Slot::Run, run.into_iter().map(Block::Media).collect());
    }
    if !audio.is_empty() {
        slots.insert(Slot::Audio, audio);
    }
    Ok(c.spread(spec, slots, Template::FeatureRun, Vec::new()))
}
