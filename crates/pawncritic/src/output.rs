// Copyright 2026 davenonymous

//! Writing the violations: for people (`human`), for compilers' error
//! parsers (`gcc`), and for tools (`json`, `lsp`, `sarif`, `checkstyle`,
//! `github`, or a `--template` of your own). `docs/output-formats.md`
//! describes every format.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Component, Path};
use std::sync::Arc;

use anstyle::{AnsiColor, Style};
use pawn_core::terminal::printable;
use serde_json::{Value, json};

use crate::engine::{FileReport, Problem, Violation};
use crate::policy::PolicyInfo;
use crate::severity::{DiagnosticLevel, Severity};

/// The output formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Human,
    Gcc,
    Json,
    Lsp,
    Sarif,
    Checkstyle,
    Github,
    Template(String),
}

impl Format {
    pub const NAMES: &'static [&'static str] = &["human", "gcc", "json", "lsp", "sarif", "checkstyle", "github"];

    pub fn parse(name: &str) -> Option<Format> {
        Some(match name {
            "human" => Format::Human,
            "gcc" => Format::Gcc,
            "json" => Format::Json,
            "lsp" => Format::Lsp,
            "sarif" => Format::Sarif,
            "checkstyle" => Format::Checkstyle,
            "github" => Format::Github,
            _ => return None,
        })
    }
}

/// How the human format looks.
#[derive(Debug, Clone, Copy)]
pub struct HumanOptions {
    pub color: bool,
    /// Show the source line under each violation.
    pub snippets: bool,
    /// Add a table of the violations per policy.
    pub statistics: bool,
    /// Leave out the summary line.
    pub quiet: bool,
}

/// Everything a format may need besides the reports.
pub struct RenderContext<'a> {
    pub policies: &'a [Arc<PolicyInfo>],
    /// The problems found before checking, such as a custom policy that
    /// could not be loaded.
    pub problems: &'a [Problem],
    pub human: HumanOptions,
    pub files_checked: usize,
}

/// The output of `reports` in `format`.
pub fn render(format: &Format, reports: &[FileReport], context: &RenderContext<'_>) -> String {
    match format {
        Format::Human => human(reports, context),
        Format::Gcc => gcc(reports),
        Format::Json => json_output(reports),
        Format::Lsp => lsp(reports),
        Format::Sarif => sarif(reports, context),
        Format::Checkstyle => checkstyle(reports),
        Format::Github => github(reports),
        Format::Template(template) => templated(reports, template),
    }
}

