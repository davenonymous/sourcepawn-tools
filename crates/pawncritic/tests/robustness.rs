// Copyright 2026 davenonymous

//! Broken custom policies must not take down the run: each failure is
//! reported, and every other policy still checks every file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use pawn_core::cli::{Configured, Input};
use pawncritic::analyzed::AnalyzedFile;
use pawncritic::check::{CheckOutcome, FailureKind, Implementation};
use pawncritic::engine::{self, Catalog, EnabledPolicy, Profile, Selection, policy_pattern};
use pawncritic::lua::sandbox::Limits;
use pawncritic::native::NativePolicy;
use pawncritic::policy::{FileKindName, OptionValue, PolicyInfo, PolicyOrigin};
use pawncritic::run::Plan;
use pawncritic::severity::Severity;

const GOOD: &str = r#"return {
    name = "Custom::ReportCalls", severity = 3, summary = "a call",
    query = "(call_expression) @violation",
}"#;

const ENDLESS_LOOP: &str = r#"return {
    name = "Custom::EndlessLoop", severity = 3, summary = "never ends",
    check = function(file, ctx) while true do end end,
}"#;

const SWALLOWS_TIMEOUT: &str = r#"return {
    name = "Custom::SwallowsTimeout", severity = 3, summary = "tries to catch the time limit",
    check = function(file, ctx)
        while true do pcall(function() while true do end end) end
    end,
}"#;

const MEMORY_HOG: &str = r#"return {
    name = "Custom::MemoryHog", severity = 3, summary = "eats memory",
    check = function(file, ctx)
        local hoard = {}
        for i = 1, math.huge do hoard[i] = string.rep("x", 65536) .. i end
    end,
}"#;

const SWALLOWS_MEMORY: &str = r#"return {
    name = "Custom::SwallowsMemory", severity = 3, summary = "tries to catch the memory limit",
    check = function(file, ctx)
        local hoard = {}
        pcall(function() for i = 1, math.huge do hoard[i] = string.rep("x", 65536) .. i end end)
        hoard = nil
        ctx:report({ line = 1 }, "caught the memory limit")
    end,
}"#;

const RUNTIME_ERROR: &str = r#"return {
    name = "Custom::RuntimeError", severity = 3, summary = "fails",
    on = { call_expression = function(node, ctx) error("boom at " .. node:text()) end },
}"#;

const BAD_REPORT: &str = r#"return {
    name = "Custom::BadReport", severity = 3, summary = "reports nonsense",
    check = function(file, ctx) ctx:report({ line = 99999 }) end,
}"#;

const SANDBOX_ESCAPE: &str = r#"return {
    name = "Custom::SandboxEscape", severity = 3, summary = "tries to reach the system",
    check = function(file, ctx)
        if io or os or load or require or dofile then ctx:report({ line = 1 }, "escaped") end
    end,
}"#;

const C_LEVEL_LOOPS: &str = r#"return {
    name = "Custom::CLevelLoops", severity = 3, summary = "loops inside library functions",
    check = function(file, ctx)
        local ok_rep = pcall(string.rep, "", math.maxinteger)
        local ok_rep_string = pcall(string.rep, "", "1e15")
        local ok_move = pcall(table.move, {}, 1, 1 << 62, 1, {})
        local ok_move_string = pcall(table.move, {}, "1", "1e12", 1, {})
        local ok_gc = pcall(setmetatable, {}, { __gc = function() while true do end end })
        if ok_rep or ok_rep_string or ok_move or ok_move_string or ok_gc then
            ctx:report({ line = 1 }, "a guard is missing")
        end
        local valid = string.rep("ab", "3") == "ababab" and string.rep("x", 2.0) == "xx"
            and table.move({ 7, 8 }, "1", 2.0, 1, {})[2] == 8
            and not pcall(string.rep, "x", "many") and not pcall(table.move, {}, 1, 1.5, 1, {})
        if not valid then ctx:report({ line = 1 }, "a guard breaks a valid call") end
    end,
}"#;

const HUGE_REPORTS: &str = r#"return {
    name = "Custom::HugeReports", severity = 3, summary = "reports a huge message",
    check = function(file, ctx) ctx:report({ line = 1 }, string.rep("x", 8 * 1024 * 1024)) end,
}"#;

