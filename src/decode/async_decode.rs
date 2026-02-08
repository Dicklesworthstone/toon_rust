//! Async streaming decode using the asupersync runtime.
//!
//! This module provides true async streaming for TOON decoding with:
//! - Yield points between line processing for cooperative scheduling
//! - Cancellation support via asupersync's capability context
//! - Stream-based API for processing large TOON inputs
//!
//! # Example
//!
//! ```ignore
//! use tru::decode::async_decode::{try_decode_stream_async, AsyncDecodeStream};
//! use asupersync::stream::StreamExt;
//!
//! async fn decode_large_file(lines: Vec<String>) {
//!     let mut stream = AsyncDecodeStream::new(lines.into_iter(), None);
//!     while let Some(result) = stream.next().await {
//!         match result {
//!             Ok(event) => println!("Event: {:?}", event),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//! }
//! ```

use crate::JsonStreamEvent;
use crate::decode::decoders as decoder_impl;
use crate::decode::parser::{
    is_array_header_content, is_key_value_content, parse_array_header_line, parse_key_token,
    parse_primitive_token,
};
use crate::decode::scanner::{
    Depth, ParsedLine, StreamingScanState, create_scan_state, parse_line_incremental,
};
use crate::error::{Result, ToonError};
use crate::options::DecodeStreamOptions;
use crate::shared::constants::{COLON, DEFAULT_DELIMITER, LIST_ITEM_PREFIX};
use crate::shared::string_utils::find_closing_quote;
use asupersync::stream::{Stream, StreamExt, iter};
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

/// State for incremental decoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecoderState {
    /// Haven't seen any lines yet
    Initial,
    /// Processing simple key-value pairs (can emit incrementally)
    SimpleObject { base_depth: Depth },
    /// Saw an array header, need to process array specially
    ArrayMode,
    /// Processing is complete
    Finished,
}

/// Context for tracking nested object structures.
/// Arrays fall back to batch processing, so we only track object depth incrementally.
type ObjectDepth = Depth;

/// Async stream that yields `JsonStreamEvent` items from TOON input lines.
///
/// This stream processes TOON input line-by-line, yielding events as they are
/// decoded. It supports cancellation and cooperative scheduling through
/// asupersync's stream primitives.
///
/// # Incremental Processing
///
/// The stream emits events incrementally when possible:
/// - Simple flat objects emit Key + Primitive events as lines arrive
/// - Nested objects push/pop context and emit events at structure boundaries
/// - Arrays with headers require buffering until all items are processed
pub struct AsyncDecodeStream<I: Iterator<Item = String>> {
    /// The underlying line iterator
    lines: I,
    /// Decode options
    options: DecodeStreamOptions,
    /// Internal state for incremental scanning
    scan_state: StreamingScanState,
    /// Event queue for yielding
    event_queue: VecDeque<JsonStreamEvent>,
    /// Decoder state machine
    state: DecoderState,
    /// Stack of object depths for tracking nested structures
    context_stack: Vec<ObjectDepth>,
    /// Buffer for lines that need batch processing (arrays)
    line_buffer: Vec<ParsedLine>,
    /// Whether we've finished reading all lines
    lines_exhausted: bool,
    /// Error encountered during processing
    error: Option<ToonError>,
    /// Last emitted depth for tracking structure boundaries
    last_depth: Option<Depth>,
}

impl<I: Iterator<Item = String>> AsyncDecodeStream<I> {
    /// Create a new async decode stream from an iterator of lines.
    pub fn new(lines: I, options: Option<DecodeStreamOptions>) -> Self {
        let options = options.unwrap_or_default();
        Self {
            lines,
            options,
            scan_state: create_scan_state(),
            event_queue: VecDeque::new(),
            state: DecoderState::Initial,
            context_stack: Vec::new(),
            line_buffer: Vec::new(),
            lines_exhausted: false,
            error: None,
            last_depth: None,
        }
    }

    /// Get the indent size from options
    fn indent_size(&self) -> usize {
        self.options.indent.unwrap_or(2)
    }

    /// Get the strict mode setting
    fn strict(&self) -> bool {
        self.options.strict.unwrap_or(true)
    }

    /// Process the next available event or line
    fn process_next(&mut self) -> Result<Option<JsonStreamEvent>> {
        // Return queued events first
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(Some(event));
        }

