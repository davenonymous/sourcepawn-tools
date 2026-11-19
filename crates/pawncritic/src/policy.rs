// Copyright 2026 davenonymous

//! What pawncritic knows about a policy before it runs it: its name,
//! severity, themes, documentation and options.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::LazyLock;

use pawn_core::FileKind;
use regex::Regex;

use crate::severity::Severity;

/// Where a policy comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyOrigin {
    /// A Lua policy that ships with pawncritic.
    Builtin(&'static str),
    /// A Lua policy from a policy directory.
    File(PathBuf),
    /// A policy written in Rust, part of pawncritic itself.
    Native,
}

impl fmt::Display for PolicyOrigin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyOrigin::Builtin(file) => write!(formatter, "built-in policy {file}"),
            PolicyOrigin::File(path) => write!(formatter, "{}", path.display()),
            PolicyOrigin::Native => write!(formatter, "built-in native policy"),
        }
    }
}

/// The type of a policy option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionType {
    String,
    Integer,
    Number,
    Boolean,
    /// A list of strings.
    List,
}

impl OptionType {
    pub fn parse(name: &str) -> Option<OptionType> {
        Some(match name {
            "string" => OptionType::String,
            "integer" => OptionType::Integer,
            "number" => OptionType::Number,
            "boolean" => OptionType::Boolean,
            "list" => OptionType::List,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            OptionType::String => "string",
            OptionType::Integer => "integer",
            OptionType::Number => "number",
            OptionType::Boolean => "boolean",
            OptionType::List => "list",
        }
    }
}

/// The value of a policy option.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    List(Vec<String>),
}

impl OptionValue {
    pub fn option_type(&self) -> OptionType {
        match self {
            OptionValue::String(_) => OptionType::String,
            OptionValue::Integer(_) => OptionType::Integer,
            OptionValue::Number(_) => OptionType::Number,
            OptionValue::Boolean(_) => OptionType::Boolean,
            OptionValue::List(_) => OptionType::List,
        }
    }

    /// Reads a value of type `expected` from a configuration file.
    pub fn from_toml(value: &toml::Value, expected: OptionType) -> Result<OptionValue, String> {
        let parsed = match (expected, value) {
            (OptionType::String, toml::Value::String(text)) => OptionValue::String(text.clone()),
            (OptionType::Integer, toml::Value::Integer(number)) => OptionValue::Integer(*number),
            (OptionType::Number, toml::Value::Integer(number)) => OptionValue::Number(*number as f64),
            (OptionType::Number, toml::Value::Float(number)) => OptionValue::Number(*number),
            (OptionType::Boolean, toml::Value::Boolean(flag)) => OptionValue::Boolean(*flag),
            (OptionType::List, toml::Value::Array(items)) => {
                let strings: Option<Vec<String>> = items.iter().map(|item| item.as_str().map(str::to_string)).collect();
                OptionValue::List(strings.ok_or("must be a list of strings")?)
            }
            _ => return Err(format!("must be {}", describe_type(expected))),
        };
        Ok(parsed)
    }
}

impl OptionValue {
    /// The value as a configuration file writes it, on one line, which
    /// [`OptionValue::from_toml`] reads back.
    pub fn to_toml(&self) -> String {
        match self {
            OptionValue::String(text) => toml_basic_string(text),
            OptionValue::Integer(number) => number.to_string(),
            OptionValue::Number(number) => toml::Value::Float(*number).to_string(),
            OptionValue::Boolean(flag) => flag.to_string(),
            OptionValue::List(items) => {
                let quoted: Vec<String> = items.iter().map(|item| toml_basic_string(item)).collect();
                format!("[{}]", quoted.join(", "))
            }
        }
    }
}

/// `text` as a TOML basic string: in double quotes, with `"`, `\` and
/// every control character escaped, so it never spans lines.
fn toml_basic_string(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            '\r' => quoted.push_str("\\r"),
            control if control.is_control() => quoted.push_str(&format!("\\u{:04X}", u32::from(control))),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

impl fmt::Display for OptionValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionValue::String(text) => write!(formatter, "{text:?}"),
            OptionValue::Integer(number) => write!(formatter, "{number}"),
            OptionValue::Number(number) => write!(formatter, "{number}"),
            OptionValue::Boolean(flag) => write!(formatter, "{flag}"),
            OptionValue::List(items) => {
                let quoted: Vec<String> = items.iter().map(|item| format!("{item:?}")).collect();
                write!(formatter, "[{}]", quoted.join(", "))
            }
        }
    }
}

/// `word` after the indefinite article it takes: `a string`, `an
/// integer`. Made for the names of Lua and option types, which all sound
/// as they are spelled except `userdata`, whose `u` takes `a` too.
pub fn with_article(word: &str) -> String {
    let starts_with_vowel_sound = word.starts_with(['a', 'e', 'i', 'o']);
    let article = if starts_with_vowel_sound { "an" } else { "a" };
    format!("{article} {word}")
}

fn describe_type(option_type: OptionType) -> &'static str {
    match option_type {
        OptionType::String => "a string",
        OptionType::Integer => "an integer",
        OptionType::Number => "a number",
        OptionType::Boolean => "true or false",
        OptionType::List => "a list of strings",
    }
}

/// A declared option of a policy.
#[derive(Debug, Clone)]
pub struct OptionSpec {
    pub name: String,
    pub default: OptionValue,
    pub description: String,
}

