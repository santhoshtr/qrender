//! Media: one Commons file of the item (a picture, a sound, a video, a 3D
//! model), with the statement that holds it.

use content::{Pid, Statement, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::compose::Composer;
use crate::schema::{Block, PictureFacts, StatementRef};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Media {
    pub role: MediaRole,
    #[serde(flatten)]
    pub statement: StatementRef,
    #[serde(flatten)]
    pub picture: PictureFacts,
    /// The item that the file pictures, from the item property of the file
    /// property (the flag of Germany, for the flag image): the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<StatementRef>,
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
        subject: None,
        rule: rule.map(str::to_string),
    }
}

/// At the place of an item property that a file property pictures (flag,
/// flag image): its current value and the current file of the file
/// property are one picture, titled by the item. Claims both. None when
/// either has no current value; the values are then shown as they are.
pub fn pictured_at<'a>(c: &mut Composer<'a>, pid: Pid) -> Option<Block> {
    let file_pid = *c.config.layouts.roles.picture.get(&pid)?;
    let item = now(c, pid).filter(|s| s.value.item().is_some())?;
    let file = now(c, file_pid)?;
    c.take([item, file]);
    let mut media = build(c, file, MediaRole::Stack, None);
    media.subject = Some(c.statement(item));
    Some(Block::Media(media))
}

/// The first current value of a property that has not ended.
fn now<'a>(c: &Composer<'a>, pid: Pid) -> Option<&'a Statement> {
    c.current(c.free(pid))
        .into_iter()
        .find(|s| !Composer::ended(s))
}

/// Markdown: the file, as a link to its page on Commons.
pub fn markdown(b: &Media, md: &mut crate::render::markdown::Markdown) {
    // The picture of the lead of the article takes the place of the head
    // picture, as on the HTML page.
    let lead = md.view.lead.as_ref();
    if b.role == MediaRole::Identity
        && let Some((a, f)) = lead.and_then(|a| Some((a, a.picture.as_ref()?)))
        && let Some(i) = f.images.first()
    {
        let page = format!(
            "https://commons.wikimedia.org/wiki/File:{}",
            i.file.replace(' ', "_")
        );
        let caption = match crate::render::markdown::plain(&f.caption) {
            c if c.is_empty() => c,
            c => format!(" — {c}"),
        };
        let value = format!("[{}](<{page}>){caption} (from {})", i.file, a.host);
        md.item(&md.property(b.statement.pid), &value);
        return;
    }
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
    match &b.subject {
        Some(subject) => {
            let value = format!("{} — {value}", md.value(subject));
            md.item(&md.property(subject.pid), &value);
        }
        None => md.item(&md.property(b.statement.pid), &value),
    }
}

#[cfg(test)]
mod picture_tests {
    use serde_json::json;

    use super::*;
    use crate::compose::test_content::{content, item};
    use crate::config::Config;

    #[test]
    fn an_item_and_its_picture_are_one() {
        let file = |f: &str, ended: bool| {
            let qualifiers: Vec<_> = ended
                .then(|| json!({"pid": "P582", "value": {"type": "time", "time": "1949-01-01T00:00:00Z"}}))
                .into_iter()
                .collect();
            json!({"pid": "P41", "value": {"type": "CommonsMedia", "file": f}, "qualifiers": qualifiers})
        };
        let c = content(
            json!([
                item("P163", "Q48160"),
                file("Flag of Germany (1867–1918).svg", true),
                file("Flag of Germany.svg", false)
            ]),
            json!({}),
        );
        let mut k = Composer::new(&c, Config::embedded());
        let Some(Block::Media(m)) = pictured_at(&mut k, "P163".parse().unwrap()) else {
            panic!("one picture");
        };
        assert_eq!(
            m.statement.value,
            crate::schema::ValueShown::File {
                file: "Flag of Germany.svg".into()
            }
        );
        assert_eq!(
            m.subject.unwrap().value.item(),
            Some("Q48160".parse().unwrap())
        );
        // The flag that ended is left for the other blocks.
        assert_eq!(k.free("P41".parse().unwrap()).len(), 1);
        // A property without a picture property: nothing.
        assert!(pictured_at(&mut k, "P41".parse().unwrap()).is_none());
    }
}