fn violations(reports: &[FileReport]) -> impl Iterator<Item = (&FileReport, &Violation)> {
    reports.iter().flat_map(|report| report.violations.iter().map(move |violation| (report, violation)))
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

/// A path as URIs and most tools want it: forward slashes.
fn slashed_path(path: &Path) -> String {
    display_path(path).replace('\\', "/")
}

struct Palette {
    enabled: bool,
}

impl Palette {
    fn paint(&self, style: Style, text: &str) -> String {
        if self.enabled { format!("{style}{text}{style:#}") } else { text.to_string() }
    }

    fn severity_style(severity: Severity) -> Style {
        let color = match severity.diagnostic_level() {
            DiagnosticLevel::Error => AnsiColor::Red,
            DiagnosticLevel::Warning => AnsiColor::Yellow,
            DiagnosticLevel::Info => AnsiColor::Cyan,
            DiagnosticLevel::Hint => AnsiColor::Blue,
        };
        Style::new().fg_color(Some(color.into())).bold()
    }
}

const TAB_DISPLAY: &str = "    ";

/// The format for people on a terminal. Everything that comes from a file
/// (paths, source lines, messages that quote code, hints) is made
/// [`printable`], so a control character in it cannot command the
/// terminal.
fn human(reports: &[FileReport], context: &RenderContext<'_>) -> String {
    let palette = Palette { enabled: context.human.color };
    let mut output = String::new();
    for (report, violation) in violations(reports) {
        let path = display_path(&report.path);
        let location = format!("{}:{}:{}:", printable(&path), violation.start.line, violation.start.column);
        let severity = format!("[{}]", violation.severity);
        let _ = writeln!(
            output,
            "{} {} {}  {}",
            palette.paint(Style::new().bold(), &location),
            palette.paint(Palette::severity_style(violation.severity), &severity),
            printable(&violation.message),
            palette.paint(Style::new().dimmed(), &printable(&violation.policy)),
        );
        if context.human.snippets {
            write_snippet(&mut output, report, violation, &palette);
        }
        if let Some(hint) = &violation.hint {
            let _ = writeln!(
                output,
                "      {} {}",
                palette.paint(Style::new().fg_color(Some(AnsiColor::Green.into())), "= hint:"),
                printable(hint)
            );
        }
        if context.human.snippets || violation.hint.is_some() {
            output.push('\n');
        }
    }
    if context.human.statistics {
        write_statistics(&mut output, reports);
    }
    if !context.human.quiet {
        output.push_str(&summary(reports, context.files_checked));
        output.push('\n');
    }
    output
}

/// The text of a piece of a source line as the snippet shows it: tabs
/// expanded, control characters replaced. The line, the part before the
/// span and the span go through it alike, so the carets stay aligned.
fn shown_source(text: &str) -> String {
    printable(&text.replace('\t', TAB_DISPLAY)).into_owned()
}

/// The violation's first line, with a gutter and carets under the span.
fn write_snippet(output: &mut String, report: &FileReport, violation: &Violation, palette: &Palette) {
    let Some(source) = &report.source else { return };
    let line_number = violation.start.line;
    if line_number > source.lines().line_count() {
        return;
    }
    let line_span = source.lines().line_span(source.text(), line_number);
    let line_text = &source.text()[line_span.range()];
    let shown = shown_source(line_text);
    let span_end = violation.span.end.min(line_span.end).max(violation.span.start);
    let before = &line_text[..violation.span.start.saturating_sub(line_span.start).min(line_text.len())];
    let covered = &source.text()[violation.span.start.min(line_span.end)..span_end];
    let indent = unicode_width::UnicodeWidthStr::width(shown_source(before).as_str());
    let width = unicode_width::UnicodeWidthStr::width(shown_source(covered).as_str()).max(1);
    let gutter = format!("{line_number:>5} | ");
    let empty_gutter = format!("{:>5} | ", "");
    let _ = writeln!(output, "{}{}", palette.paint(Style::new().dimmed(), &gutter), shown);
    let carets = "^".repeat(width);
    let _ = writeln!(
        output,
        "{}{}{}",
        palette.paint(Style::new().dimmed(), &empty_gutter),
        " ".repeat(indent),
        palette.paint(Palette::severity_style(violation.severity), &carets)
    );
}

fn write_statistics(output: &mut String, reports: &[FileReport]) {
    let mut per_policy: BTreeMap<(std::cmp::Reverse<Severity>, &str), usize> = BTreeMap::new();
    for (_, violation) in violations(reports) {
        *per_policy.entry((std::cmp::Reverse(violation.severity), violation.policy.as_str())).or_default() += 1;
    }
    if per_policy.is_empty() {
        return;
    }
    output.push_str("Violations per policy:\n");
    for ((severity, policy), count) in per_policy {
        let _ = writeln!(output, "  {count:>5}  [{}] {policy}", severity.0);
    }
    output.push('\n');
}

/// One line on how many violations were found where.
pub fn summary(reports: &[FileReport], files_checked: usize) -> String {
    let total: usize = reports.iter().map(|report| report.violations.len()).sum();
    let files_with_violations = reports.iter().filter(|report| !report.violations.is_empty()).count();
    let files = plural(files_checked, "file", "files");
    if total == 0 {
        return format!("No violations in {files}.");
    }
    let mut per_severity: BTreeMap<std::cmp::Reverse<Severity>, usize> = BTreeMap::new();
    for (_, violation) in violations(reports) {
        *per_severity.entry(std::cmp::Reverse(violation.severity)).or_default() += 1;
    }
    let breakdown: Vec<String> =
        per_severity.iter().map(|(severity, count)| format!("{count} of severity {}", severity.0)).collect();
    format!(
        "{} in {} of {files} ({}).",
        plural(total, "violation", "violations"),
        files_with_violations,
        breakdown.join(", ")
    )
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn gcc_level(severity: Severity) -> &'static str {
    match severity.diagnostic_level() {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info | DiagnosticLevel::Hint => "note",
    }
}

fn gcc(reports: &[FileReport]) -> String {
    let mut output = String::new();
    for (report, violation) in violations(reports) {
        let _ = writeln!(
            output,
            "{}:{}:{}: {}: {} [{}]",
            display_path(&report.path),
            violation.start.line,
            violation.start.column,
            gcc_level(violation.severity),
            violation.message,
            violation.policy
        );
    }
    output
}

fn violation_json(report: &FileReport, violation: &Violation) -> Value {
    json!({
        "file": display_path(&report.path),
        "policy": violation.policy,
        "severity": violation.severity.level(),
        "level": violation.severity.diagnostic_level().name(),
        "message": violation.message,
        "hint": violation.hint,
        "line": violation.start.line,
        "column": violation.start.column,
        "end_line": violation.end.line,
        "end_column": violation.end.column,
        "start_byte": violation.span.start,
        "end_byte": violation.span.end,
    })
}

/// Load errors and policy failures, for the machine formats.
fn errors_json(reports: &[FileReport]) -> Vec<Value> {
    let mut errors = Vec::new();
    for report in reports {
        if let Some(message) = &report.load_error {
            errors.push(json!({ "file": display_path(&report.path), "policy": null, "message": message }));
        }
        for failure in &report.failures {
            errors.push(
                json!({ "file": display_path(&report.path), "policy": failure.policy, "message": failure.message }),
            );
        }
    }
    errors
}

fn json_output(reports: &[FileReport]) -> String {
    let violations: Vec<Value> =
        violations(reports).map(|(report, violation)| violation_json(report, violation)).collect();
    let document = json!({ "version": 1, "violations": violations, "errors": errors_json(reports) });
    let mut text = serde_json::to_string_pretty(&document).expect("JSON values always serialize");
    text.push('\n');
    text
}

fn lsp_severity(severity: Severity) -> u8 {
    match severity.diagnostic_level() {
        DiagnosticLevel::Error => 1,
        DiagnosticLevel::Warning => 2,
        DiagnosticLevel::Info => 3,
        DiagnosticLevel::Hint => 4,
    }
}

/// The `file://` URI of `path`, made absolute against the working
/// directory.
pub fn file_uri(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    uri_of_slashed_path(&slashed_path(&absolute))
}

/// The `file://` URI of an absolute path written with forward slashes:
/// `/home/a.sp`, `C:/a.sp` or the network path `//server/share/a.sp`.
fn uri_of_slashed_path(path: &str) -> String {
    let (authority, path) = match path.strip_prefix("//") {
        Some(network) => network.split_once('/').map_or((network, ""), |(host, rest)| (host, rest)),
        None => ("", path.trim_start_matches('/')),
    };
    format!("file://{authority}/{}", percent_encoded(path, b"/-._~:"))
}

/// `path` with every byte outside the unreserved characters and `plain`
/// percent-encoded, as a URI path needs it.
fn percent_encoded(path: &str, plain: &[u8]) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || plain.contains(&byte) {
            encoded.push(byte as char);
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// The URI reference of `path` in a SARIF log: a relative path stays
/// relative (percent-encoded, with `:` encoded too, so its first segment
/// cannot read as a scheme), any other path becomes a `file://` URI.
fn sarif_uri(path: &Path) -> String {
    let is_relative = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir | Component::ParentDir));
    if is_relative { percent_encoded(&slashed_path(path), b"/-._~") } else { file_uri(path) }
}

