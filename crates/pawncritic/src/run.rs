// Copyright 2026 davenonymous

//! A run: the files to check, each with the profile of its configuration,
//! checked in parallel, and what the command's exit status makes of the
//! result.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use pawn_core::cli::{ConfigCache, ConfigError, Configured, Input};

use crate::config::Config;
use crate::engine::{self, Catalog, CatalogSource, FileReport, Job, Problem, Profile, Selection};
use crate::policy::PolicyInfo;

/// The files of a run with the profiles they are checked with, and the
/// problems met while building the profiles.
pub struct Plan {
    jobs: Vec<Job>,
    profiles: Vec<Arc<Profile>>,
    problems: Vec<Problem>,
}

impl Plan {
    /// Plans checking `inputs`, each with the profile of the configuration
    /// `configs` gives for it. Files of one configuration share a profile,
    /// and configurations with the same policy directories and limits
    /// share a catalog, so the policy files are read, and their problems
    /// reported, once per run.
    pub fn build(
        inputs: Vec<Input>,
        configs: &mut ConfigCache<Config>,
        selection: &Selection,
    ) -> Result<Plan, ConfigError> {
        let mut catalogs: HashMap<CatalogSource, Arc<Catalog>> = HashMap::new();
        let mut profile_of_config: BTreeMap<Option<PathBuf>, usize> = BTreeMap::new();
        let mut plan = Plan { jobs: Vec::with_capacity(inputs.len()), profiles: Vec::new(), problems: Vec::new() };
        for input in inputs {
            let configured = configs.for_file(&input.path)?;
            let profile = match profile_of_config.get(&configured.path) {
                Some(&index) => index,
                None => {
                    let index = plan.add_profile(&configured, selection, &mut catalogs);
                    profile_of_config.insert(configured.path.clone(), index);
                    index
                }
            };
            plan.jobs.push(Job { input, profile });
        }
        Ok(plan)
    }

    /// Plans checking `inputs` with the one profile of `configured`.
    pub fn for_config(inputs: Vec<Input>, configured: &Configured<Config>, selection: &Selection) -> Plan {
        let (profile, problems) = Profile::build(configured, selection);
        Plan { problems, ..Plan::for_profile(inputs, profile) }
    }

    /// Plans checking `inputs` with `profile`.
    pub fn for_profile(inputs: Vec<Input>, profile: Profile) -> Plan {
        let jobs = inputs.into_iter().map(|input| Job { input, profile: 0 }).collect();
        Plan { jobs, profiles: vec![Arc::new(profile)], problems: Vec::new() }
    }

    /// Builds the profile of `configured`, with the catalog of its source
    /// loaded unless `catalogs` holds it, and returns its index.
    fn add_profile(
        &mut self,
        configured: &Configured<Config>,
        selection: &Selection,
        catalogs: &mut HashMap<CatalogSource, Arc<Catalog>>,
    ) -> usize {
        let source = CatalogSource::of(&configured.config, selection);
        let catalog = match catalogs.get(&source) {
            Some(catalog) => catalog.clone(),
            None => {
                let (catalog, problems) = source.load();
                self.problems.extend(problems);
                let catalog = Arc::new(catalog);
                catalogs.insert(source.clone(), catalog.clone());
                catalog
            }
        };
        let (profile, problems) = Profile::choose(configured, selection, catalog, source.limits);
        self.problems.extend(problems);
        self.profiles.push(Arc::new(profile));
        self.profiles.len() - 1
    }

    /// The problems with policies and configurations: the run goes on
    /// without what they concern, but its result is incomplete.
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// Checks the files on `threads` threads.
    pub fn run(self, threads: usize) -> RunResult {
        let policies = chosen_policies(&self.profiles);
        let checked = engine::run(self.jobs, &self.profiles, threads);
        let closing_errors = checked.closing_errors.into_iter().map(|message| Problem { message }).collect();
        RunResult { reports: checked.reports, problems: self.problems, closing_errors, policies }
    }
}

