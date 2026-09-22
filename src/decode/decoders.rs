use crate::JsonStreamEvent;
use crate::decode::parser::{
    FieldName, is_array_header_content, is_key_value_content, map_row_values_to_primitives,
    parse_array_header_line, parse_delimited_values, parse_key_token, parse_primitive_token,
};
use crate::decode::scanner::{
    Depth, ParsedLine, StreamingLineCursor, create_scan_state, parse_lines_sync,
};
use crate::decode::validation::{
    assert_expected_count, is_data_row, validate_no_blank_lines_in_range,
    validate_no_extra_list_items, validate_no_extra_tabular_rows,
};
use crate::error::{Result, ToonError};
use crate::options::DecodeStreamOptions;
use crate::shared::constants::{DEFAULT_DELIMITER, LIST_ITEM_MARKER, LIST_ITEM_PREFIX};

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
        // The root array is the whole document; a line after it used to be ignored silently.
        if context.strict
            && let Some(extra) = cursor.peek_sync()
        {
            return Err(ToonError::validation(
                extra.line_number,
                "Unexpected content after the document root",
            ));
        }
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

    // Every remaining line belongs to the root object. A deeper line that no nested block
    // consumed used to end the decode, dropping it and every later line without a word.
    while let Some(line) = cursor.peek_sync().cloned() {
        if line.depth != 0 && context.strict {
            return Err(over_indented_line(&line, 0));
        }
        cursor.advance_sync();
        decode_key_value_sync(&mut events, &line.content, &mut cursor, line.depth, context)?;
    }

    events.push(JsonStreamEvent::EndObject);
    Ok(events)
}

fn over_indented_line(line: &ParsedLine, expected_depth: Depth) -> ToonError {
    ToonError::validation(
        line.line_number,
        format!(
            "Over-indented line: expected depth {expected_depth}, but found {}",
            line.depth
        ),
    )
}

