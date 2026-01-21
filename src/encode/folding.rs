use std::collections::HashSet;

use crate::JsonValue;
use crate::encode::normalize::{is_empty_object, is_json_object};
use crate::options::{KeyFoldingMode, ResolvedEncodeOptions};
use crate::shared::constants::DOT;
use crate::shared::validation::is_identifier_segment;

#[derive(Debug, Clone)]
pub struct FoldResult {
    pub folded_key: String,
    pub remainder: Option<JsonValue>,
    pub leaf_value: JsonValue,
    pub segment_count: usize,
}

#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn try_fold_key_chain(
    key: &str,
    value: &JsonValue,
    siblings: &[&str],
    options: &ResolvedEncodeOptions,
    root_literal_keys: Option<&HashSet<String>>,
    path_prefix: Option<&str>,
    flatten_depth: usize,
) -> Option<FoldResult> {
    if options.key_folding != KeyFoldingMode::Safe {
        return None;
    }

    if !is_json_object(value) {
        return None;
    }

    let effective_depth = flatten_depth;
    if effective_depth < 2 {
        return None;
    }

    let (segments, tail, leaf_value) = collect_single_key_chain(key, value, effective_depth);

    if segments.len() < 2 {
        return None;
    }

    if !segments.iter().all(|seg| is_identifier_segment(seg)) {
        return None;
    }

    let mut folded_key =
        String::with_capacity(segments.iter().map(String::len).sum::<usize>() + segments.len());
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            folded_key.push(DOT);
        }
        folded_key.push_str(seg);
    }

    if siblings.iter().any(|sibling| *sibling == folded_key) {
        return None;
    }

    let absolute_path = path_prefix.map_or_else(
        || folded_key.clone(),
        |prefix| format!("{prefix}{DOT}{folded_key}"),
    );

    if let Some(root_keys) = root_literal_keys {
        if root_keys.contains(&absolute_path) {
            return None;
        }
    }

    Some(FoldResult {
        folded_key,
        remainder: tail,
        leaf_value,
        segment_count: segments.len(),
    })
}

fn collect_single_key_chain(
    start_key: &str,
    start_value: &JsonValue,
    max_depth: usize,
) -> (Vec<String>, Option<JsonValue>, JsonValue) {
    let mut segments = vec![start_key.to_string()];
    let mut current_value = start_value.clone();

    while segments.len() < max_depth {
        let JsonValue::Object(ref obj) = current_value else {
            break;
        };

        if obj.len() != 1 {
            break;
        }

        let (next_key, next_value) = obj[0].clone();
        segments.push(next_key);
        current_value = next_value;
    }

    match current_value {
        JsonValue::Object(entries) if is_empty_object(&entries) => {
            let obj = JsonValue::Object(entries);
            (segments, None, obj)
        }
        JsonValue::Object(entries) => {
            let remainder = JsonValue::Object(entries);
            (segments, Some(remainder.clone()), remainder)
        }
        other => (segments, None, other),
    }
}
