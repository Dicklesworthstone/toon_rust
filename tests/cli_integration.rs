//! CLI integration tests for the `tru` binary.
//!
//! These tests exercise the actual binary using `assert_cmd` to ensure
//! end-to-end functionality works correctly.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get a Command for the `toon` binary
fn tru() -> Command {
    Command::new(env!("CARGO_BIN_EXE_toon"))
}

// ============================================================================
// Encode Tests (JSON -> TOON)
// ============================================================================

#[test]
fn encode_simple_json_to_stdout() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"name":"Alice","age":30}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("name: Alice"))
        .stdout(predicate::str::contains("age: 30"));
}

#[test]
fn encode_from_json_file() {
    let tmp = TempDir::new().unwrap();
    let input_path = tmp.path().join("input.json");
    fs::write(&input_path, r#"{"key":"value"}"#).unwrap();

    tru()
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("key: value"));
}

#[test]
fn encode_to_output_file() {
    let tmp = TempDir::new().unwrap();
    let input_path = tmp.path().join("input.json");
    let output_path = tmp.path().join("output.toon");
    fs::write(&input_path, r#"{"hello":"world"}"#).unwrap();

    tru()
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("Encoded"));

    let output = fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("hello: world"));
}

#[test]
fn encode_nested_object() {
    let json = r#"{"user":{"name":"Bob","email":"bob@example.com"}}"#;

    tru()
        .arg("--encode")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("user:"))
        .stdout(predicate::str::contains("name: Bob"))
        .stdout(predicate::str::contains("email: bob@example.com"));
}

#[test]
fn encode_array_inline() {
    let json = r#"{"items":["a","b","c"]}"#;

    tru()
        .arg("--encode")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("items[3]: a,b,c"));
}

#[test]
fn encode_with_custom_indent() {
    let json = r#"{"outer":{"inner":"value"}}"#;

    tru()
        .arg("--encode")
        .arg("--indent")
        .arg("4")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("    inner: value"));
}

#[test]
fn encode_with_pipe_delimiter() {
    let json = r#"{"items":["x","y","z"]}"#;

    tru()
        .arg("--encode")
        .arg("--delimiter")
        .arg("|")
        .write_stdin(json)
        .assert()
        .success()
        // Delimiter is used both in array header and values
        .stdout(predicate::str::contains("items[3|]: x|y|z"));
}

#[test]
fn encode_with_key_folding_safe() {
    let json = r#"{"data":{"meta":{"items":["x","y"]}}}"#;

    tru()
        .arg("--encode")
        .arg("--key-folding")
        .arg("safe")
        .write_stdin(json)
        .assert()
        .success()
        .stdout(predicate::str::contains("data.meta.items[2]: x,y"));
}

#[test]
fn encode_with_stats_flag() {
    let json = r#"{"name":"Alice","description":"This is a longer description text"}"#;

    tru()
        .arg("--encode")
        .arg("--stats")
        .write_stdin(json)
        .assert()
        .success()
        .stderr(predicate::str::contains("Token estimates"));
}

#[test]
fn encode_rejects_invalid_json() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"invalid": }"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse JSON"));
}

// ============================================================================
// Decode Tests (TOON -> JSON)
// ============================================================================

