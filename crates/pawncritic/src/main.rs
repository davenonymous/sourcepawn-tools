// Copyright 2026 davenonymous

//! The `pawncritic` command.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use pawn_core::cli::{ColorWhen, ConfigCache, ConfigChoice, InputSource, default_threads, parse_jobs, write_stdout};
use pawn_core::discover::DiscoverOptions;
use pawncritic::config::Config;
use pawncritic::describe;
use pawncritic::engine::{Problem, Profile, Selection, policy_pattern};
use pawncritic::output::{self, Format, HumanOptions, RenderContext};
use pawncritic::run::{ExitStatus, Plan};
use pawncritic::severity::Severity;
use pawncritic::theme::ThemeExpression;

/// A policy-based linter for SourcePawn, in the spirit of perlcritic.
///
/// Checks .sp and .inc files against policies, without compiling them. Exit status:
/// 0 without violations, 1 with violations, 2 when something could not be checked (an unreadable
/// file, a broken policy or configuration) or the command line is wrong.
#[derive(Debug, Parser)]
#[command(name = "pawncritic", version, max_term_width = 100)]
struct Arguments {
    /// Files and directories to check; directories are searched for .sp and .inc files. `-` reads
    /// standard input. Without paths, the current directory is checked.
    paths: Vec<PathBuf>,

    /// Report policies of this severity and above: 1 to 5, or gentle (5), stern (4), harsh (3),
    /// cruel (2), brutal (1). The default is 3.
    #[arg(short, long, value_name = "SEVERITY", help_heading = "Choosing policies")]
    severity: Option<Severity>,
    /// Same as --severity 5.
    #[arg(long, help_heading = "Choosing policies", conflicts_with_all = ["severity", "stern", "harsh", "cruel", "brutal"])]
    gentle: bool,
    /// Same as --severity 4.
    #[arg(long, help_heading = "Choosing policies", conflicts_with_all = ["severity", "harsh", "cruel", "brutal"])]
    stern: bool,
    /// Same as --severity 3.
    #[arg(long, help_heading = "Choosing policies", conflicts_with_all = ["severity", "cruel", "brutal"])]
    harsh: bool,
    /// Same as --severity 2.
    #[arg(long, help_heading = "Choosing policies", conflicts_with_all = ["severity", "brutal"])]
    cruel: bool,
    /// Same as --severity 1.
    #[arg(long, help_heading = "Choosing policies", conflicts_with = "severity")]
    brutal: bool,
    /// Only run policies whose themes match the expression, such as `modern || philosophy` or
    /// `core && !tidy`.
    #[arg(long, value_name = "EXPRESSION", help_heading = "Choosing policies")]
    theme: Option<String>,
    /// Also run the policies whose names match this case-insensitive pattern, whatever their
    /// severity and themes. Repeatable.
    #[arg(long, value_name = "PATTERN", help_heading = "Choosing policies")]
    include: Vec<String>,
    /// Never run the policies whose names match this case-insensitive pattern. Repeatable.
    #[arg(long, value_name = "PATTERN", help_heading = "Choosing policies")]
    exclude: Vec<String>,
    /// Run only the policies whose names match this case-insensitive pattern, whatever else is
    /// configured.
    #[arg(long, value_name = "PATTERN", help_heading = "Choosing policies")]
    single_policy: Option<String>,
    /// Load the custom policies (*.lua) below this directory too. Repeatable.
    #[arg(long, value_name = "DIRECTORY", help_heading = "Choosing policies")]
    policy_path: Vec<PathBuf>,
    /// Ignore `no critic` annotations.
    #[arg(long, help_heading = "Choosing policies")]
    force: bool,
    /// Use this configuration file for every file instead of the nearest .pawncritic.toml.
    #[arg(long, value_name = "FILE", help_heading = "Choosing policies", conflicts_with = "no_config")]
    config: Option<PathBuf>,
    /// Use no configuration file.
    #[arg(long, help_heading = "Choosing policies")]
    no_config: bool,

