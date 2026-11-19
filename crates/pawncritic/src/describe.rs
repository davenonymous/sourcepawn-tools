// Copyright 2026 davenonymous

//! The texts that describe the policies of a profile: the list, the
//! themes, the explanation of a policy and the configuration template.

use std::collections::BTreeSet;

use pawn_core::terminal::printable;
use regex::Regex;

use crate::engine::{Catalog, EffectivePolicy, Profile, policy_pattern};
use crate::policy::{PolicyInfo, PolicyOrigin};

/// Every policy of the catalog, one per line, with its severity and
/// themes as configured; the chosen ones marked with `*`.
pub fn list(profile: &Profile) -> String {
    let chosen = chosen_names(profile);
    let entries = profile.catalog.entries();
    let width = entries.iter().map(|entry| entry.info.name.len()).max().unwrap_or(0);
    let mut text = String::new();
    for entry in entries {
        let info = effective_info(profile, &entry.info);
        let themes: Vec<&str> = info.themes.iter().map(String::as_str).collect();
        let marker = if chosen.contains(info.name.as_str()) { "*" } else { " " };
        text.push_str(&format!("{marker} {} {:<width$}  [{}]\n", info.severity, info.name, themes.join(" ")));
    }
    text
}

/// The policy `info` with the severity and themes the profile's
/// configuration gives it.
fn effective_info<'a>(profile: &'a Profile, info: &'a PolicyInfo) -> &'a PolicyInfo {
    profile.effective.get(&info.name).map_or(info, |policy| &policy.info)
}

/// The themes of every policy of the catalog, one per line, sorted.
pub fn themes(catalog: &Catalog) -> String {
    let themes: BTreeSet<&str> =
        catalog.entries().iter().flat_map(|entry| entry.info.themes.iter().map(String::as_str)).collect();
    themes.into_iter().map(|theme| format!("{theme}\n")).collect()
}

/// The explanation of every policy named `name`, with or without its
/// category, ignoring case; an error when no policy has the name.
pub fn explain(profile: &Profile, name: &str) -> Result<String, String> {
    let pattern = named(name)?;
    let matching: Vec<&PolicyInfo> =
        profile.catalog.entries().iter().map(|entry| &entry.info).filter(|info| pattern.is_match(&info.name)).collect();
    if matching.is_empty() {
        return Err(format!("no policy is named {name}; see --list"));
    }
    let explanations: Vec<String> =
        matching.into_iter().map(|info| explanation(info, profile.effective.get(&info.name))).collect();
    Ok(explanations.join("\n"))
}

/// The pattern of a policy name as `--explain` takes it.
fn named(name: &str) -> Result<Regex, String> {
    policy_pattern(&format!("^(?:.*::)?{}$", regex::escape(name)))
}

fn chosen_names(profile: &Profile) -> BTreeSet<&str> {
    profile.policies.iter().map(|policy| policy.info.name.as_str()).collect()
}

/// The explanation of the catalog's policy `info`, with the severity,
/// themes and options of `effective` when the configuration sets it up.
fn explanation(info: &PolicyInfo, effective: Option<&EffectivePolicy>) -> String {
    let mut text = String::new();
    text.push_str(&format!("{}\n", info.name));
    text.push_str(&format!("  {}\n\n", printable(&info.summary)));
    let severity = effective.map_or(info.severity, |policy| policy.info.severity);
    text.push_str(&format!("Severity: {} ({})\n", severity, severity.name()));
    let themes: Vec<&str> =
        effective.map_or(&info.themes, |policy| &policy.info.themes).iter().map(String::as_str).collect();
    text.push_str(&format!("Themes:   {}\n", themes.join(", ")));
    let kinds: Vec<&str> = info.applies_to.iter().map(|kind| kind.name()).collect();
    text.push_str(&format!("Files:    {}\n", kinds.join(", ")));
    let origin = match &info.origin {
        PolicyOrigin::File(path) => path.display().to_string(),
        other => other.to_string(),
    };
    text.push_str(&format!("Source:   {}\n", printable(&origin)));
    if !info.description.is_empty() {
        text.push('\n');
        for line in info.description.lines() {
            text.push_str(&format!("  {}\n", printable(line)).replace("  \n", "\n"));
        }
    }
    if !info.options.is_empty() {
        text.push_str("\nOptions:\n");
        for option in &info.options {
            let current =
                effective.and_then(|policy| policy.options.get(&option.name)).filter(|value| **value != option.default);
            let configured = current.map(|value| format!(", configured: {value}")).unwrap_or_default();
            text.push_str(&format!(
                "  {} ({}, default {}{configured})\n",
                option.name,
                option.default.option_type().name(),
                option.default
            ));
            if !option.description.is_empty() {
                text.push_str(&format!("      {}\n", printable(&option.description)));
            }
        }
    }
    text
}

