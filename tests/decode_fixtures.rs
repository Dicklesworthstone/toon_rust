use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use tru::JsonValue;
use tru::decode::decode;
use tru::options::{DecodeOptions, ExpandPathsMode};

#[derive(Debug, Deserialize)]
struct FixtureFile {
    tests: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    name: String,
    input: String,
    expected: Option<serde_json::Value>,
    #[serde(alias = "expectedJson")]
    expected_json: Option<serde_json::Value>,
    options: Option<serde_json::Value>,
    #[serde(alias = "shouldError")]
    should_error: Option<bool>,
}

#[test]
fn decode_spec_fixtures() {
    let root = fixture_root();
    let decode_dir = root.join("decode");
    let files = load_fixture_files(&decode_dir);

    assert!(!files.is_empty(), "no decode fixtures found");

    for file in files {
        for case in file.tests {
            run_case(&case);
        }
    }
}

fn run_case(case: &FixtureCase) {
    let options = parse_decode_options(case.options.as_ref());
    let should_error = case.should_error.unwrap_or(false);

    let result = std::panic::catch_unwind(|| decode(&case.input, options));

    if should_error {
        assert!(
            result.is_err(),
            "expected error for fixture '{}' but decode succeeded",
            case.name
        );
        return;
    }

    let Ok(value) = result else {
        panic!("unexpected panic for fixture '{}'", case.name)
    };

    let expected = case
        .expected_json
        .clone()
        .or_else(|| case.expected.clone())
        .unwrap_or_else(|| serde_json::Value::Null);

    let actual = json_value_to_serde(value);
    assert_json_eq(&actual, &expected, &case.name);
}

fn parse_decode_options(options: Option<&serde_json::Value>) -> Option<DecodeOptions> {
    let options = options?;

    let indent = options
        .get("indent")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    let strict = options.get("strict").and_then(serde_json::Value::as_bool);
    let expand_paths = options
        .get("expandPaths")
        .and_then(|value| value.as_str())
        .and_then(|value| match value {
            "safe" => Some(ExpandPathsMode::Safe),
            "off" => Some(ExpandPathsMode::Off),
            _ => None,
        });

    Some(DecodeOptions {
        indent,
        strict,
        expand_paths,
    })
}

fn json_value_to_serde(value: JsonValue) -> serde_json::Value {
    match value {
        JsonValue::Primitive(primitive) => match primitive {
            tru::StringOrNumberOrBoolOrNull::Null => serde_json::Value::Null,
            tru::StringOrNumberOrBoolOrNull::Bool(value) => serde_json::Value::Bool(value),
            tru::StringOrNumberOrBoolOrNull::Number(value) => serde_json::Number::from_f64(value)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            tru::StringOrNumberOrBoolOrNull::String(value) => serde_json::Value::String(value),
        },
        JsonValue::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(json_value_to_serde).collect())
        }
        JsonValue::Object(entries) => {
            let mut map = serde_json::Map::new();
            for (key, value) in entries {
                map.insert(key, json_value_to_serde(value));
            }
            serde_json::Value::Object(map)
        }
    }
}

fn assert_json_eq(actual: &serde_json::Value, expected: &serde_json::Value, name: &str) {
    if json_eq(actual, expected) {
        return;
    }

    panic!("fixture '{name}' failed (expected {expected:?}, got {actual:?})");
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

#[test]
fn expand_paths_respects_quoted_dotted_array_key() {
    let input = "\"a.b\"[1]: 1";
    let options = Some(DecodeOptions {
        indent: None,
        strict: Some(true),
        expand_paths: Some(ExpandPathsMode::Safe),
    });

    let value = decode(input, options);
    let actual = json_value_to_serde(value);
    let expected = serde_json::json!({ "a.b": [1] });

    assert_json_eq(&actual, &expected, "quoted dotted array key");
}

#[test]
fn expand_paths_respects_quoted_dotted_tabular_field() {
    let input = "items[1]{\"a.b\"}:\n  1";
    let options = Some(DecodeOptions {
        indent: None,
        strict: Some(true),
        expand_paths: Some(ExpandPathsMode::Safe),
    });

    let value = decode(input, options);
    let actual = json_value_to_serde(value);
    let expected = serde_json::json!({ "items": [ { "a.b": 1 } ] });

    assert_json_eq(&actual, &expected, "quoted dotted tabular field");
}

fn fixture_root() -> PathBuf {
    if let Ok(path) = std::env::var("TOON_SPEC_FIXTURES") {
        return PathBuf::from(path);
    }

    PathBuf::from("tests/fixtures/spec")
}

fn load_fixture_files(dir: &Path) -> Vec<FixtureFile> {
    if !dir.exists() {
        return Vec::new();
    }

    let mut fixtures = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return fixtures;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        let parsed: FixtureFile = match serde_json::from_str(&contents) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };

        fixtures.push(parsed);
    }

    fixtures
}