fn lsp(reports: &[FileReport]) -> String {
    let documents: Vec<Value> = reports
        .iter()
        .filter(|report| report.load_error.is_none())
        .map(|report| {
            let diagnostics: Vec<Value> = report
                .violations
                .iter()
                .map(|violation| {
                    let message = match &violation.hint {
                        Some(hint) => format!("{} ({hint})", violation.message),
                        None => violation.message.clone(),
                    };
                    json!({
                        "range": {
                            "start": { "line": violation.start.line - 1, "character": violation.start.utf16_column },
                            "end": { "line": violation.end.line - 1, "character": violation.end.utf16_column },
                        },
                        "severity": lsp_severity(violation.severity),
                        "code": violation.policy,
                        "source": "pawncritic",
                        "message": message,
                    })
                })
                .collect();
            json!({ "uri": file_uri(&report.path), "diagnostics": diagnostics })
        })
        .collect();
    let mut text = serde_json::to_string(&documents).expect("JSON values always serialize");
    text.push('\n');
    text
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity.diagnostic_level() {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info | DiagnosticLevel::Hint => "note",
    }
}

fn sarif(reports: &[FileReport], context: &RenderContext<'_>) -> String {
    let policies = context.policies;
    let rules: Vec<Value> = policies
        .iter()
        .map(|policy| {
            json!({
                "id": policy.name,
                "shortDescription": { "text": policy.summary },
                "fullDescription": { "text": if policy.description.is_empty() { &policy.summary } else { &policy.description } },
                "defaultConfiguration": { "level": sarif_level(policy.severity) },
                "properties": { "severity": policy.severity.level(), "tags": policy.themes },
            })
        })
        .collect();
    let rule_index: BTreeMap<&str, usize> =
        policies.iter().enumerate().map(|(index, policy)| (policy.name.as_str(), index)).collect();
    let results: Vec<Value> = violations(reports)
        .map(|(report, violation)| {
            let mut result = json!({
                "ruleId": violation.policy,
                "level": sarif_level(violation.severity),
                "message": { "text": violation.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": sarif_uri(&report.path) },
                        "region": {
                            "startLine": violation.start.line,
                            "startColumn": violation.start.column,
                            "endLine": violation.end.line,
                            "endColumn": violation.end.column,
                        },
                    },
                }],
            });
            if let Some(index) = rule_index.get(violation.policy.as_str()) {
                result["ruleIndex"] = json!(index);
            }
            result
        })
        .collect();
    let notifications: Vec<Value> =
        sarif_notification_texts(reports, context.problems).into_iter().map(sarif_notification).collect();
    let document = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "pawncritic", "version": env!("CARGO_PKG_VERSION"), "rules": rules } },
            "columnKind": "unicodeCodePoints",
            "results": results,
            "invocations": [{ "executionSuccessful": notifications.is_empty(), "toolExecutionNotifications": notifications }],
        }],
    });
    let mut text = serde_json::to_string_pretty(&document).expect("JSON values always serialize");
    text.push('\n');
    text
}

