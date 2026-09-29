//! Text/code viewer layer.
//!
//! Provides line-numbered text viewing with UTF-8 validation, large-file
//! diagnostics, and error handling. Keeps all logic headless-testable.

mod buffer;

use std::fs;
use std::io::Read;
use std::path::Path;

/// 1 MiB — files larger than this skip structural parse with a diagnostic.
/// Matches the NFR-7 file-parse ceiling from the PRD.
pub const LARGE_FILE_THRESHOLD: u64 = 1_048_576;

/// A single line of text with its 1-indexed line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// 1-indexed line number.
    pub number: u32,
    /// The text of the line (UTF-8, CRLF normalized to LF, trailing whitespace preserved).
    pub text: String,
}

/// Content of a text file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextContent {
    /// File opened successfully but has no lines.
    Empty,
    /// One or more lines.
    Lines(Vec<Line>),
}

/// Opens a file and returns its line-numbered content.
///
/// - Returns `Ok(Empty)` for zero-byte files.
/// - Returns `Err(msg)` for I/O errors (missing file, permission denied).
/// - Returns `Err(msg)` for invalid UTF-8.
/// - Files over [`LARGE_FILE_THRESHOLD`] bytes open successfully with lines,
///   but the caller should surface a diagnostic via [`large_file_diagnostic`].
///   The structural-parse ceiling is a parse limitation, not a visibility cap.
pub fn open_file(path: &Path) -> Result<TextContent, String> {
    let _metadata = fs::metadata(path).map_err(|e| format!("I/O error: {e}"))?;

    let mut file = fs::File::open(path).map_err(|e| format!("I/O error: {e}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("I/O error: {e}"))?;

    let text = String::from_utf8(bytes)
        .map_err(|_| "Invalid UTF-8: the file contains bytes that are not valid UTF-8 encoding")?;

    if text.is_empty() {
        return Ok(TextContent::Empty);
    }

    // Normalize CRLF -> LF
    let text = text.replace("\r\n", "\n");
    let text = text.replace('\r', "\n");

    let lines: Vec<Line> = text
        .lines()
        .enumerate()
        .map(|(i, s)| Line {
            number: (i + 1) as u32,
            text: s.to_string(),
        })
        .collect();

    Ok(TextContent::Lines(lines))
}

/// Returns a user-visible diagnostic message for large files.
///
/// The 1 MiB ceiling is a structural-parse constraint, not a source-text
/// visibility cap. Files over this size are opened and displayed, but
/// structural analysis features are unavailable.
pub fn large_file_diagnostic() -> String {
    "This file is too large for structural analysis (>1 MiB). \
     The text is still visible, but symbol detection and graph features \
     are not available for this file."
        .to_string()
}
