// Copyright 2026 davenonymous

//! Loading the policies, choosing those that run, and running them on
//! files, in parallel and isolated from each other.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use pawn_core::cli::{Configured, Input, panic_message};
use pawn_core::{ParsedFile, Position, SourceFile, Span};
use regex::Regex;

use crate::analyzed::AnalyzedFile;
use crate::annotations::Suppressions;
use crate::builtin;
use crate::check::{CheckOutcome, Failure, FailureKind, Implementation, Instance};
use crate::config::Config;
use crate::lua::sandbox::Limits;
use crate::lua::{self, PolicySource};
use crate::native::{self, NativePolicy};
use crate::policy::{OptionValue, PolicyInfo, PolicyOrigin};
use crate::severity::Severity;
use crate::theme::ThemeExpression;

/// The severity a run reports from when neither the command line nor the
/// configuration says otherwise.
pub const DEFAULT_SEVERITY: Severity = match Severity::new(3) {
    Some(severity) => severity,
    None => panic!("3 is a valid severity"),
};

/// A known policy.
#[derive(Clone)]
pub struct CatalogEntry {
    pub info: PolicyInfo,
    pub implementation: Implementation,
}

/// A problem with a policy or a configuration that does not stop the run
/// but makes its result incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub message: String,
}

/// Every policy pawncritic knows for a run: the built-in ones and those in
/// the policy directories.
pub struct Catalog {
    entries: Vec<CatalogEntry>,
}

impl Catalog {
    /// Loads the built-in policies and the `*.lua` files below
    /// `directories`. A policy that cannot be loaded is left out and named
    /// in the problems.
    pub fn load(directories: &[PathBuf], limits: Limits) -> (Catalog, Vec<Problem>) {
        let mut problems = Vec::new();
        let mut entries: Vec<CatalogEntry> = native::all()
            .into_iter()
            .map(|policy| {
                let policy: Arc<dyn NativePolicy> = Arc::from(policy);
                CatalogEntry { info: policy.info(), implementation: Implementation::Native(policy) }
            })
            .collect();
        let mut sources: Vec<PolicySource> = builtin::SOURCES
            .iter()
            .map(|(file, code)| PolicySource { origin: PolicyOrigin::Builtin(file), code: Arc::from(*code) })
            .collect();
        for directory in directories {
            match policy_files(directory) {
                Ok(files) => {
                    for file in files {
                        match read_policy_code(&file) {
                            Ok(code) => sources.push(PolicySource { origin: PolicyOrigin::File(file), code }),
                            Err(message) => problems.push(Problem { message }),
                        }
                    }
                }
                Err(message) => problems.push(Problem { message }),
            }
        }
        for source in sources {
            let info = match &source.origin {
                PolicyOrigin::File(_) => read_custom_policy_info(&source, limits),
                _ => lua::read_policy_info(&source, limits),
            };
            match info {
                Ok(info) => entries.push(CatalogEntry { info, implementation: Implementation::Lua(source) }),
                Err(failure) => problems.push(Problem {
                    message: format!("{}: {}; the policy is skipped", source.origin, failure.message),
                }),
            }
        }
        entries.sort_by(|a, b| a.info.name.cmp(&b.info.name));
        let mut unique: Vec<CatalogEntry> = Vec::with_capacity(entries.len());
        for entry in entries {
            match unique.last() {
                Some(previous) if previous.info.name == entry.info.name => problems.push(Problem {
                    message: format!(
                        "{}: policy {} is already defined by {}; the second definition is skipped",
                        entry.info.origin, entry.info.name, previous.info.origin
                    ),
                }),
                _ => unique.push(entry),
            }
        }
        (Catalog { entries: unique }, problems)
    }

    pub fn entries(&self) -> &[CatalogEntry] {
        &self.entries
    }

    pub fn find(&self, name: &str) -> Option<&CatalogEntry> {
        self.entries.iter().find(|entry| entry.info.name == name)
    }
}

