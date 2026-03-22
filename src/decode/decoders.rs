use crate::JsonStreamEvent;
use crate::decode::parser::{
    FieldName, is_array_header_content, is_key_value_content, map_row_values_to_primitives,
    parse_array_header_line, parse_delimited_values, parse_key_token, parse_primitive_token,
};
use crate::decode::scanner::{
    Depth, ParsedLine, StreamingLineCursor, create_scan_state, parse_lines_sync,
};
use crate::decode::validation::{
    assert_expected_count, validate_no_blank_lines_in_range, validate_no_extra_list_items,
    validate_no_extra_tabular_rows,
};
use crate::error::{Result, ToonError};
use crate::options::DecodeStreamOptions;
use crate::shared::constants::{COLON, DEFAULT_DELIMITER, LIST_ITEM_MARKER, LIST_ITEM_PREFIX};
use crate::shared::string_utils::find_closing_quote;

#[derive(Debug, Clone, Copy)]
pub struct DecoderContext {
    pub indent: usize,
    pub strict: bool,
}

/// Decode TOON input into a stream of JSON events.
///
/// # Errors
///
/// Returns an error if scanning or decoding fails (invalid indentation, malformed arrays,
/// or strict-mode validation failures).
pub fn decode_stream_sync(
    source: impl IntoIterator<Item = String>,
    options: Option<DecodeStreamOptions>,
) -> Result<Vec<JsonStreamEvent>> {
    let options = options.unwrap_or(DecodeStreamOptions {
        indent: None,
        strict: None,
    });
    let context = DecoderContext {
        indent: options.indent.unwrap_or(2),
        strict: options.strict.unwrap_or(true),
    };

    let mut scan_state = create_scan_state();
    let lines = parse_lines_sync(source, context.indent, context.strict, &mut scan_state)?;
    let mut cursor = StreamingLineCursor::new(lines, scan_state.blank_lines);

    let mut events = Vec::new();

    let first = cursor.peek_sync().cloned();
    let Some(first) = first else {
        events.push(JsonStreamEvent::StartObject);
        events.push(JsonStreamEvent::EndObject);
        return Ok(events);
    };

    if is_array_header_content(&first.content)
        && let Some(header_info) = parse_array_header_line(&first.content, DEFAULT_DELIMITER)?
    {
        cursor.advance_sync();
        decode_array_from_header_sync(&mut events, header_info, &mut cursor, 0, context)?;
        return Ok(events);
    }

    cursor.advance_sync();
    let has_more = !cursor.at_end_sync();
    if !has_more && !is_key_value_line_sync(&first) {
        events.push(JsonStreamEvent::Primitive {
            value: parse_primitive_token(first.content.trim())?,
        });
        return Ok(events);
    }

    events.push(JsonStreamEvent::StartObject);
    decode_key_value_sync(&mut events, &first.content, &mut cursor, 0, context)?;

    while !cursor.at_end_sync() {
        let line = cursor.peek_sync().cloned();
        let Some(line) = line else {
            break;
        };
        if line.depth != 0 {
            break;
        }
        cursor.advance_sync();
        decode_key_value_sync(&mut events, &line.content, &mut cursor, 0, context)?;
    }

    events.push(JsonStreamEvent::EndObject);
    Ok(events)
}

