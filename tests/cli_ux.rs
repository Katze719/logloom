//! User-facing help, actionable diagnostics, and quiet machine-readable output.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

use logloom::Changelog;

fn invoke(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(arguments)
        .output()
}

fn stdin(input: &str, arguments: &[&str]) -> io::Result<Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin not piped"))?
        .write_all(input.as_bytes())?;
    child.wait_with_output()
}

#[test]
fn help_groups_options_and_keeps_all_formats_and_examples_discoverable() {
    let output = invoke(&["parse", "--help"]).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    let help = String::from_utf8(output.stdout).unwrap();
    for label in [
        "Input:",
        "Output:",
        "Filters:",
        "Examples:",
        "Input formats:",
        "Output formats:",
    ] {
        assert!(help.contains(label), "{label}");
    }
    for name in [
        "auto",
        "keep-a-changelog",
        "conventional-changelog",
        "generic-markdown",
        "github-release-notes",
        "changesets",
        "towncrier",
        "debian-changelog",
        "gnu-news",
        "gnu-changelog",
        "json",
        "json-compact",
        "yaml",
        "toml",
        "jsonl",
        "csv",
        "text",
        "markdown",
    ] {
        assert!(help.contains(name), "{name}");
    }
    assert!(help.contains("logloom parse CHANGELOG.md --latest --output-format text"));
    let output = invoke(&["parse", "-", "--input-format", "md"]).unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("keep-a-changelog")
    );
}

#[test]
fn detection_hints_address_empty_input_json_syntax_and_ambiguity_separately() {
    let empty = stdin("", &["detect", "-"]).unwrap();
    let empty_error = String::from_utf8(empty.stderr).unwrap();
    assert!(empty_error.contains("error[empty-input]"));
    assert!(empty_error.contains("Provide a changelog file"));
    assert!(!empty_error.contains("--input-format"));

    let ambiguous = stdin("# Notes\n\n- A fix.\n", &["detect", "-"]).unwrap();
    let ambiguous_error = String::from_utf8(ambiguous.stderr).unwrap();
    assert!(ambiguous_error.contains("error[ambiguous-format]"));
    assert!(ambiguous_error.contains("'logloom parse' or 'logloom validate'"));
    assert!(!ambiguous_error.contains("16 MiB"));
    assert!(!ambiguous_error.contains("^^^^"));

    let malformed = stdin("{\n\"tag_name\":\n}", &["parse", "-"]).unwrap();
    let error = String::from_utf8(malformed.stderr).unwrap();
    assert!(error.contains("error[detect-input]"));
    assert!(error.contains("<stdin>:3:1"));
    assert!(error.contains("Fix the input syntax"));
    assert_eq!(error.matches("cannot inspect JSON").count(), 1);
}

#[test]
fn invalid_limits_and_empty_filters_have_usage_errors_without_type_jargon() {
    for value in ["0", "-1", "many", "99999999999999999999999999"] {
        let output = invoke(&["parse", "examples/keep-a-changelog.md", "--limit", value]).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("positive whole number"));
        assert!(!error.contains("non-zero type"));
        assert!(!error.contains("use '-- -1'"));
    }
    for flag in ["--release", "--category", "--component", "--package"] {
        let output = invoke(&["parse", "examples/keep-a-changelog.md", flag, ""]).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
}

#[test]
fn no_matches_has_one_note_without_changing_output_or_success_status() {
    for format in ["json", "jsonl", "text"] {
        let output = invoke(&[
            "parse",
            "examples/keep-a-changelog.md",
            "--category",
            "Fiexd",
            "--output-format",
            format,
        ])
        .unwrap();
        assert!(output.status.success());
        let diagnostics = String::from_utf8(output.stderr).unwrap();
        assert_eq!(diagnostics.matches("note[no-matches]").count(), 1);
        assert!(diagnostics.contains("Check the filter values"));
        if format == "json" {
            assert_eq!(
                serde_json::from_slice::<Changelog>(&output.stdout)
                    .unwrap()
                    .releases
                    .len(),
                0
            );
        } else if format == "jsonl" {
            assert_eq!(output.stdout, Vec::<u8>::new());
        }
    }
    let matched = invoke(&[
        "parse",
        "examples/keep-a-changelog.md",
        "--category",
        "Fixed",
    ])
    .unwrap();
    assert!(matched.status.success());
    assert_eq!(matched.stderr, Vec::<u8>::new());
}

#[test]
fn repeated_warnings_are_grouped_with_every_location_and_errors_stay_individual() {
    let mut source = "# Changelog\n\n## [Unreleased]\n\n".to_owned();
    for index in 0..50 {
        writeln!(source, "### Category {index}\n\n- A fix.\n").unwrap();
    }
    let output = stdin(&source, &["validate", "-"]).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let diagnostics = String::from_utf8(output.stderr).unwrap();
    assert_eq!(diagnostics.matches("warning[custom-category]").count(), 1);
    assert!(diagnostics.contains("49 more occurrences"));
    assert!(diagnostics.contains("<stdin>:5:1"));
    let locations: Vec<usize> = diagnostics
        .lines()
        .filter_map(|line| line.split_once("also at lines ").map(|(_, rest)| rest))
        .flat_map(|line| line.split(", "))
        .map(|line| line.parse().unwrap())
        .collect();
    let expected: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with("### "))
        .map(|(index, _)| index.checked_add(1).unwrap())
        .skip(1)
        .collect();
    assert_eq!(locations, expected);
    let output = invoke(&["validate", "tests/fixtures/invalid-dates.md"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stderr)
            .unwrap()
            .matches("error[invalid-date]")
            .count(),
        4
    );
}

#[test]
fn latest_and_io_errors_point_to_the_actual_problem() {
    let output = invoke(&["parse", "examples/changesets.md", "--latest"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("\"2.0.0\" (@lantern/core) has no date"));
    assert!(error.contains("changesets.md:3:1"));
    assert!(error.contains("Replace --latest with --first-published"));
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(directory.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("file instead of a directory")
    );
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["parse", "examples/keep-a-changelog.md", "-o"])
        .arg(directory.path().join("missing/result.json"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Create the parent directory first")
    );
}

#[test]
fn output_dash_uses_stdout_and_dot_dash_can_still_name_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let input = fs::canonicalize("examples/keep-a-changelog.md").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .arg("parse")
        .arg(&input)
        .args(["-o", "-"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    assert!(serde_json::from_slice::<Changelog>(&output.stdout).is_ok());
    assert!(!directory.path().join("-").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .arg("parse")
        .arg(&input)
        .args(["-o", "./-"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(
        serde_json::from_slice::<Changelog>(&fs::read(directory.path().join("-")).unwrap()).is_ok()
    );
}
