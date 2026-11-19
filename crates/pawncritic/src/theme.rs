// Copyright 2026 davenonymous

//! Theme expressions such as `modern && !layout`, which select policies by
//! their themes.
//!
//! Grammar, loosest first:
//!
//! ```text
//! expression := term (("||" | "or") term)*
//! term       := factor (("&&" | "and") factor)*
//! factor     := ("!" | "not") factor | "(" expression ")" | theme
//! theme      := [a-z0-9_-]+
//! ```

use std::collections::BTreeSet;

/// A parsed theme expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeExpression {
    Theme(String),
    Not(Box<ThemeExpression>),
    And(Box<ThemeExpression>, Box<ThemeExpression>),
    Or(Box<ThemeExpression>, Box<ThemeExpression>),
}

impl ThemeExpression {
    /// Whether a policy with `themes` is selected.
    pub fn matches(&self, themes: &BTreeSet<String>) -> bool {
        match self {
            ThemeExpression::Theme(theme) => themes.contains(theme),
            ThemeExpression::Not(inner) => !inner.matches(themes),
            ThemeExpression::And(left, right) => left.matches(themes) && right.matches(themes),
            ThemeExpression::Or(left, right) => left.matches(themes) || right.matches(themes),
        }
    }

    /// Parses `text`.
    pub fn parse(text: &str) -> Result<ThemeExpression, InvalidThemeExpression> {
        let tokens = tokenize(text)?;
        let mut parser = Parser { tokens: &tokens, position: 0, text };
        let expression = parser.expression()?;
        match parser.tokens.get(parser.position) {
            None => Ok(expression),
            Some(token) => Err(parser.error(&format!("unexpected `{}`", token.text()))),
        }
    }
}

/// Why a text is no theme expression.
#[derive(Debug, thiserror::Error)]
#[error("theme expression `{expression}`: {message}")]
pub struct InvalidThemeExpression {
    pub expression: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    And,
    Or,
    Not,
    Open,
    Close,
}

impl Token {
    fn text(&self) -> &str {
        match self {
            Token::Word(word) => word,
            Token::And => "&&",
            Token::Or => "||",
            Token::Not => "!",
            Token::Open => "(",
            Token::Close => ")",
        }
    }
}

fn tokenize(text: &str) -> Result<Vec<Token>, InvalidThemeExpression> {
    let mut tokens = Vec::new();
    let mut characters = text.char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        match character {
            ' ' | '\t' => {}
            '(' => tokens.push(Token::Open),
            ')' => tokens.push(Token::Close),
            '!' => tokens.push(Token::Not),
            '&' | '|' => {
                if characters.next_if(|&(_, next)| next == character).is_none() {
                    return Err(InvalidThemeExpression {
                        expression: text.to_string(),
                        message: format!("`{character}` at {} must be doubled", offset + 1),
                    });
                }
                tokens.push(if character == '&' { Token::And } else { Token::Or });
            }
            character if is_theme_character(character) => {
                let mut word = character.to_string();
                while let Some((_, next)) = characters.next_if(|&(_, next)| is_theme_character(next)) {
                    word.push(next);
                }
                tokens.push(match word.to_ascii_lowercase().as_str() {
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    _ => Token::Word(word.to_ascii_lowercase()),
                });
            }
            other => {
                return Err(InvalidThemeExpression {
                    expression: text.to_string(),
                    message: format!("unexpected `{other}` at {}", offset + 1),
                });
            }
        }
    }
    Ok(tokens)
}

/// Whether `character` may appear in a theme name.
pub fn is_theme_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '-'
}

/// Checks that `theme` is a theme name: a word of ASCII lower-case
/// letters, digits, `_` and `-`, other than the operators `and`, `or` and
/// `not`.
pub fn check_theme_name(theme: &str) -> Result<(), String> {
    let is_word = !theme.is_empty()
        && theme.chars().all(|character| is_theme_character(character) && !character.is_ascii_uppercase());
    if !is_word || ["and", "or", "not"].contains(&theme) {
        return Err(format!("the theme `{theme}` must be a lowercase word other than and, or and not"));
    }
    Ok(())
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    text: &'a str,
}

impl Parser<'_> {
    fn error(&self, message: &str) -> InvalidThemeExpression {
        InvalidThemeExpression { expression: self.text.to_string(), message: message.to_string() }
    }

    fn accept(&mut self, expected: &Token) -> bool {
        let matches = self.tokens.get(self.position) == Some(expected);
        if matches {
            self.position += 1;
        }
        matches
    }

    fn expression(&mut self) -> Result<ThemeExpression, InvalidThemeExpression> {
        let mut left = self.term()?;
        while self.accept(&Token::Or) {
            left = ThemeExpression::Or(Box::new(left), Box::new(self.term()?));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<ThemeExpression, InvalidThemeExpression> {
        let mut left = self.factor()?;
        while self.accept(&Token::And) {
            left = ThemeExpression::And(Box::new(left), Box::new(self.factor()?));
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<ThemeExpression, InvalidThemeExpression> {
        if self.accept(&Token::Not) {
            return Ok(ThemeExpression::Not(Box::new(self.factor()?)));
        }
        if self.accept(&Token::Open) {
            let inner = self.expression()?;
            if !self.accept(&Token::Close) {
                return Err(self.error("missing `)`"));
            }
            return Ok(inner);
        }
        match self.tokens.get(self.position) {
            Some(Token::Word(word)) => {
                self.position += 1;
                Ok(ThemeExpression::Theme(word.clone()))
            }
            Some(token) => Err(self.error(&format!("expected a theme, found `{}`", token.text()))),
            None => Err(self.error("expected a theme at the end")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn themes(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn operators_bind_as_usual() {
        let expression = ThemeExpression::parse("modern || layout && !tidy").unwrap();
        assert!(expression.matches(&themes(&["modern"])));
        assert!(expression.matches(&themes(&["layout"])));
        assert!(!expression.matches(&themes(&["layout", "tidy"])));
    }

    #[test]
    fn words_and_parentheses() {
        let expression = ThemeExpression::parse("(core or naming) and not Bugs").unwrap();
        assert!(expression.matches(&themes(&["core"])));
        assert!(!expression.matches(&themes(&["core", "bugs"])));
    }

    #[test]
    fn errors_say_what_is_wrong() {
        assert!(ThemeExpression::parse("a &").unwrap_err().message.contains("doubled"));
        assert!(ThemeExpression::parse("(a").unwrap_err().message.contains("missing"));
        assert!(ThemeExpression::parse("a b").unwrap_err().message.contains("unexpected"));
        assert!(ThemeExpression::parse("").is_err());
    }
}
