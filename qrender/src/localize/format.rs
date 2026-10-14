//! Values as display parts, with a neutral text: the same in every
//! language (digits without grouping, ISO dates). The browser formats the
//! raw value for the page locale with `Intl`; without JavaScript, the
//! reader sees the neutral text, which is correct but not localized.
//!
//! The renderers call these functions; they are the one place for the
//! text of a value.

use content::{EntityId, Pid, Qid};

use super::View;
use super::messages::format;
use crate::compose::picture::media_kind;
use crate::schema::{MediaKind, QualifierPart, QualifierShown, ValueShown};
use crate::wikidata::{END_TIME, START_TIME};

const WIKIDATA: &str = "https://www.wikidata.org/wiki/";

/// One value, ready for a renderer.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// An item or a property: its label and link are in the label table.
    Entity(EntityId),
    Number {
        value: f64,
        kind: NumberKind,
        text: String,
        unit: Option<Qid>,
    },
    Time {
        text: String,
        /// For `<time datetime>`: years 1 to 9999 only.
        datetime: Option<String>,
        /// Wikidata precision: 9 year, 10 month, 11 day.
        precision: u8,
    },
    /// A text; with its language when it is a monolingual text.
    Text {
        text: String,
        lang: Option<String>,
    },
    Link {
        text: String,
        href: String,
    },
    /// A value with the label of its property: a quantity qualifier
    /// ("female population: 328875190"), whose number alone says nothing.
    Qualified {
        pid: Pid,
        value: Box<Part>,
    },
    /// A coordinate. On the Earth, a pin opens a map of it.
    Coordinate {
        lat: f64,
        lon: f64,
        earth: bool,
        text: String,
    },
    /// A Commons file of a qualifier: an audio file plays, another file
    /// links to its page.
    File {
        pid: Pid,
        file: String,
        audio: bool,
    },
}

/// How the browser formats a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    Plain,
    /// "1.48 × 10²³": the neutral text stays as it is.
    Scientific,
    /// A ratio shown as a percentage.
    Percent,
}

impl View {
    pub fn msg<'s>(&'s self, key: &'s str) -> &'s str {
        self.messages.get(key).map_or(key, String::as_str)
    }

    /// A year; a year before the common era with its message ("300 BCE").
    pub fn year(&self, year: i64) -> String {
        if year < 0 {
            format(self.msg("qrender-bce"), &[&(-year).to_string()])
        } else {
            year.to_string()
        }
    }

    pub fn part(&self, value: &ValueShown) -> Part {
        match value {
            ValueShown::Item { item: Some(item) } => Part::Entity(EntityId::Item(*item)),
            ValueShown::Item { item: None } | ValueShown::SomeValue { .. } => {
                self.text(self.msg("qrender-some-value"))
            }
            ValueShown::Amount { amount, unit } => number_part(*amount, *unit),
            ValueShown::Time { time, precision } => self.time(time, *precision),
            ValueShown::Text { text, lang } => Part::Text {
                text: text.clone(),
                lang: Some(lang.clone()),
            },
            ValueShown::Coordinate { lat, lon, globe } => coordinate_part(*lat, *lon, *globe),
            ValueShown::Url { url } => Part::Link {
                text: url.clone(),
                href: url.clone(),
            },
            ValueShown::File { file: text } | ValueShown::String { string: text } => {
                self.text(text)
            }
        }
    }

    /// The qualifiers of a statement as parts. A span of years reads as
    /// "1920 – 1963"; every other qualifier is one part.
    pub fn note(&self, qualifiers: &[QualifierShown]) -> Vec<Part> {
        let year = |pid| {
            qualifiers.iter().find_map(|q| match q.value {
                QualifierPart::Year { year } if q.pid == pid => Some(year),
                _ => None,
            })
        };
        let (start, end) = (year(*START_TIME), year(*END_TIME));
        let mut parts = Vec::new();
        if start.is_some() || end.is_some() {
            let side = |y: Option<i64>| y.map(|y| self.year(y)).unwrap_or_default();
            let span = format!("{} – {}", side(start), side(end));
            parts.push(self.text(span.trim()));
        }
        for q in qualifiers {
            if q.pid == *START_TIME || q.pid == *END_TIME {
                continue;
            }
            parts.push(self.qualifier_part(q));
        }
        parts
    }