/// Reads a custom policy on a thread of its own, so that top-level code
/// stuck inside one library call (which the Lua hook cannot interrupt)
/// costs at most its time limit plus the grace period, and a crash only
/// this policy. A thread that hangs is abandoned and ends with the
/// process.
fn read_custom_policy_info(source: &PolicySource, limits: Limits) -> Result<PolicyInfo, Failure> {
    let (sender, receiver) = mpsc::channel();
    let loading = source.clone();
    let started = Instant::now();
    std::thread::spawn(move || {
        let info = catch_unwind(AssertUnwindSafe(|| lua::read_policy_info(&loading, limits))).unwrap_or_else(|panic| {
            let message = format!("the policy crashed while loading: {}", panic_message(&*panic));
            Err(Failure::new(FailureKind::Crashed, message))
        });
        let _ = sender.send(info);
    });
    match receiver.recv_timeout(limits.time_per_file + limits.hang_grace) {
        Ok(info) => info,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            let seconds = started.elapsed().as_secs_f64();
            let message = format!("the policy hung while loading and was given up after {seconds:.1} s");
            Err(Failure::new(FailureKind::Hung, message))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(Failure::new(FailureKind::Crashed, "the policy crashed while loading"))
        }
    }
}

/// The code of the policy file at `path`, without a UTF-8 byte order mark
/// at its start; the whole problem message when it cannot be read or is
/// not UTF-8.
fn read_policy_code(path: &Path) -> Result<Arc<str>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: cannot read: {error}", path.display()))?;
    let without_mark = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    match std::str::from_utf8(without_mark) {
        Ok(code) => Ok(Arc::from(code)),
        Err(_) => Err(format!("{}: the file is not valid UTF-8; the policy is skipped", path.display())),
    }
}

/// The `*.lua` files below `directory`, sorted.
fn policy_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    if !directory.is_dir() {
        return Err(format!("{}: the policy directory does not exist", directory.display()));
    }
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries =
            std::fs::read_dir(&current).map_err(|error| format!("{}: cannot list: {error}", current.display()))?;
        for entry in entries {
            let path = entry.map_err(|error| format!("{}: cannot list: {error}", current.display()))?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "lua") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// What the command line says about choosing policies; it goes over the
/// configuration file.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub severity: Option<Severity>,
    pub theme: Option<ThemeExpression>,
    pub include: Vec<Regex>,
    pub exclude: Vec<Regex>,
    /// Run only the policies matching this, whatever else says.
    pub single_policy: Option<Regex>,
    /// Ignore `no critic` annotations.
    pub force: bool,
    pub policy_paths: Vec<PathBuf>,
    pub limits: Option<Limits>,
}

/// A policy chosen to run, with its effective severity, themes and options.
#[derive(Clone)]
pub struct EnabledPolicy {
    pub info: Arc<PolicyInfo>,
    pub options: BTreeMap<String, OptionValue>,
    pub implementation: Implementation,
}

/// A policy with the severity and themes its policy section gives it, and
/// its options.
#[derive(Clone)]
pub struct EffectivePolicy {
    pub info: Arc<PolicyInfo>,
    pub options: BTreeMap<String, OptionValue>,
}

/// Everything a run derives from one configuration: the catalog and the
/// policies chosen from it.
pub struct Profile {
    pub catalog: Arc<Catalog>,
    pub policies: Vec<EnabledPolicy>,
    /// Every policy that is chosen or that the configuration has a policy
    /// section for, by name, as configured. A policy whose configured
    /// options are wrong has the defaults here; the mistake is reported,
    /// and the policy skipped, only when it is chosen.
    pub effective: BTreeMap<String, EffectivePolicy>,
    pub force: bool,
    pub limits: Limits,
}

/// What a catalog is loaded from: the policy directories and the limits
/// their files are read under. Configurations with the same source share
/// one catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CatalogSource {
    pub(crate) directories: Vec<PathBuf>,
    pub(crate) limits: Limits,
}

impl CatalogSource {
    /// The source of the catalog `config` needs under `selection`.
    pub(crate) fn of(config: &Config, selection: &Selection) -> CatalogSource {
        let mut directories = config.policy_paths.clone();
        directories.extend(selection.policy_paths.iter().cloned());
        CatalogSource { directories, limits: selection.limits.unwrap_or_else(|| limits_of(config)) }
    }

    pub(crate) fn load(&self) -> (Catalog, Vec<Problem>) {
        Catalog::load(&self.directories, self.limits)
    }
}

/// Compiles a policy name pattern: a case-insensitive regular expression.
pub fn policy_pattern(pattern: &str) -> Result<Regex, String> {
    Regex::new(&format!("(?i){pattern}")).map_err(|error| format!("`{pattern}` is no valid pattern: {error}"))
}

