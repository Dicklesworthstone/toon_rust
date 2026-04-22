use std::sync::Arc;

use crate::JsonValue;
use crate::shared::constants::DEFAULT_DELIMITER;

pub type EncodeReplacer =
    Arc<dyn Fn(&str, &JsonValue, &[PathSegment]) -> Option<JsonValue> + Send + Sync>;

#[derive(Clone)]
pub struct EncodeOptions {
    pub indent: Option<usize>,
    pub delimiter: Option<char>,
    pub key_folding: Option<KeyFoldingMode>,
    pub flatten_depth: Option<usize>,
    pub replacer: Option<EncodeReplacer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFoldingMode {
    Off,
    Safe,
}

#[derive(Debug, Clone)]
pub struct DecodeOptions {
    pub indent: Option<usize>,
    pub strict: Option<bool>,
    pub expand_paths: Option<ExpandPathsMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandPathsMode {
    Off,
    Safe,
}

#[derive(Debug, Clone, Default)]
pub struct DecodeStreamOptions {
    pub indent: Option<usize>,
    pub strict: Option<bool>,
}

#[derive(Clone)]
pub struct ResolvedEncodeOptions {
    pub indent: usize,
    pub delimiter: char,
    pub key_folding: KeyFoldingMode,
    pub flatten_depth: usize,
    pub replacer: Option<EncodeReplacer>,
}

#[derive(Debug, Clone)]
pub struct ResolvedDecodeOptions {
    pub indent: usize,
    pub strict: bool,
    pub expand_paths: ExpandPathsMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[must_use]
pub fn resolve_encode_options(options: Option<EncodeOptions>) -> ResolvedEncodeOptions {
    let options = options.unwrap_or(EncodeOptions {
        indent: None,
        delimiter: None,
        key_folding: None,
        flatten_depth: None,
        replacer: None,
    });

    ResolvedEncodeOptions {
        indent: options.indent.unwrap_or(2),
        delimiter: options.delimiter.unwrap_or(DEFAULT_DELIMITER),
        key_folding: options.key_folding.unwrap_or(KeyFoldingMode::Off),
        flatten_depth: options.flatten_depth.unwrap_or(usize::MAX),
        replacer: options.replacer,
    }
}

#[must_use]
pub fn resolve_decode_options(options: Option<DecodeOptions>) -> ResolvedDecodeOptions {
    let options = options.unwrap_or(DecodeOptions {
        indent: None,
        strict: None,
        expand_paths: None,
    });

    ResolvedDecodeOptions {
        indent: options.indent.unwrap_or(2),
        strict: options.strict.unwrap_or(true),
        expand_paths: options.expand_paths.unwrap_or(ExpandPathsMode::Off),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_encode_defaults_when_none() {
        let r = resolve_encode_options(None);
        assert_eq!(r.indent, 2);
        assert_eq!(r.delimiter, DEFAULT_DELIMITER);
        assert_eq!(r.key_folding, KeyFoldingMode::Off);
        assert_eq!(r.flatten_depth, usize::MAX);
        assert!(r.replacer.is_none());
    }

    #[test]
    fn resolve_encode_uses_overrides() {
        let r = resolve_encode_options(Some(EncodeOptions {
            indent: Some(4),
            delimiter: Some('|'),
            key_folding: Some(KeyFoldingMode::Safe),
            flatten_depth: Some(3),
            replacer: None,
        }));
        assert_eq!(r.indent, 4);
        assert_eq!(r.delimiter, '|');
        assert_eq!(r.key_folding, KeyFoldingMode::Safe);
        assert_eq!(r.flatten_depth, 3);
    }

    #[test]
    fn resolve_encode_partial_overrides() {
        let r = resolve_encode_options(Some(EncodeOptions {
            indent: Some(0),
            delimiter: None,
            key_folding: None,
            flatten_depth: None,
            replacer: None,
        }));
        assert_eq!(r.indent, 0);
        assert_eq!(r.delimiter, DEFAULT_DELIMITER);
        assert_eq!(r.flatten_depth, usize::MAX);
    }

    #[test]
    fn resolve_decode_defaults_when_none() {
        let r = resolve_decode_options(None);
        assert_eq!(r.indent, 2);
        assert!(r.strict);
        assert_eq!(r.expand_paths, ExpandPathsMode::Off);
    }

    #[test]
    fn resolve_decode_uses_overrides() {
        let r = resolve_decode_options(Some(DecodeOptions {
            indent: Some(4),
            strict: Some(false),
            expand_paths: Some(ExpandPathsMode::Safe),
        }));
        assert_eq!(r.indent, 4);
        assert!(!r.strict);
        assert_eq!(r.expand_paths, ExpandPathsMode::Safe);
    }

    #[test]
    fn resolve_decode_explicit_true_strict() {
        let r = resolve_decode_options(Some(DecodeOptions {
            indent: None,
            strict: Some(true),
            expand_paths: None,
        }));
        assert!(r.strict);
    }

    #[test]
    fn key_folding_equality_and_copy() {
        let a = KeyFoldingMode::Off;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(KeyFoldingMode::Off, KeyFoldingMode::Safe);
    }

    #[test]
    fn expand_paths_equality_and_copy() {
        let a = ExpandPathsMode::Safe;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(ExpandPathsMode::Off, ExpandPathsMode::Safe);
    }

    #[test]
    fn path_segment_equality() {
        assert_eq!(
            PathSegment::Key("a".to_string()),
            PathSegment::Key("a".to_string())
        );
        assert_eq!(PathSegment::Index(3), PathSegment::Index(3));
        assert_ne!(PathSegment::Index(1), PathSegment::Index(2));
        assert_ne!(PathSegment::Key("a".into()), PathSegment::Key("b".into()));
    }

    #[test]
    fn decode_stream_options_default() {
        let d = DecodeStreamOptions::default();
        assert!(d.indent.is_none());
        assert!(d.strict.is_none());
    }

    #[test]
    fn resolve_encode_replacer_threads_through() {
        use crate::JsonValue;
        use std::sync::Arc;
        let replacer: EncodeReplacer = Arc::new(|_key, value, _path| Some(value.clone()));
        let r = resolve_encode_options(Some(EncodeOptions {
            indent: None,
            delimiter: None,
            key_folding: None,
            flatten_depth: None,
            replacer: Some(replacer.clone()),
        }));
        assert!(r.replacer.is_some());
        // Calling the replacer on a primitive returns a value.
        let out = (r.replacer.as_ref().unwrap())("k", &JsonValue::from(1i64), &[]);
        assert!(out.is_some());
    }
}