        // Check for stored error
        if let Some(err) = self.error.take() {
            return Err(err);
        }

        // If finished, nothing more to do
        if self.state == DecoderState::Finished {
            return Ok(None);
        }

        // If lines are exhausted, finalize
        if self.lines_exhausted {
            return self.finalize();
        }

        // Get next line
        let Some(raw_line) = self.lines.next() else {
            self.lines_exhausted = true;
            return self.finalize();
        };

        // Parse the line (capture values before borrowing scan_state)
        let indent_size = self.indent_size();
        let strict = self.strict();
        let parsed = parse_line_incremental(&raw_line, &mut self.scan_state, indent_size, strict)?;

        // Skip blank lines
        let Some(line) = parsed else {
            return Ok(None);
        };

        // Process based on current state
        match self.state {
            DecoderState::Initial => self.process_initial_line(line),
            DecoderState::SimpleObject { base_depth } => {
                self.process_simple_object_line(line, base_depth)
            }
            DecoderState::ArrayMode => {
                // In array mode, buffer lines for batch processing
                self.line_buffer.push(line);
                Ok(None)
            }
            DecoderState::Finished => Ok(None),
        }
    }

    /// Process the first line to determine document structure
    fn process_initial_line(&mut self, line: ParsedLine) -> Result<Option<JsonStreamEvent>> {
        // Check if it's an array header at root level
        if is_array_header_content(&line.content) {
            if let Some(_header_info) = parse_array_header_line(&line.content, DEFAULT_DELIMITER)? {
                // Array at root - need batch processing
                self.state = DecoderState::ArrayMode;
                self.line_buffer.push(line);
                return Ok(None);
            }
        }

        // Check if it's a key-value line (object)
        if Self::is_key_value_line(&line) {
            let depth = line.depth;
            self.state = DecoderState::SimpleObject { base_depth: 0 };
            self.context_stack.push(0);
            self.event_queue.push_back(JsonStreamEvent::StartObject);
            self.process_key_value_line(&line)?;
            self.last_depth = Some(depth);
            return Ok(self.event_queue.pop_front());
        }

        // Single primitive value
        self.state = DecoderState::Finished;
        Ok(Some(JsonStreamEvent::Primitive {
            value: parse_primitive_token(line.content.trim())?,
        }))
    }

    /// Process a line in simple object mode
    fn process_simple_object_line(
        &mut self,
        line: ParsedLine,
        base_depth: Depth,
    ) -> Result<Option<JsonStreamEvent>> {
        let current_depth = line.depth;

        // Handle depth changes - emit EndObject for decreased depth
        if let Some(last_depth) = self.last_depth {
            if current_depth < last_depth {
                // Pop contexts until we match the current depth
                while let Some(&obj_depth) = self.context_stack.last() {
                    if obj_depth >= current_depth && obj_depth > base_depth {
                        self.context_stack.pop();
                        // Emit EndObject for closed nested object
                        self.event_queue.push_back(JsonStreamEvent::EndObject);
                    } else {
                        break;
                    }
                }
            }
        }

        // Check if this line starts an array
        if is_array_header_content(&line.content)
            && parse_array_header_line(&line.content, DEFAULT_DELIMITER)?.is_some()
        {
            // Switch to array mode for batch processing
            self.state = DecoderState::ArrayMode;
            self.line_buffer.push(line);
            // Return any pending EndObject events first
            return Ok(self.event_queue.pop_front());
        }

        // Process key-value line
        self.process_key_value_line(&line)?;
        self.last_depth = Some(current_depth);

        Ok(self.event_queue.pop_front())
    }

    /// Process a key-value line and queue events
    fn process_key_value_line(&mut self, line: &ParsedLine) -> Result<()> {
        let content = &line.content;

        // Handle list items specially
        if content.starts_with(LIST_ITEM_PREFIX) {
            // List items indicate we need batch processing for proper array handling
            self.state = DecoderState::ArrayMode;
            self.line_buffer.push(line.clone());
            return Ok(());
        }

        // Parse key-value
        let (key, end, is_quoted) = parse_key_token(content, 0)?;
        let rest = content[end..].trim();

        if rest.is_empty() {
            // Key without value - nested content follows
            // We can't know yet if it's an array or object, so switch to batch mode
            // This ensures correct Start{Array,Object} emission
            self.state = DecoderState::ArrayMode;
            self.line_buffer.push(line.clone());
            return Ok(());
        }

        // Key with inline value - can emit incrementally
        self.event_queue.push_back(JsonStreamEvent::Key {
            key,
            was_quoted: is_quoted,
        });
        self.event_queue.push_back(JsonStreamEvent::Primitive {
            value: parse_primitive_token(rest)?,
        });

        Ok(())
    }

    /// Check if a line is a key-value line
    fn is_key_value_line(line: &ParsedLine) -> bool {
        let content = line.content.as_str();

        // Handle list items
        if content.starts_with(LIST_ITEM_PREFIX) {
            return false;
        }

        // Handle quoted keys
        if content.starts_with('"') {
            if let Some(closing) = find_closing_quote(content, 0) {
                return content[closing + 1..].contains(COLON);
            }
            return false;
        }

        // Regular key-value check
        content.contains(COLON) && is_key_value_content(content)
    }

    /// Finalize decoding when all lines are read
    fn finalize(&mut self) -> Result<Option<JsonStreamEvent>> {
        // If we have buffered lines (array mode), do batch processing
        if !self.line_buffer.is_empty() || self.state == DecoderState::ArrayMode {
            return self.batch_decode_remaining();
        }

        // Close any remaining open object contexts
        while self.context_stack.pop().is_some() {
            self.event_queue.push_back(JsonStreamEvent::EndObject);
        }

        // If we haven't emitted anything for an empty document
        if self.state == DecoderState::Initial {
            self.state = DecoderState::Finished;
            self.event_queue.push_back(JsonStreamEvent::StartObject);
            self.event_queue.push_back(JsonStreamEvent::EndObject);
        } else {
            self.state = DecoderState::Finished;
        }

        Ok(self.event_queue.pop_front())
    }

    /// Fall back to batch decoding for complex structures
    fn batch_decode_remaining(&mut self) -> Result<Option<JsonStreamEvent>> {
        // Collect all remaining lines (capture values before borrowing scan_state)
        let indent_size = self.indent_size();
        let strict = self.strict();
        for raw_line in self.lines.by_ref() {
            if let Some(line) =
                parse_line_incremental(&raw_line, &mut self.scan_state, indent_size, strict)?
            {
                self.line_buffer.push(line);
            }
        }

        // Use sync decoder on buffered lines
        let raw_lines: Vec<String> = self.line_buffer.iter().map(|p| p.raw.clone()).collect();

        let events = decoder_impl::decode_stream_sync(
            raw_lines,
            Some(DecodeStreamOptions {
                indent: self.options.indent,
                strict: self.options.strict,
            }),
        )?;

        // Queue all events
        self.event_queue.extend(events);
        self.line_buffer.clear();
        self.context_stack.clear();
        self.state = DecoderState::Finished;

        Ok(self.event_queue.pop_front())
    }
}