    /// One qualifier as a part.
    fn qualifier_part(&self, q: &QualifierShown) -> Part {
        match &q.value {
            QualifierPart::Item { item } => Part::Entity(*item),
            QualifierPart::Year { year } => self.text(&self.year(*year)),
            QualifierPart::Text { text, lang } => Part::Text {
                text: text.clone(),
                lang: Some(lang.clone()),
            },
            QualifierPart::Amount { amount } => Part::Qualified {
                pid: q.pid,
                value: Box::new(number_part(*amount, None)),
            },
            QualifierPart::String { string } => self.text(string),
            // The page of the lexeme, at the sense or the form.
            QualifierPart::Lexeme { lexeme } => Part::Qualified {
                pid: q.pid,
                value: Box::new(Part::Link {
                    text: lexeme.clone(),
                    href: format!(
                        "{WIKIDATA}Lexeme:{}#{lexeme}",
                        lexeme.split('-').next().unwrap_or(lexeme)
                    ),
                }),
            },
            QualifierPart::Coordinate { lat, lon, globe } => coordinate_part(*lat, *lon, *globe),
            QualifierPart::File { file } => Part::File {
                pid: q.pid,
                file: file.clone(),
                audio: media_kind(file) == MediaKind::Audio,
            },
        }
    }

    /// The qualifiers grouped by property, in their order: for a note that
    /// must name each property ("statement disputed by: A, B").
    pub fn labelled_note(&self, qualifiers: &[QualifierShown]) -> Vec<(Pid, Vec<Part>)> {
        let mut out: Vec<(Pid, Vec<Part>)> = Vec::new();
        for q in qualifiers {
            let part = match self.qualifier_part(q) {
                Part::Qualified { value, .. } => *value,
                part => part,
            };
            match out.iter_mut().find(|(pid, _)| *pid == q.pid) {
                Some((_, parts)) => parts.push(part),
                None => out.push((q.pid, vec![part])),
            }
        }
        out
    }

    /// A date by its Wikidata precision: a year (9 or less), a month (10),
    /// a day (11 or more).
    pub fn time(&self, time: &str, precision: Option<u8>) -> Part {
        let year = content::model::year_of(time).unwrap_or(0);
        // "+1963-12-12T..." or "-0300-01-01T...": month and day after the year.
        let rest = time.trim_start_matches(['+', '-']);
        let date = rest.split('T').next().unwrap_or_default();
        let mut fields = date.split('-').skip(1);
        let (month, day) = (fields.next().unwrap_or("01"), fields.next().unwrap_or("01"));
        let precision = precision.unwrap_or(9);
        let (text, datetime) = match precision {
            0..=9 => (self.year(year), format!("{year:04}")),
            10 => (format!("{year}-{month}"), format!("{year:04}-{month}")),
            _ => (
                format!("{year}-{month}-{day}"),
                format!("{year:04}-{month}-{day}"),
            ),
        };
        let datetime = ((1..=9999).contains(&year) && precision >= 9).then_some(datetime);
        Part::Time {
            text,
            datetime,
            precision: precision.min(11),
        }
    }

    fn text(&self, text: &str) -> Part {
        Part::Text {
            text: text.to_string(),
            lang: None,
        }
    }
}

impl NumberKind {
    /// The `data-f` of a number for the browser's formatter.
    pub fn name(self) -> &'static str {
        match self {
            Self::Plain => "number",
            Self::Scientific => "scientific",
            Self::Percent => "percent",
        }
    }
}

/// A number and its neutral text. Very large and very small numbers are
/// "m × 10ⁿ".
pub fn number_part(value: f64, unit: Option<Qid>) -> Part {
    let scientific = value.abs() >= 1e9 || (value != 0.0 && value.abs() < 1e-3);
    Part::Number {
        value,
        kind: if scientific {
            NumberKind::Scientific
        } else {
            NumberKind::Plain
        },
        text: number(value),
        unit,
    }
}

/// The neutral text of a number.
pub fn number(x: f64) -> String {
    if x.abs() >= 1e9 || (x != 0.0 && x.abs() < 1e-3) {
        let exp = x.abs().log10().floor() as i32;
        let mantissa = significant(x / 10f64.powi(exp), 3);
        return format!("{mantissa} × 10{}", superscript(exp));
    }
    if x == x.trunc() {
        return format!("{}", x as i64);
    }
    significant(x, 6)
}

