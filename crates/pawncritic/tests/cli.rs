// Copyright 2026 davenonymous

//! The `pawncritic` command: exit codes, output formats, standard input
//! and configuration files.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const CLEAN: &str = "#pragma semicolon 1\n#pragma newdecls required\n\nint g_count;\n";
const LEGACY: &str = "#pragma semicolon 1\n#pragma newdecls required\n\nvoid f(ArrayList h) {\n\tCloseHandle(h);\n}\n";

fn pawncritic(arguments: &[&str], directory: &Path, stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pawncritic"));
    command
        .args(arguments)
        .current_dir(directory)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("pawncritic starts");
    if let Some(text) = stdin {
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("pawncritic finishes")
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
fn exit_codes_tell_clean_violations_and_errors_apart() {
    let directory = project(&[("clean.sp", CLEAN), ("legacy.sp", LEGACY)]);
    assert_eq!(pawncritic(&["clean.sp"], directory.path(), None).status.code(), Some(0));
    let legacy = pawncritic(&["legacy.sp"], directory.path(), None);
    assert_eq!(legacy.status.code(), Some(1));
    let text = stdout(&legacy);
    assert!(
        text.contains("legacy.sp:5:2: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle"),
        "{text}"
    );
    assert!(text.contains("= hint: delete h;"), "{text}");
    assert!(text.ends_with("1 violation in 1 of 1 file (1 of severity 4).\n"), "{text}");
    assert_eq!(pawncritic(&["missing.sp"], directory.path(), None).status.code(), Some(2));
}

#[test]
fn machine_formats_are_well_formed() {
    let directory = project(&[("legacy.sp", LEGACY)]);
    let json: serde_json::Value =
        serde_json::from_slice(&pawncritic(&["-f", "json", "legacy.sp"], directory.path(), None).stdout).unwrap();
    let violation = &json["violations"][0];
    assert_eq!(
        (violation["policy"].as_str(), violation["line"].as_u64(), violation["column"].as_u64()),
        (Some("Modern::ProhibitCloseHandle"), Some(5), Some(2))
    );

    let lsp: serde_json::Value =
        serde_json::from_slice(&pawncritic(&["-f", "lsp", "legacy.sp"], directory.path(), None).stdout).unwrap();
    assert!(lsp[0]["uri"].as_str().unwrap().starts_with("file:///"));
    assert_eq!(lsp[0]["diagnostics"][0]["range"]["start"], serde_json::json!({ "line": 4, "character": 1 }));
    assert_eq!(lsp[0]["diagnostics"][0]["severity"], 2);

    let sarif: serde_json::Value =
        serde_json::from_slice(&pawncritic(&["-f", "sarif", "legacy.sp"], directory.path(), None).stdout).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
    assert_eq!(sarif["runs"][0]["results"][0]["ruleId"], "Modern::ProhibitCloseHandle");

    let gcc = stdout(&pawncritic(&["-f", "gcc", "legacy.sp"], directory.path(), None));
    assert_eq!(
        gcc,
        "legacy.sp:5:2: warning: `CloseHandle()` is superseded by `delete` [Modern::ProhibitCloseHandle]\n"
    );
    let github = stdout(&pawncritic(&["-f", "github", "legacy.sp"], directory.path(), None));
    assert!(github.starts_with("::warning file=legacy.sp,line=5,col=2,"), "{github}");
    let checkstyle = stdout(&pawncritic(&["-f", "checkstyle", "legacy.sp"], directory.path(), None));
    assert!(checkstyle.contains("<error line=\"5\" column=\"2\" severity=\"warning\""), "{checkstyle}");
    let template = stdout(&pawncritic(&["--template", "%f|%l|%c|%s|%p", "legacy.sp"], directory.path(), None));
    assert_eq!(template, "legacy.sp|5|2|4|Modern::ProhibitCloseHandle\n");
}

#[test]
fn standard_input_takes_the_kind_and_config_of_its_name() {
    let directory = project(&[("sub/.pawncritic.toml", "severity = 5\n")]);
    let legacy_without_pragmas = "void f(Handle h) {\n\tCloseHandle(h);\n}\n";
    let as_plugin = pawncritic(
        &["--stdin-filename", "plugin.sp", "-f", "gcc", "-"],
        directory.path(),
        Some(legacy_without_pragmas),
    );
    assert!(stdout(&as_plugin).contains("[Modern::RequireNewDecls]"));
    let as_include = pawncritic(
        &["--stdin-filename", "plugin.inc", "-f", "gcc", "-"],
        directory.path(),
        Some(legacy_without_pragmas),
    );
    assert!(!stdout(&as_include).contains("[Modern::RequireNewDecls]"));
    let configured = pawncritic(
        &["--stdin-filename", "sub/plugin.sp", "-f", "gcc", "-"],
        directory.path(),
        Some(legacy_without_pragmas),
    );
    assert_eq!(configured.status.code(), Some(0), "{}", stdout(&configured));
}

#[test]
fn the_nearest_configuration_applies() {
    let directory = project(&[
        (".pawncritic.toml", "[policies.\"Modern::ProhibitCloseHandle\"]\nseverity = 1\n"),
        ("a/legacy.sp", LEGACY),
        ("b/.pawncritic.toml", "severity = 4\n"),
        ("b/legacy.sp", LEGACY),
    ]);
    let output = stdout(&pawncritic(&["-f", "gcc", "."], directory.path(), None)).replace('\\', "/");
    assert!(!output.contains("a/legacy.sp"), "{output}");
    assert!(output.contains("b/legacy.sp:5:2"), "{output}");
}

#[test]
fn mistakes_in_configuration_files_stop_the_run() {
    let directory = project(&[(".pawncritic.toml", "severty = 3\n"), ("clean.sp", CLEAN)]);
    let output = pawncritic(&["clean.sp"], directory.path(), None);
    assert_eq!(output.status.code(), Some(2));
    let errors = String::from_utf8(output.stderr).unwrap();
    assert!(errors.contains(".pawncritic.toml") && errors.contains("severty"), "{errors}");
    assert!(errors.contains("\n1 | severty = 3"), "the TOML error keeps its lines: {errors:?}");
}

#[test]
fn policies_can_be_listed_and_explained() {
    let directory = project(&[]);
    let list = stdout(&pawncritic(&["--list"], directory.path(), None));
    assert!(list.lines().any(|line| line.starts_with("* 4 Modern::ProhibitCloseHandle")), "{list}");
    assert!(list.lines().any(|line| line.starts_with("  1 NamingConventions::RequireGlobalPrefix")), "{list}");
    let explanation = stdout(&pawncritic(&["--explain", "ProhibitDeepNesting"], directory.path(), None));
    assert!(
        explanation.contains("Severity: 3 (harsh)") && explanation.contains("max_depth (integer, default 3)"),
        "{explanation}"
    );
}

#[test]
fn control_characters_never_reach_the_terminal() {
    let hostile = "#pragma semicolon 1\n#pragma newdecls required\n\n\
                   void f(ArrayList h) {\n\tCloseHandle(h); char s[] = \"\x1b]0;owned\x07\x1b[2J\";\n}\n\
                   int g_broken = \x1b[2J;\n";
    let directory = project(&[("hostile.sp", hostile)]);
    let output = pawncritic(&["--color", "always", "--brutal", "hostile.sp"], directory.path(), None);
    let text = stdout(&output);
    assert!(text.contains("CloseHandle"), "{text}");
    assert!(text.contains("RequireParsableCode"), "{text}");
    assert!(!text.contains("\x1b]") && !text.contains("\x1b[2J"), "{text:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains('\x1b'), "{stderr:?}");
}

#[test]
fn sarif_tells_every_error_of_the_run() {
    let directory = project(&[
        (
            "critic/bom.lua",
            "\u{feff}return { name = \"Custom::Bom\", severity = 5, summary = \"a call\", query = \"(call_expression) @violation\" }",
        ),
        ("legacy.sp", LEGACY),
    ]);
    std::fs::write(directory.path().join("critic/latin.lua"), b"-- \xe9\nreturn {}").unwrap();
    std::fs::write(directory.path().join("latin.sp"), b"int caf\xe9;\n").unwrap();
    let arguments =
        ["-f", "sarif", "--policy-path", "critic", "--single-policy", "Custom::Bom", "latin.sp", "legacy.sp"];
    let output = pawncritic(&arguments, directory.path(), None);
    assert_eq!(output.status.code(), Some(2));
    let sarif: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let run = &sarif["runs"][0];
    assert_eq!(run["results"][0]["ruleId"], "Custom::Bom", "a policy file with a byte order mark loads");
    let invocation = &run["invocations"][0];
    assert_eq!(invocation["executionSuccessful"], false);
    let texts: Vec<&str> = invocation["toolExecutionNotifications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|notification| notification["message"]["text"].as_str().unwrap())
        .collect();
    let latin_policy = Path::new("critic").join("latin.lua");
    assert_eq!(
        texts,
        [
            format!("{}: the file is not valid UTF-8; the policy is skipped", latin_policy.display()),
            "latin.sp:1: not valid UTF-8; convert the file to UTF-8 first".to_string(),
        ]
    );
}

#[test]
fn a_file_is_checked_once_however_it_is_spelled() {
    let directory = project(&[("sub/legacy.sp", LEGACY)]);
    let output = pawncritic(
        &["-f", "gcc", "sub/legacy.sp", "./sub/legacy.sp", "sub/./legacy.sp", "sub"],
        directory.path(),
        None,
    );
    let text = stdout(&output).replace('\\', "/");
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        ["sub/legacy.sp:5:2: warning: `CloseHandle()` is superseded by `delete` [Modern::ProhibitCloseHandle]"]
    );
}

#[test]
fn jobs_must_be_at_least_one() {
    let directory = project(&[("clean.sp", CLEAN)]);
    let output = pawncritic(&["--jobs", "0", "clean.sp"], directory.path(), None);
    assert_eq!(output.status.code(), Some(2));
    let errors = String::from_utf8(output.stderr).unwrap();
    assert!(errors.contains("must be at least 1"), "{errors}");
}

#[test]
fn policy_text_cannot_forge_lines() {
    let policy = r#"print("x\n::add-mask::secret\n\27[2J")
return {
    name = "Custom::Forger", severity = 5, summary = "s", description = "plain\n\27[2Jescape",
    options = {
        o = { default = 1, description = "harmless\nonly = true" },
        p = { default = "\nforce = true\nonly = true\n#", description = "d" },
    },
    check = function(file, ctx)
        for _ = 1, 100 do print(string.rep("A", 4096)) end
        error("ok\n::error file=a.sp::forged", 0)
    end,
}"#;
    let directory = project(&[("critic/forger.lua", policy), ("clean.sp", CLEAN)]);
    let run = pawncritic(&["--policy-path", "critic", "--single-policy", "Forger", "clean.sp"], directory.path(), None);
    let errors = String::from_utf8(run.stderr).unwrap();
    assert!(errors.lines().all(|line| line.starts_with('[') || line.starts_with("pawncritic: ")), "{errors}");
    assert!(errors.contains("print output dropped: more than 65536 bytes in one check"), "{errors}");
    assert!(errors.len() < 80 * 1024, "{} bytes", errors.len());
    let explained = pawncritic(&["--policy-path", "critic", "--explain", "Forger"], directory.path(), None);
    assert!(!stdout(&explained).contains('\x1b'), "{}", stdout(&explained));
    let template = pawncritic(&["--policy-path", "critic", "--print-config-template"], directory.path(), None);
    let text = stdout(&template);
    assert!(text.lines().all(|line| line.is_empty() || line.starts_with('#')), "{text}");
}