    /// The output format: human, gcc, json, lsp, sarif, checkstyle or github.
    #[arg(short, long, value_name = "FORMAT", default_value = "human", value_parser = parse_format, help_heading = "Output")]
    format: Format,
    /// Write each violation with this template instead of a format; see --help-template.
    #[arg(long, value_name = "TEMPLATE", help_heading = "Output", conflicts_with = "format")]
    template: Option<String>,
    /// Color the human format: auto (when standard output is a terminal, NO_COLOR is unset or empty
    /// and TERM is not dumb), always or never.
    #[arg(long, value_name = "WHEN", default_value = "auto", help_heading = "Output")]
    color: ColorWhen,
    /// Leave the source lines out of the human format.
    #[arg(long, help_heading = "Output")]
    no_snippets: bool,
    /// Add the number of violations per policy to the human format.
    #[arg(long, help_heading = "Output")]
    statistics: bool,
    /// Leave the summary line out of the human format.
    #[arg(short, long, help_heading = "Output")]
    quiet: bool,

    /// The name standard input is checked under: it decides the file kind and the configuration
    /// file, and appears in the output.
    #[arg(long, value_name = "NAME", help_heading = "Input")]
    stdin_filename: Option<PathBuf>,
    /// Also check files that .gitignore and .ignore files leave out, and hidden files.
    #[arg(long, help_heading = "Input")]
    no_ignore: bool,
    /// How many files to check at once, at least 1; the default is the number of processors.
    #[arg(short, long, value_name = "N", value_parser = parse_jobs, help_heading = "Input")]
    jobs: Option<usize>,

    /// List the policies with their severity and themes; chosen policies are marked with `*`.
    #[arg(long, help_heading = "Information")]
    list: bool,
    /// List the themes of all policies.
    #[arg(long, help_heading = "Information")]
    list_themes: bool,
    /// Explain a policy: what it reports, why, and its options.
    #[arg(long, value_name = "POLICY", help_heading = "Information")]
    explain: Option<String>,
    /// Print a .pawncritic.toml that lists every policy and option with its default.
    #[arg(long, help_heading = "Information")]
    print_config_template: bool,
    /// Print the syntax tree of a file as policies see it, for writing policies and queries.
    #[arg(long, value_name = "FILE", help_heading = "Information")]
    dump_tree: Option<PathBuf>,
    /// With --dump-tree, also show anonymous nodes such as keywords and punctuation.
    #[arg(long, help_heading = "Information", requires = "dump_tree")]
    anonymous: bool,
    /// Explain the placeholders of --template.
    #[arg(long, help_heading = "Information")]
    help_template: bool,
}

fn parse_format(name: &str) -> Result<Format, String> {
    Format::parse(name).ok_or_else(|| format!("unknown format `{name}`; use one of {}", Format::NAMES.join(", ")))
}

fn main() -> ExitCode {
    let arguments = Arguments::parse();
    let status = run(arguments).unwrap_or_else(|message| {
        report_error(&message);
        ExitStatus::Errors
    });
    ExitCode::from(status.code())
}

fn report_error(message: &str) {
    pawn_core::cli::report_error("pawncritic", message);
}

impl Arguments {
    fn severity(&self) -> Option<Severity> {
        let flags = [(self.gentle, 5), (self.stern, 4), (self.harsh, 3), (self.cruel, 2), (self.brutal, 1)];
        let from_flag = flags.iter().find(|(set, _)| *set).and_then(|(_, level)| Severity::new(*level));
        self.severity.or(from_flag)
    }