fn depth_jump(line: &ParsedLine, expected_depth: Depth) -> ToonError {
    ToonError::validation(
        line.line_number,
        format!(
            "Indentation depth jump: expected depth {expected_depth}, but found {}",
            line.depth
        ),
    )
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

    while let Some(line) = cursor.peek_sync().cloned() {
        if line.depth < base_depth {
            break;
        }

        // The first field fixes the depth of its siblings; it may sit one level deeper than its
        // parent, not two.
        if computed_depth.is_none() && options.strict && line.depth > base_depth {
            return Err(depth_jump(&line, base_depth));
        }
        let fields_depth = *computed_depth.get_or_insert(line.depth);

        // A line deeper than the fields that the previous field did not consume (it held a
        // primitive) is malformed. Strict mode reports it; lenient mode keeps it as a field of
        // this object instead of dropping it and the rest of the document.
        if line.depth != fields_depth && options.strict {
            return Err(over_indented_line(&line, fields_depth));
        }

        cursor.advance_sync();
        decode_key_value_sync(events, &line.content, cursor, line.depth, options)?;
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

    if header.fields.is_some() {
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

    // Strict mode stops at the declared count and reports a surplus row below. Lenient mode
    // never lets `[N]` truncate the block (spec §14.1): the surplus rows used to stay
    // unconsumed and end the whole document.
    while !options.strict || row_count < header.length {
        let line = cursor.peek_sync().cloned();
        let Some(line) = line else {
            break;
        };
        if line.depth < row_depth {
            break;
        }

        // Spec §9.3: at row depth, a line whose first unquoted colon precedes the first
        // unquoted delimiter is a key-value line and ends the rows.
        if line.depth == row_depth && is_data_row(&line.content, header.delimiter) {
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

    // As for tabular rows: strict mode stops at `[N]`, lenient mode takes every item.
    while !options.strict || item_count < header.length {
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

        decode_list_item_fields_sync(events, cursor, base_depth + 1, options)?;
        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    if is_key_value_content(&after_hyphen) {
        events.push(JsonStreamEvent::StartObject);
        decode_key_value_sync(events, &after_hyphen, cursor, base_depth + 1, options)?;

        decode_list_item_fields_sync(events, cursor, base_depth + 1, options)?;
        events.push(JsonStreamEvent::EndObject);
        return Ok(());
    }

    events.push(JsonStreamEvent::Primitive {
        value: parse_primitive_token(&after_hyphen)?,
    });
    Ok(())
}

/// The remaining fields of a list-item object, one level below the hyphen line.
fn decode_list_item_fields_sync(
    events: &mut Vec<JsonStreamEvent>,
    cursor: &mut StreamingLineCursor,
    fields_depth: Depth,
    options: DecoderContext,
) -> Result<()> {
    while let Some(line) = cursor.peek_sync().cloned() {
        if line.depth < fields_depth
            || line.content.starts_with(LIST_ITEM_PREFIX)
            || line.content == LIST_ITEM_MARKER
        {
            break;
        }
        // As in `decode_object_fields_sync`: a deeper line nobody consumed is an error in
        // strict mode and a field of this object in lenient mode, never silently dropped.
        if line.depth > fields_depth && options.strict {
            return Err(over_indented_line(&line, fields_depth));
        }
        cursor.advance_sync();
        decode_key_value_sync(events, &line.content, cursor, line.depth, options)?;
    }
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

/// A key-value line has a colon outside quotes, at the root exactly as in a list item (the root
/// used to count any colon, so `a"b: c` meant an object there and a string in a list).
fn is_key_value_line_sync(line: &ParsedLine) -> bool {
    is_key_value_content(&line.content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringOrNumberOrBoolOrNull;

    fn decode(input: &str) -> Vec<JsonStreamEvent> {
        decode_stream_sync(input.lines().map(String::from), None).unwrap()
    }

    fn decode_lax(input: &str) -> Vec<JsonStreamEvent> {
        decode_stream_sync(
            input.lines().map(String::from),
            Some(DecodeStreamOptions {
                indent: None,
                strict: Some(false),
            }),
        )
        .unwrap()
    }

    #[test]
    fn empty_input_produces_empty_object() {
        let events = decode("");
        assert!(matches!(events.first(), Some(JsonStreamEvent::StartObject)));
        assert!(matches!(events.last(), Some(JsonStreamEvent::EndObject)));
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn decode_primitive_string() {
        let events = decode("hello");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            JsonStreamEvent::Primitive {
                value: StringOrNumberOrBoolOrNull::String(s),
            } if s == "hello"
        ));
    }

    #[test]
    fn decode_primitive_number() {
        let events = decode("42");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            JsonStreamEvent::Primitive {
                value: StringOrNumberOrBoolOrNull::Number(v),
            } if (*v - 42.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn decode_primitive_bool_and_null() {
        assert!(matches!(
            decode("true")[0],
            JsonStreamEvent::Primitive {
                value: StringOrNumberOrBoolOrNull::Bool(true),
            }
        ));
        assert!(matches!(
            decode("false")[0],
            JsonStreamEvent::Primitive {
                value: StringOrNumberOrBoolOrNull::Bool(false),
            }
        ));
        assert!(matches!(
            decode("null")[0],
            JsonStreamEvent::Primitive {
                value: StringOrNumberOrBoolOrNull::Null,
            }
        ));
    }

    #[test]
    fn decode_simple_key_value_object() {
        let events = decode("name: Alice\nage: 30");
        assert!(matches!(events[0], JsonStreamEvent::StartObject));
        assert!(matches!(
            &events[1],
            JsonStreamEvent::Key { key, .. } if key == "name"
        ));
        assert!(matches!(
            &events[2],
            JsonStreamEvent::Primitive {
                value: StringOrNumberOrBoolOrNull::String(s),
            } if s == "Alice"
        ));
        assert!(matches!(
            &events[3],
            JsonStreamEvent::Key { key, .. } if key == "age"
        ));
        assert!(matches!(events.last(), Some(JsonStreamEvent::EndObject)));
    }

    #[test]
    fn decode_inline_primitive_array() {
        let events = decode("tags[3]: red,green,blue");
        assert!(matches!(events[0], JsonStreamEvent::StartObject));
        assert!(matches!(
            &events[1],
            JsonStreamEvent::Key { key, .. } if key == "tags"
        ));
        assert!(matches!(
            events[2],
            JsonStreamEvent::StartArray { length: 3 }
        ));
        let red = &events[3];
        assert!(
            matches!(red, JsonStreamEvent::Primitive { value: StringOrNumberOrBoolOrNull::String(s) } if s == "red")
        );
    }

    #[test]
    fn decode_empty_array() {
        let events = decode("items[0]:");
        assert!(matches!(
            events[2],
            JsonStreamEvent::StartArray { length: 0 }
        ));
        assert!(matches!(events[3], JsonStreamEvent::EndArray));
    }

    #[test]
    fn decode_tabular_array() {
        let events = decode("users[2]{id,name}:\n  1,Alice\n  2,Bob");
        assert!(matches!(
            events[2],
            JsonStreamEvent::StartArray { length: 2 }
        ));
        // Expect 2 row objects, each with 2 key/primitive pairs
        assert!(
            events
                .iter()
                .filter(|e| matches!(e, JsonStreamEvent::StartObject))
                .count()
                >= 3
        );
    }

    #[test]
    fn decode_quoted_key_preserves_was_quoted_flag() {
        let events = decode("\"weird key\": 1");
        assert!(matches!(
            &events[1],
            JsonStreamEvent::Key { key, was_quoted: true } if key == "weird key"
        ));
    }

    #[test]
    fn decode_nested_object() {
        let events = decode("user:\n  id: 1\n  name: Alice");
        let start_count = events
            .iter()
            .filter(|e| matches!(e, JsonStreamEvent::StartObject))
            .count();
        assert_eq!(start_count, 2, "expected 2 StartObject events");
    }

    #[test]
    fn decode_list_array_of_objects() {
        let events = decode("rows[2]:\n  - a: 1\n  - a: 2");
        // Expect StartArray with length 2, then two object items
        let has_start_array = events
            .iter()
            .any(|e| matches!(e, JsonStreamEvent::StartArray { length: 2 }));
        assert!(has_start_array);
    }

    #[test]
    fn decode_extra_items_errors_in_strict_mode() {
        let result = decode_stream_sync("items[1]: a,b".lines().map(String::from), None);
        assert!(result.is_err(), "strict mode should reject length mismatch");
    }

    #[test]
    fn decode_extra_items_tolerated_in_lax_mode() {
        let events = decode_lax("items[1]: a,b");
        assert!(matches!(
            events
                .iter()
                .find(|e| matches!(e, JsonStreamEvent::StartArray { .. })),
            Some(JsonStreamEvent::StartArray { length: 1 })
        ));
    }

    #[test]
    fn decode_huge_declared_length_rejected() {
        let result = decode_stream_sync("items[9999999999]:".lines().map(String::from), None);
        assert!(result.is_err(), "cap on declared length should fire");
    }

    #[test]
    fn is_key_value_line_sync_detects_quoted_keys() {
        let line = ParsedLine {
            raw: "\"k\": v".to_string(),
            indent: 0,
            content: "\"k\": v".to_string(),
            depth: 0,
            line_number: 1,
        };
        assert!(is_key_value_line_sync(&line));
    }

    #[test]
    fn is_key_value_line_sync_detects_bare_keys() {
        let line = ParsedLine {
            raw: "k: v".to_string(),
            indent: 0,
            content: "k: v".to_string(),
            depth: 0,
            line_number: 1,
        };
        assert!(is_key_value_line_sync(&line));
    }

    #[test]
    fn is_key_value_line_sync_rejects_plain_strings() {
        let line = ParsedLine {
            raw: "plain".to_string(),
            indent: 0,
            content: "plain".to_string(),
            depth: 0,
            line_number: 1,
        };
        assert!(!is_key_value_line_sync(&line));
    }
}
