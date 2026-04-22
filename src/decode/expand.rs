use std::collections::HashSet;

use crate::decode::event_builder::{NodeValue, ObjectNode};
use crate::error::{Result, ToonError};
use crate::shared::constants::DOT;
use crate::shared::validation::is_identifier_segment;

/// Hard cap on recursion depth during path expansion. Protects against stack
/// overflow on pathologically nested TOON inputs (deeply nested arrays, deeply
/// nested objects, or dotted keys with thousands of segments).
const MAX_EXPAND_DEPTH: usize = 256;

fn depth_error() -> ToonError {
    ToonError::message(format!(
        "Path expansion exceeded maximum depth of {MAX_EXPAND_DEPTH}"
    ))
}

/// Expand dotted keys into nested objects (safe mode).
///
/// # Errors
///
/// Returns an error in strict mode when path expansion encounters a conflict,
/// or when recursion depth exceeds the hard limit.
pub fn expand_paths_safe(value: NodeValue, strict: bool) -> Result<NodeValue> {
    expand_paths_safe_inner(value, strict, 0)
}

fn expand_paths_safe_inner(value: NodeValue, strict: bool, depth: usize) -> Result<NodeValue> {
    if depth >= MAX_EXPAND_DEPTH {
        return Err(depth_error());
    }
    match value {
        NodeValue::Array(items) => {
            let mut expanded = Vec::with_capacity(items.len());
            for item in items {
                expanded.push(expand_paths_safe_inner(item, strict, depth + 1)?);
            }
            Ok(NodeValue::Array(expanded))
        }
        NodeValue::Object(obj) => Ok(NodeValue::Object(expand_object(obj, strict, depth + 1)?)),
        NodeValue::Primitive(value) => Ok(NodeValue::Primitive(value)),
    }
}

fn expand_object(obj: ObjectNode, strict: bool, depth: usize) -> Result<ObjectNode> {
    if depth >= MAX_EXPAND_DEPTH {
        return Err(depth_error());
    }
    let quoted_keys = obj.quoted_keys;
    let mut expanded = ObjectNode {
        entries: Vec::new(),
        quoted_keys: HashSet::new(),
    };

    for (key, value) in obj.entries {
        let value = expand_paths_safe_inner(value, strict, depth + 1)?;
        let is_quoted = quoted_keys.contains(&key);

        if key.contains(DOT) && !is_quoted {
            let segments: Vec<&str> = key.split(DOT).collect();
            if segments
                .iter()
                .all(|segment| is_identifier_segment(segment))
            {
                insert_path_entries(&mut expanded.entries, &segments, value, strict, depth + 1)?;
                continue;
            }
        }

        insert_literal_entry(&mut expanded.entries, key, value, strict, depth + 1)?;
    }

    Ok(expanded)
}

fn insert_path_entries(
    entries: &mut Vec<(String, NodeValue)>,
    segments: &[&str],
    value: NodeValue,
    strict: bool,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_EXPAND_DEPTH {
        return Err(depth_error());
    }
    if segments.is_empty() {
        return Ok(());
    }

    if segments.len() == 1 {
        return insert_literal_entry(entries, segments[0].to_string(), value, strict, depth + 1);
    }

    let key = segments[0].to_string();
    if let Some(index) = find_entry_index(entries, &key) {
        let needs_object = !matches!(entries[index].1, NodeValue::Object(_));
        if needs_object {
            if strict {
                return Err(ToonError::message(format!(
                    "Path expansion conflict at segment \"{key}\": expected object but found {existing}",
                    existing = node_type_name(&entries[index].1)
                )));
            }
            entries[index].1 = NodeValue::Object(ObjectNode {
                entries: Vec::new(),
                quoted_keys: HashSet::new(),
            });
        }

        if let NodeValue::Object(obj) = &mut entries[index].1 {
            return insert_path_entries(&mut obj.entries, &segments[1..], value, strict, depth + 1);
        }
    } else {
        entries.push((
            key,
            NodeValue::Object(ObjectNode {
                entries: Vec::new(),
                quoted_keys: HashSet::new(),
            }),
        ));
        let index = entries.len() - 1;
        if let NodeValue::Object(obj) = &mut entries[index].1 {
            return insert_path_entries(&mut obj.entries, &segments[1..], value, strict, depth + 1);
        }
    }

    Ok(())
}

fn insert_literal_entry(
    entries: &mut Vec<(String, NodeValue)>,
    key: String,
    value: NodeValue,
    strict: bool,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_EXPAND_DEPTH {
        return Err(depth_error());
    }
    if let Some(index) = find_entry_index(entries, &key) {
        let existing = entries[index].1.clone();
        if can_merge(&existing, &value) {
            let mut existing_obj = extract_object(existing)?;
            let source_obj = extract_object(value)?;
            merge_objects(&mut existing_obj, source_obj, strict, depth + 1)?;
            entries[index].1 = NodeValue::Object(existing_obj);
        } else if strict {
            return Err(ToonError::message(format!(
                "Path expansion conflict at key \"{key}\": cannot merge {left} with {right}",
                left = node_type_name(&existing),
                right = node_type_name(&value)
            )));
        } else {
            entries[index].1 = value;
        }
    } else {
        entries.push((key, value));
    }

    Ok(())
}

fn merge_objects(
    target: &mut ObjectNode,
    source: ObjectNode,
    strict: bool,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_EXPAND_DEPTH {
        return Err(depth_error());
    }
    for (key, value) in source.entries {
        if let Some(index) = find_entry_index(&target.entries, &key) {
            let existing = target.entries[index].1.clone();
            if can_merge(&existing, &value) {
                let mut existing_obj = extract_object(existing)?;
                let source_obj = extract_object(value)?;
                merge_objects(&mut existing_obj, source_obj, strict, depth + 1)?;
                target.entries[index].1 = NodeValue::Object(existing_obj);
            } else if strict {
                return Err(ToonError::message(format!(
                    "Path expansion conflict at key \"{key}\": cannot merge {left} with {right}",
                    left = node_type_name(&existing),
                    right = node_type_name(&value)
                )));
            } else {
                target.entries[index].1 = value;
            }
        } else {
            target.entries.push((key, value));
        }
    }
    Ok(())
}

fn find_entry_index(entries: &[(String, NodeValue)], key: &str) -> Option<usize> {
    entries.iter().position(|(k, _)| k == key)
}

const fn can_merge(left: &NodeValue, right: &NodeValue) -> bool {
    matches!((left, right), (NodeValue::Object(_), NodeValue::Object(_)))
}

fn extract_object(value: NodeValue) -> Result<ObjectNode> {
    match value {
        NodeValue::Object(obj) => Ok(obj),
        other => Err(ToonError::message(format!(
            "Expected object but found {}",
            node_type_name(&other)
        ))),
    }
}

const fn node_type_name(value: &NodeValue) -> &'static str {
    match value {
        NodeValue::Primitive(_) => "primitive",
        NodeValue::Array(_) => "array",
        NodeValue::Object(_) => "object",
    }
}
