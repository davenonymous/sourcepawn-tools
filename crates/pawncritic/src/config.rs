// Copyright 2026 davenonymous

//! The configuration file, `.pawncritic.toml`.
//!
//! The nearest one above a checked file applies to it. `docs/pawncritic.md`
//! documents every key.

use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};

use pawn_core::cli::{ConfigError, ConfigFile};
use pawn_core::discover::normalize_lexically;
use serde::Deserialize;

use crate::severity::Severity;
use crate::theme::{ThemeExpression, check_theme_name};

/// A severity in a configuration file: a number or a name.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawSeverity {
    Level(i64),
    Name(String),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimits {
    timeout_ms: Option<u64>,
    memory_mb: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    severity: Option<RawSeverity>,
    theme: Option<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    only: bool,
    #[serde(default)]
    force: bool,
    #[serde(default)]
    policy_paths: Vec<PathBuf>,
    #[serde(default)]
    limits: RawLimits,
    #[serde(default)]
    policies: BTreeMap<String, toml::Table>,
}

/// The settings of one policy.
#[derive(Debug, Clone, Default)]
pub struct PolicyConfig {
    pub severity: Option<Severity>,
    pub enabled: Option<bool>,
    pub add_themes: Vec<String>,
    /// The policy's own options, checked against its declaration when the
    /// policy is loaded.
    pub options: BTreeMap<String, toml::Value>,
}

/// The limits of Lua policies; zero is no valid limit.
#[derive(Debug, Clone, Copy, Default)]
pub struct LimitsConfig {
    pub timeout_ms: Option<NonZeroU64>,
    pub memory_mb: Option<NonZeroU64>,
}

/// A parsed configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub severity: Option<Severity>,
    pub theme: Option<ThemeExpression>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub only: bool,
    pub force: bool,
    /// Directories of custom policies, relative paths resolved against the
    /// directory of the configuration file and normalized lexically, so
    /// configurations that name one directory by different paths share
    /// it.
    pub policy_paths: Vec<PathBuf>,
    pub limits: LimitsConfig,
    pub policies: BTreeMap<String, PolicyConfig>,
}

impl ConfigFile for Config {
    const FILE_NAMES: &'static [&'static str] = &[".pawncritic.toml", "pawncritic.toml"];

    fn read(path: &Path) -> Result<Config, ConfigError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| ConfigError::Io { path: path.to_path_buf(), source })?;
        Config::parse(&text, path)
    }
}

impl Config {
    /// Parses `text`, the content of the configuration file at `path`.
    pub fn parse(text: &str, path: &Path) -> Result<Config, ConfigError> {
        let invalid = |message: String| ConfigError::Invalid { path: path.to_path_buf(), message };
        let raw: RawConfig = toml::from_str(text).map_err(|error| invalid(error.to_string().trim_end().to_string()))?;
        let base = path.parent().unwrap_or(Path::new("."));
        let severity = raw.severity.map(|severity| parse_severity(severity).map_err(&invalid)).transpose()?;
        let theme = raw
            .theme
            .map(|theme| ThemeExpression::parse(&theme).map_err(|error| invalid(error.to_string())))
            .transpose()?;
        let mut policies = BTreeMap::new();
        for (name, table) in raw.policies {
            let parsed =
                parse_policy_table(table).map_err(|message| invalid(format!("[policies.\"{name}\"]: {message}")))?;
            policies.insert(name, parsed);
        }
        Ok(Config {
            severity,
            theme,
            include: raw.include,
            exclude: raw.exclude,
            only: raw.only,
            force: raw.force,
            policy_paths: raw.policy_paths.iter().map(|directory| normalize_lexically(&base.join(directory))).collect(),
            limits: parse_limits(&raw.limits).map_err(invalid)?,
            policies,
        })
    }
}

fn parse_limits(raw: &RawLimits) -> Result<LimitsConfig, String> {
    let positive = |key: &str, value: Option<u64>| -> Result<Option<NonZeroU64>, String> {
        value
            .map(|number| NonZeroU64::new(number).ok_or_else(|| format!("[limits]: `{key}` must be positive, not 0")))
            .transpose()
    };
    Ok(LimitsConfig {
        timeout_ms: positive("timeout_ms", raw.timeout_ms)?,
        memory_mb: positive("memory_mb", raw.memory_mb)?,
    })
}

