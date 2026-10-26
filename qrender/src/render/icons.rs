//! The icons of the composed items, by configuration (`design/icons.toml`).
//! The label of an item starts with the icon of its property (or of the
//! concept item that titles it); else the icon of the datatype of its
//! value; else the fallback icon.
//!
//! An icon is a class (`i-calendar`) that sets `--icon`, a mask that
//! `base.css` draws in the colour of the dimension. The stylesheet has a
//! rule for each icon that the configuration uses, and for no other.

use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

use content::{Datatype, Pid, Qid};
use indexmap::IndexMap;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;

use super::design::inline_icon;
use crate::schema::ValueShown;

const ICONS: &str = include_str!("../../design/icons.toml");

/// The icons that the configuration can name: a name that is not here is
/// a load error. Each is a file of `assets/icons/`: a Codex icon, else a
/// Material Symbol (`material/`: see its README).
macro_rules! icons {
    ($($variant:ident = $name:literal => $file:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
        pub enum Icon {
            $(#[serde(rename = $name)] $variant,)*
        }

        impl Icon {
            pub const ALL: &[Icon] = &[$(Self::$variant,)*];

            /// The file of the icon, from `assets/icons/`.
            pub fn file(self) -> &'static str {
                match self {
                    $(Self::$variant => $file,)*
                }
            }

            /// The class that gives an element the icon.
            pub fn class(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!("i-", $name),)*
                }
            }

            fn svg(self) -> &'static str {
                match self {
                    $(Self::$variant => include_str!(concat!("../../assets/icons/", $file)),)*
                }
            }
        }
    };
}

icons! {
    Browser = "browser" => "browser.svg",
    Bright = "bright" => "bright.svg",
    Calendar = "calendar" => "calendar.svg",
    CommonsLogo = "logo-Wikimedia-Commons" => "logo-Wikimedia-Commons.svg",
    Database = "database" => "database.svg",
    Eye = "eye" => "eye.svg",
    Globe = "globe" => "globe.svg",
    Hand = "hand" => "hand.svg",
    Link = "link" => "link.svg",
    Map = "map" => "map.svg",
    MapPin = "mapPin" => "mapPin.svg",
    Puzzle = "puzzle" => "puzzle.svg",
    Rotate = "rotate" => "rotate.svg",
    Tag = "tag" => "tag.svg",
    ThreeDimensional = "threeDimensional" => "threeDimensional.svg",
    UserAvatar = "userAvatar" => "userAvatar.svg",
    UserGroup = "userGroup" => "userGroup.svg",
    VolumeUp = "volumeUp" => "volumeUp.svg",

    Altitude = "altitude" => "material/altitude.svg",
    Archive = "archive" => "material/archive.svg",
    ArrowCoolDown = "arrow_cool_down" => "material/arrow_cool_down.svg",
    AspectRatio = "aspect_ratio" => "material/aspect_ratio.svg",
    Bloodtype = "bloodtype" => "material/bloodtype.svg",
    Bolt = "bolt" => "material/bolt.svg",
    Call = "call" => "material/call.svg",
    CompareArrows = "compare_arrows" => "material/compare_arrows.svg",
    DirectionsCar = "directions_car" => "material/directions_car.svg",
    Distance = "distance" => "material/distance.svg",
    Equal = "equal" => "material/equal.svg",
    FitWidth = "fit_width" => "material/fit_width.svg",
    Heat = "heat" => "material/heat.svg",
    Height = "height" => "material/height.svg",
    LineStartCircle = "line_start_circle" => "material/line_start_circle.svg",
    LocalLibrary = "local_library" => "material/local_library.svg",
    LocationCity = "location_city" => "material/location_city.svg",
    Man = "man" => "material/man.svg",
    MenuBook = "menu_book" => "material/menu_book.svg",
    MountainFlag = "mountain_flag" => "material/mountain_flag.svg",
    OpenInPhone = "open_in_phone" => "material/open_in_phone.svg",
    Pace = "pace" => "material/pace.svg",
    Policy = "policy" => "material/policy.svg",
    School = "school" => "material/school.svg",
    Stethoscope = "stethoscope" => "material/stethoscope.svg",
    Straighten = "straighten" => "material/straighten.svg",
    Thermometer = "thermometer" => "material/thermometer.svg",
    ThermostatArrowDown = "thermostat_arrow_down" => "material/thermostat_arrow_down.svg",
    ThermostatArrowUp = "thermostat_arrow_up" => "material/thermostat_arrow_up.svg",
    Train = "train" => "material/train.svg",
    Trophy = "trophy" => "material/trophy.svg",
    Wc = "wc" => "material/wc.svg",
    Weight = "weight" => "material/weight.svg",
    Woman = "woman" => "material/woman.svg",
}

