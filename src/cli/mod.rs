pub mod args;
pub mod conversion;
pub mod json_stream;
pub mod json_stringify;

use crate::error::{Result, ToonError};
use crate::options::{DecodeOptions, EncodeOptions, ExpandPathsMode, KeyFoldingMode};
use args::{Args, ExpandPathsArg, KeyFoldingArg, Mode};
use clap::Parser;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
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

fn run_encode(args: &Args) -> Result<()> {
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
    let toon_lines = conversion::encode_to_toon_lines(&input, Some(options))?;

    // Output
    if args.stats {
        let toon_output = toon_lines.join("\n");
        write_output(args, toon_output.as_bytes())?;

        // Calculate token estimates (simple heuristic: ~4 chars per token)
        let json_tokens = estimate_tokens(&input);
        let toon_tokens = estimate_tokens(&toon_output);
        let diff = json_tokens.saturating_sub(toon_tokens);
        #[allow(clippy::cast_precision_loss)]
        let percent = if json_tokens > 0 {
            (diff as f64 / json_tokens as f64) * 100.0
        } else {
            0.0
        };

        // Print stats to stderr (so stdout can be piped)
        eprintln!();
        eprintln!("Token estimates: ~{json_tokens} (JSON) → ~{toon_tokens} (TOON)");
        if diff > 0 {
            eprintln!("Saved ~{diff} tokens (-{percent:.1}%)");
        }
    } else {
        // Streaming output
        write_lines(args, &toon_lines)?;
    }

    // Success message to stderr if writing to file
    if let Some(ref output_path) = args.output {
        let input_label = format_input_label(args);
        let output_label = output_path.display();
        eprintln!("Encoded `{input_label}` → `{output_label}`");
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

    // Decode to JSON chunks
    let json_chunks = conversion::decode_to_json_chunks(&input, Some(options))?;

    // Write output
    write_chunks(args, &json_chunks)?;

    // Success message to stderr if writing to file
    if let Some(ref output_path) = args.output {
        let input_label = format_input_label(args);
        let output_label = output_path.display();
        eprintln!("Decoded `{input_label}` → `{output_label}`");
    }

    Ok(())
}

fn read_input(args: &Args) -> Result<String> {
    if args.is_stdin() {
        read_stdin()
    } else {
        let path = args.input.as_ref().expect("input path should exist");
        read_file(path)
    }
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|e| ToonError::message(format!("Failed to read stdin: {e}")))?;
    Ok(buffer)
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| ToonError::message(format!("Failed to read file '{}': {e}", path.display())))
}

fn write_output(args: &Args, data: &[u8]) -> Result<()> {
    if let Some(ref path) = args.output {
        let mut file = File::create(path).map_err(|e| {
            ToonError::message(format!("Failed to create file '{}': {e}", path.display()))
        })?;
        file.write_all(data).map_err(|e| {
            ToonError::message(format!("Failed to write to file '{}': {e}", path.display()))
        })?;
        // Add trailing newline for file output
        file.write_all(b"\n").map_err(|e| {
            ToonError::message(format!("Failed to write to file '{}': {e}", path.display()))
        })?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle
            .write_all(data)
            .map_err(|e| ToonError::message(format!("Failed to write to stdout: {e}")))?;
        handle
            .write_all(b"\n")
            .map_err(|e| ToonError::message(format!("Failed to write to stdout: {e}")))?;
    }
    Ok(())
}

fn write_lines(args: &Args, lines: &[String]) -> Result<()> {
    if let Some(ref path) = args.output {
        let file = File::create(path).map_err(|e| {
            ToonError::message(format!("Failed to create file '{}': {e}", path.display()))
        })?;
        let mut writer = BufWriter::new(file);

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                writer
                    .write_all(b"\n")
                    .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
            }
            writer
                .write_all(line.as_bytes())
                .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
        }
        // Trailing newline
        writer
            .write_all(b"\n")
            .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                handle
                    .write_all(b"\n")
                    .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
            }
            handle
                .write_all(line.as_bytes())
                .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
        }
        // Trailing newline
        handle
            .write_all(b"\n")
            .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
    }
    Ok(())
}

fn write_chunks(args: &Args, chunks: &[String]) -> Result<()> {
    if let Some(ref path) = args.output {
        let file = File::create(path).map_err(|e| {
            ToonError::message(format!("Failed to create file '{}': {e}", path.display()))
        })?;
        let mut writer = BufWriter::new(file);

        for chunk in chunks {
            writer
                .write_all(chunk.as_bytes())
                .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
        }
        // Trailing newline
        writer
            .write_all(b"\n")
            .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for chunk in chunks {
            handle
                .write_all(chunk.as_bytes())
                .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
        }
        // Trailing newline
        handle
            .write_all(b"\n")
            .map_err(|e| ToonError::message(format!("Write error: {e}")))?;
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

/// Simple token estimation heuristic (roughly 4 chars per token for English/code).
/// This matches the behavior of tokenx used in the legacy CLI.
fn estimate_tokens(text: &str) -> usize {
    // Simple heuristic: count non-whitespace chars / 4, with minimum of word count
    let char_estimate = text.chars().filter(|c| !c.is_whitespace()).count() / 4;
    let word_estimate = text.split_whitespace().count();
    char_estimate.max(word_estimate).max(1)
}
