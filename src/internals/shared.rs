use serde::{Deserialize, Deserializer, Serializer};

const PRECISION_SCALE: u128 = 10_000;

/// Parses an Option<String> into an Option<u128> for up to four points of precision after the
/// decimal place, extra precision results in truncation. Only allows positive integers.
fn parse_u128_fixed(input: Option<String>) -> Result<Option<u128>, String> {
    let deserialized = match input {
        Some(val) => val,
        None => return Ok(None),
    };
    let trimmed = deserialized.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let dot_count = trimmed.chars().filter(|&c| c == '.').count();
    if dot_count > 1 || !trimmed.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err("Invalid characters or multiple decimals".to_string());
    }

    let mut parts: Vec<String> = trimmed.split('.').map(String::from).collect();
    parts[0] = parts[0].replace('.', "");

    let mut first = if parts[0].is_empty() {
        0
    } else {
        parts[0].parse::<u128>().map_err(|e| e.to_string())?
    };
    first = first
        .checked_mul(PRECISION_SCALE)
        .ok_or_else(|| "Integer multiplication overflowed u128 limits".to_string())?;

    if parts.len() > 1 {
        let second = parts[1].clone();
        let mut fraction: u128 = 0;
        if !second.is_empty() {
            if second.len() >= 4 {
                fraction = second[0..4].parse::<u128>().map_err(|e| e.to_string())?;
            } else {
                let padded = format!("{}{}", second, "0".repeat(4 - second.len()));
                fraction = padded.parse::<u128>().map_err(|e| e.to_string())?;
            }
        }
        if fraction > 0 {
            first = first
                .checked_add(fraction)
                .ok_or_else(|| "Integer addition overflowed u128 limits".to_string())?;
        }
    }
    Ok(Some(first))
}

pub fn deserialize_u128_fixed<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
where
    D: Deserializer<'de>,
{
    let deserialized: Option<String> = Option::deserialize(deserializer)?;
    parse_u128_fixed(deserialized).map_err(serde::de::Error::custom)
}

/// Formats a scaled i128 back into a float with four places of precision for display.
fn format_u128_fixed(val: u128) -> String {
    let integer_part = val / PRECISION_SCALE;
    let fractional_part = val % PRECISION_SCALE;
    format!("{}.{:04}", integer_part, fractional_part)
}

pub fn serialize_u128_fixed<S>(val: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format_u128_fixed(*val))
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("0", Some(0))]
    #[case("0.0", Some(0))]
    #[case("0.00001", Some(0))] // Drops trailing precision
    #[case("0.0001", Some(1))] // Smallest single value after zero
    #[case("1.1", Some(11_000))] // Appends trailing zeros
    #[case("1002", Some(10_020_000))] // No decimal precision
    #[case("1.222256", Some(12_222))] // Truncates, doesn't round
    #[case("1.1111", Some(11_111))] // Exactly 4 fraction digits
    #[case("1.12", Some(11_200))] // Pads short fraction
    #[case(".5", Some(5000))] // Trailing dot
    #[case("5.", Some(50_000))] // Leading dot
    fn test_amount_precision(#[case] input: &str, #[case] expected: Option<u128>) {
        assert_eq!(parse_u128_fixed(Some(input.to_string())).unwrap(), expected);
    }

    #[rstest]
    #[case("   ")]
    #[case("")]
    fn test_empty_and_whitespace_values(#[case] input: &str) {
        assert_eq!(parse_u128_fixed(Some(input.to_string())).unwrap(), None);
    }

    #[test]
    fn test_missing_field_is_none() {
        assert_eq!(parse_u128_fixed(None).unwrap(), None);
    }

    #[rstest]
    #[case("-10.00")]
    #[case("111a.11aa")]
    #[case("11.22.33")]
    #[case("1.11aa")]
    #[case("1.-1")]
    #[case("1,111,111")]
    fn test_deserialization_failures(#[case] bad_input: &str) {
        assert!(parse_u128_fixed(Some(bad_input.to_string())).is_err());
    }

    #[rstest]
    #[case(0, "0.0000")]
    #[case(1, "0.0001")]
    #[case(11, "0.0011")]
    #[case(100, "0.0100")]
    #[case(10_000, "1.0000")]
    #[case(11_111_111, "1111.1111")]
    fn test_format_u128_fixed(#[case] value: u128, #[case] expected: &str) {
        assert_eq!(format_u128_fixed(value), expected);
    }

    #[rstest]
    #[case("1111.1111")]
    #[case("0.0001")]
    #[case("111.1100")]
    fn test_round_trip(#[case] canonical: &str) {
        let parsed = parse_u128_fixed(Some(canonical.to_string()))
            .unwrap()
            .unwrap();
        assert_eq!(format_u128_fixed(parsed), canonical);
    }
}