impl Profile {
    /// Loads the catalog for `configured` and chooses its policies.
    pub fn build(configured: &Configured<Config>, selection: &Selection) -> (Profile, Vec<Problem>) {
        let source = CatalogSource::of(&configured.config, selection);
        let (catalog, mut problems) = source.load();
        let (profile, choice_problems) = Profile::choose(configured, selection, Arc::new(catalog), source.limits);
        problems.extend(choice_problems);
        (profile, problems)
    }

    /// Chooses the policies of `configured` from `catalog`, the catalog of
    /// its [`CatalogSource`] with `limits`.
    pub(crate) fn choose(
        configured: &Configured<Config>,
        selection: &Selection,
        catalog: Arc<Catalog>,
        limits: Limits,
    ) -> (Profile, Vec<Problem>) {
        let config = &*configured.config;
        let mut problems = Vec::new();
        let in_config = |message: String| match &configured.path {
            Some(path) => format!("{}: {message}", path.display()),
            None => message,
        };
        for name in config.policies.keys() {
            if catalog.find(name).is_none() {
                problems.push(Problem { message: in_config(format!("configures the unknown policy {name}")) });
            }
        }
        let mut config_patterns = |patterns: &[String]| -> Vec<Regex> {
            patterns
                .iter()
                .filter_map(|pattern| {
                    policy_pattern(pattern)
                        .map_err(|message| problems.push(Problem { message: in_config(message) }))
                        .ok()
                })
                .collect()
        };
        let mut include = config_patterns(&config.include);
        let mut exclude = config_patterns(&config.exclude);
        include.extend(selection.include.iter().cloned());
        exclude.extend(selection.exclude.iter().cloned());
        let severity = selection.severity.or(config.severity).unwrap_or(DEFAULT_SEVERITY);
        let theme = selection.theme.clone().or_else(|| config.theme.clone());

        let mut policies = Vec::new();
        let mut effective = BTreeMap::new();
        for entry in catalog.entries() {
            let policy_config = config.policies.get(&entry.info.name);
            let mut info = entry.info.clone();
            if let Some(policy_config) = policy_config {
                info.severity = policy_config.severity.unwrap_or(info.severity);
                info.themes.extend(policy_config.add_themes.iter().cloned());
            }
            let name = info.name.as_str();
            let is_chosen = match &selection.single_policy {
                Some(single) => single.is_match(name),
                None => {
                    let is_listed = !config.only || policy_config.is_some();
                    let is_enabled = policy_config.and_then(|policy| policy.enabled) != Some(false);
                    let is_excluded = exclude.iter().any(|pattern| pattern.is_match(name));
                    let is_included = include.iter().any(|pattern| pattern.is_match(name));
                    let meets_severity = info.severity >= severity;
                    let meets_theme = theme.as_ref().is_none_or(|theme| theme.matches(&info.themes));
                    is_listed && is_enabled && !is_excluded && (is_included || (meets_severity && meets_theme))
                }
            };
            if !is_chosen && policy_config.is_none() {
                continue;
            }
            let configured = policy_config.map(|policy| policy.options.clone()).unwrap_or_default();
            let info = Arc::new(info);
            match info.resolve_options(&configured) {
                Ok(options) => {
                    if is_chosen {
                        let implementation = entry.implementation.clone();
                        policies.push(EnabledPolicy { info: info.clone(), options: options.clone(), implementation });
                    }
                    effective.insert(info.name.clone(), EffectivePolicy { info, options });
                }
                Err(message) => {
                    if is_chosen {
                        problems.push(Problem { message: in_config(format!("{message}; the policy is skipped")) });
                    }
                    let options = info.resolve_options(&BTreeMap::new()).expect("the defaults are valid options");
                    effective.insert(info.name.clone(), EffectivePolicy { info, options });
                }
            }
        }
        let profile = Profile { catalog, policies, effective, force: selection.force || config.force, limits };
        (profile, problems)
    }
}

fn limits_of(config: &Config) -> Limits {
    let defaults = Limits::default();
    Limits {
        time_per_file: config.limits.timeout_ms.map_or(defaults.time_per_file, |ms| Duration::from_millis(ms.get())),
        memory_bytes: config.limits.memory_mb.map_or(defaults.memory_bytes, |megabytes| {
            usize::try_from(megabytes.get()).unwrap_or(usize::MAX).saturating_mul(1024 * 1024)
        }),
        hang_grace: defaults.hang_grace,
    }
}

/// A violation of a policy.
#[derive(Debug, Clone)]
pub struct Violation {
    pub policy: String,
    pub severity: Severity,
    pub message: String,
    pub hint: Option<String>,
    pub span: Span,
    pub start: Position,
    pub end: Position,
}

