//! Works: the works by the item (a person), from statements on the works
//! (author, director, composer). Pictures when most have one, else a list.

use content::{Pid, Qid};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::Picture;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Works {
    /// The properties of the works that point to the item.
    pub pids: Vec<Pid>,
    pub form: WorksForm,
    pub values: Vec<Work>,
    pub shown_count: usize,
    pub priority: f64,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorksForm {
    Run,
    List,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Work {
    pub item: Qid,
    /// The properties by which this work points to the item.
    pub via: Vec<Pid>,
    pub year: Option<i64>,
    pub image: Option<Picture>,
}

/// The works that point to the item by one of `reverse`, the spread's
/// reverse properties. None when there are none.
pub fn build(c: &Composer, reverse: &[Pid]) -> Option<Works> {
    let works: Vec<_> = c
        .content
        .reverse
        .iter()
        .filter(|w| w.pids.iter().any(|p| reverse.contains(p)))
        .collect();
    if works.is_empty() {
        return None;
    }
    let values: Vec<Work> = works
        .iter()
        .map(|w| Work {
            item: w.item,
            via: w.pids.clone(),
            year: w.year,
            image: w.image.as_ref().map(|f| c.picture(f, false)),
        })
        .collect();
    let thresholds = &c.config.layouts.thresholds;
    // The same rule as for a set of item values.
    let pictured = values.iter().filter(|v| v.image.is_some()).count();
    let form = if pictured as f64 >= thresholds.run_pictured * values.len() as f64 {
        WorksForm::Run
    } else {
        WorksForm::List
    };
    let mut pids: Vec<Pid> = works.iter().flat_map(|w| w.pids.iter().copied()).collect();
    // As text, as v3 sorts them ("P162" before "P50").
    pids.sort_by_key(|p| p.to_string());
    pids.dedup();
    Some(Works {
        pids,
        form,
        shown_count: values.len().min(thresholds.list_shown),
        priority: 1.0 + (1.0 + values.len() as f64).log2(),
        values,
    })
}