/// The types of `icons.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Icons {
    pub fallback: Icon,
    pub datatypes: HashMap<Datatype, Icon>,
    pub properties: IndexMap<Pid, Icon>,
    pub concepts: IndexMap<Qid, Icon>,
}

impl Icons {
    /// The embedded configuration. A test checks that it loads.
    pub fn embedded() -> &'static Icons {
        static EMBEDDED: LazyLock<Icons> =
            LazyLock::new(|| toml::from_str(ICONS).unwrap_or_else(|e| panic!("icons.toml: {e}")));
        &EMBEDDED
    }

    /// The icon of a property, else of the datatype of its value.
    pub fn property(&self, pid: Pid, value: Option<&ValueShown>) -> Icon {
        self.properties
            .get(&pid)
            .or_else(|| self.datatypes.get(&datatype(value?)?))
            .copied()
            .unwrap_or(self.fallback)
    }

    pub fn concept(&self, qid: Qid) -> Icon {
        self.concepts.get(&qid).copied().unwrap_or(self.fallback)
    }

    /// The icons that the configuration names, each once.
    fn used(&self) -> BTreeSet<Icon> {
        std::iter::once(self.fallback)
            .chain(self.datatypes.values().copied())
            .chain(self.properties.values().copied())
            .chain(self.concepts.values().copied())
            .collect()
    }

    /// The stylesheet rules: one for each icon used, as a data URL.
    pub fn css(&self) -> String {
        self.used()
            .into_iter()
            .map(|icon| {
                let svg = inline_icon(icon.svg());
                let url = utf8_percent_encode(&svg, NON_ALPHANUMERIC);
                format!(
                    ".{} {{ --icon: url(\"data:image/svg+xml,{url}\"); }}\n",
                    icon.class()
                )
            })
            .collect()
    }
}

/// The datatype of a value, where its form says it. A string can be of
/// several datatypes (an identifier, a string, a tabular data page).
fn datatype(value: &ValueShown) -> Option<Datatype> {
    Some(match value {
        ValueShown::Item { .. } => Datatype::WikibaseItem,
        ValueShown::Amount { .. } => Datatype::Quantity,
        ValueShown::Time { .. } => Datatype::Time,
        ValueShown::Text { .. } => Datatype::Monolingualtext,
        ValueShown::File { .. } => Datatype::CommonsMedia,
        ValueShown::Coordinate { .. } => Datatype::GlobeCoordinate,
        ValueShown::Url { .. } => Datatype::Url,
        ValueShown::Link { .. } | ValueShown::String { .. } | ValueShown::SomeValue { .. } => {
            return None;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_load() {
        let icons = Icons::embedded();
        let pid = |s: &str| s.parse::<Pid>().unwrap();
        assert_eq!(icons.property(pid("P2067"), None), Icon::Weight);
        let time = ValueShown::Time {
            time: "+1918-07-18T00:00:00Z".into(),
            precision: Some(11),
        };
        assert_eq!(icons.property(pid("P569"), Some(&time)), Icon::Calendar);
        assert_eq!(icons.property(pid("P31"), None), Icon::Puzzle);
    }

    #[test]
    fn unknown_icon_names_are_errors() {
        let bad = ICONS.replacen("\"weight\"", "\"wieght\"", 1);
        assert!(toml::from_str::<Icons>(&bad).is_err());
    }

    /// Each Material Symbol in `assets/icons/material/` is in the table: no
    /// file without a use.
    #[test]
    fn every_material_icon_is_named() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons/material");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            if name.ends_with(".svg") {
                let file = format!("material/{name}");
                assert!(
                    Icon::ALL.iter().any(|i| i.file() == file),
                    "{file} is not in the icon table"
                );
            }
        }
    }
}
