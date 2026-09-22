pub mod args;
pub mod conversion;
pub mod json_stream;
pub mod json_stringify;

use crate::error::{Result, ToonError};
use crate::options::{DecodeOptions, EncodeOptions, ExpandPathsMode, KeyFoldingMode};
use args::{Args, ExpandPathsArg, KeyFoldingArg, Mode};
use clap::Parser;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

/// Runs the CLI entrypoint.
///
/// # Errors
///
/// Returns an error if parsing, encoding, decoding, or I/O fails.
pub fn run() -> Result<()> {
    let args = Args::parse();
    let mode = args.detect_mode();

    match mode {
        Mode::Encode => run_encode(&args),
        Mode::Decode => run_decode(&args),
    }
}

/// Write one line to stderr.
///
/// A failing stderr (closed, or a full device) must not turn a finished conversion into a crash:
/// the diagnostic is dropped and the exit status still says what happened.
pub fn report(line: &str) {
    let mut handle = io::stderr().lock();
    let _ = writeln!(handle, "{line}");
}

fn run_encode(args: &Args) -> Result<()> {
    // TOON structure is indentation: with 0 spaces per level every depth collapses onto one
    // column and the output no longer denotes the input.
    if args.indent == 0 {
        return Err(ToonError::message(
            "Indentation size must be at least 1 when encoding TOON",
        ));
    }

    // Read input (JSON)
    let input = read_input(args)?;

    // Build encode options
    let options = EncodeOptions {
        indent: Some(usize::from(args.indent)),
        delimiter: Some(args.delimiter),
        key_folding: Some(match args.key_folding {
            KeyFoldingArg::Off => KeyFoldingMode::Off,
            KeyFoldingArg::Safe => KeyFoldingMode::Safe,
        }),
        flatten_depth: args.flatten_depth,
        replacer: None,
    };

    // Encode
    let toon_output = conversion::encode_to_toon_lines(&input, Some(options))?.join("\n");
    write_output(args, &toon_output)?;

    if args.stats {
        report_stats(&input, &toon_output);
    }

    // Success message to stderr if writing to file
    if let Some(output_path) = args.output_file() {
        report(&format!(
            "Encoded `{}` → `{}`",
            format_input_label(args),
            output_path.display()
        ));
    }

    Ok(())
}

fn run_decode(args: &Args) -> Result<()> {
    // Read input (TOON)
    let input = read_input(args)?;

    // Build decode options
    let options = DecodeOptions {
        indent: Some(usize::from(args.indent)),
        strict: Some(!args.no_strict),
        expand_paths: Some(match args.expand_paths {
            ExpandPathsArg::Off => ExpandPathsMode::Off,
            ExpandPathsArg::Safe => ExpandPathsMode::Safe,
        }),
    };

    // Decode to JSON text
    let json_output = conversion::decode_to_json_chunks(&input, Some(options))
        .map_err(|err| ToonError::message(format!("Failed to decode TOON: {err}")))?
        .concat();
    write_output(args, &json_output)?;

    if args.stats {
        report_stats(&json_output, &input);
    }

    // Success message to stderr if writing to file
    if let Some(output_path) = args.output_file() {
        report(&format!(
            "Decoded `{}` → `{}`",
            format_input_label(args),
            output_path.display()
        ));
    }

    Ok(())
}

fn read_input(args: &Args) -> Result<String> {
    let text = if args.is_stdin() {
        read_stdin()?
    } else {
        let path = args
            .input
            .as_ref()
            .ok_or_else(|| ToonError::message("No input file specified"))?;
        read_file(path)?
    };
    Ok(strip_bom(text))
}

/// Drop a leading UTF-8 byte order mark. It is an encoding signature, never data (RFC 8259 §8.1
/// lets a JSON reader ignore it); without this, a BOM became part of the first TOON key and made
/// BOM-prefixed JSON unreadable.
fn strip_bom(text: String) -> String {
    if text.starts_with('\u{feff}') {
        text['\u{feff}'.len_utf8()..].to_string()
    } else {
        text
    }
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(ToonError::stdin_read)?;
    Ok(buffer)
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| ToonError::file_read(path.to_path_buf(), e))
}