/// A policy that could not finish checking a file.
#[derive(Debug, Clone)]
pub struct PolicyFailure {
    pub policy: String,
    pub kind: FailureKind,
    pub message: String,
}

impl PolicyFailure {
    fn new(policy: &str, failure: Failure) -> PolicyFailure {
        PolicyFailure { policy: policy.to_string(), kind: failure.kind, message: failure.message }
    }
}

/// The result of checking one file.
#[derive(Debug, Clone)]
pub struct FileReport {
    pub path: PathBuf,
    /// The file, for showing the lines of its violations; absent when it
    /// could not be read.
    pub source: Option<Arc<SourceFile>>,
    pub violations: Vec<Violation>,
    pub failures: Vec<PolicyFailure>,
    pub load_error: Option<String>,
}

/// A file to check and the profile to check it with: an index into the
/// profiles [`run`] is given.
pub(crate) struct Job {
    pub(crate) input: Input,
    pub(crate) profile: usize,
}

/// How often the watchdog looks at the workers.
const WATCHDOG_INTERVAL: Duration = Duration::from_millis(100);

/// How many grace periods a worker may spend closing its policy states.
const CLOSING_GRACES: u32 = 5;

/// What a worker is busy with.
enum Task {
    /// Running `policy` on the file of job `job`.
    Checking { job: usize, policy: String },
    /// Closing the policy states after the last job.
    Closing,
}

/// A task with the time it started and how long it may take.
struct Activity {
    task: Task,
    since: Instant,
    allowed: Duration,
}

impl Activity {
    fn is_overdue(&self) -> bool {
        self.since.elapsed() > self.allowed
    }
}

/// What a worker and the watchdog share about the worker: its current
/// activity, and whether the watchdog gave up on it.
///
/// The watchdog gives up on a worker only while it holds the activity
/// lock, and the worker ends an activity under the same lock, so either
/// the worker finished first and is kept, or it learns that it was
/// abandoned and stops without reporting.
#[derive(Clone)]
struct WorkerHandle {
    activity: Arc<Mutex<Option<Activity>>>,
    abandoned: Arc<AtomicBool>,
}

impl WorkerHandle {
    fn new() -> WorkerHandle {
        WorkerHandle { activity: Arc::new(Mutex::new(None)), abandoned: Arc::new(AtomicBool::new(false)) }
    }

    fn is_abandoned(&self) -> bool {
        self.abandoned.load(Ordering::SeqCst)
    }

    fn begin(&self, task: Task, allowed: Duration) {
        *lock(&self.activity) = Some(Activity { task, since: Instant::now(), allowed });
    }

    /// Ends the current activity. False when the watchdog gave up on the
    /// worker meanwhile: its job was handed to another worker.
    #[must_use]
    fn finish(&self) -> bool {
        let mut activity = lock(&self.activity);
        *activity = None;
        !self.is_abandoned()
    }

    /// For the watchdog: when the worker's activity is overdue, takes it
    /// and marks the worker abandoned.
    fn abandon_if_overdue(&self) -> Option<Activity> {
        let mut activity = lock(&self.activity);
        if !activity.as_ref().is_some_and(Activity::is_overdue) {
            return None;
        }
        self.abandoned.store(true, Ordering::SeqCst);
        activity.take()
    }
}

/// A policy the watchdog gave up on: on which job, after how long.
struct HungPolicy {
    job: usize,
    after: Duration,
}

impl HungPolicy {
    /// The failure the policy reports on the file of `job`.
    fn failure(&self, job: usize) -> Failure {
        let seconds = self.after.as_secs_f64();
        if job == self.job {
            let message = format!(
                "the policy hung and was given up after {seconds:.1} s; it is skipped on every file from then on"
            );
            Failure::new(FailureKind::Hung, message)
        } else {
            let message = format!("skipped: the policy hung on an earlier file and was given up after {seconds:.1} s");
            Failure::new(FailureKind::Skipped, message)
        }
    }
}

/// What the workers share.
struct Shared {
    jobs: Vec<Job>,
    profiles: Vec<Arc<Profile>>,
    queue: Mutex<VecDeque<usize>>,
    /// Policies that hung, skipped on every file after.
    hung: Mutex<HashMap<String, HungPolicy>>,
    /// How long a worker may take to close its policy states.
    closing_allowance: Duration,
}

