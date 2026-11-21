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

/// Format an array header from an already encoded key and field names.
#[must_use]
pub fn format_header(
    length: usize,
    key: Option<&str>,
    fields: Option<&[String]>,
    delimiter: char,
) -> String {
    let mut header = String::new();

    if let Some(key) = key {
        header.push_str(key);
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
            header.push_str(field);
        }
        header.push('}');
    }

    header.push(':');
    header
}

fn parse_shortest_decimal(negative: bool, text: &str) -> Option<(bool, String, i32)> {
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (text, 0),
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
    Some((negative, digits, point))
}

/// Returns `true` iff positive finite `abs_v` equals `mantissa * 10^exp10` as an exact real number.
#[cfg(any(test, not(feature = "serde")))]
fn exact_eq_decimal(abs_v: f64, mantissa: u128, exp10: i32) -> bool {
    if mantissa == 0 || !abs_v.is_finite() || abs_v <= 0.0 {
        return false;
    }
    let bits = abs_v.to_bits();
    let raw_exp = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let raw_frac = bits & 0x000f_ffff_ffff_ffff;
    let (significand, exp2) = if raw_exp == 0 {
        (raw_frac, -1074_i32)
    } else {
        ((1_u64 << 52) | raw_frac, raw_exp - 1075)
    };
    let tz_bits = significand.trailing_zeros();
    let v_odd = u128::from(significand >> tz_bits);
    let Ok(tz_bits_i32) = i32::try_from(tz_bits) else {
        return false;
    };
    let v_exp2 = exp2 + tz_bits_i32;

    if exp10 >= 0 {
        // 5^23 > 2^53, so no finite f64 can equal `mantissa * 10^exp10` for exp10 > 22.
        let Ok(exp_u32) = u32::try_from(exp10) else {
            return false;
        };
        if exp_u32 > 22 {
            return false;
        }
        let pow5 = 5_u128.pow(exp_u32);
        let Some(scaled) = mantissa.checked_mul(pow5) else {
            return false;
        };
        let tz = scaled.trailing_zeros();
        let Ok(tz_i32) = i32::try_from(tz) else {
            return false;
        };
        (scaled >> tz) == v_odd && (exp10 + tz_i32) == v_exp2
    } else {
        // mantissa < 10^19 < 5^28, so 5^neg_exp can never divide mantissa for neg_exp > 27.
        let neg_exp = exp10.unsigned_abs();
        if neg_exp > 27 {
            return false;
        }
        let pow5 = 5_u128.pow(neg_exp);
        #[allow(clippy::manual_is_multiple_of)]
        if mantissa % pow5 != 0 {
            return false;
        }
        let quot = mantissa / pow5;
        let tz = quot.trailing_zeros();
        let Ok(tz_i32) = i32::try_from(tz) else {
            return false;
        };
        (quot >> tz) == v_odd && (exp10 + tz_i32) == v_exp2
    }
}

/// Shortest decimal digits using Rust's stdlib formatter with ECMA-262 round-half-to-even
/// tie-breaking (Rust's `core::fmt::float` breaks exact shortest-decimal midpoints upward,
/// whereas ECMA-262 / TOON §2 picks the even last digit).
#[cfg(any(test, not(feature = "serde")))]
fn shortest_digits_std(value: f64) -> Option<(bool, String, i32)> {
    let abs_v = value.abs();
    let text = format!("{abs_v:e}");
    let (negative, digits, point) = parse_shortest_decimal(value.is_sign_negative(), &text)?;
    let last_byte = *digits.as_bytes().last()?;
    if (last_byte - b'0') & 1 == 0 {
        return Some((negative, digits, point));
    }

    let k = i32::try_from(digits.len()).ok()?;
    let unit_exp10 = point - k;
    let m: u128 = digits.parse().ok()?;

    // Check if `abs_v` is the exact midpoint between `m - 1` (even) and `m` (odd).
    if m > 1
        && let Some(mid_down) = (m - 1).checked_mul(10).and_then(|x| x.checked_add(5))
        && exact_eq_decimal(abs_v, mid_down, unit_exp10 - 1)
    {
        let candidate = format!("{}e{unit_exp10}", m - 1);
        if candidate.parse::<f64>() == Ok(abs_v) {
            return parse_shortest_decimal(negative, &candidate);
        }
    }

    // Check if `abs_v` is the exact midpoint between `m` (odd) and `m + 1` (even).
    if let Some(mid_up) = m.checked_mul(10).and_then(|x| x.checked_add(5))
        && exact_eq_decimal(abs_v, mid_up, unit_exp10 - 1)
    {
        let candidate = format!("{}e{unit_exp10}", m + 1);
        if candidate.parse::<f64>() == Ok(abs_v) {
            return parse_shortest_decimal(negative, &candidate);
        }
    }

    Some((negative, digits, point))
}

