use crate::{JsonArray, JsonObject, JsonPrimitive, JsonValue, StringOrNumberOrBoolOrNull};

pub fn normalize_json_value(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Primitive(primitive) => JsonValue::Primitive(normalize_primitive(primitive)),
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(normalize_json_value).collect())
        }
        JsonValue::Object(entries) => JsonValue::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, normalize_json_value(value)))
                .collect(),
        ),
    }
}

#[must_use]
pub fn normalize_primitive(value: JsonPrimitive) -> JsonPrimitive {
    match value {
        StringOrNumberOrBoolOrNull::Number(value) => {
            if !value.is_finite() {
                StringOrNumberOrBoolOrNull::Null
            } else if value == 0.0 {
                StringOrNumberOrBoolOrNull::Number(0.0)
            } else {
                StringOrNumberOrBoolOrNull::Number(value)
            }
        }
        _ => value,
    }
}

#[must_use]
pub const fn is_json_primitive(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Primitive(_))
}

#[must_use]
pub const fn is_json_array(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Array(_))
}

#[must_use]
pub const fn is_json_object(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Object(_))
}

#[must_use]
pub const fn is_empty_object(value: &JsonObject) -> bool {
    value.is_empty()
}

#[must_use]
pub fn is_array_of_primitives(value: &JsonArray) -> bool {
    value
        .iter()
        .all(|item| matches!(item, JsonValue::Primitive(_)))
}

#[must_use]
pub fn is_array_of_arrays(value: &JsonArray) -> bool {
    value.iter().all(|item| matches!(item, JsonValue::Array(_)))
}

#[must_use]
pub fn is_array_of_objects(value: &JsonArray) -> bool {
    value
        .iter()
        .all(|item| matches!(item, JsonValue::Object(_)))
}
