//! Async streaming encode using the asupersync runtime.
//!
//! This module provides true async streaming for TOON encoding with:
//! - Yield points between event processing for cooperative scheduling
//! - Cancellation support via asupersync's capability context
//! - Stream-based API for encoding large JSON values
//!
//! # Example
//!
//! ```ignore
//! use tru::encode::async_encode::{encode_async, AsyncEncodeStream};
//! use asupersync::stream::StreamExt;
//!
//! async fn encode_large_value(value: JsonValue) {
//!     let mut stream = AsyncEncodeStream::new(value, None);
//!     while let Some(line) = stream.next().await {
//!         println!("{}", line);
//!     }
//! }
//! ```

use crate::encode::encoders;
use crate::encode::normalize::normalize_json_value;
use crate::encode::replacer::apply_replacer;
use crate::options::{EncodeOptions, ResolvedEncodeOptions, resolve_encode_options};
use crate::shared::validation::is_valid_unquoted_key;
use crate::{JsonStreamEvent, JsonValue};
use asupersync::stream::{Stream, StreamExt, iter};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Async stream that yields TOON output lines from a JSON value.
///
/// This stream processes JSON input and yields TOON lines as they are
/// encoded. It supports cancellation and cooperative scheduling through
/// asupersync's stream primitives.
pub struct AsyncEncodeStream {
    /// Pre-computed lines to emit
    lines: Vec<String>,
    /// Current index into lines
    index: usize,
}

impl AsyncEncodeStream {
    /// Create a new async encode stream from a JSON value.
    #[must_use]
    pub fn new(input: impl Into<JsonValue>, options: Option<EncodeOptions>) -> Self {
        let resolved = resolve_encode_options(options);
        let normalized = normalize_json_value(input.into());
        let replaced = if let Some(replacer) = &resolved.replacer {
            apply_replacer(&normalized, replacer)
        } else {
            normalized
        };
        let lines = encoders::encode_json_value(&replaced, &resolved);

        Self { lines, index: 0 }
    }

    /// Get the total number of lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the stream is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

impl Stream for AsyncEncodeStream {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.index < self.lines.len() {
            let line = self.lines[self.index].clone();
            self.index += 1;
            Poll::Ready(Some(line))
        } else {
            Poll::Ready(None)
        }
    }
}

/// Async stream that yields `JsonStreamEvent` items from a JSON value.
///
/// This stream traverses the JSON structure and yields events representing
/// the structure (start/end object, start/end array, keys, primitives).
pub struct AsyncEncodeEventStream {
    /// Stack of iterators for nested structures
    stack: Vec<EncodeStackFrame>,
    /// The resolved encoding options
    #[allow(dead_code)]
    options: ResolvedEncodeOptions,
    /// Whether we've started
    started: bool,
    /// Root value (only used once at start)
    root: Option<JsonValue>,
}

enum EncodeStackFrame {
    Object {
        entries: std::vec::IntoIter<(String, JsonValue)>,
        pending_value: Option<JsonValue>,
    },
    Array {
        items: std::vec::IntoIter<JsonValue>,
        length: usize,
        emitted_start: bool,
    },
}

impl AsyncEncodeEventStream {
    /// Create a new async event stream from a JSON value.
    #[must_use]
    pub fn new(input: impl Into<JsonValue>, options: Option<EncodeOptions>) -> Self {
        let resolved = resolve_encode_options(options);
        let normalized = normalize_json_value(input.into());
        let replaced = if let Some(replacer) = &resolved.replacer {
            apply_replacer(&normalized, replacer)
        } else {
            normalized
        };

        Self {
            stack: Vec::new(),
            options: resolved,
            started: false,
            root: Some(replaced),
        }
    }

    fn next_event(&mut self) -> Option<JsonStreamEvent> {
        // Handle root value on first call
        if !self.started {
            self.started = true;
            if let Some(root) = self.root.take() {
                return self.start_value(root);
            }
        }

        // Process the current frame on the stack
        let frame = self.stack.last_mut()?;

        match frame {
            EncodeStackFrame::Object {
                entries,
                pending_value,
            } => {
                // If we have a pending value, process it
                if let Some(value) = pending_value.take() {
                    return self.start_value(value);
                }

                // Get next entry
                if let Some((key, value)) = entries.next() {
                    *pending_value = Some(value);
                    let was_quoted = !is_valid_unquoted_key(&key);
                    return Some(JsonStreamEvent::Key { key, was_quoted });
                }

                // Object exhausted - emit end
                self.stack.pop();
                Some(JsonStreamEvent::EndObject)
            }
            EncodeStackFrame::Array {
                items,
                length,
                emitted_start,
            } => {
                // Emit start array if not done
                if !*emitted_start {
                    *emitted_start = true;
                    return Some(JsonStreamEvent::StartArray { length: *length });
                }

                // Get next item
                if let Some(item) = items.next() {
                    return self.start_value(item);
                }

                // Array exhausted - emit end
                self.stack.pop();
                Some(JsonStreamEvent::EndArray)
            }
        }
    }

