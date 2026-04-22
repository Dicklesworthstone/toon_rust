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

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::String(v.to_string()))
    }

    fn n(v: f64) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(v))
    }

    #[test]
    fn normalize_primitive_passes_string_through() {
        let v = StringOrNumberOrBoolOrNull::String("hello".into());
        assert_eq!(
            normalize_primitive(v),
            StringOrNumberOrBoolOrNull::String("hello".into())
        );
    }

    #[test]
    fn normalize_primitive_preserves_finite_numbers() {
        let v = StringOrNumberOrBoolOrNull::Number(3.5);
        assert_eq!(
            normalize_primitive(v),
            StringOrNumberOrBoolOrNull::Number(3.5)
        );
    }

    #[test]
    fn normalize_primitive_turns_nan_into_null() {
        let v = StringOrNumberOrBoolOrNull::Number(f64::NAN);
        assert_eq!(normalize_primitive(v), StringOrNumberOrBoolOrNull::Null);
    }

    #[test]
    fn normalize_primitive_turns_infinity_into_null() {
        let v = StringOrNumberOrBoolOrNull::Number(f64::INFINITY);
        assert_eq!(normalize_primitive(v), StringOrNumberOrBoolOrNull::Null);
        let v = StringOrNumberOrBoolOrNull::Number(f64::NEG_INFINITY);
        assert_eq!(normalize_primitive(v), StringOrNumberOrBoolOrNull::Null);
    }

    #[test]
    fn normalize_primitive_zero_is_positive() {
        let v = StringOrNumberOrBoolOrNull::Number(-0.0);
        let normalized = normalize_primitive(v);
        if let StringOrNumberOrBoolOrNull::Number(n) = normalized {
            assert!(!n.is_sign_negative(), "expected positive zero");
        } else {
            panic!("expected Number variant");
        }
    }

    #[test]
    fn normalize_primitive_preserves_bool_and_null() {
        assert_eq!(
            normalize_primitive(StringOrNumberOrBoolOrNull::Bool(true)),
            StringOrNumberOrBoolOrNull::Bool(true)
        );
        assert_eq!(
            normalize_primitive(StringOrNumberOrBoolOrNull::Null),
            StringOrNumberOrBoolOrNull::Null
        );
    }

    #[test]
    fn normalize_json_value_walks_array_and_replaces_nan() {
        let v = JsonValue::Array(vec![n(1.0), n(f64::NAN), s("x")]);
        let out = normalize_json_value(v);
        if let JsonValue::Array(items) = out {
            assert_eq!(items.len(), 3);
            assert!(matches!(
                &items[1],
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::Null)
            ));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn normalize_json_value_walks_object() {
        let v = JsonValue::Object(vec![
            ("ok".to_string(), n(1.0)),
            ("bad".to_string(), n(f64::INFINITY)),
        ]);
        let out = normalize_json_value(v);
        if let JsonValue::Object(entries) = out {
            assert!(matches!(
                &entries[1].1,
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::Null)
            ));
        } else {
            panic!("expected Object");
        }
    }

    #[test]
    fn is_json_primitive_detects_primitives() {
        assert!(is_json_primitive(&n(1.0)));
        assert!(!is_json_primitive(&JsonValue::Array(vec![])));
        assert!(!is_json_primitive(&JsonValue::Object(vec![])));
    }

    #[test]
    fn is_json_array_detects_arrays() {
        assert!(is_json_array(&JsonValue::Array(vec![])));
        assert!(!is_json_array(&n(1.0)));
    }

    #[test]
    fn is_json_object_detects_objects() {
        assert!(is_json_object(&JsonValue::Object(vec![])));
        assert!(!is_json_object(&JsonValue::Array(vec![])));
    }

    #[test]
    fn is_empty_object_detects_empty_and_non_empty() {
        let empty: JsonObject = vec![];
        let non_empty: JsonObject = vec![("a".into(), n(1.0))];
        assert!(is_empty_object(&empty));
        assert!(!is_empty_object(&non_empty));
    }

    #[test]
    fn is_array_of_primitives_checks_every_item() {
        let arr: JsonArray = vec![n(1.0), s("x")];
        assert!(is_array_of_primitives(&arr));
        let arr: JsonArray = vec![n(1.0), JsonValue::Array(vec![])];
        assert!(!is_array_of_primitives(&arr));
    }

    #[test]
    fn is_array_of_primitives_true_for_empty() {
        let arr: JsonArray = vec![];
        assert!(is_array_of_primitives(&arr));
    }

    #[test]
    fn is_array_of_arrays_checks_every_item() {
        let arr: JsonArray = vec![JsonValue::Array(vec![n(1.0)]), JsonValue::Array(vec![])];
        assert!(is_array_of_arrays(&arr));
        let arr: JsonArray = vec![JsonValue::Array(vec![]), n(1.0)];
        assert!(!is_array_of_arrays(&arr));
    }

    #[test]
    fn is_array_of_objects_checks_every_item() {
        let arr: JsonArray = vec![JsonValue::Object(vec![])];
        assert!(is_array_of_objects(&arr));
        let arr: JsonArray = vec![JsonValue::Object(vec![]), n(1.0)];
        assert!(!is_array_of_objects(&arr));
    }
}
