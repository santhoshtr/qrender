//! Pictures: their kind and intrinsic facts, and the choice of the
//! feature picture.

use content::{Statement, Value};

use super::Composer;
use crate::schema::{Credit, MediaKind, Picture, round_to};

const AUDIO: [&str; 6] = [".ogg", ".oga", ".wav", ".mp3", ".flac", ".opus"];
const VIDEO: [&str; 3] = [".webm", ".ogv", ".mp4"];

/// The kind of a media file, from its name.
pub fn media_kind(file: &str) -> MediaKind {
    let name = file.to_lowercase();
    let ends = |exts: &[&str]| exts.iter().any(|e| name.ends_with(e));
    if ends(&AUDIO) {
        MediaKind::Audio
    } else if ends(&VIDEO) {
        MediaKind::Video
    } else if name.ends_with(".stl") {
        MediaKind::Model3d
    } else if name.ends_with(".svg") {
        MediaKind::Drawing
    } else {
        MediaKind::Photo
    }
}

impl Composer<'_> {
    /// A picture with its intrinsic facts from Commons. A logo is a drawing,
    /// whatever its file format: it is shown whole.
    pub fn picture(&self, file: &str, logo: bool) -> Picture {
        let info = self.content.commons.get(file);
        let mut kind = media_kind(file);
        if logo && kind == MediaKind::Photo {
            kind = MediaKind::Drawing;
        }
        let size = info.and_then(|i| match (i.width, i.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((f64::from(w), f64::from(h))),
            _ => None,
        });
        let plays = matches!(
            kind,
            MediaKind::Video | MediaKind::Model3d | MediaKind::Audio
        );
        Picture {
            file: file.to_string(),
            kind,
            aspect: size.map(|(w, h)| round_to(w / h, 4)),
            megapixels: size.map(|(w, h)| round_to(w * h / 1e6, 2)),
            bytes: info.filter(|i| plays && i.bytes > 0).map(|i| i.bytes),
            url: info
                .filter(|i| plays && !i.url.is_empty())
                .map(|i| i.url.clone()),
            assessed: info.map(|i| i.assessments.clone()).unwrap_or_default(),
            credit: Credit {
                artist: info.map(|i| i.artist.clone()).unwrap_or_default(),
                license: info.map(|i| i.license.clone()).unwrap_or_default(),
            },
        }
    }

    /// The picture of an item value. A class value has none: the picture of
    /// "lawyer" shows one lawyer, not the person the statement is about.
    pub fn picture_of(&self, statement: &Statement) -> Option<Picture> {
        let item = statement.value.item()?;
        if self.is_class(item) {
            return None;
        }
        let image = statement.value_image.as_ref()?;
        Some(self.picture(&image.file, image.logo))
    }

    /// The statement's value has a picture.
    pub fn is_pictured(&self, statement: &Statement) -> bool {
        statement.value_image.is_some() && !statement.value.item().is_some_and(|q| self.is_class(q))
    }

    /// How good a media statement is as the feature picture of the page,
    /// or None when it cannot be one. A photograph of at least
    /// `min_megapixels`, assessed on Commons or of a preferred view
    /// property; better with a higher assessment, a more preferred view and
    /// more pixels.
    pub fn feature_score(&self, statement: &Statement) -> Option<f64> {
        let Value::CommonsMedia { file } = &statement.value else {
            return None;
        };
        let media = &self.config.layouts.media;
        let picture = self.picture(file, false);
        let megapixels = picture.megapixels.unwrap_or(0.0);
        if picture.kind != MediaKind::Photo || megapixels < media.min_megapixels {
            return None;
        }
        let assessed: u32 = picture
            .assessed
            .iter()
            .filter_map(|a| media.assessments.get(a))
            .sum();
        let view = media.views.iter().position(|p| *p == statement.pid);
        if assessed == 0 && view.is_none() {
            return None;
        }
        let views = media.views.len() as f64;
        let preference = view.map_or(0.0, |i| (views - i as f64) / views);
        Some(2.0 * f64::from(assessed) + 2.0 * preference + 0.5 * megapixels.log2())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::test_content::content;
    use crate::config::Config;
    use serde_json::json;

    #[test]
    fn kinds_from_names() {
        assert_eq!(media_kind("Nairobi.JPG"), MediaKind::Photo);
        assert_eq!(media_kind("Flag of Kenya.svg"), MediaKind::Drawing);
        assert_eq!(media_kind("En-Nairobi.ogg"), MediaKind::Audio);
        assert_eq!(media_kind("Moon.webm"), MediaKind::Video);
        assert_eq!(media_kind("Moon.stl"), MediaKind::Model3d);
    }

    fn file(width: u32, height: u32, assessments: &[&str]) -> serde_json::Value {
        json!({"width": width, "height": height, "bytes": 5000000,
               "url": "https://upload.wikimedia.org/x", "artist": "A", "license": "CC BY-SA 4.0",
               "assessments": assessments})
    }

    #[test]
    fn picture_facts() {
        let c = content(
            json!([]),
            json!({"commons": {
                "Logo.png": file(400, 100, &[]),
                "Clip.webm": file(1920, 1080, &[]),
                "View.jpg": file(4000, 3000, &["Quality images"]),
            }}),
        );
        let k = Composer::new(&c, Config::embedded());
        let logo = k.picture("Logo.png", true);
        assert_eq!(logo.kind, MediaKind::Drawing);
        assert_eq!((logo.aspect, logo.megapixels), (Some(4.0), Some(0.04)));
        assert_eq!(logo.bytes, None, "a still picture carries no size");
        let clip = k.picture("Clip.webm", false);
        assert_eq!((clip.kind, clip.bytes), (MediaKind::Video, Some(5000000)));
        assert_eq!(k.picture("View.jpg", false).assessed, ["Quality images"]);
        let unknown = k.picture("Missing.jpg", false);
        assert_eq!((unknown.aspect, unknown.credit.artist.as_str()), (None, ""));
    }

    #[test]
    fn feature_scores() {
        let media =
            |pid: &str, f: &str| json!({"pid": pid, "value": {"type": "CommonsMedia", "file": f}});
        let c = content(
            json!([
                media("P18", "View.jpg"),
                media("P18", "Small.jpg"),
                media("P242", "Map.jpg"),
                media("P242", "Assessed.jpg"),
                media("P41", "Flag.svg"),
            ]),
            json!({"commons": {
                "View.jpg": file(4000, 3000, &[]),
                "Small.jpg": file(800, 600, &[]),
                "Map.jpg": file(4000, 3000, &[]),
                "Assessed.jpg": file(4000, 3000, &["Featured pictures on Wikimedia Commons"]),
                "Flag.svg": file(4000, 3000, &["Quality images"]),
            }}),
        );
        let k = Composer::new(&c, Config::embedded());
        let s: Vec<_> = c.statements.iter().map(|s| k.feature_score(s)).collect();
        // P18 is the 4th of 5 views: 2 * 2/5 + 0.5 * log2(12)
        assert_eq!(s[0], Some(0.8 + 0.5 * 12f64.log2()));
        assert_eq!(s[1], None, "below min_megapixels");
        assert_eq!(s[2], None, "not a view and not assessed");
        assert_eq!(s[3], Some(6.0 + 0.5 * 12f64.log2()), "featured: weight 3");
        assert_eq!(s[4], None, "a drawing is not a feature photograph");
    }
}