/// Every chosen policy of every profile, once, by name.
fn chosen_policies(profiles: &[Arc<Profile>]) -> Vec<Arc<PolicyInfo>> {
    let mut by_name: BTreeMap<String, Arc<PolicyInfo>> = BTreeMap::new();
    for profile in profiles {
        for policy in &profile.policies {
            by_name.entry(policy.info.name.clone()).or_insert_with(|| policy.info.clone());
        }
    }
    by_name.into_values().collect()
}

/// What a run found.
pub struct RunResult {
    /// A report per file, in the order of the inputs.
    pub reports: Vec<FileReport>,
    /// The problems of the plan.
    pub problems: Vec<Problem>,
    /// The problems after checking: Lua states of policies that crashed or
    /// hung while they were closed.
    pub closing_errors: Vec<Problem>,
    /// The policies chosen for any file, by name.
    pub policies: Vec<Arc<PolicyInfo>>,
}

impl RunResult {
    /// The files that could not be checked, or not by every policy: one
    /// message for each file that could not be loaded and for each policy
    /// that failed on a file.
    pub fn file_errors(&self) -> Vec<String> {
        let mut messages = Vec::new();
        for report in &self.reports {
            messages.extend(report.load_error.iter().cloned());
            for failure in &report.failures {
                messages.push(format!(
                    "{}: policy {} failed: {}",
                    report.path.display(),
                    failure.policy,
                    failure.message
                ));
            }
        }
        messages
    }

    /// What the run's exit status says: errors win over violations.
    pub fn exit_status(&self) -> ExitStatus {
        let has_errors = !self.problems.is_empty()
            || !self.closing_errors.is_empty()
            || self.reports.iter().any(|report| report.load_error.is_some() || !report.failures.is_empty());
        if has_errors {
            return ExitStatus::Errors;
        }
        if self.reports.iter().any(|report| !report.violations.is_empty()) {
            return ExitStatus::Violations;
        }
        ExitStatus::Clean
    }
}

/// The outcome of a command, as its exit status tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Every file was checked and nothing was found.
    Clean,
    /// Every file was checked and violations were found.
    Violations,
    /// Something could not be checked: a file, a policy or a
    /// configuration; or the command line is wrong.
    Errors,
}

