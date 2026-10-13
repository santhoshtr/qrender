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

/// HTML: the template binding. The markup is templates/primitives/media.html.
#[derive(askama::Template)]
#[template(path = "primitives/media.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Media,
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

impl MediaRole {
    pub fn name(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Banner => "banner",
            Self::Feature => "feature",
            Self::Run => "run",
            Self::Stack => "stack",
            Self::Audio => "audio",
        }
    }
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

/// Markdown: the file, as a link to its page on Commons.
pub fn markdown(b: &Media, md: &mut crate::render::markdown::Markdown) {
    let crate::schema::ValueShown::File { file } = &b.statement.value else {
        return;
    };
    let page = format!(
        "https://commons.wikimedia.org/wiki/File:{}",
        file.replace(' ', "_")
    );
    let value = format!(
        "[{file}](<{page}>){}",
        crate::render::markdown::provenance_words(b.statement.provenance)
    );
    md.item(&md.property(b.statement.pid), &value);
}
