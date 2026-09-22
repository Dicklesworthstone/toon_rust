use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// TOON CLI — Convert between JSON and TOON formats
#[derive(Parser, Debug)]
// `bin_name`: usage lines name the program `toon` like the version line does, whatever path or
// name the binary was started under.
#[command(name = "toon", bin_name = "toon", version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
#[command(after_help = "EXAMPLES:
    toon input.json                  # Encode JSON to TOON (stdout)
    toon input.toon                  # Decode TOON to JSON (stdout)
    toon input.json -o output.toon   # Encode to file
    cat data.json | toon --encode    # Encode from stdin
    cat data.toon | toon --decode    # Decode from stdin
    toon input.json --stats          # Show token statistics")]
pub struct Args {
    /// Input file path (omit or use "-" to read from stdin)
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Output file path (omit or use "-" to write to stdout)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Encode JSON to TOON (auto-detected by default)
    #[arg(short, long, conflicts_with = "decode")]
    pub encode: bool,

    /// Decode TOON to JSON (auto-detected by default)
    #[arg(short, long, conflicts_with = "encode")]
    pub decode: bool,

    /// Delimiter for arrays: comma (`,` or `comma`), tab (`\t` or `tab`), or pipe (`|` or `pipe`)
    #[arg(long, default_value = ",", value_parser = parse_delimiter)]
    pub delimiter: char,

    /// Indentation size in spaces, 0 to 16 (encode: TOON output, at least 1; decode: TOON input
    /// and JSON output, where 0 prints compact JSON)
    // A negative value is a value (`--indent -1` is the range error), not an unknown flag.
    #[arg(long, default_value = "2", allow_negative_numbers = true,
          value_parser = clap::value_parser!(u8).range(0..=16))]
    pub indent: u8,

    /// Disable strict mode for decoding (allows lenient parsing)
    #[arg(long = "no-strict")]
    pub no_strict: bool,

    /// Key folding mode: off or safe
    #[arg(long, value_enum, default_value = "off")]
    pub key_folding: KeyFoldingArg,

    /// Maximum folded segment count when key folding is enabled
    #[arg(long, value_name = "N", allow_negative_numbers = true)]
    pub flatten_depth: Option<usize>,

    /// Path expansion mode: off or safe (decode only)
    #[arg(long, value_enum, default_value = "off")]
    pub expand_paths: ExpandPathsArg,

    /// Show token estimates for the JSON and TOON forms (stderr)
    #[arg(long)]
    pub stats: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KeyFoldingArg {
    Off,
    Safe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExpandPathsArg {
    Off,
    Safe,
}

fn parse_delimiter(s: &str) -> Result<char, String> {
    match s {
        "," | "comma" => Ok(','),
        "|" | "pipe" => Ok('|'),
        "\\t" | "\t" | "tab" => Ok('\t'),
        _ => Err(format!(
            "Invalid delimiter \"{s}\". Valid delimiters are: comma (,), tab (\\t), pipe (|)"
        )),
    }
}

impl Args {
    /// Detect the operation mode based on flags and file extension.
    #[must_use]
    pub fn detect_mode(&self) -> Mode {
        // Explicit flags take precedence
        if self.encode {
            return Mode::Encode;
        }
        if self.decode {
            return Mode::Decode;
        }

        // Auto-detect based on the file name's suffix. `Path::extension` reports none for a
        // dotfile, so a file named just `.toon` would have been read as JSON.
        if let Some(ref path) = self.input
            && let Some(name) = path.file_name()
            && let Some((_, ext)) = name.to_string_lossy().rsplit_once('.')
        {
            let ext = ext.to_lowercase();
            if ext == "json" {
                return Mode::Encode;
            }
            if ext == "toon" {
                return Mode::Decode;
            }
        }

        // Default to encode
        Mode::Encode
    }

    /// The output file, or `None` for stdout (no `-o`, or `-o -`).
    #[must_use]
    pub fn output_file(&self) -> Option<&std::path::Path> {
        self.output
            .as_deref()
            .filter(|path| path.as_os_str() != "-")
    }

    /// Returns true if reading from stdin.
    #[must_use]
    pub fn is_stdin(&self) -> bool {
        self.input.is_none() || self.input.as_ref().is_some_and(|p| p.as_os_str() == "-")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Encode,
    Decode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delimiter() {
        assert_eq!(parse_delimiter(","), Ok(','));
        assert_eq!(parse_delimiter("|"), Ok('|'));
        assert_eq!(parse_delimiter("\\t"), Ok('\t'));
        assert_eq!(parse_delimiter("tab"), Ok('\t'));
        assert!(parse_delimiter("invalid").is_err());
    }

    #[test]
    fn test_detect_mode_explicit_flags() {
        let args = Args {
            input: None,
            output: None,
            encode: true,
            decode: false,
            delimiter: ',',
            indent: 2,
            no_strict: false,
            key_folding: KeyFoldingArg::Off,
            flatten_depth: None,
            expand_paths: ExpandPathsArg::Off,
            stats: false,
        };
        assert_eq!(args.detect_mode(), Mode::Encode);
    }

    #[test]
    fn test_detect_mode_by_extension() {
        let args = Args {
            input: Some(PathBuf::from("data.toon")),
            output: None,
            encode: false,
            decode: false,
            delimiter: ',',
            indent: 2,
            no_strict: false,
            key_folding: KeyFoldingArg::Off,
            flatten_depth: None,
            expand_paths: ExpandPathsArg::Off,
            stats: false,
        };
        assert_eq!(args.detect_mode(), Mode::Decode);
    }
}