impl ExitStatus {
    /// The process exit code: 0, 1 or 2.
    pub fn code(self) -> u8 {
        match self {
            ExitStatus::Clean => 0,
            ExitStatus::Violations => 1,
            ExitStatus::Errors => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::FailureKind;
    use crate::engine::{PolicyFailure, Violation};
    use crate::severity::Severity;
    use pawn_core::cli::ConfigChoice;
    use pawn_core::{Position, Span};

    fn report(violations: usize, failures: usize, load_error: bool) -> FileReport {
        let position = Position { line: 1, column: 1, utf16_column: 0, byte: 0 };
        let violation = Violation {
            policy: "A::B".to_string(),
            severity: Severity::HIGHEST,
            message: "m".to_string(),
            hint: None,
            span: Span::new(0, 0),
            start: position,
            end: position,
        };
        let failure =
            PolicyFailure { policy: "A::B".to_string(), kind: FailureKind::Error, message: "broke".to_string() };
        FileReport {
            path: PathBuf::from("a.sp"),
            source: None,
            violations: vec![violation; violations],
            failures: vec![failure; failures],
            load_error: load_error.then(|| "a.sp: cannot read".to_string()),
        }
    }

    fn result(reports: Vec<FileReport>, problems: usize) -> RunResult {
        let problems = (0..problems).map(|_| Problem { message: "p".to_string() }).collect();
        RunResult { reports, problems, closing_errors: Vec::new(), policies: Vec::new() }
    }

    #[test]
    fn errors_win_over_violations() {
        assert_eq!(result(vec![report(0, 0, false)], 0).exit_status(), ExitStatus::Clean);
        assert_eq!(result(Vec::new(), 0).exit_status(), ExitStatus::Clean);
        assert_eq!(result(vec![report(2, 0, false)], 0).exit_status(), ExitStatus::Violations);
        assert_eq!(result(vec![report(2, 0, false)], 1).exit_status(), ExitStatus::Errors);
        assert_eq!(result(vec![report(2, 1, false), report(0, 0, false)], 0).exit_status(), ExitStatus::Errors);
        assert_eq!(result(vec![report(0, 0, true)], 0).exit_status(), ExitStatus::Errors);
        let closing = Problem { message: "closing".to_string() };
        let crashed_while_closing = RunResult { closing_errors: vec![closing], ..result(vec![report(0, 0, false)], 0) };
        assert_eq!(crashed_while_closing.exit_status(), ExitStatus::Errors);
        assert_eq!([ExitStatus::Clean, ExitStatus::Violations, ExitStatus::Errors].map(ExitStatus::code), [0, 1, 2]);
    }

    #[test]
    fn file_errors_name_the_file_and_the_policy() {
        let errors = result(vec![report(0, 1, true)], 0).file_errors();
        assert_eq!(errors, ["a.sp: cannot read", "a.sp: policy A::B failed: broke"]);
    }

    fn inputs(root: &std::path::Path, names: &[&str]) -> Vec<Input> {
        names.iter().map(|name| Input { path: root.join(name), content: Some(Vec::new()) }).collect()
    }

    #[test]
    fn files_share_the_profile_of_their_configuration() {
        let root = tempfile::tempdir().unwrap();
        for directory in ["one", "two"] {
            std::fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        std::fs::write(root.path().join("one/.pawncritic.toml"), "severity = 1").unwrap();
        std::fs::write(root.path().join("two/.pawncritic.toml"), "severity = 5").unwrap();
        let mut configs = ConfigCache::new(ConfigChoice::Nearest).unwrap();
        let names = ["one/a.sp", "two/b.sp", "one/c.sp"];

        let plan = Plan::build(inputs(root.path(), &names), &mut configs, &Selection::default()).unwrap();

        assert!(plan.problems.is_empty(), "{:?}", plan.problems);
        assert_eq!(plan.jobs.iter().map(|job| job.profile).collect::<Vec<_>>(), [0, 1, 0]);
        assert_eq!(plan.profiles.len(), 2);
        assert!(plan.profiles[0].policies.len() > plan.profiles[1].policies.len());
        assert!(Arc::ptr_eq(&plan.profiles[0].catalog, &plan.profiles[1].catalog), "the catalog is loaded once");
    }

    #[test]
    fn one_configuration_makes_one_profile() {
        let root = tempfile::tempdir().unwrap();
        let mut configs = ConfigCache::new(ConfigChoice::Defaults).unwrap();

        let plan = Plan::build(inputs(root.path(), &["a.sp", "b.sp"]), &mut configs, &Selection::default()).unwrap();

        assert_eq!(plan.jobs.iter().map(|job| job.profile).collect::<Vec<_>>(), [0, 0]);
        assert_eq!(plan.profiles.len(), 1);
    }

    #[test]
    fn a_broken_custom_policy_is_reported_once_per_run() {
        let root = tempfile::tempdir().unwrap();
        let policies = root.path().join("critic");
        std::fs::create_dir_all(&policies).unwrap();
        std::fs::write(policies.join("broken.lua"), "this is not lua").unwrap();
        for directory in ["one", "two"] {
            std::fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        std::fs::write(root.path().join("one/.pawncritic.toml"), "severity = 1").unwrap();
        std::fs::write(root.path().join("two/.pawncritic.toml"), "severity = 5").unwrap();
        let selection = Selection { policy_paths: vec![policies], ..Selection::default() };
        let mut configs = ConfigCache::new(ConfigChoice::Nearest).unwrap();

        let plan = Plan::build(inputs(root.path(), &["one/a.sp", "two/b.sp"]), &mut configs, &selection).unwrap();

        assert_eq!(plan.profiles.len(), 2);
        let messages: Vec<&str> = plan.problems.iter().map(|problem| problem.message.as_str()).collect();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("broken.lua"), "{messages:?}");
    }
}
