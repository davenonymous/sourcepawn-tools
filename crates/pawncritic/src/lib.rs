// Copyright 2026 davenonymous

//! pawncritic: a policy-based linter for SourcePawn, in the spirit of
//! perlcritic. Policies are Lua scripts (see `docs/writing-policies.md`)
//! or, for the few that need more, Rust ([`native`]). The [`engine`] loads
//! them, chooses those a run needs and runs each in a sandbox of its own;
//! a [`run::Plan`] gives every file the profile of its configuration, and
//! [`describe`] writes what the command line tells about the policies.

pub mod analyzed;
pub mod annotations;
pub mod builtin;
pub mod check;
pub mod config;
pub mod describe;
pub mod engine;
pub mod lua;
pub mod native;
pub mod output;
pub mod policy;
pub mod run;
pub mod severity;
pub mod theme;
