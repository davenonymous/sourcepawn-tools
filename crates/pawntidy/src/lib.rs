// Copyright 2026 davenonymous

//! pawntidy: a formatter for SourcePawn, in the spirit of perltidy.
//!
//! The formatter reads a file with the tree-sitter grammar and the lexer of
//! `pawn-core`, decides from the tree how each token is spaced, indented
//! and broken (`annotate`), lays the tokens out in lines (`layout`) and
//! checks the result before it hands it out (`verify`): the output must
//! hold the same tokens in the same order, plus the braces pawntidy adds
//! around single-statement bodies, must parse as well as the input, and
//! into the same tree with a block around each braced body.
//! A result that fails the check is an error, never a changed file, and
//! so is a panic of the formatter.
//!
//! Top-level items the parser cannot read, those that hold a line splice
//! and those that hold a `#` that is not first on its line are kept as
//! they are.

mod annotate;
pub mod config;
mod layout;
mod verify;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use pawn_core::cli::panic_message;
use pawn_core::{LineEnding, ParsedFile, SourceFile};

pub use config::Style;

/// A formatted file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted {
    /// The formatted text, with the line endings of the style and the byte
    /// order mark if the file had one.
    pub text: String,
    /// The first line, from 1, in which the text differs from the file as
    /// it was read; none when formatting changes nothing.
    pub first_changed_line: Option<usize>,
    /// The regions of the input kept as they are.
    pub kept: Vec<KeptRegion>,
}

/// Why lines were kept as they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptReason {
    /// The parser cannot read the item.
    SyntaxError,
    /// The item holds a line splice, a backslash at the end of a line,
    /// which joins the lines for the compiler and which the layout cannot
    /// keep.
    LineSplice,
    /// The item holds a `#` that is not first on its line, which the
    /// lexer does not read as a directive and the layout could move to
    /// the start of a line.
    StrayHash,
}

impl KeptReason {
    pub fn describe(self) -> &'static str {
        match self {
            KeptReason::SyntaxError => "the parser cannot read them",
            KeptReason::LineSplice => "they hold a line splice (a backslash at the end of a line)",
            KeptReason::StrayHash => "they hold a `#` that is not first on its line",
        }
    }

    /// The name in the JSON output.
    pub fn name(self) -> &'static str {
        match self {
            KeptReason::SyntaxError => "syntax_error",
            KeptReason::LineSplice => "line_splice",
            KeptReason::StrayHash => "stray_hash",
        }
    }
}

/// Lines of the input kept as they are, 1-based and inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeptRegion {
    pub first_line: usize,
    pub last_line: usize,
    pub reason: KeptReason,
}

fn kept_regions(annotations: &annotate::Annotations) -> Vec<KeptRegion> {
    let overlaps = |(first, last): (usize, usize), (other_first, other_last): (usize, usize)| {
        first <= other_last && other_first <= last
    };
    annotations
        .verbatim_lines
        .iter()
        .map(|&(first_line, last_line)| {
            let region = (first_line, last_line);
            let holds = |ranges: &[(usize, usize)]| ranges.iter().any(|&range| overlaps(region, range));
            let reason = if holds(&annotations.splice_lines) {
                KeptReason::LineSplice
            } else if holds(&annotations.stray_hash_lines) {
                KeptReason::StrayHash
            } else {
                KeptReason::SyntaxError
            };
            KeptRegion { first_line, last_line, reason }
        })
        .collect()
}

impl Formatted {
    /// Whether formatting changes the file.
    pub fn is_changed(&self) -> bool {
        self.first_changed_line.is_some()
    }
}

/// Why a file was not formatted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    #[error(
        "{path}:{line}: pawntidy would change the code, not only its layout ({detail}); the file is left unchanged. \
         This is a bug in pawntidy, please report it with the file"
    )]
    Verification { path: String, line: usize, detail: String },
    #[error(
        "{path}: internal error in pawntidy: {message}; the file is left unchanged, please report it with the file"
    )]
    Internal { path: String, message: String },
}

/// Formats the text of a file named `name`.
pub fn format_source(name: &Path, text: &str, style: &Style) -> Result<Formatted, FormatError> {
    let parsed = ParsedFile::parse(SourceFile::from_text(name, text.to_string()));
    format_parsed(&parsed, style)
}

/// Formats a parsed file. A panic of the formatter is caught and returned
/// as [`FormatError::Internal`].
pub fn format_parsed(file: &ParsedFile, style: &Style) -> Result<Formatted, FormatError> {
    catch_unwind(AssertUnwindSafe(|| format_unguarded(file, style))).unwrap_or_else(|panic| {
        Err(FormatError::Internal { path: file.source.name().display().to_string(), message: panic_message(&*panic) })
    })
}

fn format_unguarded(file: &ParsedFile, style: &Style) -> Result<Formatted, FormatError> {
    let annotations = annotate::annotate(file);
    let items = layout::build_items(file, &annotations);
    let lines = layout::layout(file, &annotations, &items, style);
    let mut text = if items.is_empty() { String::new() } else { layout::render(&items, &lines) };
    verify::verify(file, &items, &text).map_err(|mismatch| FormatError::Verification {
        path: file.source.name().display().to_string(),
        line: mismatch.line,
        detail: mismatch.detail,
    })?;
    if style.line_ending.resolve(file.text()) == LineEnding::CrLf {
        text = text.replace('\n', "\r\n");
    }
    if file.source.has_bom() {
        text.insert(0, '\u{feff}');
    }
    let first_changed_line = first_different_line(&file.source.text_as_read(), &text);
    Ok(Formatted { text, first_changed_line, kept: kept_regions(&annotations) })
}

/// The first line, from 1, in which `before` and `after` differ.
fn first_different_line(before: &str, after: &str) -> Option<usize> {
    if before == after {
        return None;
    }
    let mut before_lines = before.split('\n');
    let mut after_lines = after.split('\n');
    let mut line = 1;
    loop {
        match (before_lines.next(), after_lines.next()) {
            (Some(a), Some(b)) if a == b => line += 1,
            _ => return Some(line),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_changed_line_is_found() {
        assert_eq!(first_different_line("a\nb\nc", "a\nb\nc"), None);
        assert_eq!(first_different_line("a\nb\nc", "a\nB\nc"), Some(2));
        assert_eq!(first_different_line("a\nb", "a\nb\n"), Some(3));
    }

    #[test]
    fn the_result_tells_whether_and_where_the_file_changes() {
        let style = Style::default();
        let tidy = format_source(Path::new("a.sp"), "int a;\nint b;\n", &style).unwrap();
        assert_eq!((tidy.first_changed_line, tidy.is_changed()), (None, false));
        let untidy = format_source(Path::new("a.sp"), "int a;\nint  b;\n", &style).unwrap();
        assert_eq!((untidy.first_changed_line, untidy.is_changed()), (Some(2), true));
        let with_bom = format_source(Path::new("a.sp"), "\u{feff}int a;\n", &style).unwrap();
        assert_eq!((with_bom.text.as_str(), with_bom.is_changed()), ("\u{feff}int a;\n", false));
    }
}