/// The shortest decimal digits that read back as `value` (ties between two shortest candidates
/// go to the even digit, as in JavaScript's `Number::toString`), with the decimal exponent `n`
/// such that `|value| = 0.d1 d2 … dk × 10^n`. `None` for zero and non-finite values.
fn shortest_digits(value: f64) -> Option<(bool, String, i32)> {
    if value == 0.0 || !value.is_finite() {
        return None;
    }
    // Shortest round-trip decimal with even tie-breaking. When `serde` is enabled, reuse
    // `serde_json`'s fast float formatter; otherwise use pure stdlib + exact tie-to-even so the
    // core `lib` feature carries zero third-party dependencies.
    #[cfg(feature = "serde")]
    {
        let text = serde_json::Number::from_f64(value.abs())?.to_string();
        parse_shortest_decimal(value.is_sign_negative(), &text)
    }
    #[cfg(not(feature = "serde"))]
    {
        shortest_digits_std(value)
    }
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
/// That is ECMA-262 `Number::toString`: integers without a fraction (`1`, not `1.0`), plain
/// decimals for `1e-7 < |x| < 1e21`, and `1e+21` / `1.5e-7` outside that range. Non-finite
/// values are `null`.
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

    #[cfg(feature = "serde")]
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn stdlib_and_serde_shortest_digits_match_across_edge_cases() {
        let mut samples = vec![
            1.0,
            -1.0,
            1.5,
            0.1 + 0.2,
            1e21,
            1e-7,
            1e-6,
            1e20,
            -123.456,
            8_404_127_852_096_713.0 / 4.0,
            f64::MAX,
            f64::MIN,
            f64::MIN_POSITIVE,
            5e-324,
            18_446_744_073_709_551_615.0,
        ];
        // Constructed exact binary64 decimal midpoints (multiples of 0.25, 0.5, 0.05, etc. near 2^52).
        for offset in 0_u64..128 {
            let base = (8_404_127_852_096_713_u64 + offset * 2) as f64;
            samples.push(base / 4.0);
            samples.push(-base / 4.0);
            samples.push(base / 2.0);
            samples.push(base * 5.0);
        }
        // Deterministic bit-pattern sweep across exponents and mantissas (including subnormals).
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        for _ in 0..16_384 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let v = f64::from_bits(state);
            if v.is_finite() && v != 0.0 {
                samples.push(v);
            }
        }
        for v in samples {
            let serde_text = serde_json::Number::from_f64(v.abs()).unwrap().to_string();
            let serde_parsed = parse_shortest_decimal(v.is_sign_negative(), &serde_text);
            let std_parsed = shortest_digits_std(v);
            assert_eq!(
                std_parsed,
                serde_parsed,
                "mismatch for {v:?} (bits={:#018x}, serde={serde_text})",
                v.to_bits()
            );
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
        let fields = vec![encode_key("weird name")];
        let out = format_header(1, Some("data"), Some(&fields), ',');
        assert!(out.contains("{\"weird name\"}"));
    }

    #[test]
    fn format_header_zero_length() {
        assert_eq!(format_header(0, Some("items"), None, ','), "items[0]:");
    }
}