fn decode_key_value_sync(
    events: &mut Vec<JsonStreamEvent>,
    content: &str,
    cursor: &mut StreamingLineCursor,
    base_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    if let Some(header_info) = parse_array_header_line(content, DEFAULT_DELIMITER)?
        && let Some(key) = header_info.header.key.clone()
    {
        events.push(JsonStreamEvent::Key {
            key,
            was_quoted: header_info.header.key_was_quoted,
        });
        decode_array_from_header_sync(events, header_info, cursor, base_depth, options)?;
        return Ok(());
    }

    let (key, end, is_quoted) = parse_key_token(content, 0)?;
    let rest = content[end..].trim();

    events.push(JsonStreamEvent::Key {
        key,
        was_quoted: is_quoted,
    });

    if rest.is_empty() {
        let next_line = cursor.peek_sync();
        if let Some(next) = next_line
            && next.depth > base_depth
        {
            events.push(JsonStreamEvent::StartObject);
            decode_object_fields_sync(events, cursor, base_depth + 1, options)?;
            events.push(JsonStreamEvent::EndObject);
            return Ok(());
        }

        events.push(JsonStreamEvent::StartObject);
        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    events.push(JsonStreamEvent::Primitive {
        value: parse_primitive_token(rest)?,
    });
    Ok(())
}

fn decode_object_fields_sync(
    events: &mut Vec<JsonStreamEvent>,
    cursor: &mut StreamingLineCursor,
    base_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    let mut computed_depth: Option<Depth> = None;

    while !cursor.at_end_sync() {
        let line = cursor.peek_sync().cloned();
        let Some(line) = line else {
            break;
        };
        if line.depth < base_depth {
            break;
        }

        if computed_depth.is_none() {
            computed_depth = Some(line.depth);
        }

        if Some(line.depth) == computed_depth {
            cursor.advance_sync();
            decode_key_value_sync(events, &line.content, cursor, line.depth, options)?;
        } else {
            break;
        }
    }

    Ok(())
}

fn decode_array_from_header_sync(
    events: &mut Vec<JsonStreamEvent>,
    header_info: crate::decode::parser::ArrayHeaderParseResult,
    cursor: &mut StreamingLineCursor,
    base_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    let header = header_info.header;
    let inline_values = header_info.inline_values;

    events.push(JsonStreamEvent::StartArray {
        length: header.length,
    });

    if let Some(inline_values) = inline_values {
        decode_inline_primitive_array_sync(events, &header, &inline_values, options)?;
        events.push(JsonStreamEvent::EndArray);
        return Ok(());
    }

    if let Some(fields) = &header.fields
        && !fields.is_empty()
    {
        decode_tabular_array_sync(events, &header, cursor, base_depth, options)?;
        events.push(JsonStreamEvent::EndArray);
        return Ok(());
    }

    decode_list_array_sync(events, &header, cursor, base_depth, options)?;
    events.push(JsonStreamEvent::EndArray);
    Ok(())
}

fn decode_inline_primitive_array_sync(
    events: &mut Vec<JsonStreamEvent>,
    header: &crate::decode::parser::ArrayHeaderInfo,
    inline_values: &str,
    options: DecoderContext,
) -> Result<()> {
    if inline_values.trim().is_empty() {
        assert_expected_count(0, header.length, "inline array items", options.strict)?;
        return Ok(());
    }

    let values = parse_delimited_values(inline_values, header.delimiter);
    let primitives = map_row_values_to_primitives(&values)?;

    assert_expected_count(
        primitives.len(),
        header.length,
        "inline array items",
        options.strict,
    )?;

    for primitive in primitives {
        events.push(JsonStreamEvent::Primitive { value: primitive });
    }

    Ok(())
}

fn decode_tabular_array_sync(
    events: &mut Vec<JsonStreamEvent>,
    header: &crate::decode::parser::ArrayHeaderInfo,
    cursor: &mut StreamingLineCursor,
    base_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    let row_depth = base_depth + 1;
    let mut row_count = 0usize;
    let mut start_line: Option<usize> = None;
    let mut end_line: Option<usize> = None;

    while !cursor.at_end_sync() && row_count < header.length {
        let line = cursor.peek_sync().cloned();
        let Some(line) = line else {
            break;
        };
        if line.depth < row_depth {
            break;
        }

        if line.depth == row_depth {
            if start_line.is_none() {
                start_line = Some(line.line_number);
            }
            end_line = Some(line.line_number);

            cursor.advance_sync();
            let values = parse_delimited_values(&line.content, header.delimiter);
            let fields = header
                .fields
                .as_ref()
                .ok_or_else(|| ToonError::message("Tabular array is missing header fields"))?;
            assert_expected_count(
                values.len(),
                fields.len(),
                "tabular row values",
                options.strict,
            )?;

            let primitives = map_row_values_to_primitives(&values)?;
            yield_object_from_fields(events, fields, &primitives);

            row_count += 1;
        } else {
            break;
        }
    }

    assert_expected_count(row_count, header.length, "tabular rows", options.strict)?;

    if options.strict
        && let (Some(start), Some(end)) = (start_line, end_line)
    {
        validate_no_blank_lines_in_range(
            start,
            end,
            cursor.get_blank_lines(),
            options.strict,
            "tabular array",
        )?;
    }

    validate_no_extra_tabular_rows(cursor.peek_sync(), row_depth, header, options.strict)?;
    Ok(())
}

fn decode_list_array_sync(
    events: &mut Vec<JsonStreamEvent>,
    header: &crate::decode::parser::ArrayHeaderInfo,
    cursor: &mut StreamingLineCursor,
    base_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    let item_depth = base_depth + 1;
    let mut item_count = 0usize;
    let mut start_line: Option<usize> = None;
    let mut end_line: Option<usize> = None;

    while !cursor.at_end_sync() && item_count < header.length {
        let line = cursor.peek_sync().cloned();
        let Some(line) = line else {
            break;
        };
        if line.depth < item_depth {
            break;
        }

        let is_list_item =
            line.content.starts_with(LIST_ITEM_PREFIX) || line.content == LIST_ITEM_MARKER;
        if line.depth == item_depth && is_list_item {
            if start_line.is_none() {
                start_line = Some(line.line_number);
            }
            end_line = Some(line.line_number);

            decode_list_item_sync(events, cursor, item_depth, options)?;

            if let Some(current) = cursor.current() {
                end_line = Some(current.line_number);
            }

            item_count += 1;
        } else {
            break;
        }
    }

    assert_expected_count(
        item_count,
        header.length,
        "list array items",
        options.strict,
    )?;

    if options.strict
        && let (Some(start), Some(end)) = (start_line, end_line)
    {
        validate_no_blank_lines_in_range(
            start,
            end,
            cursor.get_blank_lines(),
            options.strict,
            "list array",
        )?;
    }

    validate_no_extra_list_items(
        cursor.peek_sync(),
        item_depth,
        header.length,
        options.strict,
    )?;
    Ok(())
}

fn decode_list_item_sync(
    events: &mut Vec<JsonStreamEvent>,
    cursor: &mut StreamingLineCursor,
    base_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    let line = cursor
        .next_sync()
        .ok_or_else(|| ToonError::message("Expected list item"))?;

    if line.content == LIST_ITEM_MARKER {
        events.push(JsonStreamEvent::StartObject);
        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    let after_hyphen = if line.content.starts_with(LIST_ITEM_PREFIX) {
        line.content[LIST_ITEM_PREFIX.len()..].to_string()
    } else {
        return Err(ToonError::message(format!(
            "Expected list item to start with \"{LIST_ITEM_PREFIX}\""
        )));
    };

    if after_hyphen.trim().is_empty() {
        events.push(JsonStreamEvent::StartObject);
        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    if is_array_header_content(&after_hyphen)
        && let Some(header_info) = parse_array_header_line(&after_hyphen, DEFAULT_DELIMITER)?
    {
        decode_array_from_header_sync(events, header_info, cursor, base_depth, options)?;
        return Ok(());
    }

    if let Some(header_info) = parse_array_header_line(&after_hyphen, DEFAULT_DELIMITER)?
        && header_info.header.key.is_some()
        && header_info.header.fields.is_some()
    {
        let header = header_info.header;
        events.push(JsonStreamEvent::StartObject);
        events.push(JsonStreamEvent::Key {
            key: header.key.clone().unwrap_or_default(),
            was_quoted: header.key_was_quoted,
        });
        decode_array_from_header_sync(
            events,
            crate::decode::parser::ArrayHeaderParseResult {
                header,
                inline_values: header_info.inline_values,
            },
            cursor,
            base_depth + 1,
            options,
        )?;

        let follow_depth = base_depth + 1;
        while !cursor.at_end_sync() {
            let next_line = cursor.peek_sync().cloned();
            let Some(next_line) = next_line else {
                break;
            };
            if next_line.depth < follow_depth {
                break;
            }
            if next_line.depth == follow_depth && !next_line.content.starts_with(LIST_ITEM_PREFIX) {
                cursor.advance_sync();
                decode_key_value_sync(events, &next_line.content, cursor, follow_depth, options)?;
            } else {
                break;
            }
        }

        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    if is_key_value_content(&after_hyphen) {
        events.push(JsonStreamEvent::StartObject);
        decode_key_value_sync(events, &after_hyphen, cursor, base_depth + 1, options)?;

        let follow_depth = base_depth + 1;
        while !cursor.at_end_sync() {
            let next_line = cursor.peek_sync().cloned();
            let Some(next_line) = next_line else {
                break;
            };
            if next_line.depth < follow_depth {
                break;
            }
            if next_line.depth == follow_depth && !next_line.content.starts_with(LIST_ITEM_PREFIX) {
                cursor.advance_sync();
                decode_key_value_sync(events, &next_line.content, cursor, follow_depth, options)?;
            } else {
                break;
            }
        }

        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    events.push(JsonStreamEvent::Primitive {
        value: parse_primitive_token(&after_hyphen)?,
    });
    Ok(())
}

fn yield_object_from_fields(
    events: &mut Vec<JsonStreamEvent>,
    fields: &[FieldName],
    primitives: &[crate::JsonPrimitive],
) {
    events.push(JsonStreamEvent::StartObject);
    for (idx, field) in fields.iter().enumerate() {
        events.push(JsonStreamEvent::Key {
            key: field.name.clone(),
            was_quoted: field.was_quoted,
        });
        if let Some(value) = primitives.get(idx) {
            events.push(JsonStreamEvent::Primitive {
                value: value.clone(),
            });
        } else {
            events.push(JsonStreamEvent::Primitive {
                value: crate::StringOrNumberOrBoolOrNull::Null,
            });
        }
    }
    events.push(JsonStreamEvent::EndObject);
}

fn is_key_value_line_sync(line: &ParsedLine) -> bool {
    let content = line.content.as_str();
    if content.starts_with('"') {
        if let Some(closing) = find_closing_quote(content, 0) {
            return content[closing + 1..].contains(COLON);
        }
        return false;
    }
    content.contains(COLON)
}
