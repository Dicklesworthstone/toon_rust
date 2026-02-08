use tru::cli::conversion::{decode_to_json_chunks, encode_to_toon_lines};
use tru::options::{DecodeOptions, EncodeOptions, ExpandPathsMode, KeyFoldingMode};

#[test]
fn encode_to_toon_lines_matches_encode() {
    let input = r#"{"name":"Ada","id":1}"#;
    let expected = tru::encode::encode(
        tru::JsonValue::from(serde_json::from_str::<serde_json::Value>(input).unwrap()),
        None,
    );
    let actual = encode_to_toon_lines(input, None).unwrap().join("\n");
    assert_eq!(actual, expected);
}

#[test]
fn encode_to_toon_lines_rejects_invalid_json() {
    let input = r#"{"name": }"#;
    let err = encode_to_toon_lines(input, None).unwrap_err();
    assert!(err.to_string().contains("Failed to parse JSON"));
}

#[test]
fn decode_to_json_chunks_streams_without_expand_paths() {
    let input = "items[2]: a,b";
    let options = DecodeOptions {
        indent: Some(2),
        strict: Some(true),
        expand_paths: Some(ExpandPathsMode::Off),
    };

    let output = decode_to_json_chunks(input, Some(options))
        .unwrap()
        .concat();
    let expected = serde_json::to_string_pretty(&serde_json::json!({"items": ["a", "b"]})).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn decode_to_json_chunks_expands_paths_when_enabled() {
    let input = "a.b: 1";
    let options = DecodeOptions {
        indent: Some(0),
        strict: Some(true),
        expand_paths: Some(ExpandPathsMode::Safe),
    };

    let output = decode_to_json_chunks(input, Some(options))
        .unwrap()
        .concat();
    let expected = serde_json::json!({"a": {"b": 1}});
    let actual: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_json_eq(&actual, &expected);
}

#[test]
fn encode_to_toon_lines_respects_options() {
    let input = r#"{"data":{"meta":{"items":["x","y"]}}}"#;
    let options = EncodeOptions {
        indent: Some(2),
        delimiter: Some(','),
        key_folding: Some(KeyFoldingMode::Safe),
        flatten_depth: Some(usize::MAX),
        replacer: None,
    };

    let output = encode_to_toon_lines(input, Some(options))
        .unwrap()
        .join("\n");
    assert_eq!(output, "data.meta.items[2]: x,y");
}

fn assert_json_eq(actual: &serde_json::Value, expected: &serde_json::Value) {
    if json_eq(actual, expected) {
        return;
    }
    panic!("expected {expected:?}, got {actual:?}");
}

fn json_eq(left: &serde_json::Value, right: &serde_json::Value) -> bool {
    match (left, right) {
        (serde_json::Value::Null, serde_json::Value::Null) => true,
        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => a == b,
        (serde_json::Value::String(a), serde_json::Value::String(b)) => a == b,
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
            match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() <= 1e-12,
                _ => false,
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|(a, b)| json_eq(a, b))
        }
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter()
                .all(|(key, value)| b.get(key).is_some_and(|other| json_eq(value, other)))
        }
        _ => false,
    }
}
