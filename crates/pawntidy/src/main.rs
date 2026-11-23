// Copyright 2026 davenonymous

//! The `pawntidy` command.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anstyle::{AnsiColor, Style as TextStyle};
use clap::{Parser, ValueEnum};
use pawn_core::ParsedFile;
use pawn_core::cli::{
    ColorWhen, ConfigCache, ConfigChoice, Configured, Input, InputSource, default_threads, parse_jobs, write_stdout,
};
use pawn_core::discover::DiscoverOptions;
use pawn_core::terminal::{printable, printable_lines};
use pawntidy::{Formatted, Style};
use serde_json::json;

const EXIT_CHANGES: u8 = 1;
const EXIT_ERROR: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

/// A formatter for SourcePawn, in the spirit of perltidy.
///
/// Indents with tabs (four columns wide) or, as a .pawntidy.toml chooses, spaces, aligns continued
/// lines with spaces after the indentation, puts opening braces at the end of their line, wraps
/// every body of a condition or loop in braces and breaks lines longer than the limit (120 columns
/// by default). A .pawntidy.toml sets the rest.
///
/// Without --write, --check or --diff the formatted file is printed. Exit status: 0 on success,
/// 1 when --check or --diff found files that would change, 2 on errors.
#[derive(Debug, Parser)]
#[command(name = "pawntidy", version, max_term_width = 100)]
struct Arguments {
    /// Files and directories to format; directories are searched for .sp and .inc files. `-`
    /// reads standard input. Without paths, the current directory is used.
    paths: Vec<PathBuf>,

    /// Rewrite the files that change, in place.
    #[arg(short, long, conflicts_with_all = ["check", "diff"])]
    write: bool,
    /// Change nothing; list the files that would change and exit with 1 if there are any.
    #[arg(short, long, conflicts_with = "diff")]
    check: bool,
    /// Change nothing; print the changes as a unified diff and exit with 1 if there are any.
    #[arg(short, long)]
    diff: bool,

    /// Use this configuration file for every file instead of the nearest .pawntidy.toml or
    /// pawntidy.toml.
    #[arg(long, value_name = "FILE", conflicts_with = "no_config")]
    config: Option<PathBuf>,
    /// Use no configuration file: the default style.
    #[arg(long)]
    no_config: bool,
    /// Print the style that applies to the first path (or the current directory; for `-` with
    /// --stdin-filename, to that name) as TOML.
    #[arg(long)]
    print_style: bool,

    /// The name standard input is formatted under: it decides the configuration file and
    /// appears in messages.
    #[arg(long, value_name = "NAME")]
    stdin_filename: Option<PathBuf>,
    /// Also format files that .gitignore and .ignore files leave out, and hidden files.
    #[arg(long)]
    no_ignore: bool,
    /// How many files to format at once, at least 1; the default is the number of processors.
    #[arg(short, long, value_name = "N", value_parser = parse_jobs)]
    jobs: Option<usize>,

    /// How --check, --diff and --write report: human, or json for tools.
    #[arg(short, long, value_name = "FORMAT", default_value = "human")]
    format: OutputFormat,
    /// Color the diff: auto (when standard output is a terminal, NO_COLOR is unset or empty and
    /// TERM is not dumb), always or never.
    #[arg(long, value_name = "WHEN", default_value = "auto")]
    color: ColorWhen,
    /// Leave out the summary line and the list of rewritten files.
    #[arg(short, long)]
    quiet: bool,
}

/// A file to format and its style.
type Job = (Input, Arc<Style>);

/// What became of one file.
struct Outcome {
    path: PathBuf,
    original: String,
    result: Result<Formatted, String>,
    /// Why --write could not write the formatted text.
    write_error: Option<String>,
}

impl Outcome {
    fn formatted(&self) -> Option<&Formatted> {
        self.result.as_ref().ok()
    }

    /// Why the file could not be formatted or written.
    fn error(&self) -> Option<&str> {
        self.result.as_ref().err().or(self.write_error.as_ref()).map(String::as_str)
    }

