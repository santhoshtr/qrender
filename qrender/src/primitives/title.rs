//! Title: the label and the description of the item, at the head of the
//! page. The composition holds only the item; localize gives the text.

use content::Qid;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Title {
    pub item: Qid,
}
