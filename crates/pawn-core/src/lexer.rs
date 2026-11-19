// Copyright 2026 davenonymous

//! A lossless lexer for SourcePawn.
//!
//! Every byte of the text belongs either to a token or to the whitespace
//! between two tokens, so the text between tokens is all the formatter may
//! change. The lexer never fails: what it does not understand becomes an
//! [`TokenKind::Unknown`] token of one character, and an unterminated
//! string or comment runs to the end of its line or of the text.
//!
//! A preprocessor directive is one token from its `#` to the end of its
//! logical line (backslash line continuations and block comments included),
//! but a line comment after it is a token of its own.

use crate::position::Span;

/// The kinds of tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A name or keyword (`[A-Za-z_@][A-Za-z0-9_@]*`).
    Word,
    /// An integer or float literal.
    Number,
    /// A string literal, quotes included.
    String,
    /// A character literal, quotes included.
    Char,
    /// `// ...` up to the end of the line.
    LineComment,
    /// `/* ... */`.
    BlockComment,
    /// A preprocessor directive such as `#include <sourcemod>`.
    Directive,
    /// An operator or punctuation mark.
    Punct,
    /// A character the lexer does not know.
    Unknown,
}

impl TokenKind {
    pub fn is_comment(self) -> bool {
        matches!(self, TokenKind::LineComment | TokenKind::BlockComment)
    }

    /// The name used in the policy API.
    pub fn name(self) -> &'static str {
        match self {
            TokenKind::Word => "word",
            TokenKind::Number => "number",
            TokenKind::String => "string",
            TokenKind::Char => "char",
            TokenKind::LineComment => "line_comment",
            TokenKind::BlockComment => "block_comment",
            TokenKind::Directive => "directive",
            TokenKind::Punct => "punct",
            TokenKind::Unknown => "unknown",
        }
    }
}

/// A token: its kind and where it lies in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// False for a string, character or block comment without its closing
    /// delimiter.
    pub terminated: bool,
}

impl Token {
    pub fn text<'text>(&self, text: &'text str) -> &'text str {
        &text[self.span.range()]
    }
}

/// The operators and punctuation marks, longest first so that the first
/// match is the longest.
const PUNCTUATION: &[&str] = &[
    ">>>=", "...", "<<=", ">>=", ">>>", "::", "==", "!=", "<=", ">=", "&&", "||", "++", "--", "+=", "-=", "*=", "/=",
    "%=", "&=", "|=", "^=", "~=", "<<", ">>", "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "=", "<", ">", "?",
    ":", ";", ",", ".", "(", ")", "[", "]", "{", "}",
];

/// The SourcePawn keywords, old and new.
const KEYWORDS: &[&str] = &[
    "acquire",
    "as",
    "assert",
    "break",
    "builtin",
    "case",
    "cast_to",
    "catch",
    "char",
    "const",
    "continue",
    "decl",
    "default",
    "defined",
    "delete",
    "do",
    "double",
    "else",
    "enum",
    "exit",
    "explicit",
    "false",
    "finally",
    "float",
    "for",
    "foreach",
    "forward",
    "funcenum",
    "functag",
    "function",
    "goto",
    "if",
    "implicit",
    "import",
    "in",
    "int",
    "int8",
    "int16",
    "int32",
    "int64",
    "interface",
    "intn",
    "let",
    "methodmap",
    "namespace",
    "native",
    "new",
    "null",
    "__nullable__",
    "object",
    "operator",
    "package",
    "private",
    "property",
    "protected",
    "public",
    "readonly",
    "return",
    "sealed",
    "sizeof",
    "sleep",
    "static",
    "stock",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typedef",
    "typeset",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "uintn",
    "union",
    "using",
    "var",
    "variant",
    "view_as",
    "virtual",
    "void",
    "volatile",
    "while",
    "with",
];

/// Whether `word` is a SourcePawn keyword.
pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

fn is_word_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'@'
}

fn is_word_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'@'
}

/// Splits `text` into tokens.
pub fn lex(text: &str) -> Vec<Token> {
    Lexer { text, bytes: text.as_bytes(), offset: 0, at_line_start: true, tokens: Vec::new() }.run()
}