    /// Whether formatting changes the file and nothing failed: the file
    /// would change, or with --write, was rewritten.
    fn changes(&self) -> bool {
        self.error().is_none() && self.formatted().is_some_and(Formatted::is_changed)
    }
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    match run(&arguments) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            report_error(&message);
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn report_error(message: &str) {
    pawn_core::cli::report_error("pawntidy", message);
}

fn run(arguments: &Arguments) -> Result<u8, String> {
    let choice = ConfigChoice::from_options(arguments.config.as_deref(), arguments.no_config);
    let mut styles = ConfigCache::<Style>::new(choice).map_err(|error| error.to_string())?;
    if arguments.print_style {
        let anchor = style_anchor(arguments);
        let configured = if anchor.is_dir() { styles.for_directory(&anchor) } else { styles.for_file(&anchor) };
        write_stdout(&style_toml(&configured.map_err(|error| error.to_string())?), false)?;
        return Ok(0);
    }
    let inputs = collect_inputs(arguments)?;
    let prints_result = !(arguments.write || arguments.check || arguments.diff);
    if prints_result && inputs.len() > 1 {
        return Err(
            "printing the formatted text works on one file; use --write, --check or --diff for several".to_string()
        );
    }
    let mut jobs = Vec::with_capacity(inputs.len());
    for input in inputs {
        let style = styles.for_file(&input.path).map_err(|error| error.to_string())?.config;
        jobs.push((input, style));
    }
    let threads = arguments.jobs.unwrap_or_else(default_threads);
    let mut outcomes = format_all(jobs, threads);

    for outcome in &outcomes {
        match &outcome.result {
            Err(message) => report_error(message),
            Ok(formatted) => {
                for region in &formatted.kept {
                    let (first, last) = (region.first_line, region.last_line);
                    let lines = if first == last { format!("line {first}") } else { format!("lines {first}-{last}") };
                    eprintln!(
                        "pawntidy: warning: {}: {lines} kept as they are; {}",
                        printable(&outcome.path.display().to_string()),
                        region.reason.describe()
                    );
                }
            }
        }
    }

    if prints_result {
        if let Some(formatted) = outcomes.first().and_then(Outcome::formatted) {
            write_stdout(&formatted.text, false)?;
        } else if let Some(outcome) = outcomes.first() {
            write_stdout(&outcome.original, false)?;
        }
        return Ok(if outcomes.iter().any(|outcome| outcome.error().is_some()) { EXIT_ERROR } else { 0 });
    }

    if arguments.write {
        outcomes = outcomes.into_iter().map(|outcome| write_changed(outcome, arguments.quiet)).collect();
    }
    let had_errors = outcomes.iter().any(|outcome| outcome.error().is_some());
    let changed: Vec<&Outcome> = outcomes.iter().filter(|outcome| outcome.changes()).collect();
    if arguments.format == OutputFormat::Json {
        write_stdout(&json_report(&outcomes, arguments.diff), false)?;
    } else if arguments.diff {
        let color = arguments.color.is_enabled();
        let raw: String = changed.iter().map(|outcome| unified_diff(outcome)).collect();
        // Written to a file or a pipe, the diff stays a patch of the exact
        // bytes; shown to a person, a control character in it must not
        // command the terminal.
        let is_shown = color || std::io::stdout().is_terminal();
        let text = if is_shown { printable_lines(&raw) } else { raw };
        if color { write_stdout(&colored_diff(&text), true)? } else { write_stdout(&text, false)? }
    } else if arguments.check {
        let mut text = String::new();
        for outcome in &changed {
            text.push_str(&format!("would reformat {}\n", printable(&outcome.path.display().to_string())));
        }
        write_stdout(&text, false)?;
    }
    if !arguments.quiet && arguments.format == OutputFormat::Human {
        eprintln!("{}", summary(&outcomes, changed.len(), arguments.write));
    }
    let reports_changes = (arguments.check || arguments.diff) && !changed.is_empty();
    Ok(if had_errors {
        EXIT_ERROR
    } else if reports_changes {
        EXIT_CHANGES
    } else {
        0
    })
}

/// The path --print-style reports the style for: the first path, the name
/// of standard input for `-`, or the current directory.
fn style_anchor(arguments: &Arguments) -> PathBuf {
    match (arguments.paths.first(), &arguments.stdin_filename) {
        (None, _) => PathBuf::from("."),
        (Some(path), Some(name)) if path.as_os_str() == "-" => name.clone(),
        (Some(path), _) => path.clone(),
    }
}

fn collect_inputs(arguments: &Arguments) -> Result<Vec<Input>, String> {
    let source = InputSource::from_arguments(&arguments.paths, arguments.stdin_filename.as_deref())
        .map_err(|error| error.to_string())?;
    if arguments.write && matches!(source, InputSource::Stdin { .. }) {
        return Err("--write cannot rewrite standard input; leave it out to print the result".to_string());
    }
    let options = DiscoverOptions { respect_ignore_files: !arguments.no_ignore };
    source.collect(options, std::io::stdin().lock()).map_err(|error| error.to_string())
}

fn format_one(input: &Input, style: &Style) -> Outcome {
    let path = input.path.clone();
    let source = match input.load() {
        Ok(source) => source,
        Err(error) => {
            return Outcome { path, original: String::new(), result: Err(error.to_string()), write_error: None };
        }
    };
    let original = source.text_as_read();
    let result = pawntidy::format_parsed(&ParsedFile::parse(source), style).map_err(|error| error.to_string());
    Outcome { path, original, result, write_error: None }
}

fn format_all(jobs: Vec<Job>, threads: usize) -> Vec<Outcome> {
    let jobs: Vec<Mutex<Option<Job>>> = jobs.into_iter().map(|job| Mutex::new(Some(job))).collect();
    let outcomes: Mutex<Vec<Option<Outcome>>> = Mutex::new((0..jobs.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..threads.clamp(1, jobs.len().max(1)) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(slot) = jobs.get(index) else { break };
                    let (input, style) =
                        slot.lock().expect("no job lock is poisoned").take().expect("every job is taken once");
                    let outcome = format_one(&input, &style);
                    outcomes.lock().expect("no outcome lock is poisoned")[index] = Some(outcome);
                }
            });
        }
    });
    outcomes
        .into_inner()
        .expect("all threads have finished")
        .into_iter()
        .map(|outcome| outcome.expect("every job ran"))
        .collect()
}

