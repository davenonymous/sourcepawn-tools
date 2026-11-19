// Copyright 2026 davenonymous

//! Text for a terminal. Source lines, paths and messages that quote code
//! come from files, and a control character in them (an escape sequence
//! that retitles the window or clears the screen) would reach the terminal
//! as a command.

use std::borrow::Cow;

/// The character every unprintable character becomes: one column wide,
/// like the characters it replaces are counted.
pub const REPLACEMENT: char = '\u{FFFD}';

fn is_unprintable(character: char) -> bool {
    character.is_control() && character != '\t'
}

/// `text` with every control character but the tab replaced by
/// [`REPLACEMENT`], line breaks included: for text shown on one line.
pub fn printable(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_unprintable) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(text.chars().map(|character| if is_unprintable(character) { REPLACEMENT } else { character }).collect())
}

/// `text` made [`printable`] line by line, keeping its `\n` and `\r\n`
/// line breaks: for text shown over several lines, such as a diff.
pub fn printable_lines(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (content, ending) = match line.strip_suffix("\r\n") {
            Some(content) => (content, "\r\n"),
            None => line.strip_suffix('\n').map_or((line, ""), |content| (content, "\n")),
        };
        shown.push_str(&printable(content));
        shown.push_str(ending);
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_become_replacements() {
        assert_eq!(printable("plain\ttext"), "plain\ttext");
        assert_eq!(
            printable("a\x1b]0;title\x07b\x1b[2J\n\u{9b}"),
            "a\u{FFFD}]0;title\u{FFFD}b\u{FFFD}[2J\u{FFFD}\u{FFFD}"
        );
    }

    #[test]
    fn line_breaks_stay_when_lines_are_kept() {
        assert_eq!(printable_lines("a\r\nb\nc"), "a\r\nb\nc");
        assert_eq!(printable_lines("a\x1b[2J\r\nb\rc\n"), "a\u{FFFD}[2J\r\nb\u{FFFD}c\n");
    }
}
