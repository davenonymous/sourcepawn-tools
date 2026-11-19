// Copyright 2026 davenonymous

//! Source files: loading, decoding and the facts about a file that both
//! tools need (its kind, its byte order mark and its line endings).

use std::fmt;
use std::path::{Path, PathBuf};

use crate::position::LineIndex;

/// What a file is to the SourcePawn compiler, decided by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// A plugin's main file (`.sp`), compiled on its own.
    Plugin,
    /// An include file (`.inc`), pulled in by `#include`.
    Include,
}

impl FileKind {
    /// The kind of the file at `path`: `.inc` is an include, everything else
    /// is treated as a plugin file.
    pub fn from_path(path: &Path) -> FileKind {
        let is_include = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("inc"));
        if is_include { FileKind::Include } else { FileKind::Plugin }
    }

    /// The short name used in configuration files and the policy API.
    pub fn name(self) -> &'static str {
        match self {
            FileKind::Plugin => "sp",
            FileKind::Include => "inc",
        }
    }
}

/// The line ending convention of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }

    /// The line ending most lines of `text` use; LF for a file without line
    /// breaks or with a tie.
    pub fn detect(text: &str) -> LineEnding {
        let line_feeds = text.matches('\n').count();
        let carriage_return_line_feeds = text.matches("\r\n").count();
        let bare_line_feeds = line_feeds - carriage_return_line_feeds;
        if carriage_return_line_feeds > bare_line_feeds { LineEnding::CrLf } else { LineEnding::Lf }
    }
}

/// Why a file could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("{path}: cannot read: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path}:{line}: not valid UTF-8; convert the file to UTF-8 first")]
    NotUtf8 { path: PathBuf, line: usize },
}

/// A decoded source file. The text never contains the byte order mark;
/// [`SourceFile::has_bom`] remembers whether the file had one.
pub struct SourceFile {
    name: PathBuf,
    kind: FileKind,
    text: String,
    has_bom: bool,
    lines: LineIndex,
}

const BYTE_ORDER_MARK: &str = "\u{feff}";

impl SourceFile {
    /// Reads and decodes the file at `path`.
    pub fn read(path: &Path) -> Result<SourceFile, LoadError> {
        let bytes = std::fs::read(path).map_err(|source| LoadError::Io { path: path.to_path_buf(), source })?;
        SourceFile::from_bytes(path, bytes)
    }

    /// Decodes `bytes` as the content of a file named `name`. The name
    /// decides the file kind and appears in every report about the file.
    pub fn from_bytes(name: &Path, bytes: Vec<u8>) -> Result<SourceFile, LoadError> {
        let text = String::from_utf8(bytes).map_err(|error| {
            let valid_prefix = &error.as_bytes()[..error.utf8_error().valid_up_to()];
            let line = 1 + valid_prefix.iter().filter(|&&byte| byte == b'\n').count();
            LoadError::NotUtf8 { path: name.to_path_buf(), line }
        })?;
        Ok(SourceFile::from_text(name, text))
    }

    /// Wraps already decoded text.
    pub fn from_text(name: &Path, text: String) -> SourceFile {
        let (text, has_bom) = match text.strip_prefix(BYTE_ORDER_MARK) {
            Some(rest) => (rest.to_string(), true),
            None => (text, false),
        };
        let lines = LineIndex::new(&text);
        SourceFile { name: name.to_path_buf(), kind: FileKind::from_path(name), text, has_bom, lines }
    }

    /// The name the file is reported under: its path as given.
    pub fn name(&self) -> &Path {
        &self.name
    }

    pub fn kind(&self) -> FileKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn has_bom(&self) -> bool {
        self.has_bom
    }

    /// The text as it was read: with the byte order mark if the file had
    /// one.
    pub fn text_as_read(&self) -> String {
        if self.has_bom { format!("{BYTE_ORDER_MARK}{}", self.text) } else { self.text.clone() }
    }

    pub fn lines(&self) -> &LineIndex {
        &self.lines
    }

    pub fn line_ending(&self) -> LineEnding {
        LineEnding::detect(&self.text)
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceFile")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("bytes", &self.text.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_follows_the_extension() {
        assert_eq!(FileKind::from_path(Path::new("a/b.inc")), FileKind::Include);
        assert_eq!(FileKind::from_path(Path::new("a/b.INC")), FileKind::Include);
        assert_eq!(FileKind::from_path(Path::new("a/b.sp")), FileKind::Plugin);
        assert_eq!(FileKind::from_path(Path::new("-")), FileKind::Plugin);
    }

    #[test]
    fn byte_order_mark_is_stripped_and_remembered() {
        let file = SourceFile::from_text(Path::new("x.sp"), "\u{feff}int x;".to_string());
        assert!(file.has_bom());
        assert_eq!(file.text(), "int x;");
        assert_eq!(file.text_as_read(), "\u{feff}int x;");
    }

    #[test]
    fn invalid_utf8_names_the_line() {
        let error = SourceFile::from_bytes(Path::new("x.sp"), b"a\nb\n\xff".to_vec()).unwrap_err();
        assert!(matches!(error, LoadError::NotUtf8 { line: 3, .. }));
    }

    #[test]
    fn line_endings_follow_the_majority() {
        assert_eq!(LineEnding::detect("a\r\nb\r\nc\n"), LineEnding::CrLf);
        assert_eq!(LineEnding::detect("a\nb\r\n"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("a"), LineEnding::Lf);
    }
}
