//! Pictures: single pictures that follow each other in a slot form one
//! justified run. The composer says so; a renderer only draws it.

use schemars::JsonSchema;
use serde::Serialize;

use super::media::{self, Media};
use crate::schema::{Block, MediaKind};
use crate::vocab::Slot;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Pictures {
    pub pictures: Vec<Media>,
}

/// HTML: the template binding. The markup is templates/primitives/pictures.html.
#[derive(askama::Template)]
#[template(path = "primitives/pictures.html")]
pub struct Html<'a> {
    pub ui: &'a crate::render::ui::Ui<'a>,
    pub b: &'a Pictures,
}

/// Group the pictures that follow each other in a slot. Not in the slots
/// that are a run or one picture already, nor in a gallery or a sheet.
pub fn group(slot: Slot, blocks: Vec<Block>) -> Vec<Block> {
    let keep = matches!(
        slot,
        Slot::Run
            | Slot::Feature
            | Slot::Band
            | Slot::Hero
            | Slot::Gallery
            | Slot::Sheet
            | Slot::Absent
            | Slot::Web
    );
    if keep {
        return blocks;
    }
    let mut out: Vec<Block> = Vec::new();
    for block in blocks {
        match block {
            Block::Media(m) if m.picture.kind != MediaKind::Audio => {
                if let Some(Block::Pictures(run)) = out.last_mut() {
                    run.pictures.push(m);
                } else {
                    out.push(Block::Pictures(Pictures { pictures: vec![m] }));
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Markdown: each picture.
pub fn markdown(b: &Pictures, md: &mut crate::render::markdown::Markdown) {
    for m in &b.pictures {
        media::markdown(m, md);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::compose::Composer;
    use crate::compose::test_content::content;
    use crate::config::Config;
    use crate::primitives::media::MediaRole;

    #[test]
    fn consecutive_pictures_form_a_run() {
        let file = |f: &str| json!({"pid": "P242", "value": {"type": "CommonsMedia", "file": f}});
        let c = content(
            json!([file("A.jpg"), file("B.png"), file("C.ogg"), file("D.jpg")]),
            json!({}),
        );
        let k = Composer::new(&c, Config::embedded());
        let blocks = || {
            c.statements
                .iter()
                .map(|s| Block::Media(media::build(&k, s, MediaRole::Stack, None)))
                .collect::<Vec<_>>()
        };
        let kinds = |bs: &[Block]| {
            bs.iter()
                .map(|b| match b {
                    Block::Pictures(p) => format!("pictures{}", p.pictures.len()),
                    other => format!("{:?}", other.primitive()),
                })
                .collect::<Vec<_>>()
        };
        // Two pictures, a sound (not a picture), a picture.
        assert_eq!(
            kinds(&group(Slot::Stack, blocks())),
            ["pictures2", "Media", "pictures1"]
        );
        // A run slot is a run already.
        assert_eq!(
            kinds(&group(Slot::Run, blocks())),
            ["Media", "Media", "Media", "Media"]
        );
    }
}