struct Lexer<'text> {
    text: &'text str,
    bytes: &'text [u8],
    offset: usize,
    /// Whether only whitespace lies between the start of the line and the
    /// offset, which is where a directive may start.
    at_line_start: bool,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn run(mut self) -> Vec<Token> {
        while self.offset < self.bytes.len() {
            if self.skip_whitespace() {
                continue;
            }
            let start = self.offset;
            let (kind, terminated) = self.scan_token();
            assert!(self.offset > start, "the lexer made no progress at byte {start}");
            self.tokens.push(Token { kind, span: Span::new(start, self.offset), terminated });
            self.at_line_start = false;
        }
        self.tokens
    }

    fn peek(&self, ahead: usize) -> Option<u8> {
        self.bytes.get(self.offset + ahead).copied()
    }

    /// Skips one whitespace character or line splice; false when the next
    /// character starts a token.
    fn skip_whitespace(&mut self) -> bool {
        match self.peek(0) {
            Some(b'\n') => {
                self.offset += 1;
                self.at_line_start = true;
                true
            }
            Some(b' ' | b'\t' | b'\r' | 0x0b | 0x0c) => {
                self.offset += 1;
                true
            }
            Some(b'\\') if self.line_break_length(self.offset + 1) > 0 => {
                self.offset += 1 + self.line_break_length(self.offset + 1);
                true
            }
            _ => false,
        }
    }

    /// The length of the line break at `offset`, 0 if there is none.
    fn line_break_length(&self, offset: usize) -> usize {
        match (self.bytes.get(offset), self.bytes.get(offset + 1)) {
            (Some(b'\n'), _) => 1,
            (Some(b'\r'), Some(b'\n')) => 2,
            _ => 0,
        }
    }

    fn scan_token(&mut self) -> (TokenKind, bool) {
        let first = self.bytes[self.offset];
        match first {
            b'#' if self.at_line_start => {
                self.scan_directive();
                (TokenKind::Directive, true)
            }
            b'/' if self.peek(1) == Some(b'/') => {
                self.scan_line_comment();
                (TokenKind::LineComment, true)
            }
            b'/' if self.peek(1) == Some(b'*') => (TokenKind::BlockComment, self.scan_block_comment()),
            b'"' => (TokenKind::String, self.scan_quoted(b'"')),
            b'\'' => (TokenKind::Char, self.scan_quoted(b'\'')),
            byte if byte.is_ascii_digit() => {
                self.scan_number();
                (TokenKind::Number, true)
            }
            byte if is_word_start(byte) => {
                while self.peek(0).is_some_and(is_word_continue) {
                    self.offset += 1;
                }
                (TokenKind::Word, true)
            }
            _ => self.scan_punctuation(),
        }
    }

    fn scan_punctuation(&mut self) -> (TokenKind, bool) {
        let rest = &self.text[self.offset..];
        if let Some(punctuation) = PUNCTUATION.iter().find(|punctuation| rest.starts_with(**punctuation)) {
            self.offset += punctuation.len();
            return (TokenKind::Punct, true);
        }
        let character = rest.chars().next().expect("scan_punctuation is only called before a character");
        self.offset += character.len_utf8();
        (TokenKind::Unknown, true)
    }

    /// A directive runs to the end of its logical line, but stops before a
    /// line comment, which becomes a token of its own.
    fn scan_directive(&mut self) {
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\n' => break,
                b'\r' if self.peek(1) == Some(b'\n') => break,
                b'\\' if self.line_break_length(self.offset + 1) > 0 => {
                    self.offset += 1 + self.line_break_length(self.offset + 1);
                }
                b'/' if self.peek(1) == Some(b'/') => break,
                b'/' if self.peek(1) == Some(b'*') => {
                    self.scan_block_comment();
                }
                b'"' | b'\'' => {
                    self.scan_quoted(byte);
                }
                _ => self.offset += 1,
            }
        }
        self.trim_trailing_whitespace();
    }

    /// Gives the whitespace at the end of a directive back to the gap
    /// after it.
    fn trim_trailing_whitespace(&mut self) {
        while self.offset > 0 && matches!(self.bytes[self.offset - 1], b' ' | b'\t' | b'\r') {
            self.offset -= 1;
        }
    }

    /// A line comment ends at the line break, unless a backslash splices
    /// the next line onto it, as in C.
    fn scan_line_comment(&mut self) {
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\n' => break,
                b'\r' if self.peek(1) == Some(b'\n') => break,
                b'\\' if self.line_break_length(self.offset + 1) > 0 => {
                    self.offset += 1 + self.line_break_length(self.offset + 1);
                }
                _ => self.offset += 1,
            }
        }
        self.trim_trailing_whitespace();
    }

    fn scan_block_comment(&mut self) -> bool {
        self.offset += 2;
        match self.text[self.offset..].find("*/") {
            Some(end) => {
                self.offset += end + 2;
                true
            }
            None => {
                self.offset = self.bytes.len();
                false
            }
        }
    }

    /// A string or character literal: up to the closing quote, an escaped
    /// line break continues it, a bare line break ends it unterminated.
    fn scan_quoted(&mut self, quote: u8) -> bool {
        self.offset += 1;
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\\' => {
                    let escaped = self.line_break_length(self.offset + 1).max(1);
                    self.offset = (self.offset + 1 + escaped).min(self.bytes.len());
                    self.align_to_char_boundary();
                }
                b'\n' => return false,
                b'\r' if self.peek(1) == Some(b'\n') => return false,
                byte if byte == quote => {
                    self.offset += 1;
                    return true;
                }
                _ => self.offset += 1,
            }
        }
        false
    }

    /// After skipping an escaped character byte-wise, moves on to the next
    /// character boundary.
    fn align_to_char_boundary(&mut self) {
        while self.offset < self.bytes.len() && !self.text.is_char_boundary(self.offset) {
            self.offset += 1;
        }
    }

    /// Integers in every base, floats with an exponent; digit separators and
    /// suffix letters are part of the number.
    fn scan_number(&mut self) {
        let is_hexadecimal = matches!(self.peek(1), Some(b'x' | b'X')) && self.peek(0) == Some(b'0');
        self.scan_number_part(!is_hexadecimal);
        let has_fraction = self.peek(0) == Some(b'.') && self.peek(1).is_some_and(|byte| byte.is_ascii_digit());
        if has_fraction && !is_hexadecimal {
            self.offset += 1;
            self.scan_number_part(!is_hexadecimal);
        }
    }

    /// Digits and letters, and the sign of an exponent in a decimal number
    /// (in `0x1e-3` the minus is a subtraction).
    fn scan_number_part(&mut self, allows_exponent_sign: bool) {
        while let Some(byte) = self.peek(0) {
            let is_exponent_sign = allows_exponent_sign
                && matches!(byte, b'+' | b'-')
                && matches!(self.bytes[self.offset - 1], b'e' | b'E')
                && self.peek(1).is_some_and(|next| next.is_ascii_digit());
            if !(is_word_continue(byte) || is_exponent_sign) {
                break;
            }
            self.offset += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds_and_texts(text: &str) -> Vec<(TokenKind, &str)> {
        lex(text).into_iter().map(|token| (token.kind, token.text(text))).collect()
    }

    #[test]
    fn punctuation_takes_the_longest_match() {
        let tokens = kinds_and_texts("a>>>=b...c::d");
        let texts: Vec<_> = tokens.iter().map(|(_, text)| *text).collect();
        assert_eq!(texts, ["a", ">>>=", "b", "...", "c", "::", "d"]);
    }

    #[test]
    fn directives_run_to_the_end_of_the_logical_line() {
        let text = "#define A(%1) \\\n  (%1 + 1) // note\nint x;";
        let tokens = kinds_and_texts(text);
        assert_eq!(tokens[0], (TokenKind::Directive, "#define A(%1) \\\n  (%1 + 1)"));
        assert_eq!(tokens[1], (TokenKind::LineComment, "// note"));
        assert_eq!(tokens[2], (TokenKind::Word, "int"));
    }

    #[test]
    fn a_hash_inside_a_line_is_no_directive() {
        let tokens = kinds_and_texts("x # y");
        assert_eq!(tokens[1], (TokenKind::Unknown, "#"));
    }

    #[test]
    fn strings_keep_escapes_and_stop_unterminated_at_the_line_break() {
        let text = "\"a\\\"b\" \"open\nx";
        let tokens = lex(text);
        assert_eq!(tokens[0].text(text), "\"a\\\"b\"");
        assert!(tokens[0].terminated);
        assert_eq!(tokens[1].text(text), "\"open");
        assert!(!tokens[1].terminated);
        assert_eq!(tokens[2].text(text), "x");
    }

    #[test]
    fn numbers_include_fractions_exponents_and_separators() {
        let texts: Vec<_> =
            kinds_and_texts("1.5e-3 0x1F 1_000 2. 1e-3 0x1e-3").into_iter().map(|(_, text)| text).collect();
        assert_eq!(texts, ["1.5e-3", "0x1F", "1_000", "2", ".", "1e-3", "0x1e", "-", "3"]);
    }

    #[test]
    fn comments_and_unterminated_block_comments() {
        let text = "a /* b */ c // d\n/* open";
        let tokens = lex(text);
        assert_eq!(tokens[1].kind, TokenKind::BlockComment);
        assert_eq!(tokens[3].text(text), "// d");
        assert!(!tokens[4].terminated);
    }

    #[test]
    fn unknown_characters_are_single_tokens() {
        let tokens = kinds_and_texts("a ä $");
        assert_eq!(tokens[1], (TokenKind::Unknown, "ä"));
        assert_eq!(tokens[2], (TokenKind::Unknown, "$"));
    }

    #[test]
    fn line_splices_are_whitespace() {
        let tokens = kinds_and_texts("a \\\n b");
        assert_eq!(tokens, [(TokenKind::Word, "a"), (TokenKind::Word, "b")]);
    }
}