/// The texts of the SARIF notifications: the problems found before
/// checking, then per file its load error, which names the file itself,
/// and each failed policy after the file's path.
fn sarif_notification_texts(reports: &[FileReport], problems: &[Problem]) -> Vec<String> {
    let mut texts: Vec<String> = problems.iter().map(|problem| problem.message.clone()).collect();
    for report in reports {
        texts.extend(report.load_error.iter().cloned());
        let path = display_path(&report.path);
        texts.extend(report.failures.iter().map(|failure| format!("{path}: {}", failure.message)));
    }
    texts
}

fn sarif_notification(text: String) -> Value {
    json!({ "level": "error", "message": { "text": text } })
}

fn xml_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            character if (character as u32) < 0x20 && !matches!(character, '\t' | '\n' | '\r') => {}
            character => escaped.push(character),
        }
    }
    escaped
}

fn checkstyle(reports: &[FileReport]) -> String {
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<checkstyle version=\"4.3\">\n");
    for report in reports {
        if report.violations.is_empty() && report.load_error.is_none() {
            continue;
        }
        let _ = writeln!(output, "  <file name=\"{}\">", xml_escape(&display_path(&report.path)));
        if let Some(message) = &report.load_error {
            let _ = writeln!(
                output,
                "    <error line=\"1\" severity=\"error\" message=\"{}\" source=\"pawncritic\"/>",
                xml_escape(message)
            );
        }
        for violation in &report.violations {
            let severity = match violation.severity.diagnostic_level() {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info | DiagnosticLevel::Hint => "info",
            };
            let _ = writeln!(
                output,
                "    <error line=\"{}\" column=\"{}\" severity=\"{severity}\" message=\"{}\" source=\"pawncritic.{}\"/>",
                violation.start.line,
                violation.start.column,
                xml_escape(&violation.message),
                xml_escape(&violation.policy.replace("::", "."))
            );
        }
        output.push_str("  </file>\n");
    }
    output.push_str("</checkstyle>\n");
    output
}

/// Escapes text for a GitHub workflow command; properties also escape
/// `:` and `,`.
fn github_escape(text: &str, is_property: bool) -> String {
    let mut escaped = text.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A");
    if is_property {
        escaped = escaped.replace(':', "%3A").replace(',', "%2C");
    }
    escaped
}

