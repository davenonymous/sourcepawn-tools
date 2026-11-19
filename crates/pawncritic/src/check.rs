// Copyright 2026 davenonymous

//! What checking a file with one policy gives, whatever the policy is
//! written in: its reports, and why it stopped early if it did. Also the
//! two implementations of a policy and their instances in a worker
//! thread, which turn a crash or an exhausted limit into a policy that is
//! skipped from then on.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;

use pawn_core::Span;
use pawn_core::cli::panic_message;
use pawn_core::terminal::printable;

use crate::analyzed::AnalyzedFile;
use crate::lua::sandbox::Limits;
use crate::lua::{LuaPolicy, PolicySource};
use crate::native::NativePolicy;
use crate::policy::OptionValue;

/// A violation as a policy reports it, before the engine places it and
/// applies `no critic` annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub span: Span,
    /// The message; the policy's summary when absent.
    pub message: Option<String>,
    pub hint: Option<String>,
}

/// Why a policy did not finish, which decides whether it runs again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// The policy raised an error or misused the API. It runs again on the
    /// next file.
    Error,
    /// The policy ran out of its time or memory. It is skipped on the
    /// remaining files of its thread.
    Exhausted,
    /// The policy panicked. It is skipped on the remaining files of its
    /// thread.
    Crashed,
    /// The watchdog gave up on the policy, stuck inside one library call or
    /// native code, while it checked a file or while its file was loaded.
    /// It is skipped from then on.
    Hung,
    /// The policy did not run on the file because of an earlier failure.
    Skipped,
}

/// A policy that could not finish, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
}

/// The longest failure message, in bytes; longer ones are cut, since a
/// policy's error can be any text it builds.
pub const MAX_FAILURE_MESSAGE: usize = 1_000;

impl Failure {
    /// A failure with `message` made one printable line (line breaks and
    /// other control characters replaced) and cut after
    /// [`MAX_FAILURE_MESSAGE`] bytes, with `...` at the cut.
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Failure {
        Failure { kind, message: bounded_message(&message.into()) }
    }

    /// An error of the policy itself.
    pub fn error(message: impl Into<String>) -> Failure {
        Failure::new(FailureKind::Error, message)
    }
}

/// `message` on one printable line, at most [`MAX_FAILURE_MESSAGE`] bytes
/// before a `...` that marks a cut.
fn bounded_message(message: &str) -> String {
    let line = printable(message);
    if line.len() <= MAX_FAILURE_MESSAGE {
        return line.into_owned();
    }
    let cut = (0..=MAX_FAILURE_MESSAGE).rev().find(|&index| line.is_char_boundary(index)).unwrap_or(0);
    format!("{}...", &line[..cut])
}

/// What checking a file with one policy gave: the reports made before it
/// finished or stopped, and why it stopped.
#[derive(Debug, Default)]
pub struct CheckOutcome {
    pub reports: Vec<Report>,
    pub failure: Option<Failure>,
}

impl CheckOutcome {
    /// A check that finished with `reports`.
    pub fn finished(reports: Vec<Report>) -> CheckOutcome {
        CheckOutcome { reports, failure: None }
    }

    /// A check that stopped before it reported anything.
    pub fn failed(failure: Failure) -> CheckOutcome {
        CheckOutcome { reports: Vec::new(), failure: Some(failure) }
    }
}

/// How a policy is implemented.
#[derive(Clone)]
pub enum Implementation {
    Lua(PolicySource),
    Native(Arc<dyn NativePolicy>),
}

impl Implementation {
    /// The policy ready to check files in one thread: a Lua policy gets a
    /// state of its own; one that cannot be loaded is disabled.
    pub(crate) fn instantiate(&self, options: &BTreeMap<String, OptionValue>, limits: Limits) -> Instance {
        match self {
            Implementation::Native(policy) => Instance::Native(policy.clone()),
            Implementation::Lua(source) => match LuaPolicy::load(source, options, limits) {
                Ok(policy) => Instance::Lua(policy),
                Err(failure) => {
                    Instance::Disabled(Failure::new(failure.kind, format!("cannot load: {}", failure.message)))
                }
            },
        }
    }
}

/// A policy in one thread.
pub(crate) enum Instance {
    Lua(LuaPolicy),
    Native(Arc<dyn NativePolicy>),
    /// The policy cannot run in this thread any more; every check gives
    /// this failure.
    Disabled(Failure),
}

impl Instance {
    /// Checks `file`. A crash or an exhausted limit disables the policy in
    /// this thread, so a broken policy costs its time once.
    pub(crate) fn check(&mut self, file: &Rc<AnalyzedFile>, options: &BTreeMap<String, OptionValue>) -> CheckOutcome {
        let checked = catch_unwind(AssertUnwindSafe(|| match self {
            Instance::Lua(policy) => policy.check(file),
            Instance::Native(policy) => policy.check(file, options),
            Instance::Disabled(failure) => CheckOutcome::failed(failure.clone()),
        }));
        let outcome = match checked {
            Ok(outcome) => outcome,
            Err(panic) => {
                let message = panic_message(&*panic);
                let skip = format!("skipped: the policy crashed on an earlier file ({message})");
                *self = Instance::Disabled(Failure::new(FailureKind::Skipped, skip));
                return CheckOutcome::failed(Failure::new(FailureKind::Crashed, format!("crashed: {message}")));
            }
        };
        if let Some(failure) = outcome.failure.as_ref().filter(|failure| failure.kind == FailureKind::Exhausted) {
            let skip = format!("skipped: the policy ran out of its limits on an earlier file ({})", failure.message);
            *self = Instance::Disabled(Failure::new(FailureKind::Skipped, skip));
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_messages_are_one_bounded_line() {
        let forged = Failure::error("ok\n::error file=a.sp::forged");
        assert_eq!(forged.message, "ok\u{FFFD}::error file=a.sp::forged");
        let huge = Failure::error("\u{e4}".repeat(MAX_FAILURE_MESSAGE));
        assert_eq!(huge.message.len(), MAX_FAILURE_MESSAGE + "...".len());
        assert!(huge.message.ends_with("\u{e4}..."));
    }
}