/// Write the document and its final newline, then flush, so that every write error (a full
/// device, a closed pipe) is reported instead of being lost when a buffer is dropped.
fn write_output(args: &Args, data: &str) -> Result<()> {
    if let Some(path) = args.output_file() {
        let mut file =
            File::create(path).map_err(|e| ToonError::file_create(path.to_path_buf(), e))?;
        file.write_all(data.as_bytes())
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.flush())
            .map_err(|e| ToonError::file_write(path.to_path_buf(), e))?;
    } else {
        let mut handle = io::stdout().lock();
        handle
            .write_all(data.as_bytes())
            .and_then(|()| handle.write_all(b"\n"))
            .and_then(|()| handle.flush())
            .map_err(ToonError::stdout_write)?;
    }
    Ok(())
}

fn format_input_label(args: &Args) -> String {
    if args.is_stdin() {
        "stdin".to_string()
    } else if let Some(ref path) = args.input {
        path.display().to_string()
    } else {
        "stdin".to_string()
    }
}

/// Print the token estimates of the JSON and TOON forms of one document (to stderr, so stdout can
/// be piped), in either direction, whichever form is smaller.
fn report_stats(json_text: &str, toon_text: &str) {
    let json_tokens = estimate_tokens(json_text);
    let toon_tokens = estimate_tokens(toon_text);

    report("");
    report(&format!(
        "Token estimates: ~{json_tokens} (JSON) → ~{toon_tokens} (TOON)"
    ));
    report(&savings_line(json_tokens, toon_tokens));
}

fn savings_line(json_tokens: usize, toon_tokens: usize) -> String {
    let noun = |n: usize| if n == 1 { "token" } else { "tokens" };
    match toon_tokens.cmp(&json_tokens) {
        std::cmp::Ordering::Less => {
            let diff = json_tokens - toon_tokens;
            format!(
                "Saved ~{diff} {} (-{}%)",
                noun(diff),
                percent_of(diff, json_tokens)
            )
        }
        std::cmp::Ordering::Greater => {
            let diff = toon_tokens - json_tokens;
            format!(
                "TOON is larger by ~{diff} {} (+{}%)",
                noun(diff),
                percent_of(diff, json_tokens)
            )
        }
        std::cmp::Ordering::Equal => "No token difference (0.0%)".to_string(),
    }
}

/// `part / whole` as a percentage with one decimal, rounded half up on the exact quotient (a
/// binary64 quotient rounded twice put exact ties such as 28.75% on the wrong side).
fn percent_of(part: usize, whole: usize) -> String {
    let (part, whole) = (part as u128, whole.max(1) as u128);
    let tenths = (part * 2000 + whole) / (whole * 2);
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Simple token estimation heuristic (roughly 4 chars per token for English/code).
fn estimate_tokens(text: &str) -> usize {
    // Simple heuristic: count non-whitespace chars / 4, with minimum of word count
    let char_estimate = text.chars().filter(|c| !c.is_whitespace()).count() / 4;
    let word_estimate = text.split_whitespace().count();
    char_estimate.max(word_estimate).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_is_rounded_on_the_exact_quotient() {
        assert_eq!(percent_of(23, 80), "28.8");
        assert_eq!(percent_of(49, 80), "61.3");
        assert_eq!(percent_of(1, 3), "33.3");
        assert_eq!(percent_of(2, 3), "66.7");
        assert_eq!(percent_of(0, 5), "0.0");
        assert_eq!(percent_of(5, 5), "100.0");
    }

    #[test]
    fn savings_line_covers_all_three_outcomes() {
        assert_eq!(savings_line(10, 9), "Saved ~1 token (-10.0%)");
        assert_eq!(savings_line(10, 5), "Saved ~5 tokens (-50.0%)");
        assert_eq!(savings_line(4, 5), "TOON is larger by ~1 token (+25.0%)");
        assert_eq!(savings_line(7, 7), "No token difference (0.0%)");
    }

    #[test]
    fn bom_is_stripped_only_at_the_start() {
        assert_eq!(strip_bom("\u{feff}{}".to_string()), "{}");
        assert_eq!(strip_bom("a\u{feff}".to_string()), "a\u{feff}");
    }
}