const COUNTLESS_REPORTS: &str = r#"return {
    name = "Custom::CountlessReports", severity = 3, summary = "reports forever",
    check = function(file, ctx) while true do ctx:report({ line = 1 }) end end,
}"#;

const BACKTRACKING: &str = r#"return {
    name = "Custom::Backtracking", severity = 3, summary = "a pattern that never finishes",
    check = function(file, ctx) string.find(string.rep("a", 40), string.rep("a*", 40) .. "b") end,
}"#;

const NOT_LUA: &str = "this is not lua at all";
const MISSING_SUMMARY: &str = r#"return { name = "Custom::NoSummary", severity = 3, check = function() end }"#;
const DUPLICATE: &str = r#"return {
    name = "Modern::ProhibitCloseHandle", severity = 3, summary = "a second definition",
    check = function() end,
}"#;

fn write(directory: &Path, name: &str, content: &str) {
    std::fs::write(directory.join(name), content).expect("the temporary directory is writable");
}

#[test]
fn broken_policies_are_contained() {
    let policies = tempfile::tempdir().unwrap();
    for (name, content) in [
        ("good.lua", GOOD),
        ("loop.lua", ENDLESS_LOOP),
        ("swallow.lua", SWALLOWS_TIMEOUT),
        ("memory.lua", MEMORY_HOG),
        ("swallow_memory.lua", SWALLOWS_MEMORY),
        ("error.lua", RUNTIME_ERROR),
        ("bad_report.lua", BAD_REPORT),
        ("escape.lua", SANDBOX_ESCAPE),
        ("c_loops.lua", C_LEVEL_LOOPS),
        ("huge.lua", HUGE_REPORTS),
        ("countless.lua", COUNTLESS_REPORTS),
        ("backtracking.lua", BACKTRACKING),
        ("not_lua.lua", NOT_LUA),
        ("no_summary.lua", MISSING_SUMMARY),
        ("duplicate.lua", DUPLICATE),
    ] {
        write(policies.path(), name, content);
    }
    let sources = tempfile::tempdir().unwrap();
    write(sources.path(), "a.sp", "void f(Handle h) {\n\tCloseHandle(h);\n}\n");
    write(sources.path(), "b.sp", "void g(Handle h) {\n\tCloseHandle(h);\n\tOther();\n}\n");

    let selection = Selection {
        single_policy: Some(policy_pattern("^(Custom::|Modern::ProhibitCloseHandle$)").unwrap()),
        policy_paths: vec![policies.path().to_path_buf()],
        limits: Some(Limits {
            time_per_file: Duration::from_millis(300),
            memory_bytes: 32 * 1024 * 1024,
            ..Limits::default()
        }),
        ..Selection::default()
    };
    let inputs = ["a.sp", "b.sp"].iter().map(|name| Input { path: sources.path().join(name), content: None }).collect();
    let plan = Plan::for_config(inputs, &Configured::defaults(), &selection);
    let problem_text: Vec<&str> = plan.problems().iter().map(|problem| problem.message.as_str()).collect();
    assert_eq!(problem_text.len(), 3, "{problem_text:#?}");
    assert!(problem_text.iter().any(|message| message.contains("not_lua.lua")));
    assert!(problem_text.iter().any(|message| message.contains("no_summary.lua") && message.contains("summary")));
    assert!(
        problem_text.iter().any(|message| message.contains("duplicate.lua") && message.contains("already defined"))
    );

    let reports = plan.run(1).reports;

    for report in &reports {
        let policies: Vec<&str> = report.violations.iter().map(|violation| violation.policy.as_str()).collect();
        assert!(policies.contains(&"Custom::ReportCalls"), "{}: {policies:?}", report.path.display());
        assert!(policies.contains(&"Modern::ProhibitCloseHandle"), "{}: {policies:?}", report.path.display());
        assert!(!policies.contains(&"Custom::SandboxEscape"), "the sandbox leaked");
        assert!(!policies.contains(&"Custom::CLevelLoops"), "a guard of a library function is missing");
        assert!(!policies.contains(&"Custom::SwallowsMemory"), "the memory limit was caught");
        let failed: Vec<&str> = report.failures.iter().map(|failure| failure.policy.as_str()).collect();
        assert!(!failed.contains(&"Custom::CLevelLoops"), "a guarded library call failed or hung: {failed:?}");
        for broken in [
            "Custom::EndlessLoop",
            "Custom::SwallowsTimeout",
            "Custom::MemoryHog",
            "Custom::SwallowsMemory",
            "Custom::RuntimeError",
            "Custom::BadReport",
        ] {
            assert!(failed.contains(&broken), "{}: {broken} did not fail: {failed:?}", report.path.display());
        }
    }
    let first_failures = &reports[0].failures;
    let message_of = |failures: &[engine::PolicyFailure], policy: &str| {
        failures.iter().find(|failure| failure.policy == policy).map(|failure| failure.message.clone()).unwrap()
    };
    assert!(message_of(first_failures, "Custom::EndlessLoop").contains("time limit exceeded"));
    assert!(message_of(first_failures, "Custom::SwallowsTimeout").contains("time limit exceeded"));
    assert!(message_of(first_failures, "Custom::MemoryHog").contains("memory limit exceeded"));
    assert_eq!(message_of(first_failures, "Custom::SwallowsMemory"), "memory limit exceeded");
    assert!(message_of(first_failures, "Custom::RuntimeError").contains("boom at CloseHandle(h)"));
    assert!(message_of(first_failures, "Custom::BadReport").contains("the file has 3 lines"));
    assert!(message_of(first_failures, "Custom::HugeReports").contains("more than 1000"));
    assert!(
        message_of(first_failures, "Custom::CountlessReports")
            .contains("reported more than 10000 violations on one file")
    );
    assert!(message_of(first_failures, "Custom::Backtracking").contains("hung"));
    let kind_of = |failures: &[engine::PolicyFailure], policy: &str| {
        failures.iter().find(|failure| failure.policy == policy).map(|failure| failure.kind).unwrap()
    };
    assert_eq!(kind_of(first_failures, "Custom::EndlessLoop"), FailureKind::Exhausted);
    assert_eq!(kind_of(first_failures, "Custom::MemoryHog"), FailureKind::Exhausted);
    assert_eq!(kind_of(first_failures, "Custom::RuntimeError"), FailureKind::Error);
    assert_eq!(kind_of(first_failures, "Custom::Backtracking"), FailureKind::Hung);
    let second_failures = &reports[1].failures;
    assert!(
        message_of(second_failures, "Custom::EndlessLoop").starts_with("skipped"),
        "an exhausted policy runs again"
    );
    assert_eq!(kind_of(second_failures, "Custom::EndlessLoop"), FailureKind::Skipped);
    assert_eq!(kind_of(second_failures, "Custom::RuntimeError"), FailureKind::Error);
}

