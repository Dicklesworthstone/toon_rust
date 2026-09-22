use std::fmt::Write;

use crate::JsonPrimitive;
use crate::StringOrNumberOrBoolOrNull;
use crate::shared::constants::{DEFAULT_DELIMITER, DOUBLE_QUOTE};
use crate::shared::string_utils::escape_string;
use crate::shared::validation::{is_safe_unquoted, is_valid_unquoted_key};

#[must_use]
pub fn encode_primitive(value: &JsonPrimitive, delimiter: char) -> String {
    match value {
        StringOrNumberOrBoolOrNull::Null => "null".to_string(),
        StringOrNumberOrBoolOrNull::Bool(value) => value.to_string(),
        StringOrNumberOrBoolOrNull::Number(value) => format_number(*value),
        StringOrNumberOrBoolOrNull::String(value) => encode_string_literal(value, delimiter),
    }
}

#[must_use]
pub fn encode_string_literal(value: &str, delimiter: char) -> String {
    if is_safe_unquoted(value, delimiter) {
        return value.to_string();
    }
    format!("{DOUBLE_QUOTE}{}{DOUBLE_QUOTE}", escape_string(value))
}

#[must_use]
pub fn encode_key(key: &str) -> String {
    if is_valid_unquoted_key(key) {
        return key.to_string();
    }
    format!("{DOUBLE_QUOTE}{}{DOUBLE_QUOTE}", escape_string(key))
}

#[must_use]
pub fn encode_and_join_primitives(values: &[JsonPrimitive], delimiter: char) -> String {
    if values.is_empty() {
        return String::new();
    }
    // Estimate: average 10 chars per primitive + delimiter
    let mut out = String::with_capacity(values.len() * 11);
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(delimiter);
        }
        out.push_str(&encode_primitive(value, delimiter));
    }
    out
}

#[must_use]
pub fn format_header(
    length: usize,
    key: Option<&str>,
    fields: Option<&[String]>,
    delimiter: char,
) -> String {
    let mut header = String::new();

    if let Some(key) = key {
        header.push_str(&encode_key(key));
    }

    if delimiter == DEFAULT_DELIMITER {
        let _ = write!(header, "[{length}]");
    } else {
        let _ = write!(header, "[{length}{delimiter}]");
    }

    if let Some(fields) = fields {
        header.push('{');
        for (idx, field) in fields.iter().enumerate() {
            if idx > 0 {
                header.push(delimiter);
            }
            header.push_str(&encode_key(field));
        }
        header.push('}');
    }

    header.push(':');
    header
}

/// The shortest decimal digits that read back as `value` (ties between two shortest candidates
/// go to the even digit, as in JavaScript's `Number::toString`), with the decimal exponent `n`
/// such that `|value| = 0.d1 d2 … dk × 10^n`. `None` for zero and non-finite values.
fn shortest_digits(value: f64) -> Option<(bool, String, i32)> {
    if value == 0.0 || !value.is_finite() {
        return None;
    }
    // serde_json's writer produces the shortest round-trip text with even tie-breaking; only
    // its layout (`1.0`, `1e+21`, `1e-7`) is rewritten here.
    let text = serde_json::Number::from_f64(value.abs())?.to_string();
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (text.as_str(), 0),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut point = i32::try_from(int_part.len()).ok()? + exponent;
    let mut digits: String = int_part.chars().chain(frac_part.chars()).collect();
    while digits.starts_with('0') {
        digits.remove(0);
        point -= 1;
    }
    while digits.ends_with('0') {
        digits.pop();
    }
    if digits.is_empty() {
        return None;
    }
    Some((value.is_sign_negative(), digits, point))
}

/// TOON number text (spec §2): canonical decimal, never an exponent, `-0` as `0`, and enough
/// digits to read back as the same binary64. Non-finite values are `null` (§3).
fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "null".to_string();
    }
    let Some((negative, digits, point)) = shortest_digits(value) else {
        return "0".to_string();
    };
    let mut out = String::with_capacity(digits.len() + 8);
    if negative {
        out.push('-');
    }
    let k = digits.len();
    match usize::try_from(point) {
        Ok(n) if n >= k => {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', n - k));
        }
        Ok(n) if n > 0 => {
            out.push_str(&digits[..n]);
            out.push('.');
            out.push_str(&digits[n..]);
        }
        _ => {
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', point.unsigned_abs() as usize));
            out.push_str(&digits);
        }
    }
    out
}

