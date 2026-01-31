#![forbid(unsafe_code)]

pub mod cli;
pub mod decode;
pub mod encode;
pub mod error;
pub mod options;
pub mod shared;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use decode::{
    decode, decode_from_lines, decode_stream, decode_stream_sync, try_decode,
    try_decode_from_lines, try_decode_stream, try_decode_stream_sync,
};
pub use encode::{encode, encode_lines, encode_stream_events};
pub use options::{
    DecodeOptions, DecodeStreamOptions, EncodeOptions, EncodeReplacer, ResolvedDecodeOptions,
    ResolvedEncodeOptions,
};

/// Convenience wrapper: parse JSON text and encode to TOON.
///
/// For lower-level control, parse JSON yourself and call [`encode()`].
///
/// # Errors
/// Returns an error if the JSON input is invalid.
pub fn json_to_toon(json: &str) -> crate::error::Result<String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|err| crate::error::ToonError::message(err.to_string()))?;
    Ok(encode(value, None))
}

/// Convenience wrapper: decode TOON and return compact JSON text.
///
/// For lower-level control, call [`try_decode`] and handle [`JsonValue`] directly.
///
/// # Errors
/// Returns an error if the TOON input is invalid.
pub fn toon_to_json(toon: &str) -> crate::error::Result<String> {
    let value = try_decode(toon, None)?;
    let value = serde_json::Value::from(value);
    serde_json::to_string(&value).map_err(|err| crate::error::ToonError::message(err.to_string()))
}

pub type JsonPrimitive = StringOrNumberOrBoolOrNull;
pub type JsonObject = Vec<(String, JsonValue)>;
pub type JsonArray = Vec<JsonValue>;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Primitive(JsonPrimitive),
    Array(JsonArray),
    Object(JsonObject),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonStreamEvent {
    StartObject,
    EndObject,
    StartArray { length: usize },
    EndArray,
    Key { key: String, was_quoted: bool },
    Primitive { value: JsonPrimitive },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringOrNumberOrBoolOrNull {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl StringOrNumberOrBoolOrNull {
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return Self::Null;
        }
        if value == 0.0 {
            return Self::Number(0.0);
        }
        Self::Number(value)
    }
}

impl From<StringOrNumberOrBoolOrNull> for JsonValue {
    fn from(value: StringOrNumberOrBoolOrNull) -> Self {
        Self::Primitive(value)
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::Primitive(StringOrNumberOrBoolOrNull::String(value))
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::Primitive(StringOrNumberOrBoolOrNull::String(value.to_string()))
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Primitive(StringOrNumberOrBoolOrNull::Bool(value))
    }
}

impl From<f64> for JsonValue {
    fn from(value: f64) -> Self {
        Self::Primitive(StringOrNumberOrBoolOrNull::from_f64(value))
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Primitive(StringOrNumberOrBoolOrNull::Number(value as f64))
    }
}

#[allow(clippy::use_self)]
impl From<Vec<JsonValue>> for JsonValue {
    fn from(value: Vec<JsonValue>) -> Self {
        Self::Array(value)
    }
}

impl From<JsonObject> for JsonValue {
    fn from(value: JsonObject) -> Self {
        Self::Object(value)
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Primitive(StringOrNumberOrBoolOrNull::Null),
            serde_json::Value::Bool(value) => {
                Self::Primitive(StringOrNumberOrBoolOrNull::Bool(value))
            }
            serde_json::Value::Number(value) => {
                let number = value
                    .as_f64()
                    .unwrap_or_else(|| value.to_string().parse::<f64>().unwrap_or(f64::NAN));
                Self::Primitive(StringOrNumberOrBoolOrNull::from_f64(number))
            }
            serde_json::Value::String(value) => {
                Self::Primitive(StringOrNumberOrBoolOrNull::String(value))
            }
            serde_json::Value::Array(values) => {
                Self::Array(values.into_iter().map(Self::from).collect())
            }
            serde_json::Value::Object(map) => {
                let mut entries = Vec::with_capacity(map.len());
                for (key, value) in map {
                    entries.push((key, Self::from(value)));
                }
                Self::Object(entries)
            }
        }
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Primitive(p) => match p {
                StringOrNumberOrBoolOrNull::String(value) => Self::String(value),
                StringOrNumberOrBoolOrNull::Number(value) => {
                    serde_json::Number::from_f64(value).map_or(Self::Null, Self::Number)
                }
                StringOrNumberOrBoolOrNull::Bool(value) => Self::Bool(value),
                StringOrNumberOrBoolOrNull::Null => Self::Null,
            },
            JsonValue::Array(arr) => Self::Array(arr.into_iter().map(Self::from).collect()),
            JsonValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (key, val) in obj {
                    map.insert(key, Self::from(val));
                }
                Self::Object(map)
            }
        }
    }
}
