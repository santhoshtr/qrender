//! The shared value types of the composition: a statement, its value, its
//! qualifiers and pictures, as identifiers and numbers. Primitives build
//! their blocks from these types, so the localize step can label them in
//! one place.

use content::{EntityId, Pid, Qid, QualifierValue, Rank, StatementId, Value};
use schemars::JsonSchema;
use serde::Serialize;

/// Is a statement backed by a source?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    /// At least one reference with a real source.
    Sourced,
    /// References only to a Wikimedia project or a guess (P143 and similar).
    Imported,
    /// No reference.
    None,
}

/// A statement as the composition shows it.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct StatementRef {
    pub statement: StatementId,
    pub pid: Pid,
    #[serde(flatten)]
    pub value: ValueShown,
    pub rank: Rank,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub qualifiers: Vec<QualifierShown>,
    /// The picture of an item value. None for a class value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Picture>,
    /// The value is a class: its picture would show an example only.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub class: bool,
    /// The value's description is shown under it (a work).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub described: bool,
    /// False when the value is behind "+N".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shown: Option<bool>,
}

/// A statement value, by the keys its kind uses.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ValueShown {
    /// An item. None for "unknown value".
    Item {
        item: Option<Qid>,
    },
    Amount {
        amount: f64,
        unit: Option<Qid>,
    },
    Time {
        time: String,
        precision: Option<u8>,
    },
    Text {
        text: String,
        lang: String,
    },
    File {
        file: String,
    },
    Coordinate {
        lat: f64,
        lon: f64,
        globe: Qid,
    },
    Url {
        url: String,
    },
    String {
        string: String,
    },
    /// "Unknown value" of a property that is not an item property.
    SomeValue {
        some_value: bool,
    },
}

impl From<&Value> for ValueShown {
    fn from(value: &Value) -> Self {
        match value {
            Value::WikibaseItem { item } => Self::Item { item: Some(*item) },
            Value::SomeValue {
                datatype: content::Datatype::WikibaseItem,
            } => Self::Item { item: None },
            Value::SomeValue { .. } => Self::SomeValue { some_value: true },
            Value::Quantity { amount, unit } => Self::Amount {
                amount: *amount,
                unit: *unit,
            },
            Value::Time { time, precision } => Self::Time {
                time: time.clone(),
                precision: *precision,
            },
            Value::Monolingualtext { text, lang } => Self::Text {
                text: text.clone(),
                lang: lang.clone(),
            },
            Value::CommonsMedia { file } => Self::File { file: file.clone() },
            Value::GlobeCoordinate { lat, lon, globe } => Self::Coordinate {
                lat: *lat,
                lon: *lon,
                globe: *globe,
            },
            Value::Url { url } => Self::Url { url: url.clone() },
            Value::ExternalId { id: string }
            | Value::String { string }
            | Value::TabularData { page: string }
            | Value::GeoShape { page: string }
            | Value::Other { raw: string, .. } => Self::String {
                string: string.clone(),
            },
        }
    }
}

/// A qualifier as the composition shows it. "Unknown value" qualifiers are
/// not shown, so they have no form here.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct QualifierShown {
    pub pid: Pid,
    #[serde(flatten)]
    pub value: QualifierPart,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum QualifierPart {
    Item {
        item: EntityId,
    },
    /// A date qualifier shows its year.
    Year {
        year: i64,
    },
    Text {
        text: String,
        lang: String,
    },
    Amount {
        amount: f64,
    },
    String {
        string: String,
    },
    /// A Commons file (a pronunciation audio).
    File {
        file: String,
    },
    /// A Wikidata lexeme, sense or form ("L32470-S1").
    Lexeme {
        lexeme: String,
    },
}

/// The URL of a Commons file in a qualifier value.
const COMMONS_FILE: &str = "commons.wikimedia.org/wiki/Special:FilePath/";
/// The URL of a lexeme, a sense or a form in a qualifier value.
const LEXEME: &str = "www.wikidata.org/entity/L";