/// Rewrites the file of `outcome` if it changes, through a temporary file
/// next to it, so the file is either the old or the new version, never
/// half written. A file that cannot be written comes back with its
/// `write_error`.
fn write_changed(outcome: Outcome, quiet: bool) -> Outcome {
    let Some(formatted) = outcome.formatted().filter(|_| outcome.changes()) else { return outcome };
    if let Err(message) = replace_file(&outcome.path, &formatted.text) {
        let error = format!("{}: cannot write: {message}", outcome.path.display());
        report_error(&error);
        return Outcome { write_error: Some(error), ..outcome };
    }
    if !quiet {
        eprintln!("reformatted {}", printable(&outcome.path.display().to_string()));
    }
    outcome
}

/// Replaces the file at `path` with `text`. A symbolic link is written
/// through: the temporary file is created next to the link's target and
/// renamed onto the target, so the link stays a link.
fn replace_file(path: &Path, text: &str) -> Result<(), String> {
    let target = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    let directory = target.parent().ok_or("the file has no directory")?;
    let permissions = std::fs::metadata(&target).map_err(|error| error.to_string())?.permissions();
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    temporary.write_all(text.as_bytes()).map_err(|error| error.to_string())?;
    temporary.as_file().sync_all().map_err(|error| error.to_string())?;
    std::fs::set_permissions(temporary.path(), permissions).map_err(|error| error.to_string())?;
    temporary.persist(&target).map_err(|error| error.error.to_string())?;
    Ok(())
}

fn unified_diff(outcome: &Outcome) -> String {
    let Some(formatted) = outcome.formatted() else { return String::new() };
    let name = outcome.path.display().to_string();
    let (old_name, new_name) =
        if outcome.path.is_absolute() { (name.clone(), name) } else { (format!("a/{name}"), format!("b/{name}")) };
    let diff = similar::TextDiff::from_lines(&outcome.original, &formatted.text);
    diff.unified_diff().context_radius(3).header(&old_name, &new_name).to_string()
}

/// A unified diff with its headers, hunks, additions and removals colored.
fn colored_diff(text: &str) -> String {
    let paint = |style: TextStyle, line: &str| format!("{style}{line}{style:#}");
    let mut colored = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let style = if line.starts_with("+++") || line.starts_with("---") {
            TextStyle::new().bold()
        } else if line.starts_with('+') {
            TextStyle::new().fg_color(Some(AnsiColor::Green.into()))
        } else if line.starts_with('-') {
            TextStyle::new().fg_color(Some(AnsiColor::Red.into()))
        } else if line.starts_with("@@") {
            TextStyle::new().fg_color(Some(AnsiColor::Cyan.into()))
        } else {
            colored.push_str(line);
            continue;
        };
        let (content, ending) = line.strip_suffix('\n').map_or((line, ""), |content| (content, "\n"));
        colored.push_str(&paint(style, content));
        colored.push_str(ending);
    }
    colored
}

fn json_report(outcomes: &[Outcome], with_diff: bool) -> String {
    let files: Vec<serde_json::Value> = outcomes
        .iter()
        .map(|outcome| {
            let kept: Vec<serde_json::Value> = outcome
                .formatted()
                .map(|formatted| {
                    formatted
                        .kept
                        .iter()
                        .map(|region| json!({ "first_line": region.first_line, "last_line": region.last_line, "reason": region.reason.name() }))
                        .collect()
                })
                .unwrap_or_default();
            let mut entry = json!({
                "path": outcome.path.display().to_string(),
                "changed": outcome.changes(),
                "kept": kept,
                "error": outcome.error(),
            });
            if with_diff && outcome.changes() {
                entry["diff"] = json!(unified_diff(outcome));
            }
            entry
        })
        .collect();
    let mut text =
        serde_json::to_string_pretty(&json!({ "version": 1, "files": files })).expect("JSON values always serialize");
    text.push('\n');
    text
}

fn summary(outcomes: &[Outcome], changed: usize, wrote: bool) -> String {
    let failed = outcomes.iter().filter(|outcome| outcome.error().is_some()).count();
    let files = |count: usize| if count == 1 { "1 file".to_string() } else { format!("{count} files") };
    let verb = if wrote { "reformatted" } else { "would be reformatted" };
    let mut text = format!("{} {verb}, {} left unchanged", files(changed), files(outcomes.len() - changed - failed));
    if failed > 0 {
        text.push_str(&format!(", {} failed", files(failed)));
    }
    text.push('.');
    text
}

/// The style of --print-style, after a comment that says where it comes
/// from.
fn style_toml(configured: &Configured<Style>) -> String {
    let source = match &configured.path {
        Some(path) => format!("# from {}\n", printable(&path.display().to_string())),
        None => "# the default style; no .pawntidy.toml applies\n".to_string(),
    };
    format!("{source}{}", configured.config.to_toml())
}
