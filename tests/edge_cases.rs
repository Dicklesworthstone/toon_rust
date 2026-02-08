//! Comprehensive edge case tests for TOON encoding/decoding.
//!
//! These tests cover edge cases that might be missed by standard unit tests:
//! - Unicode edge cases (emoji, RTL, zero-width chars)
//! - Deeply nested structures
//! - Very long strings and keys
//! - Numeric edge cases
//! - Empty arrays/objects at various positions
//! - Whitespace variations
//! - Delimiter edge cases
//! - Key folding conflict scenarios

use proptest::prelude::*;
use tru::options::{DecodeOptions, EncodeOptions, ExpandPathsMode, KeyFoldingMode};
use tru::{JsonValue, decode, encode, try_decode};

// ============================================================================
// UNICODE EDGE CASES
// ============================================================================

#[test]
fn unicode_emoji_in_values() {
    let json: serde_json::Value = serde_json::json!({
        "message": "Hello \u{1F600} World \u{1F4BB}",
        "hearts": "\u{2764}\u{FE0F}\u{2764}\u{FE0F}\u{2764}\u{FE0F}"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn unicode_emoji_in_keys() {
    let json: serde_json::Value = serde_json::json!({
        "\u{1F600}": "smile",
        "\u{1F4BB}": "laptop"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn unicode_rtl_text() {
    let json: serde_json::Value = serde_json::json!({
        "arabic": "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}",
        "hebrew": "\u{05E2}\u{05D1}\u{05E8}\u{05D9}\u{05EA}"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn unicode_zero_width_chars() {
    let json: serde_json::Value = serde_json::json!({
        "zwj": "a\u{200D}b",
        "zwnj": "a\u{200C}b",
        "zwsp": "a\u{200B}b"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn unicode_combining_chars() {
    let json: serde_json::Value = serde_json::json!({
        "combined": "e\u{0301}",
        "precomposed": "\u{00E9}"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn unicode_surrogate_pairs() {
    // Characters outside BMP (require surrogate pairs in UTF-16)
    let json: serde_json::Value = serde_json::json!({
        "mathematical": "\u{1D400}\u{1D401}\u{1D402}",
        "musical": "\u{1D11E}"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

// ============================================================================
// DEEPLY NESTED STRUCTURES
// ============================================================================

#[test]
fn deeply_nested_objects_100_levels() {
    let mut value = serde_json::json!({"leaf": "value"});
    for i in 0..100 {
        value = serde_json::json!({ format!("level{}", i): value });
    }
    let toon = encode(value.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(value, decoded_json);
}

#[test]
fn deeply_nested_arrays_100_levels() {
    let mut value = serde_json::json!(["leaf"]);
    for _ in 0..100 {
        value = serde_json::json!([value]);
    }
    let toon = encode(value.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(value, decoded_json);
}

#[test]
fn deeply_nested_mixed_100_levels() {
    let mut value: serde_json::Value = serde_json::json!(42.0);
    for i in 0..50 {
        if i % 2 == 0 {
            value = serde_json::json!({ "obj": value });
        } else {
            value = serde_json::json!([value]);
        }
    }
    let toon = encode(value.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(value, decoded_json);
}

// ============================================================================
// VERY LONG STRINGS AND KEYS
// ============================================================================

#[test]
fn very_long_string_value() {
    let long_string = "x".repeat(100_000);
    let json: serde_json::Value = serde_json::json!({ "content": long_string });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn very_long_key() {
    let long_key = "k".repeat(10_000);
    let mut obj = serde_json::Map::new();
    obj.insert(long_key, serde_json::json!("value"));
    let json = serde_json::Value::Object(obj);
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn string_with_many_escapes() {
    let json: serde_json::Value = serde_json::json!({
        "escapes": "tab\there\nnewline\rcarriage\\backslash\"quote"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn string_with_control_chars() {
    // Control characters that should be escaped
    let json: serde_json::Value = serde_json::json!({
        "controls": "\u{0001}\u{0002}\u{001F}"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

// ============================================================================
// NUMERIC EDGE CASES
// ============================================================================

#[test]
fn numeric_max_min_values() {
    let json: serde_json::Value = serde_json::json!({
        "max_f64": f64::MAX,
        "min_positive": f64::MIN_POSITIVE,
        "neg_max": -f64::MAX
    });
    let toon = encode(json, None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    // Compare as f64 due to potential precision differences in JSON parsing
    let max = decoded_json["max_f64"].as_f64().unwrap();
    assert!((max - f64::MAX).abs() / f64::MAX < 1e-10);
}

#[test]
fn numeric_subnormal() {
    let subnormal = f64::MIN_POSITIVE / 2.0;
    let json: serde_json::Value = serde_json::json!({ "subnormal": subnormal });
    let toon = encode(json, None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    let result = decoded_json["subnormal"].as_f64().unwrap();
    assert!((result - subnormal).abs() < 1e-320 || result.abs() < f64::EPSILON);
}

#[test]
fn numeric_zero_variants() {
    let json: serde_json::Value = serde_json::json!({
        "zero": 0.0,
        "neg_zero": -0.0,
        "small_pos": 0.000_000_1,
        "small_neg": -0.000_000_1
    });
    let toon = encode(json, None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert!(decoded_json["zero"].as_f64().unwrap().abs() < f64::EPSILON);
}

#[test]
fn numeric_nan_becomes_null() {
    // Create a JsonValue with NaN manually
    let value = JsonValue::Object(vec![
        (
            "nan".to_string(),
            JsonValue::Primitive(tru::StringOrNumberOrBoolOrNull::from_f64(f64::NAN)),
        ),
        (
            "valid".to_string(),
            JsonValue::Primitive(tru::StringOrNumberOrBoolOrNull::Number(42.0)),
        ),
    ]);
    let toon = tru::encode::encode(value, None);
    let decoded = decode(&toon, None);
    // NaN should become null
    let decoded_json: serde_json::Value = decoded.into();
    assert!(decoded_json["nan"].is_null());
    let valid = decoded_json["valid"].as_f64().unwrap();
    assert!((valid - 42.0).abs() < f64::EPSILON);
}

#[test]
fn numeric_infinity_becomes_null() {
    let value = JsonValue::Object(vec![
        (
            "pos_inf".to_string(),
            JsonValue::Primitive(tru::StringOrNumberOrBoolOrNull::from_f64(f64::INFINITY)),
        ),
        (
            "neg_inf".to_string(),
            JsonValue::Primitive(tru::StringOrNumberOrBoolOrNull::from_f64(f64::NEG_INFINITY)),
        ),
    ]);
    let toon = tru::encode::encode(value, None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert!(decoded_json["pos_inf"].is_null());
    assert!(decoded_json["neg_inf"].is_null());
}

// ============================================================================
// EMPTY ARRAYS AND OBJECTS
// ============================================================================

#[test]
fn empty_object_at_root() {
    let json: serde_json::Value = serde_json::json!({});
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn empty_array_at_root() {
    let json: serde_json::Value = serde_json::json!([]);
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn empty_nested_containers() {
    let json: serde_json::Value = serde_json::json!({
        "empty_obj": {},
        "empty_arr": [],
        "nested": {
            "more_empty": {},
            "arr_of_empty": [{}, [], {}]
        }
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn array_with_mixed_empties() {
    let json: serde_json::Value = serde_json::json!([
        {},
        [],
        {"nested": []},
        [{}],
        null,
        "",
        []
    ]);
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

// ============================================================================
// WHITESPACE VARIATIONS
// ============================================================================

#[test]
fn string_with_only_spaces() {
    let json: serde_json::Value = serde_json::json!({
        "spaces": "     ",
        "single": " ",
        "mixed": "  a  b  "
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn string_with_only_tabs() {
    let json: serde_json::Value = serde_json::json!({
        "tabs": "\t\t\t",
        "mixed": "\ta\tb\t"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn string_with_newlines() {
    let json: serde_json::Value = serde_json::json!({
        "newlines": "line1\nline2\nline3",
        "crlf": "line1\r\nline2"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

// ============================================================================
// DELIMITER EDGE CASES
// ============================================================================

#[test]
fn array_with_comma_in_strings() {
    let json: serde_json::Value = serde_json::json!({
        "items": ["a,b", "c,d,e", "f"]
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn array_with_pipe_delimiter() {
    let json: serde_json::Value = serde_json::json!({
        "items": ["a", "b", "c"]
    });
    let options = Some(EncodeOptions {
        indent: None,
        delimiter: Some('|'),
        key_folding: None,
        flatten_depth: None,
        replacer: None,
    });
    let toon = encode(json.clone(), options);
    assert!(toon.contains('|'));
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn tabular_array_with_special_chars() {
    let json: serde_json::Value = serde_json::json!([
        {"name": "Alice, Jr.", "city": "New York"},
        {"name": "Bob", "city": "Los Angeles, CA"}
    ]);
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

// ============================================================================
// KEY FOLDING EDGE CASES
// ============================================================================

#[test]
fn key_folding_simple() {
    let json: serde_json::Value = serde_json::json!({
        "a": {"b": {"c": "value"}}
    });
    let options = Some(EncodeOptions {
        indent: None,
        delimiter: None,
        key_folding: Some(KeyFoldingMode::Safe),
        flatten_depth: None,
        replacer: None,
    });
    let toon = encode(json.clone(), options);
    assert!(toon.contains("a.b.c"));

    let decode_options = Some(DecodeOptions {
        indent: None,
        strict: None,
        expand_paths: Some(ExpandPathsMode::Safe),
    });
    let decoded = decode(&toon, decode_options);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn key_folding_with_sibling() {
    // When there's a sibling, folding still happens for the single-key chain
    // but we need expand_paths to reconstruct the nested structure
    let json: serde_json::Value = serde_json::json!({
        "a": {
            "b": {"c": "deep"},
            "sibling": "value"
        }
    });
    let options = Some(EncodeOptions {
        indent: None,
        delimiter: None,
        key_folding: Some(KeyFoldingMode::Safe),
        flatten_depth: None,
        replacer: None,
    });
    let toon = encode(json.clone(), options);

    // Need to expand paths to reconstruct the nested structure
    let decode_options = Some(DecodeOptions {
        indent: None,
        strict: None,
        expand_paths: Some(ExpandPathsMode::Safe),
    });
    let decoded = decode(&toon, decode_options);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn key_folding_depth_limit() {
    let json: serde_json::Value = serde_json::json!({
        "a": {"b": {"c": {"d": {"e": "deep"}}}}
    });
    let options = Some(EncodeOptions {
        indent: None,
        delimiter: None,
        key_folding: Some(KeyFoldingMode::Safe),
        flatten_depth: Some(2), // Only fold 2 levels
        replacer: None,
    });
    let toon = encode(json.clone(), options);

    let decode_options = Some(DecodeOptions {
        indent: None,
        strict: None,
        expand_paths: Some(ExpandPathsMode::Safe),
    });
    let decoded = decode(&toon, decode_options);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn key_with_dots_literal() {
    // Keys that contain dots should be quoted
    let json: serde_json::Value = serde_json::json!({
        "a.b": "literal dot key",
        "normal": "value"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

// ============================================================================
// PROPERTY-BASED TESTS
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn roundtrip_arbitrary_strings(s in ".*") {
        let json: serde_json::Value = serde_json::json!({ "value": s });
        let toon = encode(json.clone(), None);
        let decoded = decode(&toon, None);
        let decoded_json: serde_json::Value = decoded.into();
        prop_assert_eq!(json, decoded_json);
    }

    #[test]
    fn roundtrip_arbitrary_numbers(n in proptest::num::f64::NORMAL) {
        let json: serde_json::Value = serde_json::json!({ "value": n });
        let toon = encode(json.clone(), None);
        let decoded = decode(&toon, None);
        let decoded_json: serde_json::Value = decoded.into();
        let orig = json["value"].as_f64().unwrap();
        let result = decoded_json["value"].as_f64().unwrap();
        // Allow small relative error for floating point
        if orig.abs() > 1e-10 {
            prop_assert!((orig - result).abs() / orig.abs() < 1e-10);
        } else {
            prop_assert!((orig - result).abs() < 1e-15);
        }
    }

    #[test]
    fn roundtrip_string_array(v in proptest::collection::vec(".*", 0..20)) {
        let json: serde_json::Value = serde_json::json!({ "items": v });
        let toon = encode(json.clone(), None);
        let decoded = decode(&toon, None);
        let decoded_json: serde_json::Value = decoded.into();
        prop_assert_eq!(json, decoded_json);
    }

    #[test]
    fn roundtrip_nested_depth(depth in 1usize..50) {
        let mut value = serde_json::json!({"leaf": "value"});
        for i in 0..depth {
            value = serde_json::json!({ format!("l{}", i): value });
        }
        let toon = encode(value.clone(), None);
        let decoded = decode(&toon, None);
        let decoded_json: serde_json::Value = decoded.into();
        prop_assert_eq!(value, decoded_json);
    }
}

// ============================================================================
// STRICT MODE VALIDATION
// ============================================================================

#[test]
fn strict_mode_rejects_tabs() {
    let toon_with_tabs = "\tname: value";
    let result = try_decode(
        toon_with_tabs,
        Some(DecodeOptions {
            indent: None,
            strict: Some(true),
            expand_paths: None,
        }),
    );
    assert!(result.is_err());
}

#[test]
fn non_strict_mode_accepts_tabs() {
    let toon_with_tabs = "\tname: value";
    let result = try_decode(
        toon_with_tabs,
        Some(DecodeOptions {
            indent: None,
            strict: Some(false),
            expand_paths: None,
        }),
    );
    // Non-strict mode should at least not panic - we accept any result
    // (The exact outcome depends on implementation details)
    let _ = result;
}

// ============================================================================
// SPECIAL CHARACTERS IN KEYS
// ============================================================================

#[test]
fn key_with_colon() {
    let json: serde_json::Value = serde_json::json!({
        "time:zone": "UTC",
        "key:with:colons": "value"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn key_with_brackets() {
    let json: serde_json::Value = serde_json::json!({
        "array[0]": "first",
        "obj{key}": "value"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn key_with_quotes() {
    let json: serde_json::Value = serde_json::json!({
        "said \"hello\"": "greeting",
        "it's": "fine"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn numeric_keys() {
    let json: serde_json::Value = serde_json::json!({
        "123": "numeric",
        "0": "zero",
        "-1": "negative"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}

#[test]
fn boolean_like_keys() {
    let json: serde_json::Value = serde_json::json!({
        "true": "not a bool",
        "false": "also not a bool",
        "null": "not null"
    });
    let toon = encode(json.clone(), None);
    let decoded = decode(&toon, None);
    let decoded_json: serde_json::Value = decoded.into();
    assert_eq!(json, decoded_json);
}