    fn selection(&self) -> Result<Selection, String> {
        let theme = self.theme.as_deref().map(ThemeExpression::parse).transpose().map_err(|error| error.to_string())?;
        let patterns =
            |patterns: &[String]| patterns.iter().map(|pattern| policy_pattern(pattern)).collect::<Result<Vec<_>, _>>();
        Ok(Selection {
            severity: self.severity(),
            theme,
            include: patterns(&self.include)?,
            exclude: patterns(&self.exclude)?,
            single_policy: self.single_policy.as_deref().map(policy_pattern).transpose()?,
            force: self.force,
            policy_paths: self.policy_path.clone(),
            limits: None,
        })
    }
}

fn run(arguments: Arguments) -> Result<ExitStatus, String> {
    if arguments.help_template {
        let text = format!(
            "Placeholders of --template: {}.\nExample: --template '%f:%l:%c: %m [%p]'\n",
            output::TEMPLATE_HELP
        );
        write_stdout(&text, false)?;
        return Ok(ExitStatus::Clean);
    }
    if let Some(path) = &arguments.dump_tree {
        let source = pawn_core::SourceFile::read(path).map_err(|error| error.to_string())?;
        write_stdout(&pawn_core::dump::dump_tree(&pawn_core::ParsedFile::parse(source), arguments.anonymous), false)?;
        return Ok(ExitStatus::Clean);
    }
    let format = match &arguments.template {
        Some(template) => {
            output::check_template(template)?;
            Format::Template(template.clone())
        }
        None => arguments.format.clone(),
    };
    let selection = arguments.selection()?;
    let choice = ConfigChoice::from_options(arguments.config.as_deref(), arguments.no_config);
    let mut configs = ConfigCache::<Config>::new(choice).map_err(|error| error.to_string())?;

    if arguments.list || arguments.list_themes || arguments.explain.is_some() || arguments.print_config_template {
        let config = configs.for_directory(Path::new(".")).map_err(|error| error.to_string())?;
        let (profile, problems) = Profile::build(&config, &selection);
        report_problems(&problems);
        write_stdout(&information(&arguments, &profile)?, false)?;
        return Ok(if problems.is_empty() { ExitStatus::Clean } else { ExitStatus::Errors });
    }

    let source = InputSource::from_arguments(&arguments.paths, arguments.stdin_filename.as_deref())
        .map_err(|error| error.to_string())?;
    let options = DiscoverOptions { respect_ignore_files: !arguments.no_ignore };
    let inputs = source.collect(options, std::io::stdin().lock()).map_err(|error| error.to_string())?;
    let plan = Plan::build(inputs, &mut configs, &selection).map_err(|error| error.to_string())?;
    report_problems(plan.problems());
    let result = plan.run(arguments.jobs.unwrap_or_else(default_threads));

    for message in result.file_errors() {
        report_error(&message);
    }
    report_problems(&result.closing_errors);
    let problems: Vec<Problem> = result.problems.iter().chain(&result.closing_errors).cloned().collect();
    let context = RenderContext {
        policies: &result.policies,
        problems: &problems,
        human: HumanOptions {
            color: arguments.color.is_enabled(),
            snippets: !arguments.no_snippets,
            statistics: arguments.statistics,
            quiet: arguments.quiet,
        },
        files_checked: result.reports.len(),
    };
    let text = output::render(&format, &result.reports, &context);
    write_stdout(&text, context.human.color)?;
    Ok(result.exit_status())
}

fn report_problems(problems: &[Problem]) {
    for problem in problems {
        report_error(&problem.message);
    }
}

/// The text of --list, --list-themes, --explain and
/// --print-config-template, in this order.
fn information(arguments: &Arguments, profile: &Profile) -> Result<String, String> {
    let mut text = String::new();
    if arguments.list {
        text.push_str(&describe::list(profile));
    }
    if arguments.list_themes {
        text.push_str(&describe::themes(&profile.catalog));
    }
    if let Some(name) = &arguments.explain {
        text.push_str(&describe::explain(profile, name)?);
    }
    if arguments.print_config_template {
        text.push_str(&describe::config_template(&profile.catalog));
    }
    Ok(text)
}