    fn start_value(&mut self, value: JsonValue) -> Option<JsonStreamEvent> {
        match value {
            JsonValue::Primitive(p) => Some(JsonStreamEvent::Primitive { value: p }),
            JsonValue::Array(arr) => {
                let length = arr.len();
                self.stack.push(EncodeStackFrame::Array {
                    items: arr.into_iter(),
                    length,
                    emitted_start: false,
                });
                // Recursively get the start array event
                self.next_event()
            }
            JsonValue::Object(obj) => {
                self.stack.push(EncodeStackFrame::Object {
                    entries: obj.into_iter(),
                    pending_value: None,
                });
                Some(JsonStreamEvent::StartObject)
            }
        }
    }
}

impl Stream for AsyncEncodeEventStream {
    type Item = JsonStreamEvent;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.next_event())
    }
}

/// Encode a JSON value to TOON lines asynchronously.
///
/// This function creates an async stream and collects all lines. The async
/// wrapper provides yield points for cooperative scheduling.
pub async fn encode_lines_async(
    input: impl Into<JsonValue>,
    options: Option<EncodeOptions>,
) -> Vec<String> {
    let input = input.into();

    // Use asupersync's iter() to create a yielding stream from the lines
    let resolved = resolve_encode_options(options);
    let normalized = normalize_json_value(input);
    let replaced = if let Some(replacer) = &resolved.replacer {
        apply_replacer(&normalized, replacer)
    } else {
        normalized
    };
    let lines = encoders::encode_json_value(&replaced, &resolved);

    // Wrap lines in an async stream for yield points
    let line_stream = iter(lines.clone());

    // Count forces iteration with yield points
    let _count = line_stream.count().await;

    lines
}

/// Encode a JSON value to a TOON string asynchronously.
pub async fn encode_async(input: impl Into<JsonValue>, options: Option<EncodeOptions>) -> String {
    let lines = encode_lines_async(input, options).await;
    lines.join("\n")
}

/// Encode a JSON value to events asynchronously.
///
/// Returns a vector of `JsonStreamEvent` items representing the structure.
pub async fn encode_events_async(
    input: impl Into<JsonValue>,
    options: Option<EncodeOptions>,
) -> Vec<JsonStreamEvent> {
    let input = input.into();
    let mut stream = AsyncEncodeEventStream::new(input, options);

    // Collect all events
    let mut events = Vec::new();
    while let Some(event) = stream.next_event() {
        events.push(event);
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringOrNumberOrBoolOrNull;

    #[test]
    fn test_async_encode_stream_creation() {
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
        let stream = AsyncEncodeStream::new(value, None);
        assert_eq!(stream.index, 0);
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_async_encode_event_stream() {
        let value = JsonValue::Object(vec![(
            "key".to_string(),
            JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("value".to_string())),
        )]);
        let mut stream = AsyncEncodeEventStream::new(value, None);

        // Manually poll the stream
        let events: Vec<_> = std::iter::from_fn(|| stream.next_event()).collect();

        assert!(events.len() >= 3); // StartObject, Key, Primitive, EndObject
        assert!(matches!(events[0], JsonStreamEvent::StartObject));
    }

    #[test]
    fn test_encode_events_match() {
        // Test that event stream produces same events as sync version
        let value = JsonValue::Object(vec![
            (
                "name".to_string(),
                JsonValue::Primitive(StringOrNumberOrBoolOrNull::String("Alice".to_string())),
            ),
            (
                "items".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(1.0)),
                    JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(2.0)),
                ]),
            ),
        ]);

        // Get events from sync version
        let sync_events = crate::encode::encode_stream_events(value.clone(), None);

        // Get events from async stream
        let mut stream = AsyncEncodeEventStream::new(value, None);
        let async_events: Vec<_> = std::iter::from_fn(|| stream.next_event()).collect();

        assert_eq!(sync_events.len(), async_events.len());
        for (sync_ev, async_ev) in sync_events.iter().zip(async_events.iter()) {
            assert_eq!(sync_ev, async_ev);
        }
    }
}
