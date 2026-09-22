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
            buf.push_str(&crate::encode::primitives::format_json_number(*n));
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

/// Push a JSON-escaped string (with quotes) directly to buffer.
///
/// The escaping is `serde_json`'s, the same as the streaming writer's (and JavaScript's
/// `JSON.stringify`): `\b \f \n \r \t \" \\`, `\u00XX` for the other C0 controls, everything
/// else as is. `--expand-paths safe` used to spell the same string differently.
fn push_json_string(buf: &mut String, s: &str) {
    match serde_json::to_string(s) {
        Ok(text) => buf.push_str(&text),
        Err(_) => buf.push_str("\"\""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StringOrNumberOrBoolOrNull;

    fn stringify(value: &JsonValue, indent: usize) -> String {
        let lines = json_stringify_lines(value, indent);
        assert_eq!(lines.len(), 1);
        lines.into_iter().next().unwrap()
    }

    fn s(v: &str) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::String(v.to_string()))
    }

    fn n(v: f64) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Number(v))
    }

    fn b(v: bool) -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Bool(v))
    }

    fn null() -> JsonValue {
        JsonValue::Primitive(StringOrNumberOrBoolOrNull::Null)
    }

    #[test]
    fn primitive_null() {
        assert_eq!(stringify(&null(), 0), "null");
    }

    #[test]
    fn primitive_booleans() {
        assert_eq!(stringify(&b(true), 0), "true");
        assert_eq!(stringify(&b(false), 0), "false");
    }

    #[test]
    fn primitive_number_integer_like() {
        assert_eq!(stringify(&n(42.0), 0), "42");
    }

    #[test]
    fn primitive_number_zero() {
        assert_eq!(stringify(&n(0.0), 0), "0");
        assert_eq!(stringify(&n(-0.0), 0), "0");
    }

    #[test]
    fn primitive_number_follows_javascript_number_text() {
        assert_eq!(stringify(&n(1.5), 0), "1.5");
        assert_eq!(stringify(&n(-2.25), 0), "-2.25");
        assert_eq!(
            stringify(&n(18_446_744_073_709_551_615.0), 0),
            "18446744073709552000"
        );
        assert_eq!(stringify(&n(1e21), 0), "1e+21");
        assert_eq!(stringify(&n(1.5e21), 0), "1.5e+21");
        assert_eq!(stringify(&n(0.000_001), 0), "0.000001");
        assert_eq!(stringify(&n(1e-7), 0), "1e-7");
        assert_eq!(stringify(&n(-1.25e-7), 0), "-1.25e-7");
        assert_eq!(stringify(&n(f64::MAX), 0), "1.7976931348623157e+308");
        assert_eq!(stringify(&n(5e-324), 0), "5e-324");
        // An exact tie between two shortest candidates takes the even digit.
        // 2101031963024178.25, built exactly.
        assert_eq!(
            stringify(&n(8_404_127_852_096_713.0 / 4.0), 0),
            "2101031963024178.2"
        );
    }

    #[test]
    fn primitive_number_nan_becomes_null() {
        assert_eq!(stringify(&n(f64::NAN), 0), "null");
    }

    #[test]
    fn primitive_number_infinity_becomes_null() {
        assert_eq!(stringify(&n(f64::INFINITY), 0), "null");
    }

    #[test]
    fn primitive_empty_string() {
        assert_eq!(stringify(&s(""), 0), "\"\"");
    }

    #[test]
    fn primitive_simple_string() {
        assert_eq!(stringify(&s("hello"), 0), "\"hello\"");
    }

    #[test]
    fn primitive_string_escapes_quote_and_backslash() {
        assert_eq!(stringify(&s("a\"b\\c"), 0), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn primitive_string_escapes_whitespace_controls() {
        assert_eq!(stringify(&s("a\nb\rc\td"), 0), "\"a\\nb\\rc\\td\"");
    }

    #[test]
    fn primitive_string_escapes_unicode_control() {
        // \u{0001} falls into the generic control-character branch.
        assert_eq!(stringify(&s("\u{0001}x"), 0), "\"\\u0001x\"");
    }

    #[test]
    fn primitive_string_escapes_match_the_streaming_writer() {
        // `\b` and `\f` use their short escapes; DEL and C1 controls are not escaped.
        assert_eq!(
            stringify(&s("\u{0008}\u{000c}\u{007f}\u{0085}"), 0),
            "\"\\b\\f\u{007f}\u{0085}\""
        );
    }

    #[test]
    fn empty_array_is_compact() {
        let v = JsonValue::Array(vec![]);
        assert_eq!(stringify(&v, 0), "[]");
        assert_eq!(stringify(&v, 2), "[]");
    }

    #[test]
    fn empty_object_is_compact() {
        let v = JsonValue::Object(vec![]);
        assert_eq!(stringify(&v, 0), "{}");
        assert_eq!(stringify(&v, 2), "{}");
    }

    #[test]
    fn array_no_indent() {
        let v = JsonValue::Array(vec![n(1.0), n(2.0), n(3.0)]);
        assert_eq!(stringify(&v, 0), "[1,2,3]");
    }

    #[test]
    fn array_with_indent() {
        let v = JsonValue::Array(vec![n(1.0), n(2.0)]);
        assert_eq!(stringify(&v, 2), "[\n  1,\n  2\n]");
    }

    #[test]
    fn object_no_indent() {
        let v = JsonValue::Object(vec![("a".to_string(), n(1.0)), ("b".to_string(), b(true))]);
        assert_eq!(stringify(&v, 0), "{\"a\":1,\"b\":true}");
    }

    #[test]
    fn object_with_indent() {
        let v = JsonValue::Object(vec![("a".to_string(), n(1.0))]);
        assert_eq!(stringify(&v, 2), "{\n  \"a\": 1\n}");
    }

    #[test]
    fn nested_object_array() {
        let v = JsonValue::Object(vec![(
            "items".to_string(),
            JsonValue::Array(vec![s("x"), s("y")]),
        )]);
        let out = stringify(&v, 2);
        assert!(out.contains("\"items\":"));
        assert!(out.contains("\"x\""));
        assert!(out.contains("\"y\""));
    }

    #[test]
    fn object_key_with_special_chars_is_escaped() {
        let v = JsonValue::Object(vec![("a\"b".to_string(), n(1.0))]);
        let out = stringify(&v, 0);
        assert!(out.contains("\"a\\\"b\""));
    }

    #[test]
    fn estimate_size_is_nonzero() {
        let v = JsonValue::Object(vec![
            ("a".to_string(), n(1.0)),
            ("b".to_string(), s("hello")),
        ]);
        assert!(estimate_json_size(&v, 0) > 0);
        assert!(estimate_json_size(&v, 2) >= estimate_json_size(&v, 0));
    }

    #[test]
    fn round_trip_via_serde_json_for_objects() {
        let v = JsonValue::Object(vec![
            ("name".to_string(), s("Alice")),
            ("age".to_string(), n(30.0)),
            ("active".to_string(), b(true)),
            ("nothing".to_string(), null()),
        ]);
        let out = stringify(&v, 0);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30.0);
        assert_eq!(parsed["active"], true);
        assert!(parsed["nothing"].is_null());
    }
}
