use crate::encode::normalize::normalize_json_value;
use crate::options::{EncodeReplacer, PathSegment};
use crate::{JsonArray, JsonObject, JsonValue};

pub fn apply_replacer(root: &JsonValue, replacer: &EncodeReplacer) -> JsonValue {
    let replaced_root = replacer("", root, &[]);
    if let Some(value) = replaced_root {
        let normalized = normalize_json_value(value);
        return transform_children(normalized, replacer, &[]);
    }

    transform_children(root.clone(), replacer, &[])
}

fn transform_children(
    value: JsonValue,
    replacer: &EncodeReplacer,
    path: &[PathSegment],
) -> JsonValue {
    match value {
        JsonValue::Object(entries) => JsonValue::Object(transform_object(entries, replacer, path)),
        JsonValue::Array(values) => JsonValue::Array(transform_array(values, replacer, path)),
        JsonValue::Primitive(value) => JsonValue::Primitive(value),
    }
}

fn transform_object(
    entries: JsonObject,
    replacer: &EncodeReplacer,
    path: &[PathSegment],
) -> JsonObject {
    let mut result = Vec::new();

    for (key, value) in entries {
        let mut next_path = path.to_vec();
        next_path.push(PathSegment::Key(key.clone()));

        let replacement = replacer(&key, &value, &next_path);
        if let Some(next_value) = replacement {
            let normalized = normalize_json_value(next_value);
            let transformed = transform_children(normalized, replacer, &next_path);
            result.push((key, transformed));
        }
    }

    result
}

fn transform_array(
    values: JsonArray,
    replacer: &EncodeReplacer,
    path: &[PathSegment],
) -> JsonArray {
    let mut result = Vec::new();

    for (idx, value) in values.into_iter().enumerate() {
        let mut next_path = path.to_vec();
        next_path.push(PathSegment::Index(idx));

        let key = idx.to_string();
        let replacement = replacer(&key, &value, &next_path);
        if let Some(next_value) = replacement {
            let normalized = normalize_json_value(next_value);
            let transformed = transform_children(normalized, replacer, &next_path);
            result.push(transformed);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringOrNumberOrBoolOrNull;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn s(v: &str) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::String(v.to_string()))
    }

    fn n(v: f64) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(v))
    }

    fn identity_replacer() -> EncodeReplacer {
        Arc::new(|_key, value, _path| Some(value.clone()))
    }

    #[test]
    fn identity_replacer_returns_equivalent_tree_for_primitive() {
        let root = n(42.0);
        let out = apply_replacer(&root, &identity_replacer());
        assert_eq!(out, n(42.0));
    }

    #[test]
    fn identity_replacer_returns_equivalent_tree_for_object() {
        let root = JsonValue::Object(vec![("a".into(), n(1.0)), ("b".into(), s("hi"))]);
        let out = apply_replacer(&root, &identity_replacer());
        assert_eq!(out, root);
    }

    #[test]
    fn returning_none_for_root_falls_back_to_original_transform() {
        let replacer: EncodeReplacer = Arc::new(|key, value, _path| {
            if key.is_empty() {
                None
            } else {
                Some(value.clone())
            }
        });
        let root = JsonValue::Object(vec![("a".into(), n(1.0))]);
        let out = apply_replacer(&root, &replacer);
        assert_eq!(out, root);
    }

    #[test]
    fn replacer_can_drop_object_entries_by_returning_none() {
        let replacer: EncodeReplacer = Arc::new(|key, value, _path| {
            if key == "drop" {
                None
            } else {
                Some(value.clone())
            }
        });
        let root = JsonValue::Object(vec![("keep".into(), n(1.0)), ("drop".into(), n(2.0))]);
        let out = apply_replacer(&root, &replacer);
        assert_eq!(out, JsonValue::Object(vec![("keep".into(), n(1.0))]));
    }

    #[test]
    fn replacer_can_drop_array_elements_by_returning_none() {
        let replacer: EncodeReplacer = Arc::new(|_key, value, path| {
            let PathSegment::Index(idx) = path.last()? else {
                return Some(value.clone());
            };
            if *idx == 1 { None } else { Some(value.clone()) }
        });
        let root = JsonValue::Array(vec![n(1.0), n(2.0), n(3.0)]);
        let out = apply_replacer(&root, &replacer);
        assert_eq!(out, JsonValue::Array(vec![n(1.0), n(3.0)]));
    }

    #[test]
    fn replacer_can_transform_values() {
        let replacer: EncodeReplacer = Arc::new(|_key, value, _path| {
            if let JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(num)) = value {
                Some(JsonValue::from(num * 2.0))
            } else {
                Some(value.clone())
            }
        });
        let root = JsonValue::Array(vec![n(1.0), n(2.0)]);
        let out = apply_replacer(&root, &replacer);
        assert_eq!(out, JsonValue::Array(vec![n(2.0), n(4.0)]));
    }

    #[test]
    fn replacer_normalizes_nan_returned_from_replacement() {
        let replacer: EncodeReplacer =
            Arc::new(|_key, _value, _path| Some(JsonValue::from(f64::NAN)));
        let root = n(1.0);
        let out = apply_replacer(&root, &replacer);
        assert!(matches!(
            out,
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::Null)
        ));
    }

    #[test]
    fn replacer_receives_path_segments() {
        let collected: Arc<std::sync::Mutex<Vec<Vec<PathSegment>>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let collected_inner = collected.clone();
        let replacer: EncodeReplacer = Arc::new(move |_key, value, path| {
            collected_inner.lock().unwrap().push(path.to_vec());
            Some(value.clone())
        });
        let root = JsonValue::Object(vec![("arr".into(), JsonValue::Array(vec![n(1.0), n(2.0)]))]);
        let _ = apply_replacer(&root, &replacer);
        // Take a snapshot and drop the lock before running assertions so no
        // panicking assertion poisons the mutex.
        let paths: Vec<Vec<PathSegment>> = collected.lock().unwrap().clone();
        // root visit + "arr" + arr[0] + arr[1]
        assert!(paths.iter().any(Vec::is_empty));
        assert!(
            paths
                .iter()
                .any(|p| p.len() == 1 && matches!(&p[0], PathSegment::Key(k) if k == "arr"))
        );
        assert!(paths.iter().any(|p| {
            p.last()
                .is_some_and(|seg| matches!(seg, PathSegment::Index(0)))
        }));
        assert!(paths.iter().any(|p| {
            p.last()
                .is_some_and(|seg| matches!(seg, PathSegment::Index(1)))
        }));
    }

    #[test]
    fn replacer_call_counts_are_reasonable() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_inner = count.clone();
        let replacer: EncodeReplacer = Arc::new(move |_key, value, _path| {
            count_inner.fetch_add(1, Ordering::SeqCst);
            Some(value.clone())
        });
        let root = JsonValue::Object(vec![
            ("a".into(), n(1.0)),
            ("b".into(), JsonValue::Array(vec![n(2.0), n(3.0)])),
        ]);
        let _ = apply_replacer(&root, &replacer);
        let n_calls = count.load(Ordering::SeqCst);
        // root + a + b + b[0] + b[1] = 5 calls
        assert_eq!(n_calls, 5);
    }
}
