// Copyright 2026 davenonymous

//! The `pawntidy` command: printing, checking, diffing and rewriting.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const UNTIDY: &str = "void f()\n{\n\tif (a)\n\t\tb();\n}\n";
const TIDY: &str = "void f() {\n\tif (a) {\n\t\tb();\n\t}\n}\n";

fn pawntidy(arguments: &[&str], directory: &Path, stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pawntidy"));
    command
        .args(arguments)
        .current_dir(directory)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("pawntidy starts");
    if let Some(text) = stdin {
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("pawntidy finishes")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    for (name, content) in files {
        let path = directory.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    directory
}

#[test]
fn a_single_file_is_printed_formatted() {
    let directory = project(&[("a.sp", UNTIDY)]);
    let output = pawntidy(&["a.sp"], directory.path(), None);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), TIDY);
    assert_eq!(std::fs::read_to_string(directory.path().join("a.sp")).unwrap(), UNTIDY);
    assert_eq!(stdout(&pawntidy(&["-"], directory.path(), Some(UNTIDY))), TIDY);
}

#[test]
fn check_and_diff_report_changes_without_making_them() {
    let directory = project(&[("a.sp", UNTIDY), ("b.sp", TIDY)]);
    let check = pawntidy(&["--check", "."], directory.path(), None);
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(stdout(&check).replace('\\', "/"), "would reformat ./a.sp\n");
    let diff = pawntidy(&["--diff", "a.sp"], directory.path(), None);
    assert_eq!(diff.status.code(), Some(1));
    assert!(stdout(&diff).starts_with("--- a/a.sp\n+++ b/a.sp\n"), "{}", stdout(&diff));
    let json: serde_json::Value =
        serde_json::from_slice(&pawntidy(&["--check", "-f", "json", "b.sp"], directory.path(), None).stdout).unwrap();
    assert_eq!(json["files"][0]["changed"], false);
    assert_eq!(std::fs::read_to_string(directory.path().join("a.sp")).unwrap(), UNTIDY);
}

#[test]
fn write_rewrites_changed_files_only() {
    let directory = project(&[("a.sp", UNTIDY), ("b.sp", TIDY)]);
    let output = pawntidy(&["--write", "."], directory.path(), None);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(std::fs::read_to_string(directory.path().join("a.sp")).unwrap(), TIDY);
    assert_eq!(pawntidy(&["--check", "."], directory.path(), None).status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn write_rewrites_the_target_of_a_link() {
    let directory = project(&[("real/a.sp", UNTIDY)]);
    std::os::unix::fs::symlink("real/a.sp", directory.path().join("link.sp")).unwrap();
    let output = pawntidy(&["--write", "link.sp"], directory.path(), None);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let link = std::fs::symlink_metadata(directory.path().join("link.sp")).unwrap();
    assert!(link.file_type().is_symlink(), "the link was replaced by a file");
    assert_eq!(std::fs::read_to_string(directory.path().join("real/a.sp")).unwrap(), TIDY);
}

#[test]
fn the_nearest_style_applies() {
    let directory = project(&[
        ("sub/.pawntidy.toml", "cuddle_else = false\n"),
        ("sub/a.sp", "void f() {\n\tif (a) {\n\t} else {\n\t}\n}\n"),
    ]);
    let output = stdout(&pawntidy(&["sub/a.sp"], directory.path(), None));
    assert!(output.contains("\t}\n\telse {\n"), "{output}");
}

#[test]
fn several_files_need_a_mode() {
    let directory = project(&[("a.sp", UNTIDY), ("b.sp", TIDY)]);
    let output = pawntidy(&["."], directory.path(), None);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn a_colored_diff_sends_no_control_characters_from_the_file() {
    let hostile = "void f()\n{\n\tif (a)\n\t\tb(\"\x1b]0;owned\x07\x1b[2J\");\n}\n";
    let directory = project(&[("hostile.sp", hostile)]);
    let shown = stdout(&pawntidy(&["--diff", "--color", "always", "hostile.sp"], directory.path(), None));
    assert!(shown.contains("+\tif (a) {"), "{shown:?}");
    assert!(!shown.contains("\x1b]") && !shown.contains("\x1b[2J"), "{shown:?}");
    let piped = stdout(&pawntidy(&["--diff", "--color", "never", "hostile.sp"], directory.path(), None));
    assert!(piped.contains("\x1b]0;owned\x07\x1b[2J"), "a diff for a file or pipe keeps the exact bytes: {piped:?}");
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_written_fails() {
    use std::os::unix::fs::PermissionsExt;
    let directory = project(&[("ro/a.sp", UNTIDY), ("ro/b.sp", TIDY)]);
    let read_only = directory.path().join("ro");
    std::fs::set_permissions(&read_only, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(read_only.join("probe"), "").is_ok() {
        return; // running as root, which writes anyway
    }
    let human = pawntidy(&["--write", "ro"], directory.path(), None);
    let json = pawntidy(&["--write", "-f", "json", "ro"], directory.path(), None);
    std::fs::set_permissions(&read_only, std::fs::Permissions::from_mode(0o755)).unwrap();
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert_eq!(human.status.code(), Some(2));
    assert!(stderr.ends_with("0 files reformatted, 1 file left unchanged, 1 file failed.\n"), "{stderr}");
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json.status.code(), Some(2));
    assert_eq!(report["files"][0]["changed"], false);
    assert!(report["files"][0]["error"].as_str().unwrap().starts_with("ro/a.sp: cannot write: "), "{report}");
    assert_eq!(std::fs::read_to_string(read_only.join("a.sp")).unwrap(), UNTIDY);
}

#[test]
fn the_style_of_standard_input_is_that_of_its_name() {
    let directory = project(&[("sub/.pawntidy.toml", "max_line_length = 80\n")]);
    let output = pawntidy(&["--print-style", "--stdin-filename", "sub/a.sp", "-"], directory.path(), None);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("max_line_length = 80\n"), "{}", stdout(&output));
}

#[test]
fn jobs_must_be_at_least_one() {
    let directory = project(&[("a.sp", TIDY)]);
    let output = pawntidy(&["--check", "--jobs", "0", "a.sp"], directory.path(), None);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("must be at least 1"));
}
