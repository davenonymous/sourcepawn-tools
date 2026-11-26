// Copyright 2026 davenonymous

//! Formats every `tests/fixtures/*.sp` and compares it with the
//! `.expected` file next to it, with the style of a `.toml` file of the same
//! name if there is one. Formatting the expected output again must not
//! change it. `PAWNTIDY_BLESS=1 cargo test` rewrites the expected files
//! after a deliberate change; review the diff before committing it.

use std::path::{Path, PathBuf};

use pawn_core::cli::ConfigFile;
use pawntidy::{Style, format_source};

fn fixtures() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("the fixture directory exists")
        .map(|entry| entry.expect("the fixture directory is readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sp"))
        .collect();
    inputs.sort();
    assert!(!inputs.is_empty(), "no fixtures in {}", directory.display());
    inputs
}

fn style_for(input: &Path) -> Style {
    let toml = input.with_extension("toml");
    if toml.exists() { Style::read(&toml).expect("the fixture style is valid") } else { Style::default() }
}

fn read_text(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    String::from_utf8(bytes).expect("fixtures are UTF-8")
}

#[test]
fn fixtures_format_as_expected_and_stay_stable() {
    let bless = std::env::var_os("PAWNTIDY_BLESS").is_some();
    let mut failures = Vec::new();
    for input in fixtures() {
        let style = style_for(&input);
        let formatted = format_source(&input, &read_text(&input), &style).unwrap_or_else(|error| panic!("{error}"));
        let expected_path = input.with_extension("expected");
        if bless {
            std::fs::write(&expected_path, &formatted.text).expect("the expected file is writable");
        }
        let expected = read_text(&expected_path);
        if formatted.text != expected {
            failures.push(format!(
                "{}: differs from {}:\n{}",
                input.display(),
                expected_path.display(),
                formatted.text
            ));
            continue;
        }
        let again = format_source(&expected_path, &expected, &style).unwrap_or_else(|error| panic!("{error}"));
        if again.text != expected {
            failures.push(format!("{}: formatting the expected output changes it:\n{}", input.display(), again.text));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn unreadable_items_are_reported_and_kept() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/syntax_error.sp");
    let formatted = format_source(&input, &read_text(&input), &Style::default()).unwrap();
    let kept: Vec<(usize, usize, pawntidy::KeptReason)> =
        formatted.kept.iter().map(|region| (region.first_line, region.last_line, region.reason)).collect();
    assert_eq!(kept, vec![(7, 15, pawntidy::KeptReason::SyntaxError)]);
}

#[test]
fn items_with_line_splices_are_reported_and_kept() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/line_splice.sp");
    let formatted = format_source(&input, &read_text(&input), &Style::default()).unwrap();
    let kept: Vec<(usize, usize, pawntidy::KeptReason)> =
        formatted.kept.iter().map(|region| (region.first_line, region.last_line, region.reason)).collect();
    assert_eq!(kept, vec![(1, 4, pawntidy::KeptReason::LineSplice)]);
}

#[test]
fn directives_continued_after_whitespace_are_reported_and_kept() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/directive_blank_splice.sp");
    let formatted = format_source(&input, &read_text(&input), &Style::default()).unwrap();
    let kept: Vec<(usize, usize, pawntidy::KeptReason)> =
        formatted.kept.iter().map(|region| (region.first_line, region.last_line, region.reason)).collect();
    assert_eq!(kept, vec![(1, 2, pawntidy::KeptReason::LineSplice)]);
}

#[test]
fn items_with_a_stray_hash_are_reported_and_kept() {
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stray_hash.sp");
    let formatted = format_source(&input, &read_text(&input), &Style::default()).unwrap();
    let kept: Vec<(usize, usize, pawntidy::KeptReason)> =
        formatted.kept.iter().map(|region| (region.first_line, region.last_line, region.reason)).collect();
    assert_eq!(kept, vec![(1, 4, pawntidy::KeptReason::StrayHash)]);
}
