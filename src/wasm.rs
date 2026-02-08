//! WebAssembly bindings for TOON encoding and decoding.
//!
// Allow missing_errors_doc since all JS-facing functions can throw JsError
#![allow(clippy::missing_errors_doc)]
//!
//! This module provides JavaScript-friendly APIs for TOON operations
//! when compiled to WebAssembly.
//!
//! # Usage (JavaScript)
//!
//! ```javascript
//! import init, { encode, decode, encode_with_options, decode_with_options } from 'tru';
//!
//! await init();
//!
//! // Simple encode/decode
//! const toon = encode('{"name": "Alice", "age": 30}');
//! console.log(toon);
//! // name: Alice
//! // age: 30
//!
//! const json = decode('name: Alice\nage: 30');
//! console.log(json);
//! // {"name":"Alice","age":30}
//!
//! // With options
//! const options = { keyFolding: 'safe', indent: 4 };
//! const toonWithOptions = encode_with_options('{"a":{"b":{"c":1}}}', options);
//! console.log(toonWithOptions);
//! // a.b.c: 1
//! ```

use wasm_bindgen::prelude::*;

/// Initialize the WASM module with panic hook for better error messages.
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    #[cfg(feature = "wasm")]
    console_error_panic_hook::set_once();
}

/// Encode a JSON string to TOON format.
///
/// # Arguments
///
/// * `json` - A valid JSON string to encode
///
/// # Returns
///
/// A TOON-formatted string, or throws an error if the JSON is invalid.
///
/// # Example
///
/// ```javascript
/// const toon = encode('{"name": "Alice"}');
/// // Returns: "name: Alice"
/// ```
#[wasm_bindgen]
pub fn encode(json: &str) -> Result<String, JsError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| JsError::new(&format!("Invalid JSON: {e}")))?;
    Ok(crate::encode::encode(value, None))
}

/// Encode a JSON string to TOON format with options.
///
/// # Arguments
///
/// * `json` - A valid JSON string to encode
/// * `options` - Encoding options as a JavaScript object:
///   - `indent`: Number of spaces per indent level (default: 2)
///   - `delimiter`: Array delimiter character (default: ',')
///   - `keyFolding`: 'off' or 'safe' (default: 'off')
///   - `flattenDepth`: Maximum depth for key folding (default: unlimited)
///
/// # Returns
///
/// A TOON-formatted string, or throws an error if the JSON is invalid.
#[wasm_bindgen]
pub fn encode_with_options(json: &str, options: JsValue) -> Result<String, JsError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| JsError::new(&format!("Invalid JSON: {e}")))?;

    let encode_options = parse_encode_options(options)?;
    Ok(crate::encode::encode(value, encode_options))
}

/// Decode a TOON string to JSON format.
///
/// # Arguments
///
/// * `toon` - A TOON-formatted string to decode
///
/// # Returns
///
/// A compact JSON string, or throws an error if the TOON is invalid.
///
/// # Example
///
/// ```javascript
/// const json = decode('name: Alice\nage: 30');
/// // Returns: '{"name":"Alice","age":30}'
/// ```
#[wasm_bindgen]
pub fn decode(toon: &str) -> Result<String, JsError> {
    let value = crate::decode::try_decode(toon, None)
        .map_err(|e| JsError::new(&format!("Decode error: {e}")))?;
    let serde_value: serde_json::Value = value.into();
    Ok(serde_json::to_string(&serde_value).unwrap_or_default())
}

/// Decode a TOON string to JSON format with options.
///
/// # Arguments
///
/// * `toon` - A TOON-formatted string to decode
/// * `options` - Decoding options as a JavaScript object:
///   - `strict`: Enable strict validation (default: true)
///   - `expandPaths`: 'off' or 'safe' (default: 'off')
///   - `indent`: Expected indent size (default: 2)
///
/// # Returns
///
/// A compact JSON string, or throws an error if the TOON is invalid.
#[wasm_bindgen]
pub fn decode_with_options(toon: &str, options: JsValue) -> Result<String, JsError> {
    let decode_options = parse_decode_options(options)?;
    let value = crate::decode::try_decode(toon, decode_options)
        .map_err(|e| JsError::new(&format!("Decode error: {e}")))?;
    let serde_value: serde_json::Value = value.into();
    Ok(serde_json::to_string(&serde_value).unwrap_or_default())
}

