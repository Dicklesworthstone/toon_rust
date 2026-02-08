use std::collections::HashSet;

use crate::encode::folding::try_fold_key_chain;
use crate::encode::normalize::{
    is_array_of_arrays, is_array_of_objects, is_array_of_primitives, is_empty_object,
    is_json_primitive,
};
use crate::encode::primitives::{
    encode_and_join_primitives, encode_key, encode_primitive, format_header,
};
use crate::options::ResolvedEncodeOptions;
use crate::shared::constants::{DOT, LIST_ITEM_MARKER, LIST_ITEM_PREFIX};
use crate::{JsonArray, JsonObject, JsonPrimitive, JsonValue};

#[must_use]
pub fn encode_json_value(value: &JsonValue, options: &ResolvedEncodeOptions) -> Vec<String> {
    let estimated_lines = estimate_line_count(value);
    let mut out = Vec::with_capacity(estimated_lines);
    match value {
        JsonValue::Primitive(primitive) => {
            let encoded = encode_primitive(primitive, options.delimiter);
            if !encoded.is_empty() {
                out.push(encoded);
            }
        }
        JsonValue::Array(items) => {
            encode_array_lines(None, items, 0, options, &mut out);
        }
        JsonValue::Object(entries) => {
            encode_object_lines(entries, 0, options, None, None, None, &mut out);
        }
    }
    out
}