#[test]
fn decode_simple_toon_to_stdout() {
    let toon = "name: Alice\nage: 30";

    tru()
        .arg("--decode")
        .write_stdin(toon)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "Alice""#))
        .stdout(predicate::str::contains(r#""age": 30"#));
}

#[test]
fn decode_from_toon_file() {
    let tmp = TempDir::new().unwrap();
    let input_path = tmp.path().join("input.toon");
    fs::write(&input_path, "key: value").unwrap();

    tru()
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""key": "value""#));
}

#[test]
fn decode_to_output_file() {
    let tmp = TempDir::new().unwrap();
    let input_path = tmp.path().join("input.toon");
    let output_path = tmp.path().join("output.json");
    fs::write(&input_path, "hello: world").unwrap();

    tru()
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("Decoded"));

    let output = fs::read_to_string(&output_path).unwrap();
    assert!(output.contains(r#""hello": "world""#));
}

#[test]
fn decode_nested_object() {
    let toon = "user:\n  name: Bob\n  email: bob@example.com";

    tru()
        .arg("--decode")
        .write_stdin(toon)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""user""#))
        .stdout(predicate::str::contains(r#""name": "Bob""#));
}

#[test]
fn decode_array() {
    let toon = "items[3]: a,b,c";

    tru()
        .arg("--decode")
        .write_stdin(toon)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""items""#))
        .stdout(predicate::str::contains(r#"["a", "b", "c"]"#).or(
            // Alternative formatting
            predicate::str::contains(
                r#"[
    "a",
    "b",
    "c"
  ]"#,
            ),
        ));
}

#[test]
fn decode_with_expand_paths_safe() {
    let toon = "a.b.c: 42";

    tru()
        .arg("--decode")
        .arg("--expand-paths")
        .arg("safe")
        .write_stdin(toon)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""a""#))
        .stdout(predicate::str::contains(r#""b""#))
        .stdout(predicate::str::contains(r#""c": 42"#));
}

#[test]
fn decode_with_no_strict_allows_invalid_indentation() {
    // Non-multiple of indent size (3 spaces with default indent 2)
    let toon = "outer:\n   inner: value";

    // With strict mode (default), this should fail
    tru().arg("--decode").write_stdin(toon).assert().failure();

    // With no-strict, it should succeed
    tru()
        .arg("--decode")
        .arg("--no-strict")
        .write_stdin(toon)
        .assert()
        .success();
}

// ============================================================================
// Mode Auto-Detection Tests
// ============================================================================

#[test]
fn auto_detect_encode_from_json_extension() {
    let tmp = TempDir::new().unwrap();
    let input_path = tmp.path().join("data.json");
    fs::write(&input_path, r#"{"auto":"detect"}"#).unwrap();

    // Should auto-detect encode mode from .json extension
    tru()
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("auto: detect"));
}

#[test]
fn auto_detect_decode_from_toon_extension() {
    let tmp = TempDir::new().unwrap();
    let input_path = tmp.path().join("data.toon");
    fs::write(&input_path, "auto: detect").unwrap();

    // Should auto-detect decode mode from .toon extension
    tru()
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""auto": "detect""#));
}

#[test]
fn explicit_mode_overrides_extension() {
    let tmp = TempDir::new().unwrap();
    // Create a .toon file but force encode mode
    let input_path = tmp.path().join("data.toon");
    fs::write(&input_path, r#"{"force":"encode"}"#).unwrap();

    tru()
        .arg("--encode")
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("force: encode"));
}

// ============================================================================
// Roundtrip Tests
// ============================================================================

#[test]
fn roundtrip_simple_object() {
    let original_json = r#"{"name":"Alice","age":30,"active":true}"#;

    // Encode JSON -> TOON
    let encode_output = tru()
        .arg("--encode")
        .write_stdin(original_json)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let toon = String::from_utf8(encode_output).unwrap();

    // Decode TOON -> JSON
    let decode_output = tru()
        .arg("--decode")
        .write_stdin(toon.trim())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result_json = String::from_utf8(decode_output).unwrap();

    // Parse both and compare (using flexible comparison for numeric types)
    let original: serde_json::Value = serde_json::from_str(original_json).unwrap();
    let result: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    assert_json_values_equal(&original, &result);
}

/// Compare JSON values with flexible numeric handling (integer vs float with same value)
fn assert_json_values_equal(left: &serde_json::Value, right: &serde_json::Value) {
    match (left, right) {
        (serde_json::Value::Null, serde_json::Value::Null) => {}
        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => assert_eq!(a, b),
        (serde_json::Value::String(a), serde_json::Value::String(b)) => assert_eq!(a, b),
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
            let a_f64 = a.as_f64().unwrap();
            let b_f64 = b.as_f64().unwrap();
            assert!(
                (a_f64 - b_f64).abs() < f64::EPSILON,
                "Numbers differ: {a_f64} vs {b_f64}"
            );
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "Array lengths differ");
            for (av, bv) in a.iter().zip(b.iter()) {
                assert_json_values_equal(av, bv);
            }
        }
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "Object sizes differ");
            for (key, av) in a {
                let bv = b.get(key).unwrap_or_else(|| panic!("Missing key: {key}"));
                assert_json_values_equal(av, bv);
            }
        }
        _ => panic!("Type mismatch: {left:?} vs {right:?}"),
    }
}

#[test]
fn roundtrip_nested_structure() {
    let original_json = r#"{"user":{"profile":{"name":"Bob","settings":{"theme":"dark"}}}}"#;

    // Encode
    let encode_output = tru()
        .arg("--encode")
        .write_stdin(original_json)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let toon = String::from_utf8(encode_output).unwrap();

    // Decode
    let decode_output = tru()
        .arg("--decode")
        .write_stdin(toon.trim())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result_json = String::from_utf8(decode_output).unwrap();

    // Compare
    let original: serde_json::Value = serde_json::from_str(original_json).unwrap();
    let result: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(original, result);
}

#[test]
fn roundtrip_with_array() {
    let original_json = r#"{"items":["apple","banana","cherry"]}"#;

    // Encode
    let encode_output = tru()
        .arg("--encode")
        .write_stdin(original_json)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let toon = String::from_utf8(encode_output).unwrap();

    // Decode
    let decode_output = tru()
        .arg("--decode")
        .write_stdin(toon.trim())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result_json = String::from_utf8(decode_output).unwrap();

    // Compare
    let original: serde_json::Value = serde_json::from_str(original_json).unwrap();
    let result: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(original, result);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn error_on_missing_input_file() {
    tru()
        .arg("/nonexistent/path/file.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read file"));
}

#[test]
fn error_on_invalid_delimiter() {
    tru()
        .arg("--encode")
        .arg("--delimiter")
        .arg("invalid")
        .write_stdin("{}")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid delimiter"));
}

#[test]
fn error_on_invalid_indent() {
    tru()
        .arg("--encode")
        .arg("--indent")
        .arg("99")
        .write_stdin("{}")
        .assert()
        .failure();
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn handle_empty_object() {
    tru().arg("--encode").write_stdin("{}").assert().success();
}

#[test]
fn handle_empty_array() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"empty":[]}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("empty[0]:"));
}

#[test]
fn handle_null_value() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"value":null}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("value: null"));
}