fn parse_severity(raw: RawSeverity) -> Result<Severity, String> {
    match raw {
        RawSeverity::Level(level) => level.to_string().parse(),
        RawSeverity::Name(name) => name.parse(),
    }
    .map_err(|error: crate::severity::InvalidSeverity| format!("severity: {error}"))
}

fn parse_policy_table(table: toml::Table) -> Result<PolicyConfig, String> {
    let mut config = PolicyConfig::default();
    for (key, value) in table {
        match key.as_str() {
            "severity" => {
                let raw = match value {
                    toml::Value::Integer(level) => RawSeverity::Level(level),
                    toml::Value::String(name) => RawSeverity::Name(name),
                    _ => return Err("`severity` must be a number from 1 to 5 or a name".to_string()),
                };
                config.severity = Some(parse_severity(raw)?);
            }
            "enabled" => config.enabled = Some(value.as_bool().ok_or("`enabled` must be true or false")?),
            "add_themes" => {
                let themes = value.as_array().ok_or("`add_themes` must be a list of themes")?;
                for theme in themes {
                    let theme = theme.as_str().ok_or("`add_themes` must be a list of themes")?.to_ascii_lowercase();
                    check_theme_name(&theme).map_err(|problem| format!("`add_themes`: {problem}"))?;
                    config.add_themes.push(theme);
                }
            }
            _ => {
                config.options.insert(key, value);
            }
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_configuration_parses() {
        let text = r#"
            severity = "harsh"
            theme = "modern || philosophy"
            include = ["Bugs::"]
            exclude = ["NamingConventions"]
            policy_paths = ["critic", "../shared/./critic"]
            [limits]
            timeout_ms = 1000
            [policies."CodeLayout::ProhibitLongLines"]
            severity = 2
            max_length = 100
            [policies."Modern::ProhibitCloseHandle"]
            enabled = false
            add_themes = ["Extra"]
        "#;
        let config = Config::parse(text, Path::new("/project/.pawncritic.toml")).unwrap();
        assert_eq!(config.severity, Some(Severity::new(3).unwrap()));
        assert_eq!(config.policy_paths, [PathBuf::from("/project/critic"), PathBuf::from("/shared/critic")]);
        assert_eq!(config.limits.timeout_ms, NonZeroU64::new(1000));
        let long_lines = &config.policies["CodeLayout::ProhibitLongLines"];
        assert_eq!(long_lines.severity, Some(Severity::new(2).unwrap()));
        assert_eq!(long_lines.options["max_length"], toml::Value::Integer(100));
        let close_handle = &config.policies["Modern::ProhibitCloseHandle"];
        assert_eq!(close_handle.enabled, Some(false));
        assert_eq!(close_handle.add_themes, ["extra"]);
    }

    #[test]
    fn mistakes_are_reported_with_the_file() {
        let unknown = Config::parse("severty = 3", Path::new("x.toml")).unwrap_err().to_string();
        assert!(unknown.starts_with("x.toml:") && unknown.contains("severty"), "{unknown}");
        let bad_severity = Config::parse("severity = 9", Path::new("x.toml")).unwrap_err().to_string();
        assert!(bad_severity.contains("no severity"), "{bad_severity}");
        let bad_enabled =
            Config::parse("[policies.\"A::B\"]\nenabled = 1", Path::new("x.toml")).unwrap_err().to_string();
        assert!(bad_enabled.contains("`enabled` must be true or false"), "{bad_enabled}");
        let bad_theme =
            Config::parse("[policies.\"A::B\"]\nadd_themes = [\"a b\"]", Path::new("x.toml")).unwrap_err().to_string();
        assert_eq!(
            bad_theme,
            "x.toml: [policies.\"A::B\"]: `add_themes`: the theme `a b` must be a lowercase word other than and, or and not"
        );
        for key in ["timeout_ms", "memory_mb"] {
            let zero = Config::parse(&format!("[limits]\n{key} = 0"), Path::new("x.toml")).unwrap_err().to_string();
            assert_eq!(zero, format!("x.toml: [limits]: `{key}` must be positive, not 0"));
        }
    }
}
