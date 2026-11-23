// Copyright 2026 davenonymous

//! The style pawntidy formats with, and its configuration file,
//! `.pawntidy.toml`. The nearest one above a file applies to it.
//!
//! The indentation is one tab per level or, with `indent_style = "spaces"`,
//! `indent_width` spaces per level; spaces after it align a continued line.
//! Tab stops are always four columns apart.

use std::path::Path;

use pawn_core::LineEnding;
use pawn_core::cli::{ConfigError, ConfigFile};
use serde::{Deserialize, Serialize};

/// The width of a tab stop, for measuring lines and aligning.
pub const TAB_WIDTH: usize = 4;

/// What a line is indented with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndentStyle {
    /// One tab per level.
    Tabs,
    /// `indent_width` spaces per level.
    Spaces,
}

/// The indentation of one block level, as the style resolves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentUnit {
    Tab,
    Spaces(usize),
}

impl IndentUnit {
    /// The columns one level takes.
    pub fn width(self) -> usize {
        match self {
            IndentUnit::Tab => TAB_WIDTH,
            IndentUnit::Spaces(width) => width,
        }
    }

    /// The text of `levels` levels.
    pub fn render(self, levels: usize) -> String {
        match self {
            IndentUnit::Tab => "\t".repeat(levels),
            IndentUnit::Spaces(width) => " ".repeat(levels * width),
        }
    }
}

/// The widths `indent_width` may take.
const INDENT_WIDTHS: std::ops::RangeInclusive<usize> = 2..=8;

/// How a line that continues a statement inside brackets is indented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Continuation {
    /// Aligned with spaces to the column after the open bracket.
    Align,
    /// One tab deeper than the line with the open bracket.
    Indent,
}

/// Which line endings to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEndingStyle {
    /// The line ending most lines of the file already use.
    Auto,
    Lf,
    Crlf,
}

impl LineEndingStyle {
    pub fn resolve(self, text: &str) -> LineEnding {
        match self {
            LineEndingStyle::Auto => LineEnding::detect(text),
            LineEndingStyle::Lf => LineEnding::Lf,
            LineEndingStyle::Crlf => LineEnding::CrLf,
        }
    }
}

/// The configurable parts of the style.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Style {
    pub indent_style: IndentStyle,
    /// The spaces per level with `indent_style = "spaces"`; ignored with
    /// tabs.
    pub indent_width: usize,
    /// The widest a line may be, in columns, with tabs four wide. Longer
    /// lines are wrapped where the code allows it.
    pub max_line_length: usize,
    /// The most blank lines kept in a row.
    pub max_blank_lines: usize,
    pub continuation: Continuation,
    /// Write `} else {` on one line; with false, `else` starts a line.
    pub cuddle_else: bool,
    /// Write `if (`, `for (`, `while (` and `switch (` with a space.
    pub space_before_control_paren: bool,
    /// Align the trailing `//` comments of consecutive lines.
    pub align_trailing_comments: bool,
    pub line_ending: LineEndingStyle,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            indent_style: IndentStyle::Tabs,
            indent_width: 4,
            max_line_length: 120,
            max_blank_lines: 1,
            continuation: Continuation::Align,
            cuddle_else: true,
            space_before_control_paren: true,
            align_trailing_comments: true,
            line_ending: LineEndingStyle::Auto,
        }
    }
}

/// The style is the whole configuration file.
impl ConfigFile for Style {
    const FILE_NAMES: &'static [&'static str] = &[".pawntidy.toml", "pawntidy.toml"];

    fn read(path: &Path) -> Result<Style, ConfigError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| ConfigError::Io { path: path.to_path_buf(), source })?;
        Style::parse(&text, path)
    }
}

impl Style {
    /// Parses `text`, the content of the configuration file at `path`.
    pub fn parse(text: &str, path: &Path) -> Result<Style, ConfigError> {
        let invalid = |message: String| ConfigError::Invalid { path: path.to_path_buf(), message };
        let style: Style = toml::from_str(text).map_err(|error| invalid(error.to_string().trim_end().to_string()))?;
        if style.max_line_length < 40 {
            return Err(invalid(format!("max_line_length is {}, but must be at least 40", style.max_line_length)));
        }
        if !INDENT_WIDTHS.contains(&style.indent_width) {
            return Err(invalid(format!(
                "indent_width is {}, but must be between {} and {}",
                style.indent_width,
                INDENT_WIDTHS.start(),
                INDENT_WIDTHS.end()
            )));
        }
        Ok(style)
    }

    /// The indentation of one block level.
    pub fn indent_unit(&self) -> IndentUnit {
        match self.indent_style {
            IndentStyle::Tabs => IndentUnit::Tab,
            IndentStyle::Spaces => IndentUnit::Spaces(self.indent_width),
        }
    }

    /// The style as the configuration file writes it, every key set.
    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("a style always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse_and_default() {
        let style =
            Style::parse("max_line_length = 100\ncontinuation = \"indent\"\ncuddle_else = false", Path::new("x.toml"))
                .unwrap();
        assert_eq!(style.max_line_length, 100);
        assert_eq!(style.continuation, Continuation::Indent);
        assert!(!style.cuddle_else);
        assert_eq!(style.max_blank_lines, Style::default().max_blank_lines);
    }

    #[test]
    fn indentation_is_configured_by_style_and_width() {
        let style = Style::parse("indent_style = \"spaces\"\nindent_width = 2", Path::new("x.toml")).unwrap();
        assert_eq!(style.indent_unit(), IndentUnit::Spaces(2));
        let style = Style::parse("indent_width = 8", Path::new("x.toml")).unwrap();
        assert_eq!(style.indent_unit(), IndentUnit::Tab);
        for wrong in ["indent_width = 1", "indent_width = 9", "indent_style = \"spaces\"\nindent_width = 0"] {
            let error = Style::parse(wrong, Path::new("x.toml")).unwrap_err().to_string();
            assert!(error.contains("but must be between 2 and 8"), "{error}");
        }
        let error = Style::parse("indent_style = \"mixed\"", Path::new("x.toml")).unwrap_err().to_string();
        assert!(error.contains("indent_style"), "{error}");
        let error = Style::parse("use_tabs = false", Path::new("x.toml")).unwrap_err().to_string();
        assert!(error.contains("use_tabs"), "{error}");
    }

    #[test]
    fn a_written_style_reads_back_the_same() {
        let style = Style {
            indent_style: IndentStyle::Spaces,
            indent_width: 2,
            max_line_length: 100,
            max_blank_lines: 2,
            continuation: Continuation::Indent,
            cuddle_else: false,
            space_before_control_paren: false,
            align_trailing_comments: false,
            line_ending: LineEndingStyle::Crlf,
        };
        for style in [style, Style::default()] {
            assert_eq!(Style::parse(&style.to_toml(), Path::new("x.toml")).unwrap(), style);
        }
        assert!(Style::default().to_toml().starts_with(
            "indent_style = \"tabs\"
indent_width = 4
max_line_length = 120
"
        ));
    }

    #[test]
    fn tiny_line_lengths_are_refused() {
        assert!(Style::parse("max_line_length = 10", Path::new("x.toml")).is_err());
    }
}
