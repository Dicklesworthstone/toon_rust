use std::fmt;
use std::path::PathBuf;

/// Comprehensive error types for TOON encoding and decoding operations.
#[derive(Debug)]
pub enum ToonError {
    /// Parse error with line number context
    Parse { line: usize, message: String },

    /// Validation error (strict mode violations)
    Validation { line: usize, message: String },

    /// Event stream processing error
    EventStream { message: String },

    /// Path expansion conflict or error
    PathExpansion { path: String, message: String },

    /// I/O error with operation context
    Io {
        operation: String,
        path: Option<PathBuf>,
        source: std::io::Error,
    },

    /// JSON serialization/deserialization error (the message names the operation)
    Json { message: String },

    /// Generic message (for backward compatibility)
    Message { message: String },
}

impl fmt::Display for ToonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { line, message } => write!(f, "Line {line}: {message}"),
            Self::Validation { line, message } => {
                write!(f, "Validation error at line {line}: {message}")
            }
            Self::EventStream { message } => write!(f, "Event stream error: {message}"),
            Self::PathExpansion { path, message } => {
                write!(f, "Path expansion error for '{path}': {message}")
            }
            Self::Io {
                operation,
                path,
                source,
            } => {
                if let Some(p) = path {
                    write!(f, "{operation} '{}': {source}", p.display())
                } else {
                    write!(f, "{operation}: {source}")
                }
            }
            Self::Json { message } | Self::Message { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for ToonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, ToonError>;

impl ToonError {
    // =========================================================================
    // Backward-compatible constructor (preserves existing API)
    // =========================================================================

    /// Create a generic message error (backward compatible).
    #[must_use]
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }

    // =========================================================================
    // Parse error constructors
    // =========================================================================

    /// Create a parse error with line number.
    #[must_use]
    pub fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::Parse {
            line,
            message: message.into(),
        }
    }

    /// Create a parse error for unterminated string.
    #[must_use]
    pub fn unterminated_string(line: usize) -> Self {
        Self::parse(line, "Unterminated string: missing closing quote")
    }

    /// Create a parse error for missing colon after key.
    #[must_use]
    pub fn missing_colon(line: usize) -> Self {
        Self::parse(line, "Missing colon after key")
    }

    /// Create a parse error for invalid array length.
    #[must_use]
    pub fn invalid_array_length(line: usize, value: &str) -> Self {
        Self::parse(line, format!("Invalid array length: {value}"))
    }

    // =========================================================================
    // Validation error constructors
    // =========================================================================

    /// Create a validation error with line number.
    #[must_use]
    pub fn validation(line: usize, message: impl Into<String>) -> Self {
        Self::Validation {
            line,
            message: message.into(),
        }
    }

    /// Create a validation error for tabs in indentation.
    #[must_use]
    pub fn tabs_not_allowed(line: usize) -> Self {
        Self::validation(line, "Tabs are not allowed in indentation in strict mode")
    }

    /// Create a validation error for incorrect indentation.
    #[must_use]
    pub fn invalid_indentation(line: usize, expected: usize, found: usize) -> Self {
        Self::validation(
            line,
            format!("Indentation must be exact multiple of {expected}, but found {found} spaces"),
        )
    }

    // =========================================================================
    // Event stream error constructors
    // =========================================================================

    /// Create an event stream error.
    #[must_use]
    pub fn event_stream(message: impl Into<String>) -> Self {
        Self::EventStream {
            message: message.into(),
        }
    }

    /// Create an error for mismatched end event.
    #[must_use]
    pub fn mismatched_end(expected: &str, found: &str) -> Self {
        Self::event_stream(format!(
            "Mismatched end event: expected {expected}, found {found}"
        ))
    }

    /// Create an error for unexpected event.
    #[must_use]
    pub fn unexpected_event(event: &str, context: &str) -> Self {
        Self::event_stream(format!("Unexpected {event} event {context}"))
    }

    // =========================================================================
    // Path expansion error constructors
    // =========================================================================

    /// Create a path expansion error.
    #[must_use]
    pub fn path_expansion(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::PathExpansion {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create an error for path conflict during expansion.
    #[must_use]
    pub fn path_conflict(path: &str, existing: &str) -> Self {
        Self::path_expansion(path, format!("conflicts with existing key '{existing}'"))
    }

    // =========================================================================
    // I/O error constructors
    // =========================================================================

    /// Create an I/O error with path context.
    #[must_use]
    pub fn io(operation: impl Into<String>, path: Option<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            operation: operation.into(),
            path,
            source,
        }
    }

    /// Create an error for file read failure.
    #[must_use]
    pub fn file_read(path: PathBuf, source: std::io::Error) -> Self {
        Self::io("Failed to read file", Some(path), source)
    }

    /// Create an error for file write failure.
    #[must_use]
    pub fn file_write(path: PathBuf, source: std::io::Error) -> Self {
        Self::io("Failed to write to file", Some(path), source)
    }

    /// Create an error for file creation failure.
    #[must_use]
    pub fn file_create(path: PathBuf, source: std::io::Error) -> Self {
        Self::io("Failed to create file", Some(path), source)
    }

    /// Create an error for stdin read failure.
    #[must_use]
    pub fn stdin_read(source: std::io::Error) -> Self {
        Self::io("Failed to read stdin", None, source)
    }

    /// Create an error for stdout write failure.
    #[must_use]
    pub fn stdout_write(source: std::io::Error) -> Self {
        Self::io("Failed to write to stdout", None, source)
    }

    // =========================================================================
    // JSON error constructors
    // =========================================================================

    /// Create a JSON error.
    #[must_use]
    pub fn json(message: impl Into<String>) -> Self {
        Self::Json {
            message: message.into(),
        }
    }

    /// Create a JSON parse error.
    #[cfg(feature = "serde")]
    #[must_use]
    pub fn json_parse(err: &serde_json::Error) -> Self {
        Self::json(format!("Failed to parse JSON: {err}"))
    }

    /// Create a JSON stringify error.
    #[cfg(feature = "serde")]
    #[must_use]
    pub fn json_stringify(err: &serde_json::Error) -> Self {
        Self::json(format!("Failed to stringify JSON: {err}"))
    }
}