fn github(reports: &[FileReport]) -> String {
    let mut output = String::new();
    for (report, violation) in violations(reports) {
        let command = match violation.severity.diagnostic_level() {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Info | DiagnosticLevel::Hint => "notice",
        };
        let _ = writeln!(
            output,
            "::{command} file={},line={},col={},endLine={},endColumn={},title={}::{}",
            github_escape(&slashed_path(&report.path), true),
            violation.start.line,
            violation.start.column,
            violation.end.line,
            violation.end.column,
            github_escape(&violation.policy, true),
            github_escape(&violation.message, false)
        );
    }
    output
}

/// The placeholders of `--template`.
pub const TEMPLATE_HELP: &str = "%f file, %l line, %c column, %L end line, %C end column, %s severity, %S severity name, \
%d level (error, warning, info, hint), %p policy, %m message, %h hint, %n line break, %% percent sign";

/// Checks that `template` uses only known placeholders.
pub fn check_template(template: &str) -> Result<(), String> {
    let mut characters = template.chars();
    while let Some(character) = characters.next() {
        if character != '%' {
            continue;
        }
        match characters.next() {
            Some('f' | 'l' | 'c' | 'L' | 'C' | 's' | 'S' | 'd' | 'p' | 'm' | 'h' | 'n' | '%') => {}
            Some(other) => {
                return Err(format!("the template uses the unknown placeholder %{other}; known are {TEMPLATE_HELP}"));
            }
            None => return Err("the template ends in a lone %".to_string()),
        }
    }
    Ok(())
}

fn templated(reports: &[FileReport], template: &str) -> String {
    let mut output = String::new();
    for (report, violation) in violations(reports) {
        let mut characters = template.chars();
        while let Some(character) = characters.next() {
            if character != '%' {
                output.push(character);
                continue;
            }
            match characters.next() {
                Some('f') => output.push_str(&display_path(&report.path)),
                Some('l') => output.push_str(&violation.start.line.to_string()),
                Some('c') => output.push_str(&violation.start.column.to_string()),
                Some('L') => output.push_str(&violation.end.line.to_string()),
                Some('C') => output.push_str(&violation.end.column.to_string()),
                Some('s') => output.push_str(&violation.severity.to_string()),
                Some('S') => output.push_str(violation.severity.name()),
                Some('d') => output.push_str(violation.severity.diagnostic_level().name()),
                Some('p') => output.push_str(&violation.policy),
                Some('m') => output.push_str(&violation.message),
                Some('h') => output.push_str(violation.hint.as_deref().unwrap_or("")),
                Some('n') => output.push('\n'),
                Some('%') => output.push('%'),
                Some(other) => unreachable!("check_template refuses the placeholder %{other}"),
                None => unreachable!("check_template refuses a trailing %"),
            }
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_checked() {
        assert!(check_template("%f:%l:%c %m (%p)%n").is_ok());
        assert!(check_template("%x").unwrap_err().contains("%x"));
        assert!(check_template("50%").is_err());
    }

    #[test]
    fn uris_escape_what_they_must() {
        assert_eq!(uri_of_slashed_path("/a b/c#d.sp"), "file:///a%20b/c%23d.sp");
        assert_eq!(uri_of_slashed_path("C:/Users/a.sp"), "file:///C:/Users/a.sp");
        assert_eq!(uri_of_slashed_path("//server/share/a.sp"), "file://server/share/a.sp");
    }

    #[test]
    fn sarif_uris_of_relative_paths_are_encoded_and_absolute_ones_are_file_uris() {
        assert_eq!(sarif_uri(Path::new("my dir/a#1.sp")), "my%20dir/a%231.sp");
        assert_eq!(sarif_uri(Path::new("../50%/a:b.sp")), "../50%25/a%3Ab.sp");
        let absolute = std::path::absolute("/x y/a.sp").unwrap();
        assert_eq!(sarif_uri(&absolute), file_uri(&absolute));
        assert!(file_uri(&absolute).starts_with("file://") && file_uri(&absolute).ends_with("/x%20y/a.sp"));
    }

    #[test]
    fn escaping_for_xml_and_github() {
        assert_eq!(xml_escape("a<\"b\">&"), "a&lt;&quot;b&quot;&gt;&amp;");
        assert_eq!(github_escape("a:b,c%\n", true), "a%3Ab%2Cc%25%0A");
        assert_eq!(github_escape("a:b", false), "a:b");
    }
}