fn encode_object_lines(
    value: &JsonObject,
    depth: usize,
    options: &ResolvedEncodeOptions,
    root_literal_keys: Option<&HashSet<String>>,
    path_prefix: Option<&str>,
    remaining_depth: Option<usize>,
    out: &mut Vec<String>,
) {
    // Collect keys as references to avoid cloning
    let keys: Vec<&str> = value.iter().map(|(key, _)| key.as_str()).collect();

    let mut root_literal_set = HashSet::new();
    let root_literal_keys = if depth == 0 && root_literal_keys.is_none() {
        for key in &keys {
            if key.contains(DOT) {
                root_literal_set.insert((*key).to_string());
            }
        }
        Some(&root_literal_set)
    } else {
        root_literal_keys
    };

    let effective_flatten_depth = remaining_depth.unwrap_or(options.flatten_depth);

    for (key, val) in value {
        encode_key_value_pair_lines(
            key,
            val,
            depth,
            options,
            &keys,
            root_literal_keys,
            path_prefix,
            effective_flatten_depth,
            out,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_key_value_pair_lines(
    key: &str,
    value: &JsonValue,
    depth: usize,
    options: &ResolvedEncodeOptions,
    siblings: &[&str],
    root_literal_keys: Option<&HashSet<String>>,
    path_prefix: Option<&str>,
    flatten_depth: usize,
    out: &mut Vec<String>,
) {
    let current_path =
        path_prefix.map_or_else(|| key.to_string(), |prefix| format!("{prefix}{DOT}{key}"));

    if let Some(folded) = try_fold_key_chain(
        key,
        value,
        siblings,
        options,
        root_literal_keys,
        path_prefix,
        flatten_depth,
    ) {
        let encoded_key = encode_key(&folded.folded_key);

        if folded.remainder.is_none() {
            match folded.leaf_value {
                JsonValue::Primitive(primitive) => {
                    let encoded = encode_primitive(&primitive, options.delimiter);
                    out.push(indented_key_value_line(
                        depth,
                        &encoded_key,
                        &encoded,
                        options.indent,
                    ));
                    return;
                }
                JsonValue::Array(items) => {
                    encode_array_lines(Some(&folded.folded_key), &items, depth, options, out);
                    return;
                }
                JsonValue::Object(entries) => {
                    if is_empty_object(&entries) {
                        out.push(indented_key_colon_line(depth, &encoded_key, options.indent));
                        return;
                    }
                }
            }
        }

        if let Some(JsonValue::Object(entries)) = folded.remainder {
            out.push(indented_key_colon_line(depth, &encoded_key, options.indent));
            let remaining_depth = flatten_depth.saturating_sub(folded.segment_count);
            let folded_path = if let Some(prefix) = path_prefix {
                format!("{prefix}{DOT}{}", folded.folded_key)
            } else {
                folded.folded_key.clone()
            };
            encode_object_lines(
                &entries,
                depth + 1,
                options,
                root_literal_keys,
                Some(&folded_path),
                Some(remaining_depth),
                out,
            );
            return;
        }
    }

    let encoded_key = encode_key(key);

    match value {
        JsonValue::Primitive(primitive) => {
            let encoded = encode_primitive(primitive, options.delimiter);
            out.push(indented_key_value_line(
                depth,
                &encoded_key,
                &encoded,
                options.indent,
            ));
        }
        JsonValue::Array(items) => {
            encode_array_lines(Some(key), items, depth, options, out);
        }
        JsonValue::Object(entries) => {
            out.push(indented_key_colon_line(depth, &encoded_key, options.indent));
            if !is_empty_object(entries) {
                encode_object_lines(
                    entries,
                    depth + 1,
                    options,
                    root_literal_keys,
                    Some(&current_path),
                    Some(flatten_depth),
                    out,
                );
            }
        }
    }
}

fn encode_array_lines(
    key: Option<&str>,
    value: &JsonArray,
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    if value.is_empty() {
        let header = format_header(0, key, None, options.delimiter);
        out.push(indented_line(depth, &header, options.indent));
        return;
    }

    if is_array_of_primitives(value) {
        let array_line = encode_inline_array_line(value, options.delimiter, key);
        out.push(indented_line(depth, &array_line, options.indent));
        return;
    }

    if is_array_of_arrays(value) {
        let all_primitive_arrays = value.iter().all(|item| match item {
            JsonValue::Array(items) => is_array_of_primitives(items),
            _ => false,
        });
        if all_primitive_arrays {
            encode_array_of_arrays_as_list_items_lines(key, value, depth, options, out);
            return;
        }
    }

    if is_array_of_objects(value) {
        if let Some(header) = extract_tabular_header(value) {
            encode_array_of_objects_as_tabular_lines(key, value, &header, depth, options, out);
        } else {
            encode_mixed_array_as_list_items_lines(key, value, depth, options, out);
        }
        return;
    }

    encode_mixed_array_as_list_items_lines(key, value, depth, options, out);
}

fn encode_array_of_arrays_as_list_items_lines(
    key: Option<&str>,
    values: &JsonArray,
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    let header = format_header(values.len(), key, None, options.delimiter);
    out.push(indented_line(depth, &header, options.indent));

    for item in values {
        if let JsonValue::Array(items) = item {
            let line = encode_inline_array_line(items, options.delimiter, None);
            out.push(indented_list_item(depth + 1, &line, options.indent));
        }
    }
}

fn encode_inline_array_line(values: &JsonArray, delimiter: char, key: Option<&str>) -> String {
    let primitives: Vec<JsonPrimitive> = values
        .iter()
        .filter_map(|item| match item {
            JsonValue::Primitive(primitive) => Some(primitive.clone()),
            _ => None,
        })
        .collect();
    let header = format_header(values.len(), key, None, delimiter);
    if primitives.is_empty() {
        return header;
    }
    let joined = encode_and_join_primitives(&primitives, delimiter);
    // Build "header joined" without format!
    let mut out = String::with_capacity(header.len() + 1 + joined.len());
    out.push_str(&header);
    out.push(' ');
    out.push_str(&joined);
    out
}

fn encode_array_of_objects_as_tabular_lines(
    key: Option<&str>,
    rows: &JsonArray,
    header: &[String],
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    let formatted_header = format_header(rows.len(), key, Some(header), options.delimiter);
    out.push(indented_line(depth, &formatted_header, options.indent));
    write_tabular_rows_lines(rows, header, depth + 1, options, out);
}

fn write_tabular_rows_lines(
    rows: &JsonArray,
    header: &[String],
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    for row in rows {
        if let JsonValue::Object(entries) = row {
            let mut values = Vec::with_capacity(header.len());
            for key in header {
                let value = object_get(entries, key).expect("tabular header missing key");
                if let JsonValue::Primitive(primitive) = value {
                    values.push(primitive.clone());
                } else {
                    panic!("tabular row contains non-primitive value");
                }
            }
            let joined = encode_and_join_primitives(&values, options.delimiter);
            out.push(indented_line(depth, &joined, options.indent));
        }
    }
}

fn extract_tabular_header(rows: &JsonArray) -> Option<Vec<String>> {
    if rows.is_empty() {
        return None;
    }

    let JsonValue::Object(first) = &rows[0] else {
        return None;
    };

    if first.is_empty() {
        return None;
    }

    let header: Vec<String> = first.iter().map(|(key, _)| key.clone()).collect();
    if is_tabular_array(rows, &header) {
        Some(header)
    } else {
        None
    }
}

fn is_tabular_array(rows: &JsonArray, header: &[String]) -> bool {
    for row in rows {
        let JsonValue::Object(entries) = row else {
            return false;
        };

        if entries.len() != header.len() {
            return false;
        }

        for key in header {
            let Some(value) = object_get(entries, key) else {
                return false;
            };
            if !is_json_primitive(value) {
                return false;
            }
        }
    }
    true
}

fn encode_mixed_array_as_list_items_lines(
    key: Option<&str>,
    items: &JsonArray,
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    let header = format_header(items.len(), key, None, options.delimiter);
    out.push(indented_line(depth, &header, options.indent));

    for item in items {
        encode_list_item_value_lines(item, depth + 1, options, out);
    }
}

fn encode_object_as_list_item_lines(
    obj: &JsonObject,
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    if obj.is_empty() {
        out.push(indented_line(depth, LIST_ITEM_MARKER, options.indent));
        return;
    }

    let first = obj[0].clone();
    let rest = if obj.len() > 1 {
        obj[1..].to_vec()
    } else {
        Vec::new()
    };
    let (first_key, first_value) = first;

    if let JsonValue::Array(items) = &first_value {
        if is_array_of_objects(items) {
            if let Some(header) = extract_tabular_header(items) {
                let formatted = format_header(
                    items.len(),
                    Some(&first_key),
                    Some(&header),
                    options.delimiter,
                );
                out.push(indented_list_item(depth, &formatted, options.indent));
                write_tabular_rows_lines(items, &header, depth + 2, options, out);
                if !rest.is_empty() {
                    encode_object_lines(&rest, depth + 1, options, None, None, None, out);
                }
                return;
            }
        }
    }

    let encoded_key = encode_key(&first_key);

    match first_value {
        JsonValue::Primitive(primitive) => {
            let encoded = encode_primitive(&primitive, options.delimiter);
            out.push(indented_list_item_key_value(
                depth,
                &encoded_key,
                &encoded,
                options.indent,
            ));
        }
        JsonValue::Array(items) => {
            if items.is_empty() {
                let header = format_header(0, None, None, options.delimiter);
                out.push(indented_list_item_key_header(
                    depth,
                    &encoded_key,
                    &header,
                    options.indent,
                ));
            } else if is_array_of_primitives(&items) {
                let line = encode_inline_array_line(&items, options.delimiter, None);
                out.push(indented_list_item_key_header(
                    depth,
                    &encoded_key,
                    &line,
                    options.indent,
                ));
            } else {
                let header = format_header(items.len(), None, None, options.delimiter);
                out.push(indented_list_item_key_header(
                    depth,
                    &encoded_key,
                    &header,
                    options.indent,
                ));
                for item in &items {
                    encode_list_item_value_lines(item, depth + 2, options, out);
                }
            }
        }
        JsonValue::Object(entries) => {
            out.push(indented_list_item_key_colon(
                depth,
                &encoded_key,
                options.indent,
            ));
            if !is_empty_object(&entries) {
                encode_object_lines(&entries, depth + 2, options, None, None, None, out);
            }
        }
    }

    if !rest.is_empty() {
        encode_object_lines(&rest, depth + 1, options, None, None, None, out);
    }
}

fn encode_list_item_value_lines(
    value: &JsonValue,
    depth: usize,
    options: &ResolvedEncodeOptions,
    out: &mut Vec<String>,
) {
    match value {
        JsonValue::Primitive(primitive) => {
            let encoded = encode_primitive(primitive, options.delimiter);
            out.push(indented_list_item(depth, &encoded, options.indent));
        }
        JsonValue::Array(items) => {
            if is_array_of_primitives(items) {
                let line = encode_inline_array_line(items, options.delimiter, None);
                out.push(indented_list_item(depth, &line, options.indent));
            } else {
                let header = format_header(items.len(), None, None, options.delimiter);
                out.push(indented_list_item(depth, &header, options.indent));
                for item in items {
                    encode_list_item_value_lines(item, depth + 1, options, out);
                }
            }
        }
        JsonValue::Object(entries) => {
            encode_object_as_list_item_lines(entries, depth, options, out);
        }
    }
}

fn object_get<'a>(entries: &'a JsonObject, key: &str) -> Option<&'a JsonValue> {
    entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn indented_line(depth: usize, content: &str, indent_size: usize) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    let capacity = indent_chars.saturating_add(content.len());
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(content);
    out
}

/// Build "key: value" line directly without intermediate format! allocation
fn indented_key_value_line(depth: usize, key: &str, value: &str, indent_size: usize) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    // key + ": " + value
    let capacity = indent_chars
        .saturating_add(key.len())
        .saturating_add(2)
        .saturating_add(value.len());
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value);
    out
}

