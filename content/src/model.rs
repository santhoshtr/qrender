//! The content of one item (`content/0.1`): statements with identifiers and
//! typed values, and the side data that the composer reads. It has no
//! labels and no text, so it is the same for every language.

use indexmap::IndexMap;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};

use crate::citation::Cited;
use crate::id::{EntityId, Pid, Qid, StatementId};

pub const CONTENT_SCHEMA: &str = "content/0.1";

const ENTITY: &str = "http://www.wikidata.org/entity/";
const GENID: &str = "http://www.wikidata.org/.well-known/genid/";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{datatype} value {raw:?}: {reason}")]
pub struct ValueError {
    pub datatype: String,
    pub raw: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub schema: String,
    pub qid: Qid,
    pub statements: Vec<Statement>,
    /// For each statement, the property set of each of its references.
    pub references: IndexMap<StatementId, Vec<Vec<Pid>>>,
    /// For each statement, the hashes of its references that cite a
    /// source (in `citations`).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub statement_citations: IndexMap<StatementId, Vec<String>>,
    /// The references that cite a source, by hash: a reference that many
    /// statements share is here once.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub citations: IndexMap<String, Cited>,
    pub container: Container,
    /// Quantities of the item and of its container, normalized to SI units.
    pub quantities: IndexMap<Qid, QuantityRecord>,
    pub peers: Option<Peers>,
    /// Commons metadata, by file name.
    pub commons: IndexMap<String, CommonsFile>,
    /// Value items that are classes (they have P279): never pictured.
    pub value_classes: Vec<Qid>,
    /// The number of sitelinks of each item value: how widely it is known.
    /// A set of values puts the most known first.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub value_sitelinks: IndexMap<Qid, u32>,
    /// Site id ("enwiki") to page title.
    pub sitelinks: IndexMap<String, String>,
    pub class_layout: String,
    /// Statements on other items that point to this one.
    pub reverse: Vec<ReverseWork>,
    /// Commons data files, by the URL that a TabularData value holds.
    pub tabular: IndexMap<String, Table>,
    /// The logo (P154) of qualifier values that are shown by their logo.
    #[serde(default)]
    pub logos: IndexMap<Qid, String>,
    /// The range constraint of each quantity property of the item that has
    /// one: the values a property allows. A range can be the scale of an
    /// index (0 to 1) or only a sanity bound (0 to 400).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub ranges: IndexMap<Pid, Range>,
    /// The own dates of the event items of timeline properties (the
    /// significant events): a timeline places an event by them when its
    /// statement has no date.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub event_dates: IndexMap<Qid, EventDates>,
    /// The day of each holiday item of calendar properties (the public
    /// holidays): the label of the day is its date, its month orders it.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub holiday_days: IndexMap<Qid, HolidayDay>,
    /// The outlines of geoshapes (Commons .map pages), by the URL that a
    /// GeoShape value holds.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub shapes: IndexMap<String, Shape>,
    /// The language code (P424) of each item that is the value of a
    /// "language of work or name" (P407) qualifier: the names block and
    /// the sounds place a value by the code of its language.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub language_codes: IndexMap<Qid, String>,
    /// The formatter URL (P1630) of each web property of the item: the
    /// address of an account is its value in place of "$1".
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub formatters: IndexMap<Pid, String>,
    /// The text statements (motto text) whose words are a label or an
    /// alias of an item value of their paired property (motto), in the
    /// language of the words or in `mul`, without regard to case: text
    /// statement -> that item.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub text_items: IndexMap<StatementId, Qid>,
    /// The wanted properties of the class layout that the item has no
    /// statement for, with the datatype as Wikibase names it
    /// ("wikibase-item"): the page asks the reader for them.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub wanted: IndexMap<Pid, String>,
}

/// The outline of a geoshape, simplified for a map on a page, and what its
/// credit needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    /// The Commons page: "Data:India.map".
    pub title: String,
    /// The rings of its polygons and its lines: longitude, latitude.
    pub lines: Vec<Vec<[f64; 2]>>,
    /// West, south, east, north. East is above 180 when the shape crosses
    /// the antimeridian; its longitudes are then continuous.
    pub bbox: [f64; 4],
    pub license: String,
    /// The sources, as wikitext.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sources: String,
}