#[test]
fn errors_that_name_a_limit_are_plain_errors() {
    let policies = tempfile::tempdir().unwrap();
    for (name, limit) in [("FakeTimeout", "time limit exceeded"), ("FakeMemory", "memory limit exceeded")] {
        let code = format!(
            r#"return {{ name = "Custom::{name}", severity = 3, summary = "x", check = function() error("{limit}") end }}"#
        );
        write(policies.path(), &format!("{name}.lua"), &code);
    }
    let selection = Selection {
        single_policy: Some(policy_pattern("^Custom::").unwrap()),
        policy_paths: vec![policies.path().to_path_buf()],
        ..Selection::default()
    };
    let inputs = ["a.sp", "b.sp"].iter().map(|name| Input { path: name.into(), content: Some(Vec::new()) }).collect();

    let plan = Plan::for_config(inputs, &Configured::defaults(), &selection);
    assert!(plan.problems().is_empty(), "{:?}", plan.problems());
    let reports = plan.run(1).reports;

    for report in &reports {
        assert_eq!(report.failures.len(), 2, "{}: {:?}", report.path.display(), report.failures);
        for failure in &report.failures {
            assert_eq!(failure.kind, FailureKind::Error, "{}: {failure:?}", report.path.display());
            assert!(failure.message.ends_with("limit exceeded"), "{}: {failure:?}", report.path.display());
        }
    }
}

/// A native policy that sleeps on every file, longer on the first.
struct Sleeper {
    name: &'static str,
    first_file: Duration,
    every_file: Duration,
    has_slept_once: AtomicBool,
}

impl Sleeper {
    fn new(name: &'static str, first_file: Duration, every_file: Duration) -> Sleeper {
        Sleeper { name, first_file, every_file, has_slept_once: AtomicBool::new(false) }
    }
}

