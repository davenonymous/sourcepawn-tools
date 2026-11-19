// Copyright 2026 davenonymous

//! Runs every policy fixture in `tests/policies/`. A fixture starts with
//! header comments:
//!
//! ```text
//! // policy: Modern::ProhibitCloseHandle
//! // options: check_includes = true      (TOML, optional, repeatable)
//! // expect: 5 9 9                       (the lines reported, in order)
//! ```
//!
//! and runs that one policy alone, with `no critic` annotations honored.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pawn_core::cli::{Configured, Input};
use pawncritic::config::{Config, PolicyConfig};
use pawncritic::engine::{Catalog, Selection, policy_pattern};
use pawncritic::lua::sandbox::Limits;
use pawncritic::run::Plan;

struct Fixture {
    path: PathBuf,
    policy: String,
    options: BTreeMap<String, toml::Value>,
    expected_lines: Vec<usize>,
}

fn fixture_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).expect("the fixture directory exists") {
        let path = entry.expect("the fixture directory is readable").path();
        if path.is_dir() {
            files.extend(fixture_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "sp" || extension == "inc") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn read_fixture(path: &Path) -> Fixture {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut policy = None;
    let mut options = String::new();
    let mut expected_lines = None;
    let without_bom = text.trim_start_matches('\u{feff}');
    for line in without_bom.lines().take_while(|line| line.starts_with("//")) {
        let header = line.trim_start_matches('/').trim();
        if let Some(name) = header.strip_prefix("policy:") {
            policy = Some(name.trim().to_string());
        } else if let Some(option) = header.strip_prefix("options:") {
            options.push_str(option.trim());
            options.push('\n');
        } else if let Some(lines) = header.strip_prefix("expect:") {
            expected_lines = Some(
                lines.split_whitespace().map(|number| number.parse().expect("expected lines are numbers")).collect(),
            );
        }
    }
    let options: toml::Table =
        toml::from_str(&options).unwrap_or_else(|error| panic!("{}: options: {error}", path.display()));
    Fixture {
        path: path.to_path_buf(),
        policy: policy.unwrap_or_else(|| panic!("{}: no `// policy:` header", path.display())),
        options: options.into_iter().collect(),
        expected_lines: expected_lines.unwrap_or_else(|| panic!("{}: no `// expect:` header", path.display())),
    }
}

fn reported_lines(fixture: &Fixture) -> Vec<usize> {
    let mut config = Config::default();
    config
        .policies
        .insert(fixture.policy.clone(), PolicyConfig { options: fixture.options.clone(), ..PolicyConfig::default() });
    let selection = Selection {
        single_policy: Some(policy_pattern(&format!("^{}$", regex::escape(&fixture.policy))).unwrap()),
        ..Selection::default()
    };
    let configured = Configured { path: None, config: Arc::new(config) };
    let input = Input { path: fixture.path.clone(), content: None };
    let plan = Plan::for_config(vec![input], &configured, &selection);
    assert!(plan.problems().is_empty(), "{}: {:?}", fixture.path.display(), plan.problems());
    let result = plan.run(1);
    assert_eq!(result.policies.len(), 1, "{}: the policy {} is unknown", fixture.path.display(), fixture.policy);
    let report = &result.reports[0];
    assert!(report.failures.is_empty(), "{}: {:?}", fixture.path.display(), report.failures);
    report.violations.iter().map(|violation| violation.start.line).collect()
}

fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/policies")
}

#[test]
fn every_fixture_reports_the_expected_lines() {
    let mut failures = Vec::new();
    for path in fixture_files(&fixture_directory()) {
        let fixture = read_fixture(&path);
        let reported = reported_lines(&fixture);
        if reported != fixture.expected_lines {
            failures.push(format!(
                "{}: expected lines {:?}, reported {reported:?}",
                path.display(),
                fixture.expected_lines
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_built_in_policy_has_a_fixture() {
    let covered: BTreeSet<String> =
        fixture_files(&fixture_directory()).iter().map(|path| read_fixture(path).policy).collect();
    let (catalog, problems) = Catalog::load(&[], Limits::default());
    assert!(problems.is_empty(), "{problems:?}");
    let missing: Vec<&str> = catalog
        .entries()
        .iter()
        .map(|entry| entry.info.name.as_str())
        .filter(|name| !covered.contains(*name))
        .collect();
    assert!(missing.is_empty(), "policies without a fixture: {missing:?}");
}

#[test]
fn every_built_in_policy_is_documented() {
    let documentation = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/pawncritic.md");
    let text = std::fs::read_to_string(&documentation).expect("docs/pawncritic.md exists");
    let (catalog, _) = Catalog::load(&[], Limits::default());
    let missing: Vec<&str> = catalog
        .entries()
        .iter()
        .map(|entry| entry.info.name.as_str())
        .filter(|name| !text.contains(&format!("| `{name}` |")))
        .collect();
    assert!(missing.is_empty(), "policies missing from the table in docs/pawncritic.md: {missing:?}");
}

#[test]
fn a_long_run_of_spaces_inside_a_line_costs_linear_time() {
    let policy = "CodeLayout::ProhibitTrailingWhitespace";
    let selection =
        Selection { single_policy: Some(policy_pattern(&format!("^{policy}$")).unwrap()), ..Selection::default() };
    let line = format!("int g_Value ={}1;\n", " ".repeat(200_000));
    let input = Input { path: PathBuf::from("spaces.sp"), content: Some(line.into_bytes()) };
    let plan = Plan::for_config(vec![input], &Configured::defaults(), &selection);
    assert!(plan.problems().is_empty(), "{:?}", plan.problems());

    let reports = plan.run(1).reports;

    assert!(reports[0].failures.is_empty(), "{:?}", reports[0].failures);
    assert!(reports[0].violations.is_empty(), "{:?}", reports[0].violations);
}
