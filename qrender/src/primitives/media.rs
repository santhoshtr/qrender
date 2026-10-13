//! Media: one Commons file of the item (a picture, a sound, a video, a 3D
//! model), with the statement that holds it.

use content::{Statement, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::{PictureFacts, StatementRef};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Media {
    pub role: MediaRole,
    #[serde(flatten)]
    pub statement: StatementRef,
    #[serde(flatten)]
    pub picture: PictureFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// Where the file is shown: the design system gives each role its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaRole {
    /// The head picture of the page.
    Identity,
    /// The page banner, across the head.
    Banner,
    /// The best photograph of the media spread.
    Feature,
    /// One of the media spread's run of pictures.
    Run,
    /// A file of a facts spread.
    Stack,
    Audio,
}

/// `statement` holds a Commons file.
pub fn build(c: &Composer, statement: &Statement, role: MediaRole, rule: Option<&str>) -> Media {
    let Value::CommonsMedia { file } = &statement.value else {
        unreachable!("a media block is made of a CommonsMedia statement")
    };
    Media {
        role,
        statement: c.statement(statement),
        picture: c.picture(file, false).facts,
        rule: rule.map(str::to_string),
    }
}
