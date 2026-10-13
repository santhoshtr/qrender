//! Absent: a property that most peers have and the item lacks. Shown only
//! when the peer set is large enough for "most" to mean something.

use content::{Pid, Qid};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Absent {
    pub pid: Pid,
    pub peers_with_it: u32,
    pub peers: usize,
    pub class: Qid,
    /// The datatype of the property, as Wikibase names it ("wikibase-item").
    pub datatype: String,
}

/// HTML: the template binding. The markup is templates/primitives/absent.html.
#[derive(askama::Template)]
#[template(path = "primitives/absent.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Absent,
}

pub fn build(c: &Composer) -> Vec<Absent> {
    let Some(peers) = &c.content.peers else {
        return Vec::new();
    };
    let count = peers.members.len();
    if count < c.config.layouts.thresholds.absence_min_peers {
        return Vec::new();
    }
    peers
        .missing
        .iter()
        .map(|(pid, datatype)| Absent {
            pid: *pid,
            peers_with_it: peers.frequency.get(pid).copied().unwrap_or(0),
            peers: count,
            class: peers.class,
            datatype: datatype.clone(),
        })
        .collect()
}

/// Markdown: what most peers have and the item lacks.
pub fn markdown(b: &Absent, md: &mut crate::render::markdown::Markdown) {
    let value = format!(
        "none ({} of {} {} have it)",
        b.peers_with_it,
        b.peers,
        md.label(content::EntityId::Item(b.class))
    );
    md.item(&md.property(b.pid), &value);
}