enum WorkerEvent {
    Checked {
        worker: usize,
        job: usize,
        report: FileReport,
    },
    /// The worker closed its policy states and ended; with the message of
    /// the panic when closing them crashed.
    Exited {
        worker: usize,
        crash: Option<String>,
    },
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Checks `jobs` on `threads` threads; the reports keep the order of the
/// jobs.
///
/// A watchdog guards the workers: when a policy runs past its
/// [`Limits::watchdog_allowance`], its worker thread is abandoned (it
/// cannot be stopped safely and ends with the process; when it comes back
/// it stops without reporting or taking another job), the policy is
/// skipped from then on, and the file is checked again by a new worker
/// without it. A worker that crashes or is given up while it closes its
/// policy states after the last job gives a closing error.
pub(crate) fn run(jobs: Vec<Job>, profiles: &[Arc<Profile>], threads: usize) -> Checked {
    let job_count = jobs.len();
    let longest_grace = profiles.iter().map(|profile| profile.limits.hang_grace).max().unwrap_or_default();
    let shared = Arc::new(Shared {
        jobs,
        profiles: profiles.to_vec(),
        queue: Mutex::new((0..job_count).collect()),
        hung: Mutex::new(HashMap::new()),
        closing_allowance: longest_grace * CLOSING_GRACES,
    });
    let (sender, receiver) = mpsc::channel();
    let mut workers: HashMap<usize, WorkerHandle> = HashMap::new();
    let mut next_worker = 0;
    for _ in 0..threads.clamp(1, job_count.max(1)) {
        workers.insert(next_worker, spawn_worker(next_worker, &shared, &sender));
        next_worker += 1;
    }
    let mut reports: Vec<Option<FileReport>> = (0..job_count).map(|_| None).collect();
    let mut closing_errors = Vec::new();
    while !workers.is_empty() {
        match receiver.recv_timeout(WATCHDOG_INTERVAL) {
            Ok(WorkerEvent::Checked { worker, job, report }) if workers.contains_key(&worker) => {
                reports[job] = Some(report);
            }
            Ok(WorkerEvent::Exited { worker, crash }) => {
                workers.remove(&worker);
                closing_errors.extend(crash.map(|message| {
                    format!("internal error in pawncritic while closing the policy states: {message}; please report it")
                }));
            }
            Ok(WorkerEvent::Checked { .. }) | Err(_) => {}
        }
        let abandoned: Vec<(usize, Activity)> = workers
            .iter()
            .filter_map(|(&worker, handle)| handle.abandon_if_overdue().map(|activity| (worker, activity)))
            .collect();
        for (worker, activity) in abandoned {
            workers.remove(&worker);
            let Task::Checking { job, policy } = activity.task else {
                let seconds = activity.since.elapsed().as_secs_f64();
                closing_errors.push(format!("closing the policy states was given up after {seconds:.1} s"));
                continue;
            };
            lock(&shared.hung).insert(policy, HungPolicy { job, after: activity.since.elapsed() });
            lock(&shared.queue).push_front(job);
            workers.insert(next_worker, spawn_worker(next_worker, &shared, &sender));
            next_worker += 1;
        }
    }
    let reports = reports
        .into_iter()
        .enumerate()
        .map(|(job, report)| {
            report.unwrap_or_else(|| internal_error_report(&shared.jobs[job], "no worker finished checking the file"))
        })
        .collect();
    Checked { reports, closing_errors }
}

/// What [`run`] gives.
pub(crate) struct Checked {
    /// A report per job, in the order of the jobs.
    pub(crate) reports: Vec<FileReport>,
    /// The workers that crashed or were given up while closing their
    /// policy states: the reports are complete, but the run is not clean.
    pub(crate) closing_errors: Vec<String>,
}

fn spawn_worker(worker: usize, shared: &Arc<Shared>, sender: &mpsc::Sender<WorkerEvent>) -> WorkerHandle {
    let handle = WorkerHandle::new();
    let (shared, sender, own_handle) = (shared.clone(), sender.clone(), handle.clone());
    std::thread::spawn(move || work(worker, &shared, &own_handle, &sender));
    handle
}

/// A worker: checks jobs from the queue until it is empty or the watchdog
/// gave up on it.
fn work(worker: usize, shared: &Shared, handle: &WorkerHandle, sender: &mpsc::Sender<WorkerEvent>) {
    let mut checker = Checker::new(shared, handle);
    while !handle.is_abandoned() {
        let Some(job) = lock(&shared.queue).pop_front() else { break };
        let report = match catch_unwind(AssertUnwindSafe(|| checker.check(job))) {
            Ok(Some(report)) => report,
            Ok(None) => return,
            Err(panic) => {
                checker = Checker::new(shared, handle);
                internal_error_report(&shared.jobs[job], &panic_message(&*panic))
            }
        };
        if !handle.finish() {
            return;
        }
        if sender.send(WorkerEvent::Checked { worker, job, report }).is_err() {
            return;
        }
    }
    if handle.is_abandoned() {
        return;
    }
    handle.begin(Task::Closing, shared.closing_allowance);
    let closed = catch_unwind(AssertUnwindSafe(|| drop(checker)));
    let crash = closed.err().map(|panic| panic_message(&*panic));
    if handle.finish() {
        let _ = sender.send(WorkerEvent::Exited { worker, crash });
    }
}

fn internal_error_report(job: &Job, message: &str) -> FileReport {
    FileReport {
        path: job.input.path.clone(),
        source: None,
        violations: Vec::new(),
        failures: Vec::new(),
        load_error: Some(format!(
            "{}: internal error in pawncritic: {message}; please report it with the file",
            job.input.path.display()
        )),
    }
}

/// Checks files on one thread; holds that thread's Lua states.
struct Checker<'run> {
    shared: &'run Shared,
    handle: &'run WorkerHandle,
    instances: HashMap<(usize, usize), Instance>,
}

