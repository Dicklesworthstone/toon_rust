use crate::decode::parser::ArrayHeaderInfo;
use crate::decode::scanner::{BlankLineInfo, Depth, ParsedLine};
use crate::error::{Result, ToonError};
use crate::shared::constants::{COLON, LIST_ITEM_PREFIX};
use crate::shared::string_utils::find_unquoted_char;

/// Assert the expected count in strict mode.
///
/// # Errors
///
/// Returns an error when strict mode is enabled and counts differ.
pub fn assert_expected_count(
    actual: usize,
    expected: usize,
    item_type: &str,
    strict: bool,
) -> Result<()> {
    if strict && actual != expected {
        return Err(ToonError::message(format!(
            "Expected {expected} {item_type}, but got {actual}"
        )));
    }
    Ok(())
}

/// Validate that there are no extra list items beyond the expected count.
///
/// # Errors
///
/// Returns an error in strict mode when extra list items are found.
pub fn validate_no_extra_list_items(
    next_line: Option<&ParsedLine>,
    item_depth: Depth,
    expected_count: usize,
    strict: bool,
) -> Result<()> {
    if strict
        && let Some(line) = next_line
        && line.depth == item_depth
        && line.content.starts_with(LIST_ITEM_PREFIX)
    {
        return Err(ToonError::message(format!(
            "Expected {expected_count} list array items, but found more"
        )));
    }
    Ok(())
}

/// Validate that there are no extra tabular rows beyond the expected count.
///
/// # Errors
///
/// Returns an error in strict mode when extra tabular rows are found.
pub fn validate_no_extra_tabular_rows(
    next_line: Option<&ParsedLine>,
    row_depth: Depth,
    header: &ArrayHeaderInfo,
    strict: bool,
) -> Result<()> {
    if strict
        && let Some(line) = next_line
        && line.depth == row_depth
        && !line.content.starts_with(LIST_ITEM_PREFIX)
        && is_data_row(&line.content, header.delimiter)
    {
        return Err(ToonError::message(format!(
            "Expected {} tabular rows, but found more",
            header.length
        )));
    }
    Ok(())
}

/// Validate that no blank lines appear within the specified range.
///
/// # Errors
///
/// Returns an error in strict mode when blank lines appear within the range.
pub fn validate_no_blank_lines_in_range(
    start_line: usize,
    end_line: usize,
    blank_lines: &[BlankLineInfo],
    strict: bool,
    context: &str,
) -> Result<()> {
    if !strict {
        return Ok(());
    }

    if let Some(first_blank) = blank_lines
        .iter()
        .find(|blank| blank.line_number > start_line && blank.line_number < end_line)
    {
        return Err(ToonError::message(format!(
            "Line {}: Blank lines inside {context} are not allowed in strict mode",
            first_blank.line_number
        )));
    }

    Ok(())
}