/// Build "key:" line directly without intermediate format! allocation
fn indented_key_colon_line(depth: usize, key: &str, indent_size: usize) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    // key + ":"
    let capacity = indent_chars.saturating_add(key.len()).saturating_add(1);
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(key);
    out.push(':');
    out
}

fn indented_list_item(depth: usize, content: &str, indent_size: usize) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    let prefix_len = LIST_ITEM_PREFIX.len();
    let capacity = indent_chars
        .saturating_add(prefix_len)
        .saturating_add(content.len());
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(LIST_ITEM_PREFIX);
    out.push_str(content);
    out
}

/// Build "- key: value" list item directly without intermediate format! allocation
fn indented_list_item_key_value(
    depth: usize,
    key: &str,
    value: &str,
    indent_size: usize,
) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    let prefix_len = LIST_ITEM_PREFIX.len();
    // "- " + key + ": " + value
    let capacity = indent_chars
        .saturating_add(prefix_len)
        .saturating_add(key.len())
        .saturating_add(2)
        .saturating_add(value.len());
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(LIST_ITEM_PREFIX);
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value);
    out
}

/// Build "- key:" list item directly without intermediate format! allocation
fn indented_list_item_key_colon(depth: usize, key: &str, indent_size: usize) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    let prefix_len = LIST_ITEM_PREFIX.len();
    // "- " + key + ":"
    let capacity = indent_chars
        .saturating_add(prefix_len)
        .saturating_add(key.len())
        .saturating_add(1);
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(LIST_ITEM_PREFIX);
    out.push_str(key);
    out.push(':');
    out
}

