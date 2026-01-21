use toon_rust::cli::json_stream::json_stream_from_events;
use toon_rust::cli::json_stringify::json_stringify_lines;
use toon_rust::{
    JsonStreamEvent, JsonValue, StringOrNumberOrBoolOrNull, decode_stream_sync, encode,
    encode_stream_events,
};

#[test]
fn json_stringify_lines_matches_serde_for_compact() {
    let value = JsonValue::Object(vec![
        (
            "a".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(1.0)),
        ),
        (
            "b".to_string(),
            JsonValue::Array(vec![
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::Bool(true)),
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("x".to_string())),
            ]),
        ),
    ]);

    let chunks = json_stringify_lines(&value, 0);
    let actual = chunks.concat();

    let expected = serde_json::to_string(&serde_value(&value)).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn json_stringify_lines_matches_serde_for_pretty() {
    let value = JsonValue::Array(vec![
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Null),
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(3.5)),
        JsonValue::Object(vec![(
            "key".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("value".to_string())),
        )]),
    ]);

    let chunks = json_stringify_lines(&value, 2);
    let actual = chunks.concat();

    let expected = serde_json::to_string_pretty(&serde_value(&value)).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn json_stream_from_events_matches_stringify() {
    let events = vec![
        JsonStreamEvent::StartObject,
        JsonStreamEvent::Key {
            key: "a".to_string(),
            was_quoted: false,
        },
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(1.0),
        },
        JsonStreamEvent::Key {
            key: "b".to_string(),
            was_quoted: false,
        },
        JsonStreamEvent::StartArray { length: 2 },
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Bool(true),
        },
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::String("x".to_string()),
        },
        JsonStreamEvent::EndArray,
        JsonStreamEvent::EndObject,
    ];

    let actual = json_stream_from_events(events, 2).unwrap().concat();

    let value = JsonValue::Object(vec![
        (
            "a".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(1.0)),
        ),
        (
            "b".to_string(),
            JsonValue::Array(vec![
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::Bool(true)),
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("x".to_string())),
            ]),
        ),
    ]);
    let expected = json_stringify_lines(&value, 2).concat();

    assert_eq!(actual, expected);
}

#[test]
fn json_stream_from_events_rejects_mismatched_end() {
    let events = vec![JsonStreamEvent::EndObject];
    let err = json_stream_from_events(events, 0).unwrap_err();
    assert!(err.to_string().contains("Mismatched endObject"));
}

fn serde_value(value: &JsonValue) -> serde_json::Value {
    match value {
        JsonValue::Primitive(primitive) => match primitive {
            StringOrNumberOrBoolOrNull::Null => serde_json::Value::Null,
            StringOrNumberOrBoolOrNull::Bool(value) => serde_json::Value::Bool(*value),
            StringOrNumberOrBoolOrNull::Number(value) => serde_json::Number::from_f64(*value)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            StringOrNumberOrBoolOrNull::String(value) => serde_json::Value::String(value.clone()),
        },
        JsonValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(serde_value).collect())
        }
        JsonValue::Object(entries) => {
            let mut map = serde_json::Map::new();
            for (key, value) in entries {
                map.insert(key.clone(), serde_value(value));
            }
            serde_json::Value::Object(map)
        }
    }
}

#[test]
fn encode_stream_events_primitive() {
    let value = JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(42.0));
    let events = encode_stream_events(value, None);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(42.0)
        }
    );
}

#[test]
fn encode_stream_events_simple_object() {
    let value = JsonValue::Object(vec![
        (
            "name".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("Alice".to_string())),
        ),
        (
            "age".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(30.0)),
        ),
    ]);

    let events = encode_stream_events(value, None);

    assert_eq!(events.len(), 6);
    assert_eq!(events[0], JsonStreamEvent::StartObject);
    assert_eq!(
        events[1],
        JsonStreamEvent::Key {
            key: "name".to_string(),
            was_quoted: false
        }
    );
    assert_eq!(
        events[2],
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::String("Alice".to_string())
        }
    );
    assert_eq!(
        events[3],
        JsonStreamEvent::Key {
            key: "age".to_string(),
            was_quoted: false
        }
    );
    assert_eq!(
        events[4],
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(30.0)
        }
    );
    assert_eq!(events[5], JsonStreamEvent::EndObject);
}

#[test]
fn encode_stream_events_array() {
    let value = JsonValue::Array(vec![
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(1.0)),
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(2.0)),
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(3.0)),
    ]);

    let events = encode_stream_events(value, None);

    assert_eq!(events.len(), 5);
    assert_eq!(events[0], JsonStreamEvent::StartArray { length: 3 });
    assert_eq!(
        events[1],
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(1.0)
        }
    );
    assert_eq!(
        events[2],
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(2.0)
        }
    );
    assert_eq!(
        events[3],
        JsonStreamEvent::Primitive {
            value: StringOrNumberOrBoolOrNull::Number(3.0)
        }
    );
    assert_eq!(events[4], JsonStreamEvent::EndArray);
}

#[test]
fn encode_stream_events_quoted_key() {
    // Keys with special characters should have was_quoted=true
    let value = JsonValue::Object(vec![(
        "my-key".to_string(),
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(1.0)),
    )]);

    let events = encode_stream_events(value, None);

    assert_eq!(events.len(), 4);
    assert_eq!(events[0], JsonStreamEvent::StartObject);
    // "my-key" contains a hyphen so it needs quoting
    assert_eq!(
        events[1],
        JsonStreamEvent::Key {
            key: "my-key".to_string(),
            was_quoted: true
        }
    );
}

#[test]
fn encode_stream_events_roundtrip_with_decode() {
    // Test that encode_stream_events produces events compatible with decode
    let value = JsonValue::Object(vec![
        (
            "users".to_string(),
            JsonValue::Array(vec![
                JsonValue::Object(vec![
                    (
                        "id".to_string(),
                        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(1.0)),
                    ),
                    (
                        "name".to_string(),
                        JsonValue::Primitive(StringOrNumberOrBoolOrNull::String(
                            "Alice".to_string(),
                        )),
                    ),
                ]),
                JsonValue::Object(vec![
                    (
                        "id".to_string(),
                        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(2.0)),
                    ),
                    (
                        "name".to_string(),
                        JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("Bob".to_string())),
                    ),
                ]),
            ]),
        ),
        (
            "count".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(2.0)),
        ),
    ]);

    // Encode to TOON, then decode back to events
    let toon = encode(value.clone(), None);
    let decode_events = decode_stream_sync(
        toon.lines().map(str::to_string),
        None,
    );

    // Get events from encode_stream_events
    let encode_events = encode_stream_events(value, None);

    // The events should reconstruct to the same JSON structure
    // (Note: exact event sequences may differ due to TOON optimizations like tabular arrays)
    // So we verify by converting both back to JSON and comparing
    let from_decode = json_stream_from_events(decode_events, 0).unwrap().concat();
    let from_encode = json_stream_from_events(encode_events, 0).unwrap().concat();

    // Parse and compare as JSON values to handle formatting differences
    let decode_json: serde_json::Value = serde_json::from_str(&from_decode).unwrap();
    let encode_json: serde_json::Value = serde_json::from_str(&from_encode).unwrap();

    assert_eq!(decode_json, encode_json);
}
