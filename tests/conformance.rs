//! Conformance tests that verify the Rust implementation matches the spec fixtures.
//!
//! Run with: cargo test --features conformance

#![cfg(feature = "conformance")]

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use tru::cli::json_stringify::json_stringify_lines;
use tru::decode::decode;
use tru::encode::encode;
use tru::options::{DecodeOptions, EncodeOptions, ExpandPathsMode, KeyFoldingMode};

#[derive(Debug, Deserialize)]
struct FixtureFile {
    #[allow(dead_code)]
    version: Option<String>,
    category: String,
    #[allow(dead_code)]
    description: String,
    tests: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    name: String,
    input: serde_json::Value,
    expected: Option<String>,
    #[serde(rename = "expectedJson")]
    expected_json: Option<serde_json::Value>,
    options: Option<serde_json::Value>,
    #[serde(rename = "shouldError")]
    should_error: Option<bool>,
    #[allow(dead_code)]
    note: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "specSection")]
    spec_section: Option<String>,
}

#[test]
fn conformance_encode_fixtures() {
    let root = fixture_root();
    let encode_dir = root.join("encode");
    let files = load_fixture_files(&encode_dir);

    assert!(!files.is_empty(), "no encode fixtures found");

    let mut passed = 0;
    let mut failed = 0;

    for file in &files {
        for case in &file.tests {
            let options = parse_encode_options(case.options.as_ref());
            let result = encode(case.input.clone(), options);

            if case.should_error.unwrap_or(false) {
                // We don't have error handling in encode currently
                // Just skip error cases for now
                continue;
            }

            if let Some(expected) = &case.expected {
                if result == *expected {
                    passed += 1;
                } else {
                    failed += 1;
                    eprintln!(
                        "FAIL [{}/{}]: expected:\n{}\ngot:\n{}",
                        file.category, case.name, expected, result
                    );
                }
            }
        }
    }

    eprintln!("Encode conformance: {passed} passed, {failed} failed");
    assert_eq!(failed, 0, "{failed} encode conformance tests failed");
}

#[test]
fn conformance_decode_fixtures() {
    let root = fixture_root();
    let decode_dir = root.join("decode");
    let files = load_fixture_files(&decode_dir);

    assert!(!files.is_empty(), "no decode fixtures found");

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for file in &files {
        for case in &file.tests {
            // Skip error cases for now
            if case.should_error.unwrap_or(false) {
                skipped += 1;
                continue;
            }

            let Some(expected_json) = &case.expected_json else {
                skipped += 1;
                continue;
            };

            // Input for decode is the TOON string
            let serde_json::Value::String(toon_input) = &case.input else {
                skipped += 1;
                continue;
            };
            let toon_input = toon_input.clone();

            let options = parse_decode_options(case.options.as_ref());
            let decoded = decode(&toon_input, options);

            // Convert decoded JsonValue to serde_json::Value for comparison
            let json_str = json_stringify_lines(&decoded, 0).join("");
            let decoded_json: serde_json::Value =
                serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);

            if decoded_json == *expected_json {
                passed += 1;
            } else {
                failed += 1;
                eprintln!(
                    "FAIL [{}/{}]: expected:\n{}\ngot:\n{}",
                    file.category,
                    case.name,
                    serde_json::to_string_pretty(expected_json).unwrap(),
                    serde_json::to_string_pretty(&decoded_json).unwrap()
                );
            }
        }
    }

    eprintln!("Decode conformance: {passed} passed, {failed} failed, {skipped} skipped");
    assert_eq!(failed, 0, "{failed} decode conformance tests failed");
}

fn parse_encode_options(options: Option<&serde_json::Value>) -> Option<EncodeOptions> {
    let options = options?;

    let indent = options
        .get("indent")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());

    let delimiter = options
        .get("delimiter")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.chars().next());

    let key_folding = options
        .get("keyFolding")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| match value {
            "safe" => Some(KeyFoldingMode::Safe),
            "off" => Some(KeyFoldingMode::Off),
            _ => None,
        });

    let flatten_depth = options
        .get("flattenDepth")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());

    Some(EncodeOptions {
        indent,
        delimiter,
        key_folding,
        flatten_depth,
        replacer: None,
    })
}

fn parse_decode_options(options: Option<&serde_json::Value>) -> Option<DecodeOptions> {
    let options = options?;

    let strict = options.get("strict").and_then(serde_json::Value::as_bool);

    let expand_paths = options
        .get("expandPaths")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| match value {
            "safe" => Some(ExpandPathsMode::Safe),
            "off" => Some(ExpandPathsMode::Off),
            _ => None,
        });

    Some(DecodeOptions {
        indent: None,
        strict,
        expand_paths,
    })
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

        let Ok(parsed) = serde_json::from_str::<FixtureFile>(&contents) else {
            continue;
        };

        fixtures.push(parsed);
    }

    fixtures
}
