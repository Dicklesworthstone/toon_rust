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
        // An unquoted key cannot contain a colon, so a bracket after the first colon belongs to
        // the value (e.g. `note: "see [2] ref: x"`), never to an array header.
        let first_colon = content.find(COLON);
        content
            .find(OPEN_BRACKET)
            .filter(|&idx| first_colon.is_none_or(|colon| idx < colon))
    };

    let Some(bracket_start) = bracket_start else {
        return Ok(None);
    };

    let Some(bracket_end) = content[bracket_start..].find(CLOSE_BRACKET) else {
        return Ok(None);
    };
    let bracket_end = bracket_start + bracket_end;

    // After `]` only whitespace may precede the fields segment `{…}` or the colon. Any other
    // text (`items[2][3]: a,b`, `a[1]extra: x`) means the line is not an array header (spec §6,
    // v3.0.3); it is decoded as a key-value line whose key is everything before the colon,
    // instead of the text being silently dropped.
    let segment_start = skip_whitespace(content, bracket_end + 1);
    let mut fields_range: Option<(usize, usize)> = None;
    let colon_index = if content[segment_start..].starts_with(OPEN_BRACE) {
        // The fields segment ends at the first `}` outside quotes: a quoted field name may hold a
        // brace (`{"a}b",c}`), which the encoder writes raw inside the quotes.
        let Some(brace_close) = find_unquoted_char(content, CLOSE_BRACE, segment_start) else {
            return Ok(None);
        };
        fields_range = Some((segment_start + 1, brace_close));
        let colon = skip_whitespace(content, brace_close + 1);
        if !content[colon..].starts_with(COLON) {
            return Ok(None);
        }
        colon
    } else if content[segment_start..].starts_with(COLON) {
        segment_start
    } else {
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

    // Enforce the declared-length cap only once we know this line is a real
    // array header (bracket parsed cleanly); surface a hard error instead of
    // silently falling back to key-value handling. A length too large for the
    // platform word is over the cap too, not a reason to read the line as a key.
    if length > MAX_DECLARED_ARRAY_LENGTH {
        let digits = bracket_content.trim_end_matches([TAB, PIPE]);
        return Err(ToonError::message(format!(
            "Declared array length {digits} exceeds maximum allowed ({MAX_DECLARED_ARRAY_LENGTH})"
        )));
    }

    let fields = match fields_range {
        Some((start, end)) => Some(parse_field_names(&content[start..end], delimiter)?),
        None => None,
    };

    // A fields-bearing header announces rows on the following lines; values after its colon
    // used to be decoded as a primitive array with the field names ignored.
    if fields.is_some() && !after_colon.is_empty() {
        return Err(ToonError::message(
            "Unexpected content after fields-bearing header colon",
        ));
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

/// Hard cap on the declared length that appears inside an array header `[N]`.
///
/// A TOON file claiming e.g. `[9999999999]` should not be allowed to drive
/// downstream allocations or loop bounds sized from that number; 100 million
/// is well beyond any realistic payload while still preventing resource abuse.
pub const MAX_DECLARED_ARRAY_LENGTH: usize = 100_000_000;

/// Parse the bracket length segment, extracting length and delimiter.
///
/// The cap on declared length is enforced by [`parse_array_header_line`] after a
/// successful parse so that unrelated `[abc]` literals inside key-value content
/// still fall through to non-array handling instead of erroring.
///
/// # Errors
///
/// Returns an error if the length is not a valid unsigned integer.
pub fn parse_bracket_segment(seg: &str, default_delimiter: char) -> Result<(usize, char)> {
    let (digits, delimiter) = match seg.chars().last() {
        Some(last @ (TAB | PIPE)) => (&seg[..seg.len() - 1], last),
        _ => (seg, default_delimiter),
    };

    // The length is `1*DIGIT` (spec §6): no sign, no spaces. Rust's integer parser also took
    // `+2`. A value beyond the platform word saturates, so the caller's cap rejects it.
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ToonError::message(format!("Invalid array length: {seg}")));
    }
    let length = digits.bytes().fold(0usize, |acc, b| {
        acc.saturating_mul(10).saturating_add(usize::from(b - b'0'))
    });

    Ok((length, delimiter))
}

/// Parse the names of a fields segment (the text between `{` and `}`).
///
/// # Errors
///
/// Returns an error for an empty list, an empty name, or a malformed quoted name.
fn parse_field_names(fields_content: &str, delimiter: char) -> Result<Vec<FieldName>> {
    if fields_content.trim().is_empty() {
        return Err(ToonError::message("Empty field list in array header"));
    }
    parse_delimited_values(fields_content, delimiter)
        .into_iter()
        .map(|field| {
            let trimmed = field.trim();
            if trimmed.is_empty() {
                return Err(ToonError::message("Empty field name in field list"));
            }
            let was_quoted = trimmed.starts_with(DOUBLE_QUOTE);
            let name = parse_string_literal(trimmed)?;
            Ok(FieldName { name, was_quoted })
        })
        .collect()
}

/// The first index at or after `from` that is not ASCII whitespace.
fn skip_whitespace(content: &str, from: usize) -> usize {
    content[from..]
        .find(|c: char| !c.is_ascii_whitespace())
        .map_or(content.len(), |idx| from + idx)
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
    // Decide "quoted" after the leading whitespace, as every other test does: `  "a": 1`
    // used to keep the quotes as part of an unquoted key.
    let start = skip_whitespace(content, start);
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
pub const fn is_key_value_content(content: &str) -> bool {
    find_unquoted_char(content, COLON, 0).is_some()
}
