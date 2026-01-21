use std::fmt::Write;

use crate::JsonValue;

/// Stream JSON stringification chunks for a `JsonValue`.
/// Returns a Vec with a single string (optimized to avoid many small allocations).
#[must_use]
pub fn json_stringify_lines(value: &JsonValue, indent: usize) -> Vec<String> {
    // Estimate size: rough guess based on value complexity
    let estimated_size = estimate_json_size(value, indent);
    let mut buf = String::with_capacity(estimated_size);
    stringify_value_to_buf(value, 0, indent, &mut buf);
    vec![buf]
}

/// Estimate the JSON output size for pre-allocation
fn estimate_json_size(value: &JsonValue, indent: usize) -> usize {
    match value {
        JsonValue::Primitive(p) => match p {
            crate::StringOrNumberOrBoolOrNull::Null => 4,
            crate::StringOrNumberOrBoolOrNull::Bool(_) => 5,
            crate::StringOrNumberOrBoolOrNull::Number(_) => 20,
            crate::StringOrNumberOrBoolOrNull::String(s) => s.len() + 10,
        },
        JsonValue::Array(items) => {
            let base = items
                .iter()
                .map(|v| estimate_json_size(v, indent))
                .sum::<usize>();
            base + items.len() * (2 + indent) + 4
        }
        JsonValue::Object(entries) => {
            let base: usize = entries
                .iter()
                .map(|(k, v)| k.len() + 4 + estimate_json_size(v, indent))
                .sum();
            base + entries.len() * (2 + indent) + 4
        }
    }
}

fn stringify_value_to_buf(value: &JsonValue, depth: usize, indent: usize, buf: &mut String) {
    match value {
        JsonValue::Primitive(primitive) => {
            stringify_primitive_to_buf(primitive, buf);
        }
        JsonValue::Array(values) => stringify_array_to_buf(values, depth, indent, buf),
        JsonValue::Object(entries) => stringify_object_to_buf(entries, depth, indent, buf),
    }
}

fn stringify_array_to_buf(values: &[JsonValue], depth: usize, indent: usize, buf: &mut String) {
    if values.is_empty() {
        buf.push_str("[]");
        return;
    }

    buf.push('[');

    if indent > 0 {
        for (idx, value) in values.iter().enumerate() {
            buf.push('\n');
            push_indent(buf, (depth + 1) * indent);
            stringify_value_to_buf(value, depth + 1, indent, buf);
            if idx + 1 < values.len() {
                buf.push(',');
            }
        }
        buf.push('\n');
        push_indent(buf, depth * indent);
    } else {
        for (idx, value) in values.iter().enumerate() {
            stringify_value_to_buf(value, depth + 1, indent, buf);
            if idx + 1 < values.len() {
                buf.push(',');
            }
        }
    }
    buf.push(']');
}

fn stringify_object_to_buf(
    entries: &[(String, JsonValue)],
    depth: usize,
    indent: usize,
    buf: &mut String,
) {
    if entries.is_empty() {
        buf.push_str("{}");
        return;
    }

    buf.push('{');

    if indent > 0 {
        for (idx, (key, value)) in entries.iter().enumerate() {
            buf.push('\n');
            push_indent(buf, (depth + 1) * indent);
            // Escape key inline
            push_json_string(buf, key);
            buf.push_str(": ");
            stringify_value_to_buf(value, depth + 1, indent, buf);
            if idx + 1 < entries.len() {
                buf.push(',');
            }
        }
        buf.push('\n');
        push_indent(buf, depth * indent);
    } else {
        for (idx, (key, value)) in entries.iter().enumerate() {
            push_json_string(buf, key);
            buf.push(':');
            stringify_value_to_buf(value, depth + 1, indent, buf);
            if idx + 1 < entries.len() {
                buf.push(',');
            }
        }
    }
    buf.push('}');
}

fn stringify_primitive_to_buf(value: &crate::JsonPrimitive, buf: &mut String) {
    match value {
        crate::StringOrNumberOrBoolOrNull::Null => buf.push_str("null"),
        crate::StringOrNumberOrBoolOrNull::Bool(true) => buf.push_str("true"),
        crate::StringOrNumberOrBoolOrNull::Bool(false) => buf.push_str("false"),
        crate::StringOrNumberOrBoolOrNull::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                buf.push_str(&num.to_string());
            } else {
                buf.push_str("null");
            }
        }
        crate::StringOrNumberOrBoolOrNull::String(s) => {
            push_json_string(buf, s);
        }
    }
}

/// Push spaces for indentation
#[inline]
fn push_indent(buf: &mut String, count: usize) {
    for _ in 0..count {
        buf.push(' ');
    }
}

/// Push a JSON-escaped string (with quotes) directly to buffer
fn push_json_string(buf: &mut String, s: &str) {
    buf.push('"');
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if c.is_control() => {
                // Use \uXXXX format for control characters
                let _ = write!(buf, "\\u{:04x}", c as u32);
            }
            c => buf.push(c),
        }
    }
    buf.push('"');
}
