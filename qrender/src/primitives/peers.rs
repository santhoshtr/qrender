//! Peers: the item in a lineup of comparable items, by one quantity.

use content::{Pid, Qid};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::round_to;

/// Members shown in the lineup; the item is always shown too.
const SHOWN: usize = 8;
/// Lineups in one spread.
const LINEUPS: usize = 2;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Peers {
    pub pid: Pid,
    /// The class of the peer set.
    pub class: Qid,
    /// The container that the peer set shares, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Qid>,
    /// The item's place, from the largest.
    pub rank: usize,
    pub of: usize,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Member {
    pub item: Qid,
    pub amount: f64,
    /// A share of the largest amount.
    pub fraction: f64,
    #[serde(rename = "self")]
    pub is_self: bool,
    pub shown: bool,
}

/// A lineup for each comparable quantity that the item and at least
/// `thresholds.peers` peers have, largest first; at most two.
pub fn build(c: &Composer) -> Vec<Peers> {
    let Some(peers) = &c.content.peers else {
        return Vec::new();
    };
    let qid = c.content.qid;
    let own = c.content.quantities.get(&qid).map(|q| &q.values);
    let mut out = Vec::new();
    for pid in &c.config.layouts.compare.pids {
        let mut members: Vec<(Qid, f64)> = peers
            .quantities
            .iter()
            .filter_map(|(q, r)| Some((*q, r.values.get(pid)?.amount)))
            .collect();
        let Some(mine) = own.and_then(|o| o.get(pid)) else {
            continue;
        };
        if members.len() < c.config.layouts.thresholds.peers {
            continue;
        }
        if !members.iter().any(|(q, _)| *q == qid) {
            members.push((qid, mine.amount));
        }
        // Largest first; the sort is stable, so ties keep their order.
        members.sort_by(|a, b| b.1.total_cmp(&a.1));
        let top = members[0].1;
        let rank = members
            .iter()
            .position(|(q, _)| *q == qid)
            .expect("the item is a member")
            + 1;
        out.push(Peers {
            pid: *pid,
            class: peers.class,
            container: peers.container,
            rank,
            of: members.len(),
            members: members
                .iter()
                .enumerate()
                .map(|(i, &(item, amount))| Member {
                    item,
                    amount,
                    fraction: if top == 0.0 {
                        0.0
                    } else {
                        round_to(amount / top, 4)
                    },
                    is_self: item == qid,
                    shown: i < SHOWN || item == qid,
                })
                .collect(),
        });
    }
    out.truncate(LINEUPS);
    out
}
