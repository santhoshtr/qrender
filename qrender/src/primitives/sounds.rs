//! Sounds: the recordings of one property, such as the pronunciation of
//! the name in several languages, are one block with one player and a
//! choice of language. The composer groups them and names the language of
//! each recording; the renderer draws the choice.

use content::{EntityId, Pid, Qid};
use schemars::JsonSchema;
use serde::Serialize;

use super::media::{self, Media};
use crate::schema::{Block, MediaKind, QualifierPart};
use crate::wikidata::LANGUAGE;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Sounds {
    pub pid: Pid,
    pub sounds: Vec<Sound>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Sound {
    #[serde(flatten)]
    pub media: Media,
    /// The language of the recording (its "language of work or name"
    /// qualifier): the name of its choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Qid>,
}

/// HTML: the template binding. The markup is templates/primitives/sounds.html.
#[derive(askama::Template)]
#[template(path = "primitives/sounds.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Sounds,
}

/// Group the recordings of one property that follow each other in a slot.
/// A single recording stays a media block.
pub fn group(blocks: Vec<Block>) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for block in blocks {
        let Block::Media(m) = block else {
            out.push(block);
            continue;
        };
        if m.picture.kind != MediaKind::Audio {
            out.push(Block::Media(m));
            continue;
        }
        let pid = m.statement.pid;
        match out.last_mut() {
            Some(Block::Sounds(s)) if s.pid == pid => s.sounds.push(sound(m)),
            Some(Block::Media(prev))
                if prev.picture.kind == MediaKind::Audio && prev.statement.pid == pid =>
            {
                let first = sound(prev.clone());
                *out.last_mut().expect("a last block") = Block::Sounds(Sounds {
                    pid,
                    sounds: vec![first, sound(m)],
                });
            }
            _ => out.push(Block::Media(m)),
        }
    }
    out
}

fn sound(media: Media) -> Sound {
    let language = media
        .statement
        .qualifiers
        .iter()
        .find_map(|q| match q.value {
            QualifierPart::Item {
                item: EntityId::Item(item),
            } if q.pid == *LANGUAGE => Some(item),
            _ => None,
        });
    Sound { media, language }
}

/// Markdown: each recording.
pub fn markdown(b: &Sounds, md: &mut crate::render::markdown::Markdown) {
    for s in &b.sounds {
        media::markdown(&s.media, md);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use content::Statement;

    use super::*;
    use crate::compose::Composer;
    use crate::compose::test_content::content;
    use crate::config::Config;

    fn media_of(c: &Composer, s: &Statement) -> Block {
        Block::Media(media::build(c, s, media::MediaRole::Stack, None))
    }

    #[test]
    fn recordings_of_one_property_are_one_block() {
        let audio = |pid: &str, f: &str, lang: Option<&str>| {
            let qualifiers: Vec<_> = lang
                .map(|l| json!({"pid": "P407", "value": {"type": "entity", "entity": l}}))
                .into_iter()
                .collect();
            json!({"pid": pid, "value": {"type": "CommonsMedia", "file": f}, "qualifiers": qualifiers})
        };
        let c = content(
            json!([
                audio("P443", "De.ogg", Some("Q188")),
                audio("P443", "Ro.ogg", Some("Q7913")),
                audio("P443", "X.ogg", None),
                audio("P51", "Anthem.ogg", None),
                {"pid": "P18", "value": {"type": "CommonsMedia", "file": "A.jpg"}},
                audio("P443", "It.ogg", Some("Q652")),
            ]),
            json!({}),
        );
        let k = Composer::new(&c, Config::embedded());
        let blocks = c.statements.iter().map(|s| media_of(&k, s)).collect();
        let grouped = group(blocks);
        let shape: Vec<String> = grouped
            .iter()
            .map(|b| match b {
                Block::Sounds(s) => format!(
                    "sounds {} {:?}",
                    s.sounds.len(),
                    s.sounds
                        .iter()
                        .map(|x| x.language.map(|q| q.to_string()))
                        .collect::<Vec<_>>()
                ),
                other => format!("{:?}", other.primitive()),
            })
            .collect();
        assert_eq!(
            shape,
            [
                r#"sounds 3 [Some("Q188"), Some("Q7913"), None]"#,
                "Media", // one recording of another property
                "Media", // a picture
                "Media", // a recording alone
            ]
        );
    }
}