/// A ratio as a percentage: two significant digits below 10 %.
pub fn percent(ratio: f64) -> Part {
    let pct = ratio * 100.0;
    let text = if pct < 10.0 {
        format!("{} %", significant(pct, 2))
    } else {
        format!("{pct:.0} %")
    };
    Part::Number {
        value: ratio,
        kind: NumberKind::Percent,
        text,
        unit: None,
    }
}

/// "lat, lon" with six significant digits each.
pub fn coordinate(lat: f64, lon: f64) -> String {
    format!("{}, {}", significant(lat, 6), significant(lon, 6))
}

/// A coordinate as a part; on the Earth (Q2) it has a map.
fn coordinate_part(lat: f64, lon: f64, globe: Qid) -> Part {
    Part::Coordinate {
        lat,
        lon,
        earth: globe.number() == 2,
        text: coordinate(lat, lon),
    }
}

/// Where a statement is edited on Wikidata: the item page, at the property.
pub fn edit_url(qid: Qid, pid: content::Pid) -> String {
    format!("{WIKIDATA}{qid}#{pid}")
}

/// The page of an entity on Wikidata.
pub fn entity_url(id: EntityId) -> String {
    match id {
        EntityId::Item(q) => format!("{WIKIDATA}{q}"),
        EntityId::Property(p) => format!("{WIKIDATA}Property:{p}"),
    }
}

/// `x` with `digits` significant digits, as Python's `format(x, ".{digits}g")`:
/// fixed notation for exponents from -4 to digits - 1, else "1.5e+06";
/// trailing zeros removed.
pub fn significant(x: f64, digits: usize) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    let sci = format!("{:.*e}", digits.saturating_sub(1), x);
    let (mantissa, exp) = sci.split_once('e').expect("scientific notation");
    let exp: i32 = exp.parse().expect("an exponent");
    if exp < -4 || exp >= digits as i32 {
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{}e{sign}{:02}", trim_zeros(mantissa), exp.abs());
    }
    let decimals = (digits as i32 - 1 - exp).max(0) as usize;
    trim_zeros(&format!("{x:.decimals$}")).to_string()
}

fn trim_zeros(s: &str) -> &str {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    }
}

fn superscript(n: i32) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '-' => '⁻',
            d => "⁰¹²³⁴⁵⁶⁷⁸⁹"
                .chars()
                .nth(d.to_digit(10).unwrap_or(0) as usize)
                .unwrap_or(d),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn significant_digits_like_python() {
        // Checked with Python 3: format(x, ".Ng").
        assert_eq!(significant(36.817222222, 6), "36.8172");
        assert_eq!(significant(-1.286388888, 6), "-1.28639");
        assert_eq!(significant(1234567.5, 6), "1.23457e+06");
        assert_eq!(significant(0.00012345, 3), "0.000123");
        assert_eq!(significant(0.000012345, 3), "1.23e-05");
        assert_eq!(significant(1.5, 3), "1.5");
        assert_eq!(significant(7.4363, 2), "7.4");
        assert_eq!(significant(100.0, 3), "100");
    }

    #[test]
    fn numbers() {
        assert_eq!(number(4397073.0), "4397073");
        assert_eq!(number(73.4767), "73.4767");
        assert_eq!(number(7.35e22), "7.35 × 10²²");
        assert_eq!(number(0.0001), "1 × 10⁻⁴");
        assert_eq!(number(0.0), "0");
        let Part::Number { text, kind, .. } = percent(0.074363) else {
            panic!()
        };
        assert_eq!((text.as_str(), kind), ("7.4 %", NumberKind::Percent));
        let Part::Number { text, .. } = percent(0.273266) else {
            panic!()
        };
        assert_eq!(text, "27 %");
    }

    #[test]
    fn urls() {
        assert_eq!(
            edit_url("Q3870".parse().unwrap(), "P17".parse().unwrap()),
            "https://www.wikidata.org/wiki/Q3870#P17"
        );
        assert_eq!(
            entity_url("P17".parse().unwrap()),
            "https://www.wikidata.org/wiki/Property:P17"
        );
    }
}