/// A `.pawncritic.toml` with every setting, policy and option of the
/// catalog commented out at its default.
pub fn config_template(catalog: &Catalog) -> String {
    let mut text = String::from(
        "# .pawncritic.toml: the nearest one above a file applies to it.\n\
         # Every setting below shows its default; uncomment to change it.\n\n\
         # severity = 3                  # report from this severity up: 1 to 5 or a name\n\
         # theme is unset: policies of every theme run. To run only those whose themes match:\n\
         # theme = \"modern || philosophy\"\n\
         # include = []                  # always run policies matching these patterns\n\
         # exclude = []                  # never run policies matching these patterns\n\
         # only = false                  # run only the policies configured below\n\
         # force = false                 # ignore no critic annotations\n\
         # policy_paths = []             # directories of custom policies, relative to this file\n\n\
         # [limits]\n\
         # timeout_ms = 5000             # time a Lua policy may spend on one file\n\
         # memory_mb = 128               # memory a Lua policy may hold\n",
    );
    for entry in catalog.entries() {
        let info = &entry.info;
        text.push_str(&format!(
            "\n# {}\n# [policies.\"{}\"]\n# enabled = true\n# severity = {}\n# add_themes = []\n",
            printable(&info.summary),
            info.name,
            info.severity
        ));
        for option in &info.options {
            text.push_str(&format!("# {} = {}", option.name, option.default.to_toml()));
            if !option.description.is_empty() {
                text.push_str(&format!("    # {}", printable(&option.description)));
            }
            text.push('\n');
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::engine::Selection;
    use pawn_core::cli::Configured;
    use std::path::Path;
    use std::sync::Arc;

    fn profile(config: &str) -> Profile {
        profile_with(config, &Selection::default())
    }

    fn profile_with(config: &str, selection: &Selection) -> Profile {
        let config = Config::parse(config, Path::new("/p/.pawncritic.toml")).unwrap();
        let configured = Configured { path: None, config: Arc::new(config) };
        let (profile, problems) = Profile::build(&configured, selection);
        assert!(problems.is_empty(), "{problems:?}");
        profile
    }

    #[test]
    fn a_policy_is_explained_by_its_short_name_as_configured() {
        let text = explain(
            &profile("[policies.\"Complexity::ProhibitLongFunctions\"]\nmax_lines = 40"),
            "prohibitlongfunctions",
        )
        .unwrap();
        assert!(text.starts_with("Complexity::ProhibitLongFunctions\n  "), "{text}");
        assert!(text.contains("\nSeverity: 3 (harsh)\n"), "{text}");
        assert!(text.contains("\nSource:   built-in policy Complexity/ProhibitLongFunctions.lua\n"), "{text}");
        assert!(text.contains("\nOptions:\n  max_lines (integer, default "), "{text}");
        assert!(text.contains(", configured: 40)\n"), "{text}");
        let not_chosen = profile(
            "severity = 5\n[policies.\"Complexity::ProhibitLongFunctions\"]\nseverity = 2\nmax_lines = 40\nadd_themes = [\"review\"]",
        );
        let text = explain(&not_chosen, "ProhibitLongFunctions").unwrap();
        assert!(text.contains("\nSeverity: 2 (cruel)\n") && text.contains(", configured: 40)\n"), "{text}");
        assert!(text.contains("\nThemes:   complexity, core, philosophy, review\n"), "{text}");
        let listed = list(&not_chosen);
        assert!(listed.lines().any(|line| line.starts_with("  2 Complexity::ProhibitLongFunctions ")), "{listed}");
        assert!(listed.contains(" [complexity core philosophy review]\n"), "{listed}");
        let unknown = explain(&profile(""), "NoSuchPolicy").unwrap_err();
        assert_eq!(unknown, "no policy is named NoSuchPolicy; see --list");
    }

    /// A line of the template that is a setting once its `# ` goes: a
    /// table header or `key = value`.
    fn is_setting(line: &str) -> bool {
        line.starts_with('[')
            || line.split_once(" = ").is_some_and(|(key, _)| {
                !key.is_empty() && key.chars().all(|character| character.is_ascii_lowercase() || character == '_')
            })
    }

    #[test]
    fn the_template_uncommented_configures_every_default() {
        let catalog = profile("").catalog;
        let text = config_template(&catalog);
        assert!(text.contains("\n# [policies.\"Modern::ProhibitCloseHandle\"]\n# enabled = true\n"), "{text}");
        let uncommented: String = text
            .lines()
            .filter_map(|line| line.strip_prefix("# "))
            .filter(|line| is_setting(line))
            .map(|line| format!("{line}\n"))
            .collect();

        let every_policy = Selection { single_policy: Some(policy_pattern("").unwrap()), ..Selection::default() };

        let profile = profile_with(&uncommented, &every_policy);

        assert!(uncommented.starts_with("severity = 3 "), "{uncommented}");
        assert!(uncommented.contains("\n[limits]\ntimeout_ms = 5000 "), "{uncommented}");
        assert_eq!(profile.policies.len(), catalog.entries().len(), "every policy is configured and runs");
        for policy in &profile.policies {
            for option in &policy.info.options {
                assert_eq!(policy.options[&option.name], option.default, "{} {}", policy.info.name, option.name);
            }
        }
        assert!(themes(&catalog).lines().any(|theme| theme == "modern"));
    }
}
