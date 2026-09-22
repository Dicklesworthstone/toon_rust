use std::path::PathBuf;
use thiserror::Error;

/// Comprehensive error types for TOON encoding and decoding operations.
#[derive(Debug, Error)]
pub enum ToonError {
    /// Parse error with line number context
    #[error("Line {line}: {message}")]
    Parse { line: usize, message: String },

    /// Validation error (strict mode violations)
    #[error("Validation error at line {line}: {message}")]
    Validation { line: usize, message: String },

    /// Event stream processing error
    #[error("Event stream error: {message}")]
    EventStream { message: String },

    /// Path expansion conflict or error
    #[error("Path expansion error for '{path}': {message}")]
    PathExpansion { path: String, message: String },

    /// I/O error with operation context
    #[error("{operation}{}: {source}", path.as_ref().map(|p| format!(" '{}'", p.display())).unwrap_or_default())]
    Io {
        operation: String,
        path: Option<PathBuf>,
        #[source]
        source: std::io::Error,
    },

    /// JSON serialization/deserialization error (the message names the operation)
    #[error("{message}")]
    Json { message: String },

    /// Generic message (for backward compatibility)
    #[error("{message}")]
    Message { message: String },
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
    #[must_use]
    pub fn json_parse(err: &serde_json::Error) -> Self {
        Self::json(format!("Failed to parse JSON: {err}"))
    }

    /// Create a JSON stringify error.
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

impl From<serde_json::Error> for ToonError {
    fn from(err: serde_json::Error) -> Self {
        Self::json(format!("JSON error: {err}"))
    }
}
