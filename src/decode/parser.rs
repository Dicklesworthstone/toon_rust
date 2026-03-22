use crate::error::{Result, ToonError};
use crate::shared::constants::{
    BACKSLASH, CLOSE_BRACE, CLOSE_BRACKET, COLON, DOUBLE_QUOTE, OPEN_BRACE, OPEN_BRACKET, PIPE, TAB,
};
use crate::shared::literal_utils::{is_boolean_or_null_literal, is_numeric_literal};
use crate::shared::string_utils::{find_closing_quote, find_unquoted_char, unescape_string};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayHeaderInfo {
    pub key: Option<String>,
    pub key_was_quoted: bool,
    pub length: usize,
    pub delimiter: char,
    pub fields: Option<Vec<FieldName>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName {
    pub name: String,
    pub was_quoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayHeaderParseResult {
    pub header: ArrayHeaderInfo,
    pub inline_values: Option<String>,
}

/// Parse a TOON array header line, returning header metadata and inline values.
///
/// # Errors
///
/// Returns an error for malformed quoted keys or string literals.
pub fn parse_array_header_line(
    content: &str,
    default_delimiter: char,
) -> Result<Option<ArrayHeaderParseResult>> {
    let trimmed = content.trim_start();

    let bracket_start = if trimmed.starts_with(DOUBLE_QUOTE) {
        let closing = find_closing_quote(trimmed, 0)
            .ok_or_else(|| ToonError::message("Unterminated string: missing closing quote"))?;
        let after_quote = &trimmed[closing + 1..];
        if !after_quote.starts_with(OPEN_BRACKET) {
            return Ok(None);
        }
        let leading_ws = content.len() - trimmed.len();
        let key_end = leading_ws + closing + 1;
        content[key_end..]
            .find(OPEN_BRACKET)
            .map(|idx| key_end + idx)
    } else {
        content.find(OPEN_BRACKET)
    };

    let Some(bracket_start) = bracket_start else {
        return Ok(None);
    };

    let Some(bracket_end) = content[bracket_start..].find(CLOSE_BRACKET) else {
        return Ok(None);
    };
    let bracket_end = bracket_start + bracket_end;

    let mut brace_end = bracket_end + 1;
    let brace_start = content[bracket_end + 1..]
        .find(OPEN_BRACE)
        .map(|idx| bracket_end + 1 + idx);
    let colon_after_bracket = content[bracket_end + 1..]
        .find(COLON)
        .map(|idx| bracket_end + 1 + idx);

    if let (Some(brace_start), Some(colon_after_bracket)) = (brace_start, colon_after_bracket)
        && brace_start < colon_after_bracket
        && let Some(found_end) = content[brace_start..].find(CLOSE_BRACE)
    {
        let found_end = brace_start + found_end;
        brace_end = found_end + 1;
    }

    let colon_index = content[brace_end..].find(COLON).map(|idx| brace_end + idx);
    let Some(colon_index) = colon_index else {
        return Ok(None);
    };

    let mut key: Option<String> = None;
    let mut key_was_quoted = false;
    if bracket_start > 0 {
        let raw_key = content[..bracket_start].trim();
        if raw_key.starts_with(DOUBLE_QUOTE) {
            key = Some(parse_string_literal(raw_key)?);
            key_was_quoted = true;
        } else if !raw_key.is_empty() {
            key = Some(raw_key.to_string());
        }
    }

    let after_colon = content[colon_index + 1..].trim();
    let bracket_content = &content[bracket_start + 1..bracket_end];

    let Ok((length, delimiter)) = parse_bracket_segment(bracket_content, default_delimiter) else {
        return Ok(None);
    };

    let mut fields: Option<Vec<FieldName>> = None;
    if let Some(brace_start) = brace_start
        && brace_start < colon_index
        && let Some(found_end) = content[brace_start..].find(CLOSE_BRACE)
    {
        let found_end = brace_start + found_end;
        if found_end < colon_index {
            let fields_content = &content[brace_start + 1..found_end];
            let parsed_fields = parse_delimited_values(fields_content, delimiter)
                .into_iter()
                .map(|field| {
                    let trimmed = field.trim();
                    let was_quoted = trimmed.starts_with(DOUBLE_QUOTE);
                    let name = parse_string_literal(trimmed)?;
                    Ok(FieldName { name, was_quoted })
                })
                .collect::<Result<Vec<_>>>()?;
            fields = Some(parsed_fields);
        }
    }

    Ok(Some(ArrayHeaderParseResult {
        header: ArrayHeaderInfo {
            key,
            key_was_quoted,
            length,
            delimiter,
            fields,
        },
        inline_values: if after_colon.is_empty() {
            None
        } else {
            Some(after_colon.to_string())
        },
    }))
}

/// Parse the bracket length segment, extracting length and delimiter.
///
/// # Errors
///
/// Returns an error if the length is invalid.
pub fn parse_bracket_segment(seg: &str, default_delimiter: char) -> Result<(usize, char)> {
    let mut content = seg.to_string();
    let mut delimiter = default_delimiter;

    if content.ends_with(TAB) {
        delimiter = TAB;
        content.pop();
    } else if content.ends_with(PIPE) {
        delimiter = PIPE;
        content.pop();
    }

    let length = content
        .parse::<usize>()
        .map_err(|_| ToonError::message(format!("Invalid array length: {seg}")))?;

    Ok((length, delimiter))
}

#[must_use]
pub fn parse_delimited_values(input: &str, delimiter: char) -> Vec<String> {
    // Pre-estimate capacity based on delimiter count
    let estimated_count = input.chars().filter(|&c| c == delimiter).count() + 1;
    let mut values = Vec::with_capacity(estimated_count);
    let mut buffer = String::with_capacity(64); // Reasonable default for field values
    let mut in_quotes = false;
    let mut iter = input.chars();

    while let Some(ch) = iter.next() {
        if ch == BACKSLASH && in_quotes {
            buffer.push(ch);
            if let Some(next) = iter.next() {
                buffer.push(next);
            }
            continue;
        }

        if ch == DOUBLE_QUOTE {
            in_quotes = !in_quotes;
            buffer.push(ch);
            continue;
        }

        if ch == delimiter && !in_quotes {
            values.push(buffer.trim().to_string());
            buffer.clear();
            continue;
        }

        buffer.push(ch);
    }

    if !buffer.is_empty() || !values.is_empty() {
        values.push(buffer.trim().to_string());
    }

    values
}

/// Map delimited string values into JSON primitives.
///
/// # Errors
///
/// Returns an error if any token is a malformed quoted string.
pub fn map_row_values_to_primitives(values: &[String]) -> Result<Vec<crate::JsonPrimitive>> {
    values
        .iter()
        .map(|value| parse_primitive_token(value))
        .collect()
}

/// Parse a primitive token into a JSON primitive.
///
/// # Errors
///
/// Returns an error if a quoted string token is unterminated or malformed.
pub fn parse_primitive_token(token: &str) -> Result<crate::JsonPrimitive> {
    let trimmed = token.trim();

    if trimmed.is_empty() {
        return Ok(crate::StringOrNumberOrBoolOrNull::String(String::new()));
    }

    if trimmed.starts_with(DOUBLE_QUOTE) {
        return Ok(crate::StringOrNumberOrBoolOrNull::String(
            parse_string_literal(trimmed)?,
        ));
    }

    if is_boolean_or_null_literal(trimmed) {
        return Ok(match trimmed {
            "true" => crate::StringOrNumberOrBoolOrNull::Bool(true),
            "false" => crate::StringOrNumberOrBoolOrNull::Bool(false),
            _ => crate::StringOrNumberOrBoolOrNull::Null,
        });
    }

    if is_numeric_literal(trimmed) {
        let parsed = trimmed.parse::<f64>().unwrap_or(f64::NAN);
        let normalized = if parsed == 0.0 && parsed.is_sign_negative() {
            0.0
        } else {
            parsed
        };
        return Ok(crate::StringOrNumberOrBoolOrNull::Number(normalized));
    }

    Ok(crate::StringOrNumberOrBoolOrNull::String(
        trimmed.to_string(),
    ))
}

/// Parse a quoted string literal, unescaping escape sequences.
///
/// # Errors
///
/// Returns an error for unterminated quotes or invalid escape sequences.
pub fn parse_string_literal(token: &str) -> Result<String> {
    let trimmed = token.trim();

    if trimmed.starts_with(DOUBLE_QUOTE) {
        let closing = find_closing_quote(trimmed, 0)
            .ok_or_else(|| ToonError::message("Unterminated string: missing closing quote"))?;
        if closing != trimmed.len() - 1 {
            return Err(ToonError::message(
                "Unexpected characters after closing quote",
            ));
        }
        let content = &trimmed[1..closing];
        return unescape_string(content).map_err(ToonError::message);
    }

    Ok(trimmed.to_string())
}

/// Parse an unquoted key up to the colon delimiter.
///
/// # Errors
///
/// Returns an error if no colon is found after the key.
pub fn parse_unquoted_key(content: &str, start: usize) -> Result<(String, usize)> {
    let mut pos = start;
    while pos < content.len() && content.as_bytes()[pos] as char != COLON {
        pos += 1;
    }

    if pos >= content.len() || content.as_bytes()[pos] as char != COLON {
        return Err(ToonError::message("Missing colon after key"));
    }

    let key = content[start..pos].trim().to_string();
    pos += 1;
    Ok((key, pos))
}

/// Parse a quoted key and validate the following colon.
///
/// # Errors
///
/// Returns an error for unterminated quotes or missing colon.
pub fn parse_quoted_key(content: &str, start: usize) -> Result<(String, usize)> {
    let closing = find_closing_quote(content, start)
        .ok_or_else(|| ToonError::message("Unterminated quoted key"))?;
    let key_content = &content[start + 1..closing];
    let key = unescape_string(key_content).map_err(ToonError::message)?;
    let mut pos = closing + 1;
    if pos >= content.len() || content.as_bytes()[pos] as char != COLON {
        return Err(ToonError::message("Missing colon after key"));
    }
    pos += 1;
    Ok((key, pos))
}

/// Parse a key token (quoted or unquoted) and return key, end index, and quoted flag.
///
/// # Errors
///
/// Returns an error if the key is malformed or missing a trailing colon.
pub fn parse_key_token(content: &str, start: usize) -> Result<(String, usize, bool)> {
    let is_quoted = content.as_bytes().get(start).map(|b| *b as char) == Some(DOUBLE_QUOTE);
    let (key, end) = if is_quoted {
        parse_quoted_key(content, start)?
    } else {
        parse_unquoted_key(content, start)?
    };
    Ok((key, end, is_quoted))
}

#[must_use]
pub fn is_array_header_content(content: &str) -> bool {
    content.trim_start().starts_with(OPEN_BRACKET)
        && find_unquoted_char(content, COLON, 0).is_some()
}

#[must_use]
pub fn is_key_value_content(content: &str) -> bool {
    find_unquoted_char(content, COLON, 0).is_some()
}
