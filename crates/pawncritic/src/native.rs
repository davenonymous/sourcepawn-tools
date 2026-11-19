// Copyright 2026 davenonymous

//! Policies written in Rust: those that need more than the Lua API gives,
//! such as running pawntidy.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use pawn_core::cli::ConfigCache;
use pawntidy::Style;

use crate::analyzed::AnalyzedFile;
use crate::check::{CheckOutcome, Failure, Report};
use crate::policy::{FileKindName, OptionSpec, OptionValue, PolicyInfo, PolicyOrigin};
use crate::severity::Severity;

/// A policy implemented in Rust.
pub trait NativePolicy: Send + Sync {
    fn info(&self) -> PolicyInfo;
    fn check(&self, file: &AnalyzedFile, options: &BTreeMap<String, OptionValue>) -> CheckOutcome;
}

/// Every native policy.
pub fn all() -> Vec<Box<dyn NativePolicy>> {
    vec![Box::new(RequireParsableCode), Box::new(RequireTidyCode::new())]
}

fn info(
    name: &str,
    severity: u8,
    themes: &[&str],
    summary: &str,
    description: &str,
    options: Vec<OptionSpec>,
) -> PolicyInfo {
    PolicyInfo {
        name: name.to_string(),
        severity: Severity::new(i64::from(severity)).expect("native policies have valid severities"),
        themes: themes.iter().map(|theme| theme.to_string()).collect(),
        summary: summary.to_string(),
        description: description.to_string(),
        applies_to: BTreeSet::from([FileKindName::Sp, FileKindName::Inc]),
        options,
        origin: PolicyOrigin::Native,
    }
}

fn integer_option(options: &BTreeMap<String, OptionValue>, name: &str) -> i64 {
    match options.get(name) {
        Some(OptionValue::Integer(value)) => *value,
        other => unreachable!("option {name} is a declared integer, got {other:?}"),
    }
}

/// Reports what the parser could not understand.
pub struct RequireParsableCode;

impl NativePolicy for RequireParsableCode {
    fn info(&self) -> PolicyInfo {
        info(
            "Core::RequireParsableCode",
            5,
            &["core", "bugs"],
            "the parser cannot read this code",
            "pawncritic reads code with the tree-sitter SourcePawn grammar, without compiling it. Where the\n\
             grammar cannot place the code, the policies see an error node instead of the real structure and\n\
             may miss violations or report wrong ones. Usually the code does not compile either; sometimes a\n\
             macro or a rare construct is valid but beyond the grammar. Then silence this policy for the line\n\
             with a `// no critic (RequireParsableCode)` annotation.",
            vec![OptionSpec {
                name: "max_reports".to_string(),
                default: OptionValue::Integer(5),
                description: "The most syntax errors reported per file; follow-up errors add little.".to_string(),
            }],
        )
    }

    fn check(&self, file: &AnalyzedFile, options: &BTreeMap<String, OptionValue>) -> CheckOutcome {
        let limit = usize::try_from(integer_option(options, "max_reports")).unwrap_or(0);
        let errors = file.parsed.syntax.errors(file.text());
        CheckOutcome::finished(
            errors
                .into_iter()
                .take(limit)
                .map(|error| Report { span: error.span, message: Some(error.message), hint: None })
                .collect(),
        )
    }
}

/// Reports files pawntidy would change. Shared by the worker threads, it
/// finds each `.pawntidy.toml` once per run.
pub struct RequireTidyCode {
    styles: Mutex<ConfigCache<Style>>,
}

impl RequireTidyCode {
    pub fn new() -> RequireTidyCode {
        RequireTidyCode { styles: Mutex::new(ConfigCache::nearest()) }
    }

    /// The style of the `.pawntidy.toml` nearest to `file`.
    fn style_for(&self, file: &Path) -> Result<Arc<Style>, String> {
        let mut styles = self.styles.lock().unwrap_or_else(PoisonError::into_inner);
        styles.for_file(file).map(|configured| configured.config).map_err(|error| error.to_string())
    }

    /// The report on the first line pawntidy would change, if it would
    /// change any.
    fn untidy_report(&self, file: &AnalyzedFile) -> Result<Option<Report>, String> {
        let source = &file.parsed.source;
        let style = self.style_for(source.name())?;
        let formatted = pawntidy::format_parsed(&file.parsed, &style).map_err(|error| error.to_string())?;
        let Some(line) = formatted.first_changed_line else { return Ok(None) };
        let lines = source.lines();
        let line = line.min(lines.line_count());
        let span = lines.line_span(source.text(), line);
        Ok(Some(Report {
            span: pawn_core::Span::new(span.start, span.start),
            message: Some(format!("pawntidy would change this file from line {line} on")),
            hint: Some(format!("run `pawntidy --write {}`", source.name().display())),
        }))
    }
}

impl Default for RequireTidyCode {
    fn default() -> RequireTidyCode {
        RequireTidyCode::new()
    }
}

impl NativePolicy for RequireTidyCode {
    fn info(&self) -> PolicyInfo {
        info(
            "CodeLayout::RequireTidyCode",
            1,
            &["core", "layout", "tidy"],
            "the file is not formatted as pawntidy formats it",
            "Formatting by hand drifts; a formatter keeps every file the same. This policy runs pawntidy\n\
             with the `.pawntidy.toml` that applies to the file and reports the first line pawntidy would\n\
             change. Run `pawntidy --write` on the file to fix it.",
            Vec::new(),
        )
    }

    fn check(&self, file: &AnalyzedFile, _options: &BTreeMap<String, OptionValue>) -> CheckOutcome {
        match self.untidy_report(file) {
            Ok(report) => CheckOutcome::finished(report.into_iter().collect()),
            Err(message) => CheckOutcome::failed(Failure::error(message)),
        }
    }
}
