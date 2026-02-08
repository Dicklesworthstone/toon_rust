use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use tru::encode::encode;
use tru::options::{EncodeOptions, KeyFoldingMode};

#[derive(Debug, Deserialize)]
struct FixtureFile {
    tests: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    name: String,
    input: serde_json::Value,
    expected: String,
    options: Option<serde_json::Value>,
}

#[test]
fn encode_spec_fixtures() {
    let root = fixture_root();
    let encode_dir = root.join("encode");
    let files = load_fixture_files(&encode_dir);

    assert!(!files.is_empty(), "no encode fixtures found");

    for file in files {
        for case in file.tests {
            run_case(&case);
        }
    }
}

fn run_case(case: &FixtureCase) {
    let options = parse_encode_options(case.options.as_ref());
    let output = encode(case.input.clone(), options);

    assert_eq!(
        output, case.expected,
        "fixture '{}' failed (expected {:?}, got {:?})",
        case.name, case.expected, output
    );
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
