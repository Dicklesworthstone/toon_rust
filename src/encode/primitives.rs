use std::fmt::Write;

use crate::JsonPrimitive;
use crate::StringOrNumberOrBoolOrNull;
use crate::shared::constants::{DEFAULT_DELIMITER, DOUBLE_QUOTE};
use crate::shared::string_utils::escape_string;
use crate::shared::validation::{is_safe_unquoted, is_valid_unquoted_key};

#[must_use]
pub fn encode_primitive(value: &JsonPrimitive, delimiter: char) -> String {
    match value {
        StringOrNumberOrBoolOrNull::Null => "null".to_string(),
        StringOrNumberOrBoolOrNull::Bool(value) => value.to_string(),
        StringOrNumberOrBoolOrNull::Number(value) => format_number(*value),
        StringOrNumberOrBoolOrNull::String(value) => encode_string_literal(value, delimiter),
    }
}

#[must_use]
pub fn encode_string_literal(value: &str, delimiter: char) -> String {
    if is_safe_unquoted(value, delimiter) {
        return value.to_string();
    }
    format!("{DOUBLE_QUOTE}{}{DOUBLE_QUOTE}", escape_string(value))
}

#[must_use]
pub fn encode_key(key: &str) -> String {
    if is_valid_unquoted_key(key) {
        return key.to_string();
    }
    format!("{DOUBLE_QUOTE}{}{DOUBLE_QUOTE}", escape_string(key))
}

#[must_use]
pub fn encode_and_join_primitives(values: &[JsonPrimitive], delimiter: char) -> String {
    if values.is_empty() {
        return String::new();
    }
    // Estimate: average 10 chars per primitive + delimiter
    let mut out = String::with_capacity(values.len() * 11);
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(delimiter);
        }
        out.push_str(&encode_primitive(value, delimiter));
    }
    out
}

#[must_use]
pub fn format_header(
    length: usize,
    key: Option<&str>,
    fields: Option<&[String]>,
    delimiter: char,
) -> String {
    let mut header = String::new();

    if let Some(key) = key {
        header.push_str(&encode_key(key));
    }

    if delimiter == DEFAULT_DELIMITER {
        let _ = write!(header, "[{length}]");
    } else {
        let _ = write!(header, "[{length}{delimiter}]");
    }

    if let Some(fields) = fields {
        header.push('{');
        for (idx, field) in fields.iter().enumerate() {
            if idx > 0 {
                header.push(delimiter);
            }
            header.push_str(&encode_key(field));
        }
        header.push('}');
    }

    header.push(':');
    header
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.is_nan() || !value.is_finite() {
        return "null".to_string();
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_null_primitive() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Null, ','),
            "null"
        );
    }

    #[test]
    fn encode_bool_primitive() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Bool(true), ','),
            "true"
        );
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Bool(false), ','),
            "false"
        );
    }

    #[test]
    fn encode_number_zero_is_bare_zero() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Number(0.0), ','),
            "0"
        );
    }

    #[test]
    fn encode_number_nan_is_null() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Number(f64::NAN), ','),
            "null"
        );
    }

    #[test]
    fn encode_number_infinity_is_null() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::Number(f64::INFINITY), ','),
            "null"
        );
    }

    #[test]
    fn encode_simple_string_is_unquoted() {
        assert_eq!(
            encode_primitive(&StringOrNumberOrBoolOrNull::String("hello".into()), ','),
            "hello"
        );
    }

    #[test]
    fn encode_string_with_comma_is_quoted_when_delimiter_is_comma() {
        let out = encode_primitive(&StringOrNumberOrBoolOrNull::String("a,b".into()), ',');
        assert!(out.starts_with('"'));
        assert!(out.ends_with('"'));
        assert!(out.contains("a,b"));
    }

    #[test]
    fn encode_string_with_newline_is_escaped_and_quoted() {
        let out = encode_string_literal("line\nfeed", ',');
        assert_eq!(out, "\"line\\nfeed\"");
    }

    #[test]
    fn encode_string_that_looks_like_bool_is_quoted() {
        let out = encode_string_literal("true", ',');
        assert_eq!(out, "\"true\"");
    }

    #[test]
    fn encode_key_valid_is_unquoted() {
        assert_eq!(encode_key("valid_key"), "valid_key");
    }

    #[test]
    fn encode_key_with_space_is_quoted() {
        let out = encode_key("has space");
        assert!(out.starts_with('"'));
        assert!(out.contains("has space"));
    }

    #[test]
    fn encode_key_with_quotes_is_escaped() {
        let out = encode_key("a\"b");
        assert_eq!(out, "\"a\\\"b\"");
    }

    #[test]
    fn encode_and_join_primitives_empty_is_empty() {
        assert_eq!(encode_and_join_primitives(&[], ','), "");
    }

    #[test]
    fn encode_and_join_primitives_joins_with_delimiter() {
        let vals = vec![
            StringOrNumberOrBoolOrNull::Number(1.0),
            StringOrNumberOrBoolOrNull::String("two".into()),
            StringOrNumberOrBoolOrNull::Bool(true),
        ];
        assert_eq!(encode_and_join_primitives(&vals, ','), "1,two,true");
    }

    #[test]
    fn encode_and_join_primitives_different_delimiters() {
        let vals = vec![
            StringOrNumberOrBoolOrNull::Number(1.0),
            StringOrNumberOrBoolOrNull::Number(2.0),
        ];
        assert_eq!(encode_and_join_primitives(&vals, '|'), "1|2");
        assert_eq!(encode_and_join_primitives(&vals, '\t'), "1\t2");
    }

    #[test]
    fn format_header_length_only_default_delimiter() {
        assert_eq!(format_header(3, None, None, ','), "[3]:");
    }

    #[test]
    fn format_header_length_only_custom_delimiter() {
        assert_eq!(format_header(3, None, None, '|'), "[3|]:");
    }

    #[test]
    fn format_header_with_key_and_fields() {
        let fields = vec!["id".to_string(), "name".to_string()];
        assert_eq!(
            format_header(2, Some("users"), Some(&fields), ','),
            "users[2]{id,name}:"
        );
    }

    #[test]
    fn format_header_with_quoted_field_name() {
        let fields = vec!["weird name".to_string()];
        let out = format_header(1, Some("data"), Some(&fields), ',');
        assert!(out.contains("{\"weird name\"}"));
    }

    #[test]
    fn format_header_zero_length() {
        assert_eq!(format_header(0, Some("items"), None, ','), "items[0]:");
    }
}
