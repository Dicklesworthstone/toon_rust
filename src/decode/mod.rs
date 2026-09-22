pub mod decoders;
pub mod event_builder;
pub mod expand;
pub mod parser;
pub mod scanner;
pub mod validation;

#[cfg(feature = "async-stream")]
mod async_decode;

use crate::decode::decoders as decoder_impl;
use crate::decode::event_builder::{build_node_from_events, node_to_json};
use crate::decode::expand::expand_paths_safe;
use crate::error::Result;
use crate::options::{DecodeOptions, DecodeStreamOptions, ExpandPathsMode, resolve_decode_options};
use crate::{JsonStreamEvent, JsonValue};

#[cfg(feature = "async-stream")]
pub use async_decode::{
    AsyncDecodeStream, decode_stream_async, try_decode_async, try_decode_stream_async,
};

/// Try to decode a TOON string into a JSON value, returning a Result.
///
/// This is the fallible version of [`decode`]. Use this when you want to handle
/// decoding errors gracefully instead of panicking.
///
/// # Errors
///
/// Returns an error if decoding fails due to malformed input or strict-mode validation errors.
pub fn try_decode(input: &str, options: Option<DecodeOptions>) -> Result<JsonValue> {
    let lines = input
        .split('\n')
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    try_decode_from_lines(lines, options)
}

/// Decode a TOON string into a JSON value.
///
/// # Panics
///
/// Panics if decoding fails due to malformed input or strict-mode validation errors.
/// Use [`try_decode`] for a fallible version that returns `Result`.
#[must_use]
pub fn decode(input: &str, options: Option<DecodeOptions>) -> JsonValue {
    try_decode(input, options).unwrap_or_else(|err| panic!("{err}"))
}

/// Try to decode TOON lines into a JSON value, returning a Result.
///
/// This is the fallible version of [`decode_from_lines`]. Use this when you want to handle
/// decoding errors gracefully instead of panicking.
///
/// # Errors
///
/// Returns an error if decoding fails due to malformed input or strict-mode validation errors.
pub fn try_decode_from_lines(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeOptions>,
) -> Result<JsonValue> {
    let resolved = resolve_decode_options(options);
    let events = decoder_impl::decode_stream_sync(
        lines,
        Some(DecodeStreamOptions {
            indent: Some(resolved.indent),
            strict: Some(resolved.strict),
        }),
    )?;

    let mut node = build_node_from_events(events, resolved.strict)?;

    if resolved.expand_paths == ExpandPathsMode::Safe {
        node = expand_paths_safe(node, resolved.strict)?;
    }

    Ok(node_to_json(node))
}

#[must_use]
/// Decode TOON lines into a JSON value.
///
/// # Panics
///
/// Panics if decoding fails due to malformed input or strict-mode validation errors.
/// Use [`try_decode_from_lines`] for a fallible version that returns `Result`.
pub fn decode_from_lines(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeOptions>,
) -> JsonValue {
    try_decode_from_lines(lines, options).unwrap_or_else(|err| panic!("{err}"))
}

/// Try to decode TOON lines into a stream of events, returning a Result.
///
/// This is the fallible version of [`decode_stream_sync`]. Use this when you want to handle
/// decoding errors gracefully instead of panicking.
///
/// # Errors
///
/// Returns an error if decoding fails due to malformed input or strict-mode validation errors.
pub fn try_decode_stream_sync(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Result<Vec<JsonStreamEvent>> {
    decoder_impl::decode_stream_sync(lines, options)
}

#[must_use]
/// Decode TOON lines into a stream of events.
///
/// # Panics
///
/// Panics if decoding fails due to malformed input or strict-mode validation errors.
/// Use [`try_decode_stream_sync`] for a fallible version that returns `Result`.
pub fn decode_stream_sync(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Vec<JsonStreamEvent> {
    try_decode_stream_sync(lines, options).unwrap_or_else(|err| panic!("{err}"))
}

/// Try to decode TOON lines into a stream of events asynchronously, returning a Result.
///
/// This is the fallible version of [`decode_stream`]. Use this when you want to handle
/// decoding errors gracefully instead of panicking.
///
/// When the `async-stream` feature is enabled, this uses the asupersync runtime for
/// true async streaming with cancellation support. Otherwise, it falls back to
/// synchronous decoding wrapped in an async function.
///
/// # Errors
///
/// Returns an error if decoding fails due to malformed input or strict-mode validation errors.
#[cfg(not(feature = "async-stream"))]
pub async fn try_decode_stream(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Result<Vec<JsonStreamEvent>> {
    decoder_impl::decode_stream_sync(lines, options)
}

/// Try to decode TOON lines into a stream of events asynchronously, returning a Result.
///
/// This version uses the asupersync runtime for true async streaming with
/// cancellation support and yield points between line processing.
///
/// # Errors
///
/// Returns an error if decoding fails due to malformed input or strict-mode validation errors.
#[cfg(feature = "async-stream")]
pub async fn try_decode_stream(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Result<Vec<JsonStreamEvent>> {
    async_decode::try_decode_stream_async(lines, options).await
}

#[must_use]
/// Decode TOON lines into a stream of events asynchronously.
///
/// # Panics
///
/// Panics if decoding fails due to malformed input or strict-mode validation errors.
/// Use [`try_decode_stream`] for a fallible version that returns `Result`.
pub async fn decode_stream(
    lines: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Vec<JsonStreamEvent> {
    try_decode_stream(lines, options)
        .await
        .unwrap_or_else(|err| panic!("{err}"))
}