/// JSON number text as JavaScript's `JSON.stringify` writes it.
///
/// That is ECMA-262 `Number::toString`: integers without a fraction (`1`, not `1.0`), plain decimals for `1e-7 < |x| < 1e21`, and
/// `1e+21` / `1.5e-7` outside that range. Non-finite values are `null`.
#[must_use]
pub fn format_json_number(value: f64) -> String {
    if !value.is_finite() {
        return "null".to_string();
    }
    let Some((negative, digits, point)) = shortest_digits(value) else {
        return "0".to_string();
    };
    if (-5..=21).contains(&point) {
        return format_number(value);
    }
    let mut out = String::with_capacity(digits.len() + 8);
    if negative {
        out.push('-');
    }
    out.push_str(&digits[..1]);
    if digits.len() > 1 {
        out.push('.');
        out.push_str(&digits[1..]);
    }
    let exponent = point - 1;
    let _ = write!(
        out,
        "e{}{}",
        if exponent < 0 { '-' } else { '+' },
        exponent.abs()
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_null_primitive() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Null, ','),
            "null"
        );
    }

    #[test]
    fn encode_bool_primitive() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Bool(true), ','),
            "true"
        );
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Bool(false), ','),
            "false"
        );
    }

    #[test]
    fn encode_number_zero_is_bare_zero() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Number(0.0), ','),
            "0"
        );
    }

    #[test]
    fn encode_number_nan_is_null() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Number(f64::NAN), ','),
            "null"
        );
    }

    #[test]
    fn encode_number_infinity_is_null() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Number(f64::INFINITY), ','),
            "null"
        );
    }

    #[test]
    fn encode_number_is_canonical_decimal_without_exponent() {
        let text = |v: f64| encode_primitive(&StringOrNumberOrBoolOrNull::Number(v), ',');
        assert_eq!(text(1.0), "1");
        assert_eq!(text(-0.0), "0");
        assert_eq!(text(1.5), "1.5");
        assert_eq!(text(1e21), "1000000000000000000000");
        assert_eq!(text(1e-7), "0.0000001");
        assert_eq!(text(-123.456), "-123.456");
        assert_eq!(text(0.1 + 0.2), "0.30000000000000004");
        // An exact tie between two shortest candidates takes the even digit, as JavaScript's
        // `String(x)` does; rounding it up gave a second spelling of the same value.
        // 2101031963024178.25, built exactly (the literal itself would read as the .2 text).
        let tie = 8_404_127_852_096_713.0 / 4.0;
        assert_eq!(text(tie), "2101031963024178.2");
        for v in [f64::MAX, f64::MIN_POSITIVE, 5e-324, 123_456.789, -9.87e-12] {
            assert_eq!(text(v).parse::<f64>(), Ok(v), "{v} must read back exactly");
        }
    }

    #[test]
    fn encode_simple_string_is_unquoted() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::String("hello".into()), ','),
            "hello"
        );
    }

    #[test]
    fn encode_string_with_comma_is_quoted_when_delimiter_is_comma() {
        let out = encode_primitive(&StringOrNumberOrBoolOrNull::String("a,b".into()), ',');
        assert!(out.starts_with('"'));
        assert!(out.ends_with('"'));
        assert!(out.contains("a,b"));
    }

    #[test]
    fn encode_string_with_newline_is_escaped_and_quoted() {
        let out = encode_string_literal("line\nfeed", ',');
        assert_eq!(out, "\"line\\nfeed\"");
    }

    #[test]
    fn encode_string_that_looks_like_bool_is_quoted() {
        let out = encode_string_literal("true", ',');
        assert_eq!(out, "\"true\"");
    }

    #[test]
    fn encode_key_valid_is_unquoted() {
        assert_eq!(encode_key("valid_key"), "valid_key");
    }

    #[test]
    fn encode_key_with_space_is_quoted() {
        let out = encode_key("has space");
        assert!(out.starts_with('"'));
        assert!(out.contains("has space"));
    }

    #[test]
    fn encode_key_with_quotes_is_escaped() {
        let out = encode_key("a\"b");
        assert_eq!(out, "\"a\\\"b\"");
    }

    #[test]
    fn encode_and_join_primitives_empty_is_empty() {
        assert_eq!(encode_and_join_primitives(&[], ','), "");
    }

    #[test]
    fn encode_and_join_primitives_joins_with_delimiter() {
        let vals = vec![
            StringOrNumberOrBoolOrNull::Number(1.0),
            StringOrNumberOrBoolOrNull::String("two".into()),
            StringOrNumberOrBoolOrNull::Bool(true),
        ];
        assert_eq!(encode_and_join_primitives(&vals, ','), "1,two,true");
    }

    #[test]
    fn encode_and_join_primitives_different_delimiters() {
        let vals = vec![
            StringOrNumberOrBoolOrNull::Number(1.0),
            StringOrNumberOrBoolOrNull::Number(2.0),
        ];
        assert_eq!(encode_and_join_primitives(&vals, '|'), "1|2");
        assert_eq!(encode_and_join_primitives(&vals, '\t'), "1\t2");
    }

    #[test]
    fn format_header_length_only_default_delimiter() {
        assert_eq!(format_header(3, None, None, ','), "[3]:");
    }

    #[test]
    fn format_header_length_only_custom_delimiter() {
        assert_eq!(format_header(3, None, None, '|'), "[3|]:");
    }

    #[test]
    fn format_header_with_key_and_fields() {
        let fields = vec!["id".to_string(), "name".to_string()];
        assert_eq!(
            format_header(2, Some("users"), Some(&fields), ','),
            "users[2]{id,name}:"
        );
    }

    #[test]
    fn format_header_with_quoted_field_name() {
        let fields = vec!["weird name".to_string()];
        let out = format_header(1, Some("data"), Some(&fields), ',');
        assert!(out.contains("{\"weird name\"}"));
    }

    #[test]
    fn format_header_zero_length() {
        assert_eq!(format_header(0, Some("items"), None, ','), "items[0]:");
    }
}
