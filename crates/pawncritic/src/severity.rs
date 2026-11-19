// Copyright 2026 davenonymous

//! Severities, as in perlcritic: 5 is the most severe, 1 the least. A run
//! at severity N reports every policy of severity N and above.

use std::fmt;
use std::str::FromStr;

/// A severity from 1 to 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Severity(u8);

/// The perlcritic names of the severities, from 1 to 5.
const NAMES: [&str; 5] = ["brutal", "cruel", "harsh", "stern", "gentle"];

impl Severity {
    pub const LOWEST: Severity = Severity(1);
    pub const HIGHEST: Severity = Severity(5);

    /// The severity `level`, if it lies between 1 and 5.
    pub const fn new(level: i64) -> Option<Severity> {
        if level >= 1 && level <= 5 { Some(Severity(level as u8)) } else { None }
    }

    pub fn level(self) -> u8 {
        self.0
    }

    /// The perlcritic name of the run that starts reporting at this
    /// severity: `gentle` for 5 down to `brutal` for 1.
    pub fn name(self) -> &'static str {
        NAMES[usize::from(self.0) - 1]
    }

    /// The diagnostic level tools such as editors understand.
    pub fn diagnostic_level(self) -> DiagnosticLevel {
        match self.0 {
            5 => DiagnosticLevel::Error,
            3 | 4 => DiagnosticLevel::Warning,
            2 => DiagnosticLevel::Info,
            _ => DiagnosticLevel::Hint,
        }
    }
}

/// How severe a violation is in the vocabulary of compilers and editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticLevel {
    pub fn name(self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Info => "info",
            DiagnosticLevel::Hint => "hint",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Why a text is no severity.
#[derive(Debug, thiserror::Error)]
#[error("`{0}` is no severity: use 1 to 5 or gentle, stern, harsh, cruel or brutal")]
pub struct InvalidSeverity(pub String);

impl FromStr for Severity {
    type Err = InvalidSeverity;

    /// Parses `1` to `5` or a perlcritic name such as `harsh`.
    fn from_str(text: &str) -> Result<Severity, InvalidSeverity> {
        let trimmed = text.trim();
        if let Ok(level) = trimmed.parse::<i64>() {
            return Severity::new(level).ok_or_else(|| InvalidSeverity(text.to_string()));
        }
        NAMES
            .iter()
            .position(|name| name.eq_ignore_ascii_case(trimmed))
            .map(|index| Severity(index as u8 + 1))
            .ok_or_else(|| InvalidSeverity(text.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severities_parse_from_numbers_and_names() {
        assert_eq!("3".parse::<Severity>().unwrap(), Severity(3));
        assert_eq!("Gentle".parse::<Severity>().unwrap(), Severity::HIGHEST);
        assert_eq!("brutal".parse::<Severity>().unwrap(), Severity::LOWEST);
        assert!("0".parse::<Severity>().is_err());
        assert!("mild".parse::<Severity>().is_err());
    }

    #[test]
    fn names_follow_perlcritic() {
        assert_eq!(Severity(4).name(), "stern");
        assert_eq!(Severity(5).diagnostic_level(), DiagnosticLevel::Error);
        assert_eq!(Severity(1).diagnostic_level(), DiagnosticLevel::Hint);
    }
}