impl From<std::io::Error> for ToonError {
    fn from(err: std::io::Error) -> Self {
        Self::io("I/O error", None, err)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Error> for ToonError {
    fn from(err: serde_json::Error) -> Self {
        Self::json(format!("JSON error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn display_and_source_match_expected_formats() {
        assert_eq!(ToonError::parse(3, "bad").to_string(), "Line 3: bad");
        assert_eq!(
            ToonError::validation(7, "wrong indent").to_string(),
            "Validation error at line 7: wrong indent"
        );
        assert_eq!(
            ToonError::event_stream("eof").to_string(),
            "Event stream error: eof"
        );
        assert_eq!(
            ToonError::path_expansion("a.b", "conflict").to_string(),
            "Path expansion error for 'a.b': conflict"
        );
        let io_with_path = ToonError::file_read(
            PathBuf::from("data.toon"),
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        );
        assert_eq!(
            io_with_path.to_string(),
            "Failed to read file 'data.toon': missing"
        );
        assert!(io_with_path.source().is_some());

        let io_without_path = ToonError::stdin_read(std::io::Error::other("broken"));
        assert_eq!(io_without_path.to_string(), "Failed to read stdin: broken");
        assert!(io_without_path.source().is_some());

        let json_err = ToonError::json("Failed to parse JSON: bad");
        assert_eq!(json_err.to_string(), "Failed to parse JSON: bad");
        assert!(json_err.source().is_none());

        assert_eq!(ToonError::message("boom").to_string(), "boom");
    }
}
