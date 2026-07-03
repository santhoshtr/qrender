//! Display formatting for typed values on cards.

use qjson::Value;

/// Wikidata time precisions
const PRECISION_YEAR: u8 = 9;
const PRECISION_MONTH: u8 = 10;

/// Truncate an ISO timestamp ("1899-01-01T00:00:00Z") to its stated
/// precision. Language-neutral numeric form; localized date formatting
/// is a later iteration.
pub fn format_time(iso: &str, precision: Option<u8>) -> String {
    let date = iso.split('T').next().unwrap_or(iso);
    // Split from the right so negative years ("-0500-01-01") keep their sign
    let month_end = date.len().saturating_sub(3); // "1899-01"
    let year_end = date.len().saturating_sub(6); // "1899"
    match precision {
        Some(p) if p <= PRECISION_YEAR => date[..year_end].to_string(),
        Some(PRECISION_MONTH) => date[..month_end].to_string(),
        _ => date.to_string(),
    }
}

/// Card-facing display string: like Value::display() but with times
/// truncated to precision and quantities carrying their unit label.
pub fn display_value(value: &Value) -> String {
    match value {
        Value::Time { iso, precision } => format_time(iso, *precision),
        Value::Quantity {
            raw, unit_label, ..
        } => match unit_label {
            Some(unit) => format!("{raw} {unit}"),
            None => raw.clone(),
        },
        other => other.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_truncates_to_precision() {
        assert_eq!(format_time("1899-01-01T00:00:00Z", Some(9)), "1899");
        assert_eq!(format_time("1899-06-01T00:00:00Z", Some(10)), "1899-06");
        assert_eq!(format_time("1899-06-15T00:00:00Z", Some(11)), "1899-06-15");
        assert_eq!(format_time("1899-06-15T00:00:00Z", None), "1899-06-15");
    }

    #[test]
    fn negative_years_keep_sign() {
        assert_eq!(format_time("-0500-01-01T00:00:00Z", Some(9)), "-0500");
    }
}
