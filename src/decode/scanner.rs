use crate::error::{Result, ToonError};
use crate::shared::constants::{SPACE, TAB};

pub type Depth = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub raw: String,
    pub indent: usize,
    pub content: String,
    pub depth: Depth,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlankLineInfo {
    pub line_number: usize,
    pub indent: usize,
    pub depth: Depth,
}

#[derive(Debug, Clone)]
pub struct StreamingScanState {
    pub line_number: usize,
    pub blank_lines: Vec<BlankLineInfo>,
}

#[must_use]
pub const fn create_scan_state() -> StreamingScanState {
    StreamingScanState {
        line_number: 0,
        blank_lines: Vec::new(),
    }
}

/// Parse a line with indentation and strict-mode validation.
///
/// # Errors
///
/// Returns an error if strict mode rules are violated (tabs in indentation or
/// indentation not a multiple of the indent size).
pub fn parse_line_incremental(
    raw: &str,
    state: &mut StreamingScanState,
    indent_size: usize,
    strict: bool,
) -> Result<Option<ParsedLine>> {
    state.line_number += 1;
    let line_number = state.line_number;

    let mut indent = 0usize;
    let raw_bytes = raw.as_bytes();
    while indent < raw_bytes.len() && raw_bytes[indent] == SPACE as u8 {
        indent += 1;
    }

    // Check if line is blank before allocating content string
    let content_slice = &raw[indent..];
    if content_slice.trim().is_empty() {
        let depth = compute_depth_from_indent(indent, indent_size);
        state.blank_lines.push(BlankLineInfo {
            line_number,
            indent,
            depth,
        });
        return Ok(None);
    }

    // Only allocate content string for non-blank lines
    let content = content_slice.to_string();
    let depth = compute_depth_from_indent(indent, indent_size);

    if strict {
        let mut whitespace_end = 0usize;
        while whitespace_end < raw_bytes.len()
            && (raw_bytes[whitespace_end] == SPACE as u8 || raw_bytes[whitespace_end] == TAB as u8)
        {
            whitespace_end += 1;
        }

        if raw[..whitespace_end].contains(TAB) {
            return Err(ToonError::tabs_not_allowed(line_number));
        }

        if indent_size == 0 {
            if indent > 0 {
                return Err(ToonError::validation(
                    line_number,
                    format!(
                        "Indentation not allowed when indent size is 0, but found {indent} spaces"
                    ),
                ));
            }
        } else if indent > 0 && !indent.is_multiple_of(indent_size) {
            return Err(ToonError::invalid_indentation(
                line_number,
                indent_size,
                indent,
            ));
        }
    }

    Ok(Some(ParsedLine {
        raw: raw.to_string(),
        indent,
        content,
        depth,
        line_number,
    }))
}

/// Parse all lines from the source, skipping blank lines but recording them for validation.
///
/// # Errors
///
/// Returns an error if any line violates strict indentation rules.
pub fn parse_lines_sync(
    source: impl IntoIterator<Item = String>,
    indent_size: usize,
    strict: bool,
    state: &mut StreamingScanState,
) -> Result<Vec<ParsedLine>> {
    let mut lines = Vec::new();
    for raw in source {
        if let Some(parsed) = parse_line_incremental(&raw, state, indent_size, strict)? {
            lines.push(parsed);
        }
    }
    Ok(lines)
}

#[must_use]
pub const fn compute_depth_from_indent(indent_spaces: usize, indent_size: usize) -> Depth {
    if indent_size == 0 {
        return 0;
    }
    indent_spaces / indent_size
}

#[derive(Debug, Clone)]
pub struct StreamingLineCursor {
    lines: Vec<ParsedLine>,
    index: usize,
    last_line: Option<ParsedLine>,
    blank_lines: Vec<BlankLineInfo>,
}

impl StreamingLineCursor {
    #[must_use]
    pub const fn new(lines: Vec<ParsedLine>, blank_lines: Vec<BlankLineInfo>) -> Self {
        Self {
            lines,
            index: 0,
            last_line: None,
            blank_lines,
        }
    }

    #[must_use]
    pub fn get_blank_lines(&self) -> &[BlankLineInfo] {
        &self.blank_lines
    }

    #[must_use]
    pub fn peek_sync(&self) -> Option<&ParsedLine> {
        self.lines.get(self.index)
    }

    pub fn advance_sync(&mut self) {
        if self.index < self.lines.len() {
            // Store index instead of cloning
            self.last_line = Some(self.lines[self.index].clone());
            self.index += 1;
        }
    }

    pub fn next_sync(&mut self) -> Option<ParsedLine> {
        if self.index < self.lines.len() {
            let line = self.lines[self.index].clone();
            self.last_line = Some(line.clone());
            self.index += 1;
            Some(line)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn current(&self) -> Option<&ParsedLine> {
        self.last_line.as_ref()
    }

    #[must_use]
    pub const fn at_end_sync(&self) -> bool {
        self.index >= self.lines.len()
    }
}