/// Build "- key[N]:" list item directly without intermediate format! allocation
fn indented_list_item_key_header(
    depth: usize,
    key: &str,
    header: &str,
    indent_size: usize,
) -> String {
    // Use saturating arithmetic to prevent overflow with deeply nested structures
    let indent_chars = indent_size.saturating_mul(depth);
    let prefix_len = LIST_ITEM_PREFIX.len();
    // "- " + key + header
    let capacity = indent_chars
        .saturating_add(prefix_len)
        .saturating_add(key.len())
        .saturating_add(header.len());
    let mut out = String::with_capacity(capacity);
    for _ in 0..indent_chars {
        out.push(' ');
    }
    out.push_str(LIST_ITEM_PREFIX);
    out.push_str(key);
    out.push_str(header);
    out
}

/// Estimate the number of output lines for pre-allocation.
/// This is a rough heuristic - over-estimation is fine, under-estimation causes reallocation.
fn estimate_line_count(value: &JsonValue) -> usize {
    match value {
        JsonValue::Primitive(_) => 1,
        JsonValue::Array(items) => {
            // Header line + recursively estimate each item
            1 + items.iter().map(estimate_line_count).sum::<usize>()
        }
        JsonValue::Object(entries) => {
            // Each entry produces at least one line
            entries
                .iter()
                .map(|(_, v)| estimate_line_count(v))
                .sum::<usize>()
                .max(1)
        }
    }
}