impl<'run> Checker<'run> {
    fn new(shared: &'run Shared, handle: &'run WorkerHandle) -> Checker<'run> {
        Checker { shared, handle, instances: HashMap::new() }
    }

    /// Checks the file of job `job_index` with every policy of its profile;
    /// `None` when the watchdog gave up on this worker meanwhile.
    fn check(&mut self, job_index: usize) -> Option<FileReport> {
        let job = &self.shared.jobs[job_index];
        let source = match job.input.load() {
            Ok(source) => source,
            Err(error) => {
                return Some(FileReport {
                    path: job.input.path.clone(),
                    source: None,
                    violations: Vec::new(),
                    failures: Vec::new(),
                    load_error: Some(error.to_string()),
                });
            }
        };
        let file = Rc::new(AnalyzedFile::new(ParsedFile::parse(source)));
        let profile = &self.shared.profiles[job.profile];
        let suppressions = if profile.force { Suppressions::default() } else { Suppressions::of(&file) };
        let mut violations = Vec::new();
        let mut failures = Vec::new();
        for (index, policy) in profile.policies.iter().enumerate() {
            if !policy.info.applies_to(file.parsed.source.kind()) {
                continue;
            }
            if let Some(hung) = lock(&self.shared.hung).get(&policy.info.name) {
                failures.push(PolicyFailure::new(&policy.info.name, hung.failure(job_index)));
                continue;
            }
            let task = Task::Checking { job: job_index, policy: policy.info.name.clone() };
            self.handle.begin(task, profile.limits.watchdog_allowance());
            let outcome = self.run_policy(job.profile, index, policy, &file);
            if !self.handle.finish() {
                return None;
            }
            if let Some(failure) = outcome.failure {
                failures.push(PolicyFailure::new(&policy.info.name, failure));
            }
            for report in outcome.reports {
                let (start, end) = file.positions(report.span);
                if suppressions.is_suppressed(&policy.info.name, start.line) {
                    continue;
                }
                violations.push(Violation {
                    policy: policy.info.name.clone(),
                    severity: policy.info.severity,
                    message: report.message.unwrap_or_else(|| policy.info.summary.clone()),
                    hint: report.hint,
                    span: report.span,
                    start,
                    end,
                });
            }
        }
        violations.sort_by(|a, b| (a.start, &a.policy, &a.message).cmp(&(b.start, &b.policy, &b.message)));
        violations.dedup_by(|a, b| a.start == b.start && a.policy == b.policy && a.message == b.message);
        Some(FileReport {
            path: job.input.path.clone(),
            source: Some(file.parsed.source.clone()),
            violations,
            failures,
            load_error: None,
        })
    }

    /// Runs one policy on one file with this thread's instance of it.
    fn run_policy(
        &mut self,
        profile: usize,
        index: usize,
        policy: &EnabledPolicy,
        file: &Rc<AnalyzedFile>,
    ) -> CheckOutcome {
        let limits = self.shared.profiles[profile].limits;
        let instance = self
            .instances
            .entry((profile, index))
            .or_insert_with(|| policy.implementation.instantiate(&policy.options, limits));
        instance.check(file, &policy.options)
    }
}
