//! The locale and the text direction of a page language. The browser
//! formats numbers and dates with `Intl` for the locale; a Wikimedia
//! language code is not always a BCP 47 tag that `Intl` knows.

use content::Lang;
use schemars::JsonSchema;
use serde::Serialize;

/// Wikimedia codes that differ from their BCP 47 tag.
const LOCALES: [(&str, &str); 9] = [
    ("als", "gsw"),
    ("bat-smg", "sgs"),
    ("be-tarask", "be"),
    ("fiu-vro", "vro"),
    ("roa-rup", "rup"),
    ("simple", "en"),
    ("zh-classical", "lzh"),
    ("zh-min-nan", "nan"),
    ("zh-yue", "yue"),
];

/// Languages written from right to left.
const RTL: [&str; 15] = [
    "ar", "arz", "azb", "ckb", "dv", "fa", "he", "ks", "mzn", "pnb", "ps", "sd", "ug", "ur", "yi",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    Ltr,
    Rtl,
}

pub fn locale(lang: &Lang) -> String {
    let code = lang.as_str();
    LOCALES
        .iter()
        .find(|(wiki, _)| *wiki == code)
        .map_or(code, |(_, bcp47)| bcp47)
        .to_string()
}

pub fn dir(lang: &Lang) -> Dir {
    if RTL.contains(&lang.as_str()) {
        Dir::Rtl
    } else {
        Dir::Ltr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_and_direction() {
        let l = |s: &str| s.parse::<Lang>().unwrap();
        assert_eq!(locale(&l("ml")), "ml");
        assert_eq!(locale(&l("be-tarask")), "be");
        assert_eq!(locale(&l("simple")), "en");
        assert_eq!(dir(&l("ar")), Dir::Rtl);
        assert_eq!(dir(&l("ml")), Dir::Ltr);
    }
}
