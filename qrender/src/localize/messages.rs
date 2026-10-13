//! Interface messages, in the banana format of translatewiki.net
//! (`i18n/<lang>.json`). A language without a catalog, or a message
//! without a translation, falls back to English.

use std::collections::BTreeMap;

use content::Lang;

const CATALOGS: [(&str, &str); 1] = [("en", include_str!("../../i18n/en.json"))];

/// The messages of a language, with English for the ones it lacks.
pub fn for_lang(lang: &Lang) -> BTreeMap<String, String> {
    let mut messages = catalog("en");
    if lang.as_str() != "en" {
        messages.extend(catalog(lang.as_str()));
    }
    messages
}

fn catalog(lang: &str) -> BTreeMap<String, String> {
    let Some((_, json)) = CATALOGS.iter().find(|(l, _)| *l == lang) else {
        return BTreeMap::new();
    };
    let all: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("i18n/{lang}.json: {e}"));
    all.into_iter()
        .filter_map(|(k, v)| Some((k, v.as_str()?.to_string())))
        .collect()
}

/// A message with its parameters in place of $1, $2, ...
pub fn format(template: &str, params: &[&str]) -> String {
    let mut out = template.to_string();
    // Highest first, so that $1 does not replace the start of $10.
    for (i, p) in params.iter().enumerate().rev() {
        out = out.replace(&format!("${}", i + 1), p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_catalog_and_fallback() {
        let en = for_lang(&"en".parse().unwrap());
        assert_eq!(en["qrender-edit"], "Edit");
        assert!(!en.contains_key("@metadata"));
        // No Malayalam catalog yet: English.
        assert_eq!(for_lang(&"ml".parse().unwrap()), en);
    }

    #[test]
    fn every_message_is_documented() {
        let en = catalog("en");
        let qqq: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(include_str!("../../i18n/qqq.json")).unwrap();
        for key in en.keys() {
            assert!(qqq.contains_key(key), "i18n/qqq.json has no {key}");
        }
    }

    #[test]
    fn parameters() {
        assert_eq!(format("$1 BCE", &["300"]), "300 BCE");
        assert_eq!(format("$2 of $1", &["a", "b"]), "b of a");
    }
}
