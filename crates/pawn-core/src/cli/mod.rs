// Copyright 2026 davenonymous

//! What the command lines of pawncritic and pawntidy share: the files a
//! run works on ([`input`]), the configuration files that apply to them
//! ([`config`]), colors, standard output, error messages and the number
//! of threads.

pub mod config;
pub mod input;

use std::ffi::OsStr;
use std::io::{IsTerminal, Write};

pub use config::{ConfigCache, ConfigChoice, ConfigError, ConfigFile, Configured};
pub use input::{Input, InputError, InputSource};

use crate::terminal::printable_lines;

/// When to color the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorWhen {
    /// When standard output is a terminal, `NO_COLOR` is unset or empty and
    /// `TERM` is not `dumb`.
    Auto,
    Always,
    Never,
}

impl ColorWhen {
    /// Whether this run colors what it writes to standard output.
    pub fn is_enabled(self) -> bool {
        match self {
            ColorWhen::Always => true,
            ColorWhen::Never => false,
            ColorWhen::Auto => is_auto_color_enabled(
                std::io::stdout().is_terminal(),
                std::env::var_os("NO_COLOR").as_deref(),
                std::env::var_os("TERM").as_deref(),
            ),
        }
    }
}

/// The decision of [`ColorWhen::Auto`] from the facts it depends on.
fn is_auto_color_enabled(stdout_is_terminal: bool, no_color: Option<&OsStr>, term: Option<&OsStr>) -> bool {
    stdout_is_terminal && no_color.is_none_or(OsStr::is_empty) && term.is_none_or(|term| term != "dumb")
}

/// Writes `text` to standard output. With `color`, the color codes in it
/// are written so the terminal shows them, on Windows too; without, the
/// text is written byte for byte. A reader that went away (a closed pipe,
/// as with `| head`) is no error.
pub fn write_stdout(text: &str, color: bool) -> Result<(), String> {
    let stdout = std::io::stdout().lock();
    let written = if color {
        write_all(&mut anstream::AutoStream::new(stdout, anstream::ColorChoice::Always), text)
    } else {
        write_all(&mut { stdout }, text)
    };
    match written {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(format!("cannot write the output: {error}")),
    }
}

fn write_all(stream: &mut impl Write, text: &str) -> std::io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}

/// Writes an error of the command `tool` to standard error, made
/// printable: it may quote paths, code and policy messages. Its line
/// breaks stay (a TOML error spans several lines).
pub fn report_error(tool: &str, message: &str) {
    eprintln!("{tool}: error: {}", printable_lines(message));
}

/// The message a panic was raised with.
pub fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = panic.downcast_ref::<&str>() {
        return text.to_string();
    }
    if let Some(text) = panic.downcast_ref::<String>() {
        return text.clone();
    }
    "unknown panic".to_string()
}

/// How many files a run works on at once when the command line does not
/// say: the number of processors.
pub fn default_threads() -> usize {
    std::thread::available_parallelism().map_or(1, usize::from)
}

/// The value of `--jobs`: how many files to work on at once, at least 1.
pub fn parse_jobs(text: &str) -> Result<usize, String> {
    let jobs: usize = text.parse().map_err(|error: std::num::ParseIntError| error.to_string())?;
    if jobs == 0 {
        return Err("must be at least 1".to_string());
    }
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_colors_only_a_capable_terminal() {
        let some = |text: &'static str| Some(OsStr::new(text));
        assert!(is_auto_color_enabled(true, None, some("xterm")));
        assert!(is_auto_color_enabled(true, some(""), None));
        assert!(!is_auto_color_enabled(false, None, some("xterm")));
        assert!(!is_auto_color_enabled(true, some("1"), some("xterm")));
        assert!(!is_auto_color_enabled(true, None, some("dumb")));
    }

    #[test]
    fn jobs_are_at_least_one() {
        assert_eq!(parse_jobs("3"), Ok(3));
        assert_eq!(parse_jobs("0"), Err("must be at least 1".to_string()));
        assert!(parse_jobs("many").is_err());
    }

    #[test]
    fn panic_messages_are_read_from_both_payload_types() {
        let borrowed = std::panic::catch_unwind(|| panic!("plain")).unwrap_err();
        let owned = std::panic::catch_unwind(|| panic!("formatted {}", 1)).unwrap_err();
        assert_eq!(panic_message(&*borrowed), "plain");
        assert_eq!(panic_message(&*owned), "formatted 1");
    }
}