impl NativePolicy for Sleeper {
    fn info(&self) -> PolicyInfo {
        PolicyInfo {
            name: self.name.to_string(),
            severity: Severity::new(3).unwrap(),
            themes: BTreeSet::new(),
            summary: "sleeps".to_string(),
            description: String::new(),
            applies_to: BTreeSet::from([FileKindName::Sp, FileKindName::Inc]),
            options: Vec::new(),
            origin: PolicyOrigin::Native,
        }
    }

    fn check(&self, _file: &AnalyzedFile, _options: &BTreeMap<String, OptionValue>) -> CheckOutcome {
        let is_first = !self.has_slept_once.swap(true, Ordering::SeqCst);
        std::thread::sleep(if is_first { self.first_file } else { self.every_file });
        CheckOutcome::finished(Vec::new())
    }
}

#[test]
fn an_abandoned_worker_that_comes_back_loses_no_job() {
    let limits = Limits {
        time_per_file: Duration::from_millis(100),
        hang_grace: Duration::from_millis(100),
        ..Limits::default()
    };
    let sleepers = [
        Sleeper::new("Test::HangsOnce", Duration::from_secs(1), Duration::ZERO),
        Sleeper::new("Test::SlowEach", Duration::from_millis(20), Duration::from_millis(20)),
    ];
    let policies = sleepers
        .into_iter()
        .map(|sleeper| {
            let policy: Arc<dyn NativePolicy> = Arc::new(sleeper);
            EnabledPolicy {
                info: Arc::new(policy.info()),
                options: BTreeMap::new(),
                implementation: Implementation::Native(policy),
            }
        })
        .collect();
    let catalog = Arc::new(Catalog::load(&[], limits).0);
    let profile = Profile { catalog, policies, effective: BTreeMap::new(), force: false, limits };
    let sources = tempfile::tempdir().unwrap();
    let inputs = (0..100)
        .map(|index| {
            let path: PathBuf = sources.path().join(format!("f{index:03}.sp"));
            Input { path, content: Some(b"void f() {}\n".to_vec()) }
        })
        .collect();

    let reports = Plan::for_profile(inputs, profile).run(1).reports;

    assert_eq!(reports.len(), 100);
    for (index, report) in reports.iter().enumerate() {
        assert_eq!(report.load_error, None, "file {index}");
        let failures: Vec<&str> = report.failures.iter().map(|failure| failure.message.as_str()).collect();
        let hung_here = failures.iter().filter(|message| message.starts_with("the policy hung")).count();
        assert_eq!(hung_here, usize::from(index == 0), "file {index}: {failures:?}");
        assert!(
            report.failures.iter().all(|failure| failure.policy == "Test::HangsOnce"),
            "file {index}: {failures:?}"
        );
    }
}

#[test]
fn a_policy_that_hangs_while_loading_is_skipped() {
    let policies = tempfile::tempdir().unwrap();
    let hangs_at_top_level = r#"string.find(string.rep("a", 40), string.rep("a*", 40) .. "b")
return { name = "Custom::HangsWhileLoading", severity = 3, summary = "never loads", check = function() end }"#;
    write(policies.path(), "hangs.lua", hangs_at_top_level);
    write(policies.path(), "good.lua", GOOD);
    let selection = Selection {
        policy_paths: vec![policies.path().to_path_buf()],
        limits: Some(Limits {
            time_per_file: Duration::from_millis(100),
            hang_grace: Duration::from_millis(100),
            ..Limits::default()
        }),
        ..Selection::default()
    };

    let started = std::time::Instant::now();
    let (profile, problems) = Profile::build(&Configured::defaults(), &selection);

    assert!(started.elapsed() < Duration::from_secs(5), "loading took {:?}", started.elapsed());
    let problem_text: Vec<&str> = problems.iter().map(|problem| problem.message.as_str()).collect();
    assert_eq!(problems.len(), 1, "{problem_text:#?}");
    assert!(problem_text[0].contains("hangs.lua") && problem_text[0].contains("hung while loading"));
    assert!(profile.catalog.find("Custom::ReportCalls").is_some(), "the other custom policy still loads");
    assert!(profile.catalog.find("Custom::HangsWhileLoading").is_none());
}
