//! Identifiers. Each constructor validates its input, so a value of these
//! types is safe to put in a query or a URL.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a valid {kind}: {value:?}")]
pub struct IdError {
    pub kind: &'static str,
    pub value: String,
}

impl IdError {
    fn new(kind: &'static str, value: &str) -> Self {
        Self {
            kind,
            value: value.to_string(),
        }
    }
}

/// An entity identifier: a prefix letter and a number. The number is kept,
/// so the type is `Copy` and sorts by number (Q2 before Q10).
macro_rules! entity_id {
    ($name:ident, $prefix:literal, $kind:literal) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(u64);

        impl $name {
            pub fn number(self) -> u64 {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, IdError> {
                s.strip_prefix($prefix)
                    .filter(|n| !n.is_empty() && !n.starts_with('0'))
                    .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|n| n.parse().ok())
                    .map(Self)
                    .ok_or_else(|| IdError::new($kind, s))
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(s: String) -> Result<Self, IdError> {
                s.parse()
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.to_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }
    };
}

entity_id!(Qid, 'Q', "item id");
entity_id!(Pid, 'P', "property id");

/// An item or a property: the key of a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum EntityId {
    Item(Qid),
    Property(Pid),
}

impl FromStr for EntityId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, IdError> {
        match s.as_bytes().first() {
            Some(b'Q') => s.parse().map(Self::Item),
            Some(b'P') => s.parse().map(Self::Property),
            _ => Err(IdError::new("entity id", s)),
        }
    }
}

impl TryFrom<String> for EntityId {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, IdError> {
        s.parse()
    }
}

impl From<EntityId> for String {
    fn from(id: EntityId) -> String {
        id.to_string()
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Item(q) => q.fmt(f),
            Self::Property(p) => p.fmt(f),
        }
    }
}

impl From<Qid> for EntityId {
    fn from(q: Qid) -> Self {
        Self::Item(q)
    }
}

impl From<Pid> for EntityId {
    fn from(p: Pid) -> Self {
        Self::Property(p)
    }
}

/// The identifier of a statement, as the query service gives it: the
/// entity, a separator and a GUID ("Q3870-1A7BF03E-..."). The case is kept
/// as it comes, because references are keyed by the same string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StatementId(String);

impl StatementId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for StatementId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, IdError> {
        let entity_end = s
            .char_indices()
            .skip(1)
            .find(|(_, c)| !c.is_ascii_digit())
            .map(|(i, _)| i);
        let valid = matches!(s.as_bytes().first(), Some(b'Q' | b'q' | b'P' | b'p'))
            && entity_end.is_some_and(|i| {
                i > 1
                    && matches!(s.as_bytes()[i], b'-' | b'$')
                    && s.len() > i + 1
                    && s[i + 1..]
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() || b == b'-')
            });
        if valid {
            Ok(Self(s.to_string()))
        } else {
            Err(IdError::new("statement id", s))
        }
    }
}

impl TryFrom<String> for StatementId {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, IdError> {
        s.parse()
    }
}

impl From<StatementId> for String {
    fn from(id: StatementId) -> String {
        id.0
    }
}

impl fmt::Display for StatementId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A Wikimedia language code: "en", "ml", "tcy", "mul", "be-tarask",
/// "zh-min-nan". Lower case letters, then subtags of letters and digits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Lang(String);

impl Lang {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Lang {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, IdError> {
        let mut parts = s.split('-');
        let primary = parts.next().unwrap_or_default();
        let valid = (2..=8).contains(&primary.len())
            && primary.bytes().all(|b| b.is_ascii_lowercase())
            && parts.all(|p| {
                (1..=8).contains(&p.len())
                    && p.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            });
        if valid {
            Ok(Self(s.to_string()))
        } else {
            Err(IdError::new("language code", s))
        }
    }
}

impl TryFrom<String> for Lang {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, IdError> {
        s.parse()
    }
}

impl From<Lang> for String {
    fn from(lang: Lang) -> String {
        lang.0
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ids() {
        let q: Qid = "Q3870".parse().unwrap();
        assert_eq!(q.number(), 3870);
        assert_eq!(q.to_string(), "Q3870");
        assert!("Q2".parse::<Qid>().unwrap() < "Q10".parse::<Qid>().unwrap());
        for bad in [
            "", "Q", "Q0", "Q012", "q12", "P12", "Q12 ", "Q1;DROP", "Q-1",
        ] {
            assert!(bad.parse::<Qid>().is_err(), "{bad:?}");
        }
        assert_eq!("P31".parse::<Pid>().unwrap().to_string(), "P31");
        assert_eq!(
            "P31".parse::<EntityId>().unwrap(),
            EntityId::Property("P31".parse().unwrap())
        );
        assert!("L1".parse::<EntityId>().is_err());
    }

    #[test]
    fn entity_ids_serialize_as_strings() {
        let q: Qid = serde_json::from_str("\"Q42\"").unwrap();
        assert_eq!(serde_json::to_string(&q).unwrap(), "\"Q42\"");
        assert!(serde_json::from_str::<Qid>("\"Q42x\"").is_err());
    }

    #[test]
    fn statement_ids() {
        for good in [
            "Q3870-1A7BF03E-D843-481A-B22B-F5152EB57622",
            "q90-014E7095-C5F0-49AE-9DDF-EDAF20C6820F",
            "Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9",
        ] {
            assert!(good.parse::<StatementId>().is_ok(), "{good}");
        }
        for bad in ["", "Q-ABC", "Q42", "Q42-", "Q42-XYZ", "X42-ABC"] {
            assert!(bad.parse::<StatementId>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn languages() {
        for good in [
            "en",
            "ml",
            "tcy",
            "mul",
            "be-tarask",
            "zh-min-nan",
            "simple",
        ] {
            assert!(good.parse::<Lang>().is_ok(), "{good}");
        }
        for bad in ["", "e", "EN", "en_US", "en-", "a\"b", "toolonglang"] {
            assert!(bad.parse::<Lang>().is_err(), "{bad:?}");
        }
    }
}