#[test]
fn handle_boolean_values() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"yes":true,"no":false}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("yes: true"))
        .stdout(predicate::str::contains("no: false"));
}

#[test]
fn handle_numeric_values() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"integer":42,"float":3.14,"negative":-1}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("integer: 42"))
        .stdout(predicate::str::contains("float: 3.14"))
        .stdout(predicate::str::contains("negative: -1"));
}

#[test]
fn handle_special_characters_in_string() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"text":"hello\nworld"}"#)
        .assert()
        .success();
}

#[test]
fn handle_unicode() {
    tru()
        .arg("--encode")
        .write_stdin(r#"{"greeting":"こんにちは","emoji":"🎉"}"#)
        .assert()
        .success();
}

#[test]
fn stdin_dash_argument() {
    // Using "-" explicitly should read from stdin
    tru()
        .arg("-")
        .arg("--encode")
        .write_stdin(r#"{"stdin":"dash"}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("stdin: dash"));
}

// ============================================================================
// Help and Version
// ============================================================================

#[test]
fn help_flag_shows_usage() {
    tru()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("TOON"))
        .stdout(predicate::str::contains("--encode"))
        .stdout(predicate::str::contains("--decode"))
        .stdout(predicate::str::contains("EXAMPLES"));
}

#[test]
fn version_flag_shows_version() {
    tru()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("tru"));
}
