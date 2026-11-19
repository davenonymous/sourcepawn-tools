// Copyright 2026 davenonymous

//! Byte spans and the line and column positions reported to users and
//! tools.

use std::ops::Range;

/// A range of bytes in a source text, start inclusive, end exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        assert!(start <= end, "span start {start} lies after its end {end}");
        Span { start, end }
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn contains(self, other: Span) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

/// A position in a source text. Lines and columns count from 1; a column
/// counts Unicode scalar values, so a tab is one column. The UTF-16
/// column counts from 0, as the Language Server Protocol wants it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub utf16_column: usize,
    pub byte: usize,
}

/// The start of every line of a text, for turning byte offsets into
/// positions.
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    text_len: usize,
}

impl LineIndex {
    pub fn new(text: &str) -> LineIndex {
        let line_starts = std::iter::once(0).chain(text.match_indices('\n').map(|(offset, _)| offset + 1)).collect();
        LineIndex { line_starts, text_len: text.len() }
    }

    /// The number of lines; a trailing line break does not start a line.
    pub fn line_count(&self) -> usize {
        let last_start = *self.line_starts.last().expect("a line index always has a first line");
        if last_start == self.text_len && self.line_starts.len() > 1 {
            self.line_starts.len() - 1
        } else {
            self.line_starts.len()
        }
    }

    /// The 1-based line that holds `byte`.
    pub fn line_of(&self, byte: usize) -> usize {
        self.line_starts.partition_point(|&start| start <= byte)
    }

    /// The bytes of 1-based `line`, without its line break.
    pub fn line_span(&self, text: &str, line: usize) -> Span {
        assert!(line >= 1 && line <= self.line_starts.len(), "line {line} is out of range");
        let start = self.line_starts[line - 1];
        let mut end = self.line_starts.get(line).map_or(text.len(), |&next| next - 1);
        if end > start && text.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        Span::new(start, end.max(start))
    }

    /// The byte offset of 1-based `line` and `column`, or `None` if the
    /// line does not exist. A column past the end of the line yields the
    /// end of the line.
    pub fn byte_of(&self, text: &str, line: usize, column: usize) -> Option<usize> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        let span = self.line_span(text, line);
        let line_text = &text[span.range()];
        let offset =
            line_text.char_indices().nth(column.saturating_sub(1)).map_or(line_text.len(), |(offset, _)| offset);
        Some(span.start + offset)
    }

    /// The position of `byte`, which must lie on a character boundary of
    /// `text`.
    pub fn position(&self, text: &str, byte: usize) -> Position {
        let byte = byte.min(text.len());
        let line = self.line_of(byte);
        let line_start = self.line_starts[line - 1];
        let before = &text[line_start..byte];
        Position {
            line,
            column: before.chars().count() + 1,
            utf16_column: before.chars().map(char::len_utf16).sum(),
            byte,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_count_lines_and_characters() {
        let text = "ab\n\täx\r\nz";
        let index = LineIndex::new(text);
        assert_eq!(index.line_count(), 3);
        let x = text.find('x').unwrap();
        let position = index.position(text, x);
        assert_eq!((position.line, position.column, position.utf16_column), (2, 3, 2));
        assert_eq!(&text[index.line_span(text, 2).range()], "\täx");
        assert_eq!(index.position(text, text.len()).line, 3);
    }

    #[test]
    fn a_trailing_line_break_starts_no_line() {
        assert_eq!(LineIndex::new("a\nb\n").line_count(), 2);
        assert_eq!(LineIndex::new("").line_count(), 1);
    }
}