fn is_data_row(content: &str, delimiter: char) -> bool {
    // Find first unquoted colon and delimiter to properly handle quoted strings
    let colon_pos = find_unquoted_char(content, COLON, 0);
    let delimiter_pos = find_unquoted_char(content, delimiter, 0);

    // If no unquoted colon, it's definitely a data row
    if colon_pos.is_none() {
        return true;
    }

    // If delimiter comes before colon (outside quotes), it's a data row
    if let Some(delimiter_pos) = delimiter_pos
        && let Some(colon_pos) = colon_pos
    {
        return delimiter_pos < colon_pos;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_line(content: &str, depth: usize, line_number: usize) -> ParsedLine {
        ParsedLine {
            raw: content.to_string(),
            indent: depth * 2,
            content: content.to_string(),
            depth,
            line_number,
        }
    }

    #[test]
    fn assert_expected_count_matches_is_ok() {
        assert!(assert_expected_count(3, 3, "items", true).is_ok());
    }

    #[test]
    fn assert_expected_count_mismatch_strict_errors() {
        assert!(assert_expected_count(2, 3, "items", true).is_err());
    }

    #[test]
    fn assert_expected_count_mismatch_lax_ok() {
        assert!(assert_expected_count(2, 3, "items", false).is_ok());
    }

    #[test]
    fn validate_no_extra_list_items_no_next_is_ok() {
        assert!(validate_no_extra_list_items(None, 1, 3, true).is_ok());
    }

    #[test]
    fn validate_no_extra_list_items_wrong_depth_is_ok() {
        let line = make_line("- one", 0, 5);
        assert!(validate_no_extra_list_items(Some(&line), 1, 3, true).is_ok());
    }

    #[test]
    fn validate_no_extra_list_items_same_depth_strict_errors() {
        let line = make_line("- extra", 1, 10);
        assert!(validate_no_extra_list_items(Some(&line), 1, 3, true).is_err());
    }

    #[test]
    fn validate_no_extra_list_items_same_depth_lax_ok() {
        let line = make_line("- extra", 1, 10);
        assert!(validate_no_extra_list_items(Some(&line), 1, 3, false).is_ok());
    }

    #[test]
    fn validate_no_extra_list_items_non_list_content_ok() {
        let line = make_line("k: v", 1, 10);
        assert!(validate_no_extra_list_items(Some(&line), 1, 3, true).is_ok());
    }

    fn make_tabular_header(delimiter: char) -> ArrayHeaderInfo {
        ArrayHeaderInfo {
            key: None,
            key_was_quoted: false,
            length: 2,
            delimiter,
            fields: Some(vec![]),
        }
    }

    #[test]
    fn validate_no_extra_tabular_rows_accepts_non_data_row() {
        let header = make_tabular_header(',');
        let line = make_line("k: v", 1, 10);
        assert!(validate_no_extra_tabular_rows(Some(&line), 1, &header, true).is_ok());
    }

    #[test]
    fn validate_no_extra_tabular_rows_rejects_data_row_strict() {
        let header = make_tabular_header(',');
        let line = make_line("extra,data", 1, 10);
        assert!(validate_no_extra_tabular_rows(Some(&line), 1, &header, true).is_err());
    }

    #[test]
    fn validate_no_extra_tabular_rows_wrong_depth_ok() {
        let header = make_tabular_header(',');
        let line = make_line("extra,data", 0, 10);
        assert!(validate_no_extra_tabular_rows(Some(&line), 1, &header, true).is_ok());
    }

    #[test]
    fn validate_no_extra_tabular_rows_list_item_ok() {
        let header = make_tabular_header(',');
        let line = make_line("- extra", 1, 10);
        assert!(validate_no_extra_tabular_rows(Some(&line), 1, &header, true).is_ok());
    }

    #[test]
    fn validate_no_blank_lines_in_range_none_is_ok() {
        let blanks: Vec<BlankLineInfo> = Vec::new();
        assert!(validate_no_blank_lines_in_range(1, 5, &blanks, true, "ctx").is_ok());
    }

    #[test]
    fn validate_no_blank_lines_in_range_outside_window_ok() {
        let blanks = vec![BlankLineInfo {
            line_number: 10,
            indent: 0,
            depth: 0,
        }];
        assert!(validate_no_blank_lines_in_range(1, 5, &blanks, true, "ctx").is_ok());
    }

    #[test]
    fn validate_no_blank_lines_in_range_inside_window_errors_strict() {
        let blanks = vec![BlankLineInfo {
            line_number: 3,
            indent: 0,
            depth: 0,
        }];
        let err =
            validate_no_blank_lines_in_range(1, 5, &blanks, true, "tabular array").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Line 3"));
        assert!(msg.contains("tabular array"));
    }

    #[test]
    fn validate_no_blank_lines_in_range_lax_ok() {
        let blanks = vec![BlankLineInfo {
            line_number: 3,
            indent: 0,
            depth: 0,
        }];
        assert!(validate_no_blank_lines_in_range(1, 5, &blanks, false, "ctx").is_ok());
    }

    #[test]
    fn is_data_row_without_colon() {
        assert!(is_data_row("a,b,c", ','));
    }

    #[test]
    fn is_data_row_with_delimiter_before_colon() {
        assert!(is_data_row("a,b:c", ','));
    }

    #[test]
    fn is_data_row_with_colon_before_delimiter_is_key_value() {
        assert!(!is_data_row("a:b,c", ','));
    }

    #[test]
    fn is_data_row_with_quoted_colon_inside_string() {
        // The colon is inside quotes so the first unquoted delimiter wins.
        assert!(is_data_row("\"a:b\",c", ','));
    }
}