/// The day of a holiday: its "day in year for periodic occurrence" (P837),
/// a calendar day ("July 4") or a rule ("fourth Thursday in November").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolidayDay {
    pub day: Qid,
    /// The month of the day: "part of" (P361) of a calendar day, "month of
    /// the year" (P2922) of a rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<Qid>,
    /// A calendar day, not a rule.
    pub fixed: bool,
}

/// The dates of an event item, as Wikidata times: its point in time
/// (P585), start time (P580), end time (P582).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventDates {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point_in_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

/// The lowest and the highest value that a property allows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub id: StatementId,
    pub pid: Pid,
    pub rank: Rank,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub qualifiers: Vec<Qualifier>,
    /// The picture of an item value: its image, or else its logo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_image: Option<ValueImage>,
}

/// Deprecated statements are dropped when the content is fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Rank {
    Preferred,
    Normal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueImage {
    pub file: String,
    /// The picture is the logo (P154) of the value, not its image (P18). A
    /// logo is shown whole, never cropped.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub logo: bool,
}

/// The datatype of a property, with the names Wikibase uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Datatype {
    WikibaseItem,
    Quantity,
    Time,
    Monolingualtext,
    CommonsMedia,
    GlobeCoordinate,
    Url,
    ExternalId,
    String,
    TabularData,
    GeoShape,
    /// A datatype that the composer does not know (lexemes, math, ...).
    Other,
}

impl Datatype {
    pub fn parse(name: &str) -> Self {
        match name {
            "WikibaseItem" => Self::WikibaseItem,
            "Quantity" => Self::Quantity,
            "Time" => Self::Time,
            "Monolingualtext" => Self::Monolingualtext,
            "CommonsMedia" => Self::CommonsMedia,
            "GlobeCoordinate" => Self::GlobeCoordinate,
            "Url" => Self::Url,
            "ExternalId" => Self::ExternalId,
            "String" => Self::String,
            "TabularData" => Self::TabularData,
            "GeoShape" => Self::GeoShape,
            _ => Self::Other,
        }
    }
}

/// A statement value, typed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Value {
    WikibaseItem {
        item: Qid,
    },
    Quantity {
        amount: f64,
        /// None for a count (no unit, Q199).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<Qid>,
    },
    Time {
        /// As the query service gives it: "1899-01-01T00:00:00Z", "-0300-...".
        time: String,
        /// Wikidata precision: 9 year, 10 month, 11 day.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        precision: Option<u8>,
    },
    Monolingualtext {
        text: String,
        lang: String,
    },
    CommonsMedia {
        file: String,
    },
    GlobeCoordinate {
        lat: f64,
        lon: f64,
        globe: Qid,
    },
    Url {
        url: String,
    },
    ExternalId {
        id: String,
    },
    String {
        string: String,
    },
    TabularData {
        page: String,
    },
    GeoShape {
        page: String,
    },
    /// "Unknown value": the statement says a value exists, not which.
    SomeValue {
        datatype: Datatype,
    },
    Other {
        datatype: String,
        raw: String,
    },
}

impl Value {
    /// A value from its raw form in a query result. `datatype` is the local
    /// name of `wikibase:propertyType` ("WikibaseItem").
    pub fn from_raw(
        datatype: &str,
        raw: &str,
        unit: Option<Qid>,
        precision: Option<u8>,
        lang: Option<&str>,
    ) -> Result<Self, ValueError> {
        let error = |reason| ValueError {
            datatype: datatype.to_string(),
            raw: raw.to_string(),
            reason,
        };
        let kind = Datatype::parse(datatype);
        if raw.starts_with(GENID) {
            return Ok(Self::SomeValue { datatype: kind });
        }
        Ok(match kind {
            Datatype::WikibaseItem => Self::WikibaseItem {
                item: raw
                    .strip_prefix(ENTITY)
                    .and_then(|q| q.parse().ok())
                    .ok_or_else(|| error("not an item URI"))?,
            },
            Datatype::Quantity => Self::Quantity {
                amount: raw.parse().map_err(|_| error("not a number"))?,
                unit: unit.filter(|u| u.number() != 199),
            },
            Datatype::Time => {
                year_of(raw).ok_or_else(|| error("not a date"))?;
                Self::Time {
                    time: raw.to_string(),
                    precision,
                }
            }
            Datatype::Monolingualtext => Self::Monolingualtext {
                text: raw.to_string(),
                lang: lang.ok_or_else(|| error("no language"))?.to_string(),
            },
            Datatype::CommonsMedia => Self::CommonsMedia {
                file: file_name(raw),
            },
            Datatype::GlobeCoordinate => {
                let (lat, lon, globe) = coordinate(raw).ok_or_else(|| error("not a point"))?;
                Self::GlobeCoordinate { lat, lon, globe }
            }
            Datatype::Url => Self::Url {
                url: raw.to_string(),
            },
            Datatype::ExternalId => Self::ExternalId {
                id: raw.to_string(),
            },
            Datatype::String => Self::String {
                string: raw.to_string(),
            },
            Datatype::TabularData => Self::TabularData {
                page: raw.to_string(),
            },
            Datatype::GeoShape => Self::GeoShape {
                page: raw.to_string(),
            },
            Datatype::Other => Self::Other {
                datatype: datatype.to_string(),
                raw: raw.to_string(),
            },
        })
    }