impl QualifierShown {
    pub fn new(pid: Pid, value: &QualifierValue) -> Option<Self> {
        let value = match value {
            QualifierValue::Entity { entity } => QualifierPart::Item { item: *entity },
            QualifierValue::Time { .. } => QualifierPart::Year {
                year: value.year()?,
            },
            QualifierValue::SomeValue => return None,
            QualifierValue::Text { text, lang } => QualifierPart::Text {
                text: text.clone(),
                lang: lang.clone(),
            },
            QualifierValue::Amount { amount } => QualifierPart::Amount { amount: *amount },
            // A Commons file (pronunciation audio) comes as its URL.
            QualifierValue::String { string } if string.contains(COMMONS_FILE) => {
                QualifierPart::File {
                    file: content::model::file_name(string),
                }
            }
            // A lexeme sense (P7018) comes as its URL.
            QualifierValue::String { string } if string.contains(LEXEME) => QualifierPart::Lexeme {
                lexeme: string.rsplit('/').next().unwrap_or(string).to_string(),
            },
            QualifierValue::String { string } => QualifierPart::String {
                string: string.clone(),
            },
        };
        Some(Self { pid, value })
    }
}

/// The kind of a media file, from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Photo,
    /// Shown whole, never cropped: SVG files and logos.
    Drawing,
    Audio,
    Video,
    Model3d,
}

impl MediaKind {
    /// The name the design system uses (`k-<name>`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Drawing => "drawing",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Model3d => "model3d",
        }
    }
}

/// A picture (or a sound, a video, a 3D model): intrinsic facts only.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Picture {
    pub file: String,
    #[serde(flatten)]
    pub facts: PictureFacts,
}

/// What a picture is: everything but its file name. A media block shows
/// these beside the statement that holds the file.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct PictureFacts {
    pub kind: MediaKind,
    /// Width / height, rounded to 4 places.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub megapixels: Option<f64>,
    /// Size and URL, for the kinds that play: audio, video, 3D.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Commons assessments: "Quality images", ...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assessed: Vec<String>,
    pub credit: Credit,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Credit {
    pub artist: String,
    pub license: String,
}

/// `x` rounded to `places` decimal places, as Python's `round()` does it:
/// the exact binary value, ties to even. Formatting does the same, so the
/// numbers in the JSON are stable and match v3.
pub fn round_to(x: f64, places: usize) -> f64 {
    format!("{x:.places$}").parse().unwrap_or(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_matches_python() {
        // Values checked with Python 3: round(x, n).
        assert_eq!(round_to(4.0 / 3.0, 4), 1.3333);
        assert_eq!(round_to(2.675, 2), 2.67); // 2.675 is 2.67499... in binary
        assert_eq!(round_to(0.125, 2), 0.12); // a tie: to even
        assert_eq!(round_to(5.882_352, 3), 5.882);
        assert_eq!(round_to(-1.5, 0), -2.0);
    }

    #[test]
    fn values_serialize_like_v3() {
        let q = |s: &str| s.parse::<Qid>().unwrap();
        let json = |v: &ValueShown| serde_json::to_string(v).unwrap();
        assert_eq!(
            json(&ValueShown::from(&Value::Quantity {
                amount: 4397073.0,
                unit: None
            })),
            r#"{"amount":4397073.0,"unit":null}"#
        );
        assert_eq!(
            json(&ValueShown::from(&Value::WikibaseItem { item: q("Q114") })),
            r#"{"item":"Q114"}"#
        );
        assert_eq!(
            json(&ValueShown::from(&Value::ExternalId { id: "123".into() })),
            r#"{"string":"123"}"#
        );
        let p582: Pid = "P582".parse().unwrap();
        let year = QualifierShown::new(
            p582,
            &QualifierValue::Time {
                time: "-0300-01-01T00:00:00Z".into(),
            },
        );
        assert_eq!(
            serde_json::to_string(&year).unwrap(),
            r#"{"pid":"P582","year":-300}"#
        );
        assert_eq!(QualifierShown::new(p582, &QualifierValue::SomeValue), None);
        // A Commons file comes as its URL: the file, decoded.
        let p443: Pid = "P443".parse().unwrap();
        let audio = QualifierShown::new(
            p443,
            &QualifierValue::String {
                string:
                    "http://commons.wikimedia.org/wiki/Special:FilePath/LL-Q1571%20%28mar%29-A.wav"
                        .into(),
            },
        );
        assert_eq!(
            audio.map(|q| q.value),
            Some(QualifierPart::File {
                file: "LL-Q1571 (mar)-A.wav".into()
            })
        );
        // A lexeme sense comes as its URL: its id.
        let sense = QualifierShown::new(
            "P7018".parse().unwrap(),
            &QualifierValue::String {
                string: "http://www.wikidata.org/entity/L32470-S1".into(),
            },
        );
        assert_eq!(
            sense.map(|q| q.value),
            Some(QualifierPart::Lexeme {
                lexeme: "L32470-S1".into()
            })
        );
    }
}