/// Everything about a policy except its code.
#[derive(Debug, Clone)]
pub struct PolicyInfo {
    /// `Category::Name`, such as `Modern::ProhibitCloseHandle`.
    pub name: String,
    pub severity: Severity,
    pub themes: BTreeSet<String>,
    /// One line; also the message of a violation that brings none.
    pub summary: String,
    /// The longer explanation `--explain` prints.
    pub description: String,
    pub applies_to: BTreeSet<FileKindName>,
    pub options: Vec<OptionSpec>,
    pub origin: PolicyOrigin,
}

/// A file kind as a policy names it, ordered for stable output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileKindName {
    Sp,
    Inc,
}

impl FileKindName {
    pub fn parse(name: &str) -> Option<FileKindName> {
        match name {
            "sp" => Some(FileKindName::Sp),
            "inc" => Some(FileKindName::Inc),
            _ => None,
        }
    }

    pub fn of(kind: FileKind) -> FileKindName {
        match kind {
            FileKind::Plugin => FileKindName::Sp,
            FileKind::Include => FileKindName::Inc,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FileKindName::Sp => "sp",
            FileKindName::Inc => "inc",
        }
    }
}

static POLICY_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Z][A-Za-z0-9]*(::[A-Z][A-Za-z0-9]*)+$").expect("the policy name pattern is valid")
});

/// Whether `name` has the `Category::Name` shape policy names must have.
pub fn is_valid_policy_name(name: &str) -> bool {
    POLICY_NAME.is_match(name)
}

impl PolicyInfo {
    pub fn applies_to(&self, kind: FileKind) -> bool {
        self.applies_to.contains(&FileKindName::of(kind))
    }

    pub fn option(&self, name: &str) -> Option<&OptionSpec> {
        self.options.iter().find(|option| option.name == name)
    }

    /// The options with their defaults, overridden by `configured`, which
    /// must name declared options with values of their types.
    pub fn resolve_options(
        &self,
        configured: &BTreeMap<String, toml::Value>,
    ) -> Result<BTreeMap<String, OptionValue>, String> {
        let mut values: BTreeMap<String, OptionValue> =
            self.options.iter().map(|option| (option.name.clone(), option.default.clone())).collect();
        for (name, value) in configured {
            let Some(spec) = self.option(name) else {
                let known: Vec<&str> = self.options.iter().map(|option| option.name.as_str()).collect();
                let hint = if known.is_empty() {
                    "it has no options".to_string()
                } else {
                    format!("it has {}", known.join(", "))
                };
                return Err(format!("policy {} has no option `{name}`; {hint}", self.name));
            };
            let parsed = OptionValue::from_toml(value, spec.default.option_type())
                .map_err(|problem| format!("option `{name}` of policy {} {problem}", self.name))?;
            values.insert(name.clone(), parsed);
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_names_need_a_category() {
        assert!(is_valid_policy_name("Modern::ProhibitCloseHandle"));
        assert!(is_valid_policy_name("A::B::C"));
        assert!(!is_valid_policy_name("ProhibitCloseHandle"));
        assert!(!is_valid_policy_name("modern::Prohibit"));
        assert!(!is_valid_policy_name("Modern::"));
    }

    #[test]
    fn option_values_survive_a_round_trip_through_toml() {
        let values = [
            OptionValue::String("quote \" and \\ and \u{e4}".to_string()),
            OptionValue::String("\nforce = true\r\n#\u{7f}\u{85}\t".to_string()),
            OptionValue::Integer(-7),
            OptionValue::Number(1.0),
            OptionValue::Number(0.25),
            OptionValue::Boolean(false),
            OptionValue::List(vec!["a".to_string(), "b c\nd".to_string()]),
            OptionValue::List(Vec::new()),
        ];
        for value in values {
            assert!(!value.to_toml().contains(['\n', '\r']), "{}", value.to_toml());
            let table: toml::Table = toml::from_str(&format!("key = {}", value.to_toml())).unwrap();
            assert_eq!(OptionValue::from_toml(&table["key"], value.option_type()), Ok(value.clone()), "{value}");
        }
    }

    #[test]
    fn type_names_take_their_article() {
        let named: Vec<String> = ["integer", "error", "string", "userdata", "nil"].map(with_article).into();
        assert_eq!(named, ["an integer", "an error", "a string", "a userdata", "a nil"]);
    }

    #[test]
    fn options_are_type_checked() {
        let info = PolicyInfo {
            name: "A::B".to_string(),
            severity: Severity::HIGHEST,
            themes: BTreeSet::new(),
            summary: String::new(),
            description: String::new(),
            applies_to: BTreeSet::from([FileKindName::Sp]),
            options: vec![OptionSpec {
                name: "max".to_string(),
                default: OptionValue::Integer(3),
                description: String::new(),
            }],
            origin: PolicyOrigin::Native,
        };
        let good = BTreeMap::from([("max".to_string(), toml::Value::Integer(5))]);
        assert_eq!(info.resolve_options(&good).unwrap()["max"], OptionValue::Integer(5));
        let wrong_type = BTreeMap::from([("max".to_string(), toml::Value::String("5".to_string()))]);
        assert!(info.resolve_options(&wrong_type).unwrap_err().contains("must be an integer"));
        let unknown = BTreeMap::from([("min".to_string(), toml::Value::Integer(5))]);
        assert!(info.resolve_options(&unknown).unwrap_err().contains("has no option `min`"));
    }
}