    pub fn datatype(&self) -> Datatype {
        match self {
            Self::WikibaseItem { .. } => Datatype::WikibaseItem,
            Self::Quantity { .. } => Datatype::Quantity,
            Self::Time { .. } => Datatype::Time,
            Self::Monolingualtext { .. } => Datatype::Monolingualtext,
            Self::CommonsMedia { .. } => Datatype::CommonsMedia,
            Self::GlobeCoordinate { .. } => Datatype::GlobeCoordinate,
            Self::Url { .. } => Datatype::Url,
            Self::ExternalId { .. } => Datatype::ExternalId,
            Self::String { .. } => Datatype::String,
            Self::TabularData { .. } => Datatype::TabularData,
            Self::GeoShape { .. } => Datatype::GeoShape,
            Self::SomeValue { datatype } => *datatype,
            Self::Other { .. } => Datatype::Other,
        }
    }

    pub fn item(&self) -> Option<Qid> {
        match self {
            Self::WikibaseItem { item } => Some(*item),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Qualifier {
    pub pid: Pid,
    pub value: QualifierValue,
}

/// A qualifier value. The query does not give the datatype of a
/// qualifier, so the kind is read from the raw value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum QualifierValue {
    Entity { entity: EntityId },
    Time { time: String },
    SomeValue,
    Text { text: String, lang: String },
    Amount { amount: f64 },
    String { string: String },
}

impl QualifierValue {
    /// The kind, in this order: an entity URI, a date, "unknown value", a
    /// text with a language, a number, else a string.
    pub fn from_raw(raw: &str, lang: Option<&str>) -> Self {
        if let Some(entity) = raw.strip_prefix(ENTITY).and_then(|e| e.parse().ok()) {
            Self::Entity { entity }
        } else if looks_like_date(raw) {
            Self::Time {
                time: raw.to_string(),
            }
        } else if raw.starts_with(GENID) {
            Self::SomeValue
        } else if let Some(lang) = lang {
            Self::Text {
                text: raw.to_string(),
                lang: lang.to_string(),
            }
        } else if let Ok(amount) = raw.parse() {
            Self::Amount { amount }
        } else {
            Self::String {
                string: raw.to_string(),
            }
        }
    }

    /// A qualifier value of a known datatype (`wikibase:propertyType`). A
    /// string stays a string ("1997" is not a number), which the guess of
    /// `from_raw` cannot know.
    pub fn from_typed(datatype: &str, raw: &str, lang: Option<&str>) -> Self {
        if raw.starts_with(GENID) {
            return Self::SomeValue;
        }
        let entity = || raw.strip_prefix(ENTITY).and_then(|e| e.parse().ok());
        let string = || Self::String {
            string: raw.to_string(),
        };
        match Datatype::parse(datatype) {
            Datatype::Time => Self::Time {
                time: raw.to_string(),
            },
            Datatype::Quantity => raw
                .parse()
                .map_or_else(|_| string(), |amount| Self::Amount { amount }),
            Datatype::Monolingualtext => match lang {
                Some(lang) => Self::Text {
                    text: raw.to_string(),
                    lang: lang.to_string(),
                },
                None => string(),
            },
            // Items, and the entity types the composer has no name for
            // (properties, lexemes): an entity when it is one.
            Datatype::WikibaseItem | Datatype::Other => {
                entity().map_or_else(string, |entity| Self::Entity { entity })
            }
            _ => string(),
        }
    }

    pub fn year(&self) -> Option<i64> {
        match self {
            Self::Time { time } => year_of(time),
            _ => None,
        }
    }
}

/// The year of a date as the query service writes it: "1899-01-01T...",
/// "-0300-01-01T..." (300 BCE). The sign is kept.
pub fn year_of(time: &str) -> Option<i64> {
    let (negative, rest) = match time.as_bytes().first()? {
        b'-' => (true, &time[1..]),
        b'+' => (false, &time[1..]),
        _ => (false, time),
    };
    let digits = &rest[..rest.find('-')?];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year: i64 = digits.parse().ok()?;
    Some(if negative { -year } else { year })
}

/// At least four digits of year, a month, a day and a time.
fn looks_like_date(raw: &str) -> bool {
    let rest = raw.strip_prefix(['+', '-']).unwrap_or(raw);
    let Some(year_end) = rest.find(|c: char| !c.is_ascii_digit()) else {
        return false;
    };
    let tail = &rest.as_bytes()[year_end..];
    year_end >= 4
        && tail.len() >= 7
        && tail[0] == b'-'
        && tail[1..3].iter().all(u8::is_ascii_digit)
        && tail[3] == b'-'
        && tail[4..6].iter().all(u8::is_ascii_digit)
        && tail[6] == b'T'
}

/// The file name in a Commons URL, decoded: ".../Special:FilePath/A%20b.jpg"
/// gives "A b.jpg".
pub fn file_name(url: &str) -> String {
    let last = url.rsplit('/').next().unwrap_or(url);
    percent_decode_str(last).decode_utf8_lossy().into_owned()
}

/// "Point(lon lat)", with "<http://www.wikidata.org/entity/Q405> " before it
/// when the globe is not the Earth: latitude, longitude, globe.
pub fn coordinate(raw: &str) -> Option<(f64, f64, Qid)> {
    let globe = match raw.find(ENTITY) {
        Some(start) => {
            let id = &raw[start + ENTITY.len()..];
            id[..id.find('>')?].parse().ok()?
        }
        None => "Q2".parse().ok()?,
    };
    let point = &raw[raw.find("Point(")? + "Point(".len()..];
    let point = &point[..point.find(')')?];
    let (lon, lat) = point.split_once(' ')?;
    Some((lat.parse().ok()?, lon.parse().ok()?, globe))
}

/// What contains the item (P397, P171, P17, P131, P361, the current value).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Container {
    pub pid: Option<Pid>,
    pub qid: Option<Qid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantityRecord {
    pub image: Option<String>,
    pub values: IndexMap<Pid, NormalQuantity>,
}

/// A quantity in its SI unit. A count keeps Q199 ("1") as its unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalQuantity {
    pub amount: f64,
    pub unit: Qid,
    pub preferred: bool,
}

/// The peer set: the most specific class (with the container, when that
/// gives enough members) that has 5 to 150 members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peers {
    pub class: Qid,
    pub container_pid: Option<Pid>,
    pub container: Option<Qid>,
    pub members: Vec<Qid>,
    /// For each property, the number of members that have it.
    pub frequency: IndexMap<Pid, u32>,
    pub quantities: IndexMap<Qid, QuantityRecord>,
    /// Properties that most peers have and the item lacks, with their
    /// datatype as Wikibase names it ("wikibase-item").
    #[serde(default)]
    pub missing: IndexMap<Pid, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonsFile {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bytes: u64,
    pub url: String,
    /// Plain text: Commons gives HTML, and the tags are removed.
    pub artist: String,
    pub license: String,
    /// Commons assessment categories: "Quality images", ...
    #[serde(default)]
    pub assessments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseWork {
    pub item: Qid,
    /// The properties of the work that point to the item (P50 author, ...).
    pub pids: Vec<Pid>,
    pub image: Option<String>,
    /// The earliest publication or inception year.
    pub year: Option<i64>,
}

/// A Commons data file (a .tab page).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub title: String,
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub license: String,
    #[serde(default)]
    pub description: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cell {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: &str) -> Qid {
        s.parse().unwrap()
    }

    #[test]
    fn years() {
        assert_eq!(year_of("1899-01-01T00:00:00Z"), Some(1899));
        assert_eq!(year_of("+2013-12-05T00:00:00Z"), Some(2013));
        assert_eq!(year_of("-0300-01-01T00:00:00Z"), Some(-300));
        assert_eq!(year_of("-4527000000-01-01T00:00:00Z"), Some(-4_527_000_000));
        assert_eq!(year_of("Virgil"), None);
    }

    #[test]
    fn values() {
        let v = |t, raw| Value::from_raw(t, raw, None, None, None).unwrap();
        assert_eq!(
            v("WikibaseItem", "http://www.wikidata.org/entity/Q114"),
            Value::WikibaseItem { item: q("Q114") }
        );
        assert_eq!(
            Value::from_raw("Quantity", "4397073", Some(q("Q199")), None, None).unwrap(),
            Value::Quantity {
                amount: 4397073.0,
                unit: None
            }
        );
        assert_eq!(
            v(
                "CommonsMedia",
                "http://commons.wikimedia.org/wiki/Special:FilePath/Nairobi%20skyline.jpg"
            ),
            Value::CommonsMedia {
                file: "Nairobi skyline.jpg".into()
            }
        );
        assert_eq!(
            v("GlobeCoordinate", "Point(36.817222222 -1.286388888)"),
            Value::GlobeCoordinate {
                lat: -1.286388888,
                lon: 36.817222222,
                globe: q("Q2")
            }
        );
        assert_eq!(
            v(
                "GlobeCoordinate",
                "<http://www.wikidata.org/entity/Q405> Point(0.0 0.0)"
            ),
            Value::GlobeCoordinate {
                lat: 0.0,
                lon: 0.0,
                globe: q("Q405")
            }
        );
        assert_eq!(
            v(
                "WikibaseItem",
                "http://www.wikidata.org/.well-known/genid/5b11355bf35ce814"
            ),
            Value::SomeValue {
                datatype: Datatype::WikibaseItem
            }
        );
        assert_eq!(v("Math", "x^2").datatype(), Datatype::Other);
        assert!(Value::from_raw("WikibaseItem", "Q1", None, None, None).is_err());
        assert!(Value::from_raw("Quantity", "many", None, None, None).is_err());
        assert!(Value::from_raw("Monolingualtext", "Nairobi", None, None, None).is_err());
    }

    #[test]
    fn qualifier_kinds() {
        let k = |raw| QualifierValue::from_raw(raw, None);
        assert!(matches!(
            k("http://www.wikidata.org/entity/P1082"),
            QualifierValue::Entity {
                entity: EntityId::Property(_)
            }
        ));
        assert_eq!(k("2025-12-22T00:00:00Z").year(), Some(2025));
        assert_eq!(k("-0300-01-01T00:00:00Z").year(), Some(-300));
        assert_eq!(
            k("http://www.wikidata.org/.well-known/genid/5b1"),
            QualifierValue::SomeValue
        );
        assert_eq!(
            QualifierValue::from_raw("Site officiel", Some("fr")),
            QualifierValue::Text {
                text: "Site officiel".into(),
                lang: "fr".into()
            }
        );
        assert_eq!(k("1447128"), QualifierValue::Amount { amount: 1447128.0 });
        // A year without a full date is a number, as in v3.
        assert_eq!(k("1997"), QualifierValue::Amount { amount: 1997.0 });
        assert_eq!(
            k("Virgil Throckmorton"),
            QualifierValue::String {
                string: "Virgil Throckmorton".into()
            }
        );
    }

    #[test]
    fn typed_qualifiers() {
        let t = QualifierValue::from_typed;
        assert_eq!(
            t("String", "1997", None),
            QualifierValue::String {
                string: "1997".into()
            }
        );
        assert_eq!(
            t("Quantity", "1997", None),
            QualifierValue::Amount { amount: 1997.0 }
        );
        assert!(matches!(
            t(
                "WikibaseProperty",
                "http://www.wikidata.org/entity/P31",
                None
            ),
            QualifierValue::Entity { .. }
        ));
        assert_eq!(t("Time", "1963-12-12T00:00:00Z", None).year(), Some(1963));
        assert_eq!(
            t("ExternalId", "1234", None),
            QualifierValue::String {
                string: "1234".into()
            }
        );
    }

    #[test]
    fn value_json_shape() {
        let json = serde_json::to_string(&Value::Quantity {
            amount: 1.5,
            unit: Some(q("Q11573")),
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"Quantity","amount":1.5,"unit":"Q11573"}"#);
    }
}