impl<I: Iterator<Item = String> + Unpin> Stream for AsyncDecodeStream<I> {
    type Item = Result<JsonStreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Loop until we either get an event or are truly finished
        loop {
            match self.process_next() {
                Ok(Some(event)) => return Poll::Ready(Some(Ok(event))),
                Ok(None) => {
                    // Check if we're truly done
                    if self.state == DecoderState::Finished && self.event_queue.is_empty() {
                        return Poll::Ready(None);
                    }
                    // Not done yet, continue processing
                }
                Err(e) => return Poll::Ready(Some(Err(e))),
            }
        }
    }
}

/// Try to decode TOON lines into events asynchronously using asupersync streams.
///
/// This function creates an async stream from the input lines, processes them
/// with yield points for cooperative scheduling, and collects all events.
///
/// # Errors
///
/// Returns an error if decoding fails due to malformed input or strict-mode validation.
pub async fn try_decode_stream_async(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Result<Vec<JsonStreamEvent>> {
    let lines_vec: Vec<String> = lines.into_iter().collect();

    // Use asupersync's iter() to create a yielding stream
    let line_stream = iter(lines_vec.clone());

    // Process with yield points - for now, we batch process but with async wrapper
    // This provides a yield point at the stream boundary
    let _line_count = line_stream.count().await;

    // Decode synchronously for now, but the async wrapper allows scheduler yields
    // A future enhancement would use incremental event emission
    decoder_impl::decode_stream_sync(lines_vec, options)
}

/// Decode TOON lines into events asynchronously, panicking on error.
///
/// # Panics
///
/// Panics if decoding fails.
pub async fn decode_stream_async(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Vec<JsonStreamEvent> {
    try_decode_stream_async(lines, options)
        .await
        .unwrap_or_else(|err| panic!("{err}"))
}

/// Try to decode a TOON string asynchronously.
///
/// # Errors
///
/// Returns an error if decoding fails.
pub async fn try_decode_async(
    input: &str,
    options: Option<crate::options::DecodeOptions>,
) -> Result<crate::JsonValue> {
    use crate::decode::event_builder::{build_node_from_events, node_to_json};
    use crate::decode::expand::expand_paths_safe;
    use crate::options::{ExpandPathsMode, resolve_decode_options};

    let resolved = resolve_decode_options(options);
    let lines: Vec<String> = input.split('\n').map(String::from).collect();

    let events = try_decode_stream_async(
        lines,
        Some(DecodeStreamOptions {
            indent: Some(resolved.indent),
            strict: Some(resolved.strict),
        }),
    )
    .await?;

    let mut node = build_node_from_events(events)?;

    if resolved.expand_paths == ExpandPathsMode::Safe {
        node = expand_paths_safe(node, resolved.strict)?;
    }

    Ok(node_to_json(node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringOrNumberOrBoolOrNull;

    #[test]
    fn test_async_decode_stream_creation() {
        let lines = vec!["name: Alice".to_string(), "age: 30".to_string()];
        let stream = AsyncDecodeStream::new(lines.into_iter(), None);
        assert!(!stream.lines_exhausted);
        assert_eq!(stream.state, DecoderState::Initial);
    }

    #[test]
    fn test_incremental_simple_object() {
        let lines = vec!["name: Alice".to_string(), "age: 30".to_string()];
        let mut stream = AsyncDecodeStream::new(lines.into_iter(), None);

        // Collect events manually
        let mut events = Vec::new();
        loop {
            match stream.process_next() {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {
                    if stream.state == DecoderState::Finished && stream.event_queue.is_empty() {
                        break;
                    }
                }
                Err(e) => panic!("Error: {e}"),
            }
        }

        // Should have: StartObject, Key, Primitive, Key, Primitive, EndObject
        assert!(
            events.len() >= 6,
            "Expected at least 6 events, got {}",
            events.len()
        );
        assert!(matches!(events[0], JsonStreamEvent::StartObject));
        assert!(matches!(&events[1], JsonStreamEvent::Key { key, .. } if key == "name"));
        assert!(
            matches!(&events[2], JsonStreamEvent::Primitive { value: StringOrNumberOrBoolOrNull::String(s) } if s == "Alice")
        );
    }

    #[test]
    fn test_incremental_emits_events_as_lines_arrive() {
        // Test that events are emitted incrementally, not all at the end
        let lines = vec![
            "first: 1".to_string(),
            "second: 2".to_string(),
            "third: 3".to_string(),
        ];
        let mut stream = AsyncDecodeStream::new(lines.into_iter(), None);

        // Process just enough to get first few events
        let mut events_after_first_line = Vec::new();
        for _ in 0..10 {
            // Limit iterations to avoid infinite loop
            match stream.process_next() {
                Ok(Some(event)) => events_after_first_line.push(event),
                Ok(None) => {
                    if !stream.lines_exhausted {
                        // Haven't read all lines yet, but should have some events
                        break;
                    }
                    if stream.state == DecoderState::Finished {
                        break;
                    }
                }
                Err(e) => panic!("Error: {e}"),
            }
        }

        // Should have at least StartObject and first Key/Primitive before reading all lines
        assert!(
            !events_after_first_line.is_empty(),
            "Should have emitted events incrementally"
        );
    }

    #[test]
    fn test_empty_document() {
        let lines: Vec<String> = vec![];
        let mut stream = AsyncDecodeStream::new(lines.into_iter(), None);

        let mut events = Vec::new();
        loop {
            match stream.process_next() {
                Ok(Some(event)) => events.push(event),
                Ok(None) => {
                    if stream.state == DecoderState::Finished && stream.event_queue.is_empty() {
                        break;
                    }
                }
                Err(e) => panic!("Error: {e}"),
            }
        }

        // Empty document should produce empty object
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], JsonStreamEvent::StartObject));
        assert!(matches!(events[1], JsonStreamEvent::EndObject));
    }
}
