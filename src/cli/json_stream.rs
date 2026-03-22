use crate::JsonStreamEvent;
use crate::error::{Result, ToonError};

#[derive(Debug, Clone)]
enum JsonContext {
    Object {
        needs_comma: bool,
        expect_value: bool,
    },
    Array {
        needs_comma: bool,
    },
}

/// Convert JSON stream events into JSON string chunks.
///
/// # Errors
///
/// Returns an error if the event stream is malformed (mismatched start/end
/// events or primitives without keys in an object).
#[allow(clippy::too_many_lines)]
pub fn json_stream_from_events(
    events: impl IntoIterator<Item = JsonStreamEvent>,
    indent: usize,
) -> Result<Vec<String>> {
    let mut stack: Vec<JsonContext> = Vec::new();
    let mut depth = 0usize;
    let mut out = Vec::new();

    for event in events {
        let parent = stack.last_mut();
        match event {
            JsonStreamEvent::StartObject => {
                if let Some(parent) = parent {
                    match parent {
                        JsonContext::Array { needs_comma } => {
                            if *needs_comma {
                                out.push(",".to_string());
                            }
                            if indent > 0 {
                                out.push("\n".to_string());
                                out.push(" ".repeat(depth * indent));
                            }
                        }
                        JsonContext::Object { .. } => {}
                    }
                }

                out.push("{".to_string());
                stack.push(JsonContext::Object {
                    needs_comma: false,
                    expect_value: false,
                });
                depth += 1;
            }
            JsonStreamEvent::EndObject => {
                let Some(context) = stack.pop() else {
                    return Err(ToonError::message("Mismatched endObject event"));
                };
                if !matches!(context, JsonContext::Object { .. }) {
                    return Err(ToonError::message("Mismatched endObject event"));
                }
                depth = depth.saturating_sub(1);
                if indent > 0
                    && let JsonContext::Object { needs_comma, .. } = context
                    && needs_comma
                {
                    out.push("\n".to_string());
                    out.push(" ".repeat(depth * indent));
                }
                out.push("}".to_string());

                if let Some(parent) = stack.last_mut() {
                    match parent {
                        JsonContext::Object {
                            needs_comma,
                            expect_value,
                        } => {
                            *expect_value = false;
                            *needs_comma = true;
                        }
                        JsonContext::Array { needs_comma } => {
                            *needs_comma = true;
                        }
                    }
                }
            }
            JsonStreamEvent::StartArray { .. } => {
                if let Some(parent) = parent {
                    match parent {
                        JsonContext::Array { needs_comma } => {
                            if *needs_comma {
                                out.push(",".to_string());
                            }
                            if indent > 0 {
                                out.push("\n".to_string());
                                out.push(" ".repeat(depth * indent));
                            }
                        }
                        JsonContext::Object { .. } => {}
                    }
                }

                out.push("[".to_string());
                stack.push(JsonContext::Array { needs_comma: false });
                depth += 1;
            }
            JsonStreamEvent::EndArray => {
                let Some(context) = stack.pop() else {
                    return Err(ToonError::message("Mismatched endArray event"));
                };
                if !matches!(context, JsonContext::Array { .. }) {
                    return Err(ToonError::message("Mismatched endArray event"));
                }
                depth = depth.saturating_sub(1);
                if indent > 0
                    && let JsonContext::Array { needs_comma } = context
                    && needs_comma
                {
                    out.push("\n".to_string());
                    out.push(" ".repeat(depth * indent));
                }
                out.push("]".to_string());

                if let Some(parent) = stack.last_mut() {
                    match parent {
                        JsonContext::Object {
                            needs_comma,
                            expect_value,
                        } => {
                            *expect_value = false;
                            *needs_comma = true;
                        }
                        JsonContext::Array { needs_comma } => {
                            *needs_comma = true;
                        }
                    }
                }
            }
            JsonStreamEvent::Key { key, .. } => {
                let Some(JsonContext::Object {
                    needs_comma,
                    expect_value,
                }) = stack.last_mut()
                else {
                    return Err(ToonError::message("Key event outside of object context"));
                };

                if *needs_comma {
                    out.push(",".to_string());
                }
                if indent > 0 {
                    out.push("\n".to_string());
                    out.push(" ".repeat(depth * indent));
                }

                out.push(serde_json::to_string(&key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(if indent > 0 { ": " } else { ":" }.to_string());

                *expect_value = true;
                *needs_comma = true;
            }
            JsonStreamEvent::Primitive { value } => {
                if let Some(parent) = stack.last_mut() {
                    match parent {
                        JsonContext::Array { needs_comma } => {
                            if *needs_comma {
                                out.push(",".to_string());
                            }
                            if indent > 0 {
                                out.push("\n".to_string());
                                out.push(" ".repeat(depth * indent));
                            }
                        }
                        JsonContext::Object { expect_value, .. } => {
                            if !*expect_value {
                                return Err(ToonError::message(
                                    "Primitive event in object without preceding key",
                                ));
                            }
                        }
                    }
                }

                out.push(stringify_primitive(&value));

                if let Some(parent) = stack.last_mut() {
                    match parent {
                        JsonContext::Object { expect_value, .. } => {
                            *expect_value = false;
                        }
                        JsonContext::Array { needs_comma } => {
                            *needs_comma = true;
                        }
                    }
                }
            }
        }
    }

    if !stack.is_empty() {
        return Err(ToonError::message(
            "Incomplete event stream: unclosed objects or arrays",
        ));
    }

    Ok(out)
}

fn stringify_primitive(value: &crate::JsonPrimitive) -> String {
    match value {
        crate::StringOrNumberOrBoolOrNull::Null => "null".to_string(),
        crate::StringOrNumberOrBoolOrNull::Bool(value) => value.to_string(),
        crate::StringOrNumberOrBoolOrNull::Number(value) => serde_json::Number::from_f64(*value)
            .map_or_else(|| "null".to_string(), |num| num.to_string()),
        crate::StringOrNumberOrBoolOrNull::String(value) => {
            serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
        }
    }
}
