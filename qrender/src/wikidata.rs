//! The properties of Wikidata's own data model that the code names. They
//! are structure, not judgment: what an item is, its picture, the time
//! qualifiers. Every editorial choice about a property is in the
//! configuration.

use std::sync::LazyLock;

use content::Pid;

macro_rules! properties {
    ($($name:ident = $id:literal, $label:literal;)*) => {$(
        #[doc = $label]
        pub static $name: LazyLock<Pid> =
            LazyLock::new(|| $id.parse().expect("a property id"));
    )*};
}

properties! {
    INSTANCE_OF = "P31", "instance of";
    IMAGE = "P18", "image";
    PAGE_BANNER = "P948", "page banner";
    START_TIME = "P580", "start time";
    END_TIME = "P582", "end time";
    POINT_IN_TIME = "P585", "point in time";
    DATE_OF_BIRTH = "P569", "date of birth";
    DATE_OF_DEATH = "P570", "date of death";
    AGE = "P3629", "age of subject at event";
    AREA = "P2046", "area";
    LANGUAGE = "P407", "language of work or name";
}

/// Percent (Q11229): the unit of a proportion.
pub static PERCENT: LazyLock<content::Qid> =
    LazyLock::new(|| "Q11229".parse().expect("an item id"));

/// The months of the Gregorian calendar, January (Q108) to December (Q126):
/// the order of the days of holidays.
pub static MONTHS: LazyLock<[content::Qid; 12]> = LazyLock::new(|| {
    [
        "Q108", "Q109", "Q110", "Q118", "Q119", "Q120", "Q121", "Q122", "Q123", "Q124", "Q125",
        "Q126",
    ]
    .map(|q| q.parse().expect("an item id"))
});