/// Decode a TOON string to a pretty-printed JSON format.
///
/// # Arguments
///
/// * `toon` - A TOON-formatted string to decode
///
/// # Returns
///
/// A pretty-printed JSON string with 2-space indentation.
#[wasm_bindgen]
pub fn decode_pretty(toon: &str) -> Result<String, JsError> {
    let value = crate::decode::try_decode(toon, None)
        .map_err(|e| JsError::new(&format!("Decode error: {e}")))?;
    let serde_value: serde_json::Value = value.into();
    serde_json::to_string_pretty(&serde_value)
        .map_err(|e| JsError::new(&format!("JSON stringify error: {e}")))
}

/// Get the library version.
#[must_use]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// Helper functions for parsing JavaScript options

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value
)]
fn parse_encode_options(
    options: JsValue,
) -> Result<Option<crate::options::EncodeOptions>, JsError> {
    use crate::options::{EncodeOptions, KeyFoldingMode};

    if options.is_undefined() || options.is_null() {
        return Ok(None);
    }

    let obj = js_sys::Object::try_from(&options)
        .ok_or_else(|| JsError::new("Options must be an object"))?;

    let indent = js_sys::Reflect::get(obj, &"indent".into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as usize);

    let delimiter = js_sys::Reflect::get(obj, &"delimiter".into())
        .ok()
        .and_then(|v| v.as_string())
        .and_then(|s| s.chars().next());

    let key_folding = js_sys::Reflect::get(obj, &"keyFolding".into())
        .ok()
        .and_then(|v| v.as_string())
        .and_then(|s| match s.as_str() {
            "off" => Some(KeyFoldingMode::Off),
            "safe" => Some(KeyFoldingMode::Safe),
            _ => None,
        });

    let flatten_depth = js_sys::Reflect::get(obj, &"flattenDepth".into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as usize);

    Ok(Some(EncodeOptions {
        indent,
        delimiter,
        key_folding,
        flatten_depth,
        replacer: None,
    }))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value
)]
fn parse_decode_options(
    options: JsValue,
) -> Result<Option<crate::options::DecodeOptions>, JsError> {
    use crate::options::{DecodeOptions, ExpandPathsMode};

    if options.is_undefined() || options.is_null() {
        return Ok(None);
    }

    let obj = js_sys::Object::try_from(&options)
        .ok_or_else(|| JsError::new("Options must be an object"))?;

    let indent = js_sys::Reflect::get(obj, &"indent".into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as usize);

    let strict = js_sys::Reflect::get(obj, &"strict".into())
        .ok()
        .and_then(|v| v.as_bool());

    let expand_paths = js_sys::Reflect::get(obj, &"expandPaths".into())
        .ok()
        .and_then(|v| v.as_string())
        .and_then(|s| match s.as_str() {
            "off" => Some(ExpandPathsMode::Off),
            "safe" => Some(ExpandPathsMode::Safe),
            _ => None,
        });

    Ok(Some(DecodeOptions {
        indent,
        strict,
        expand_paths,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple() {
        let result = encode(r#"{"name": "Alice"}"#).unwrap();
        assert_eq!(result, "name: Alice");
    }

    #[test]
    fn test_decode_simple() {
        let result = decode("name: Alice").unwrap();
        assert_eq!(result, r#"{"name":"Alice"}"#);
    }

    #[test]
    fn test_roundtrip() {
        // Use floats in the original JSON since TOON uses f64 internally
        let json = r#"{"users":[{"id":1.0,"name":"Alice"},{"id":2.0,"name":"Bob"}]}"#;
        let toon = encode(json).unwrap();
        let decoded = decode(&toon).unwrap();
        // Parse both to compare values
        let original: serde_json::Value = serde_json::from_str(json).unwrap();
        let roundtrip: serde_json::Value = serde_json::from_str(&decoded).unwrap();
        assert_eq!(original, roundtrip);
    }
}
