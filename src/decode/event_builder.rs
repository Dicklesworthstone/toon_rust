use std::collections::{HashMap, HashSet};

use crate::error::{Result, ToonError};
use crate::{JsonPrimitive, JsonStreamEvent, JsonValue};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeValue {
    Primitive(JsonPrimitive),
    Array(Vec<Self>),
    Object(ObjectNode),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectNode {
    pub entries: Vec<(String, NodeValue)>,
    pub quoted_keys: HashSet<String>,
}

#[derive(Debug, Clone)]
enum BuildContext {
    Object {
        entries: Vec<(String, NodeValue)>,
        /// Position of each key in `entries`, to find a repeated key without a scan.
        index: HashMap<String, usize>,
        /// The pending key and whether it was quoted in the source.
        current_key: Option<(String, bool)>,
        quoted_keys: HashSet<String>,
    },
    Array {
        items: Vec<NodeValue>,
    },
}

#[derive(Debug, Clone)]
struct BuildState {
    stack: Vec<BuildContext>,
    root: Option<NodeValue>,
    strict: bool,
}

/// Build a decoded node tree from a stream of events.
///
/// Sibling keys are unique in the result (spec v4 §14.3): a repeated key is an error in strict
/// mode, and in lenient mode the later value replaces the earlier one in place.
///
/// # Errors
///
/// Returns an error if the event stream is malformed (mismatched start/end
/// events, missing keys, or incomplete stacks), or in strict mode for a repeated key.
pub fn build_node_from_events(
    events: impl IntoIterator<Item = JsonStreamEvent>,
    strict: bool,
) -> Result<NodeValue> {
    let mut state = BuildState {
        stack: Vec::new(),
        root: None,
        strict,
    };

    for event in events {
        apply_event(&mut state, event)?;
    }

    finalize_state(state)
}

pub fn node_to_json(value: NodeValue) -> JsonValue {
    match value {
        NodeValue::Primitive(value) => JsonValue::Primitive(value),
        NodeValue::Array(items) => JsonValue::Array(items.into_iter().map(node_to_json).collect()),
        NodeValue::Object(obj) => JsonValue::Object(
            obj.entries
                .into_iter()
                .map(|(key, value)| (key, node_to_json(value)))
                .collect(),
        ),
    }
}

fn apply_event(state: &mut BuildState, event: JsonStreamEvent) -> Result<()> {
    match event {
        JsonStreamEvent::StartObject => {
            state.stack.push(BuildContext::Object {
                entries: Vec::new(),
                index: HashMap::new(),
                current_key: None,
                quoted_keys: HashSet::new(),
            });
        }
        JsonStreamEvent::EndObject => {
            let Some(context) = state.stack.pop() else {
                return Err(ToonError::unexpected_event("endObject", "with empty stack"));
            };
            let BuildContext::Object {
                entries,
                quoted_keys,
                ..
            } = context
            else {
                return Err(ToonError::mismatched_end("Object", "Array"));
            };
            let node = NodeValue::Object(ObjectNode {
                entries,
                quoted_keys,
            });
            attach(state, node, "Object endObject event without preceding key")?;
        }
        JsonStreamEvent::StartArray { .. } => {
            state.stack.push(BuildContext::Array { items: Vec::new() });
        }
        JsonStreamEvent::EndArray => {
            let Some(context) = state.stack.pop() else {
                return Err(ToonError::unexpected_event("endArray", "with empty stack"));
            };
            let BuildContext::Array { items } = context else {
                return Err(ToonError::mismatched_end("Array", "Object"));
            };
            attach(
                state,
                NodeValue::Array(items),
                "Array endArray event without preceding key",
            )?;
        }
        JsonStreamEvent::Key { key, was_quoted } => {
            let Some(BuildContext::Object { current_key, .. }) = state.stack.last_mut() else {
                return Err(ToonError::unexpected_event(
                    "Key",
                    "outside of object context",
                ));
            };
            *current_key = Some((key, was_quoted));
        }
        JsonStreamEvent::Primitive { value } => {
            attach(
                state,
                NodeValue::Primitive(value),
                "Primitive event without preceding key in object",
            )?;
        }
    }

    Ok(())
}

/// Attach a finished value to the innermost open container, or make it the root.
fn attach(state: &mut BuildState, node: NodeValue, missing_key: &str) -> Result<()> {
    let strict = state.strict;
    match state.stack.last_mut() {
        None => state.root = Some(node),
        Some(BuildContext::Array { items }) => items.push(node),
        Some(BuildContext::Object {
            entries,
            index,
            current_key,
            quoted_keys,
        }) => {
            let Some((key, was_quoted)) = current_key.take() else {
                return Err(ToonError::message(missing_key));
            };
            if let Some(&position) = index.get(&key) {
                // A repeated sibling key: plain `--decode` used to write both, a JSON text with
                // duplicate member names, while `--expand-paths safe` rejected the same document.
                if strict {
                    return Err(ToonError::message(format!(
                        "Duplicate sibling key \"{key}\""
                    )));
                }
                entries[position].1 = node;
            } else {
                index.insert(key.clone(), entries.len());
                entries.push((key.clone(), node));
            }
            // Quoting belongs to the entry that holds the key; it decides path expansion.
            if was_quoted {
                quoted_keys.insert(key);
            } else {
                quoted_keys.remove(&key);
            }
        }
    }
    Ok(())
}

fn finalize_state(state: BuildState) -> Result<NodeValue> {
    if !state.stack.is_empty() {
        return Err(ToonError::event_stream(
            "Incomplete event stream: stack not empty at end",
        ));
    }

    state
        .root
        .ok_or_else(|| ToonError::event_stream("No root value built from events"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringOrNumberOrBoolOrNull;

    /// The builder in strict mode, the default of every decoder.
    fn build_node_from_events(
        events: impl IntoIterator<Item = JsonStreamEvent>,
    ) -> Result<NodeValue> {
        super::build_node_from_events(events, true)
    }

    fn object_with(entries: Vec<JsonStreamEvent>) -> Vec<JsonStreamEvent> {
        let mut events = vec![JsonStreamEvent::StartObject];
        events.extend(entries);
        events.push(JsonStreamEvent::EndObject);
        events
    }

    #[test]
    fn repeated_key_is_an_error_in_strict_mode() {
        let events = object_with(vec![key("a"), prim_num(1.0), key("a"), prim_num(2.0)]);
        let err = build_node_from_events(events).unwrap_err();
        assert_eq!(err.to_string(), "Duplicate sibling key \"a\"");
    }

    #[test]
    fn repeated_key_is_last_write_wins_in_lenient_mode() {
        let events = object_with(vec![
            key("a"),
            prim_num(1.0),
            key("b"),
            prim_num(2.0),
            key("a"),
            prim_num(3.0),
        ]);
        let root = super::build_node_from_events(events, false).unwrap();
        let NodeValue::Object(obj) = root else {
            unreachable!("an object was built")
        };
        let keys: Vec<_> = obj.entries.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["a", "b"]);
        assert_eq!(
            obj.entries[0].1,
            NodeValue::Primitive(StringOrNumberOrBoolOrNull::Number(3.0))
        );
    }

    #[test]
    fn quoting_follows_the_entry_that_wins() {
        let events = object_with(vec![
            quoted_key("a.b"),
            prim_num(1.0),
            key("a.b"),
            prim_num(2.0),
        ]);
        let root = super::build_node_from_events(events, false).unwrap();
        let NodeValue::Object(obj) = root else {
            unreachable!("an object was built")
        };
        assert!(!obj.quoted_keys.contains("a.b"));
    }

    fn prim(v: &str) -> JsonStreamEvent {
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::String(v.to_string()),
        }
    }

    fn prim_num(n: f64) -> JsonStreamEvent {
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(n),
        }
    }

    fn key(k: &str) -> JsonStreamEvent {
        JsonStreamEvent::Key {
            key: k.to_string(),
            was_quoted: false,
        }
    }

    fn quoted_key(k: &str) -> JsonStreamEvent {
        JsonStreamEvent::Key {
            key: k.to_string(),
            was_quoted: true,
        }
    }

    #[test]
    fn build_root_primitive() {
        let root = build_node_from_events([prim_num(42.0)]).unwrap();
        assert!(matches!(
            root,
            NodeValue::Primitive(StringOrNumberOrBoolOrNull::Number(n)) if (n - 42.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn build_empty_object() {
        let root =
            build_node_from_events([JsonStreamEvent::StartObject, JsonStreamEvent::EndObject])
                .unwrap();
        if let NodeValue::Object(obj) = root {
            assert_eq!(obj.entries, [] as [(String, NodeValue); 0]);
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn build_empty_array() {
        let root = build_node_from_events([
            JsonStreamEvent::StartArray { length: 0 },
            JsonStreamEvent::EndArray,
        ])
        .unwrap();
        assert!(matches!(root, NodeValue::Array(items) if items.is_empty()));
    }

    #[test]
    fn build_object_with_primitive_entry() {
        let root = build_node_from_events([
            JsonStreamEvent::StartObject,
            key("name"),
            prim("Alice"),
            JsonStreamEvent::EndObject,
        ])
        .unwrap();
        if let NodeValue::Object(obj) = root {
            assert_eq!(obj.entries.len(), 1);
            assert_eq!(obj.entries[0].0, "name");
            assert!(!obj.quoted_keys.contains("name"));
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn quoted_keys_are_tracked() {
        let root = build_node_from_events([
            JsonStreamEvent::StartObject,
            quoted_key("weird key"),
            prim("x"),
            JsonStreamEvent::EndObject,
        ])
        .unwrap();
        if let NodeValue::Object(obj) = root {
            assert!(obj.quoted_keys.contains("weird key"));
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn nested_object_round_trips() {
        let root = build_node_from_events([
            JsonStreamEvent::StartObject,
            key("outer"),
            JsonStreamEvent::StartObject,
            key("inner"),
            prim_num(1.0),
            JsonStreamEvent::EndObject,
            JsonStreamEvent::EndObject,
        ])
        .unwrap();
        if let NodeValue::Object(obj) = root {
            let (k, v) = &obj.entries[0];
            assert_eq!(k, "outer");
            assert!(matches!(v, NodeValue::Object(_)));
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn array_in_object() {
        let root = build_node_from_events([
            JsonStreamEvent::StartObject,
            key("nums"),
            JsonStreamEvent::StartArray { length: 2 },
            prim_num(1.0),
            prim_num(2.0),
            JsonStreamEvent::EndArray,
            JsonStreamEvent::EndObject,
        ])
        .unwrap();
        if let NodeValue::Object(obj) = root {
            let (_, v) = &obj.entries[0];
            assert!(matches!(v, NodeValue::Array(items) if items.len() == 2));
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn object_in_array() {
        let root = build_node_from_events([
            JsonStreamEvent::StartArray { length: 1 },
            JsonStreamEvent::StartObject,
            key("k"),
            prim_num(1.0),
            JsonStreamEvent::EndObject,
            JsonStreamEvent::EndArray,
        ])
        .unwrap();
        if let NodeValue::Array(items) = root {
            assert!(matches!(&items[0], NodeValue::Object(_)));
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn unbalanced_end_object_errors() {
        let result = build_node_from_events([JsonStreamEvent::EndObject]);
        assert!(result.is_err());
    }

    #[test]
    fn unbalanced_end_array_errors() {
        let result = build_node_from_events([JsonStreamEvent::EndArray]);
        assert!(result.is_err());
    }

    #[test]
    fn mismatched_end_errors() {
        let result =
            build_node_from_events([JsonStreamEvent::StartObject, JsonStreamEvent::EndArray]);
        assert!(result.is_err());
    }

    #[test]
    fn key_outside_object_errors() {
        let result = build_node_from_events([key("dangling"), prim("x")]);
        assert!(result.is_err());
    }

    #[test]
    fn incomplete_stream_errors() {
        let result = build_node_from_events([JsonStreamEvent::StartObject]);
        assert!(result.is_err());
    }

    #[test]
    fn empty_stream_errors() {
        let result = build_node_from_events([]);
        assert!(result.is_err());
    }

    #[test]
    fn primitive_in_object_without_key_errors() {
        let result = build_node_from_events([
            JsonStreamEvent::StartObject,
            prim("no-key"),
            JsonStreamEvent::EndObject,
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn node_to_json_preserves_structure() {
        let root = NodeValue::Object(ObjectNode {
            entries: vec![(
                "a".to_string(),
                NodeValue::Array(vec![NodeValue::Primitive(
                    StringOrNumberOrBoolOrNull::Number(1.0),
                )]),
            )],
            quoted_keys: HashSet::new(),
        });
        let json = node_to_json(root);
        if let JsonValue::Object(entries) = json {
            let (k, v) = &entries[0];
            assert_eq!(k, "a");
            assert!(matches!(v, JsonValue::Array(items) if items.len() == 1));
        } else {
            panic!("expected object");
        }
    }
}
