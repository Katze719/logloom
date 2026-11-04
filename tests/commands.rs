//! Auto mode, format inspection, and validation through the executable.

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
fn auto_is_the_default_and_detect_prints_only_the_explicit_format_name() {
    for (file, expected) in [
        ("examples/keep-a-changelog.md", "keep-a-changelog"),
        (
            "examples/conventional-changelog.md",
            "conventional-changelog",
        ),
        ("examples/generic-markdown.md", "generic-markdown"),
        ("examples/github-release-notes.json", "github-release-notes"),
        ("examples/changesets-fragment.md", "changesets"),
        ("examples/towncrier.rst", "towncrier"),
        ("examples/debian.changelog", "debian-changelog"),
        ("examples/gnu-news.txt", "gnu-news"),
        ("examples/gnu-ChangeLog", "gnu-changelog"),
    ] {
        let detected = invoke(&["detect", file, "--color", "always"]).unwrap();
        assert!(
            detected.status.success(),
            "{}",
            String::from_utf8_lossy(&detected.stderr)
        );
        assert_eq!(
            String::from_utf8(detected.stdout).unwrap(),
            format!("{expected}\n")
        );
        assert_eq!(detected.stderr, Vec::<u8>::new());
        for extra in [vec![], vec!["--input-format", "auto"]] {
            let mut args = vec!["parse", file];
            args.extend(extra);
            let output = invoke(&args).unwrap();
            assert!(
                output.status.success(),
                "{file}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(model.format.as_str(), expected);
        }
    }
}

#[test]
fn ambiguous_auto_input_needs_an_explicit_format_and_leaves_the_output_intact() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("CHANGELOG.md");
    let output_path = directory.path().join("result.json");
    fs::write(
        &input,
        "# Changelog\n\n## 1.0.0 (2026-11-04)\n\n- A change.\n",
    )
    .unwrap();
    fs::write(&output_path, "keep me").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&input)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(fs::read_to_string(&output_path).unwrap(), "keep me");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("error[ambiguous-format]"));
    assert!(error.contains("conventional-changelog"));
    assert!(error.contains("towncrier"));
    assert!(error.contains("--input-format"));
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&input)
        .args(["--input-format", "generic-markdown"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn detect_and_validate_have_distinct_jobs_and_validation_never_exports() {
    let output = invoke(&["detect", "tests/fixtures/invalid-dates.md"]).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"keep-a-changelog\n");
    assert_eq!(output.stderr, Vec::<u8>::new());
    for (file, code, marker) in [
        ("tests/fixtures/standard.md", 0, ""),
        (
            "tests/fixtures/unconventional.md",
            0,
            "warning[missing-title]",
        ),
        ("tests/fixtures/invalid-dates.md", 1, "error[invalid-date]"),
        (
            "tests/fixtures/malformed-heading.md",
            1,
            "error[malformed-heading]",
        ),
    ] {
        let output = invoke(&[
            "validate",
            file,
            "--input-format",
            "keep-a-changelog",
            "--color",
            "never",
        ])
        .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, Vec::<u8>::new());
        if marker.is_empty() {
            assert_eq!(output.stderr, Vec::<u8>::new());
        } else {
            assert!(String::from_utf8(output.stderr).unwrap().contains(marker));
        }
    }
    let output = invoke(&["validate", "examples/conventional-changelog.md"]).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn detection_failures_use_source_diagnostics_and_explicit_colors() {
    for (input, marker) in [
        ("not a changelog", "error[unknown-format]"),
        ("## Notes\n\n- Fix.\n", "error[ambiguous-format]"),
        ("{\n\"tag_name\":\n}", "error[detect-input]"),
    ] {
        let output = stdin(input, &["detect", "-", "--color", "always"]).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(output.stderr.contains(&0x1b));
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(marker));
        assert!(error.contains("<stdin>"));
        assert!(error.contains("--input-format"));
    }
}

#[test]
fn new_commands_share_stdin_fragment_hints_and_io_exit_codes() {
    for command in ["detect", "validate", "parse"] {
        let output = stdin("", &[command, "-", "--fragment-name", "42.misc.rst"]).unwrap();
        assert!(
            output.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if command == "validate" {
            assert_eq!(output.stdout, Vec::<u8>::new());
        }
        if command == "detect" {
            assert_eq!(output.stdout, b"towncrier\n");
        }
        let output = invoke(&[command, "this-file-does-not-exist.md"]).unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
    for args in [
        vec!["validate", "-", "--category", "Added"],
        vec!["validate", "-", "--output-format", "json"],
    ] {
        let output = invoke(&args).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
}

#[test]
fn omitted_file_reads_changelog_in_the_current_directory_and_explicit_paths_override_it() {
    let directory = tempfile::tempdir().unwrap();
    let original = include_str!("fixtures/only-unreleased.md");
    fs::write(directory.path().join("CHANGELOG.md"), original).unwrap();
    fs::write(
        directory.path().join("other.md"),
        include_str!("../examples/conventional-changelog.md"),
    )
    .unwrap();
    fs::write(
        directory.path().join("invalid.md"),
        include_str!("fixtures/invalid-dates.md"),
    )
    .unwrap();
    for command in ["parse", "detect", "validate"] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .current_dir(directory.path())
            .arg(command)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stderr, Vec::<u8>::new());
        match command {
            "parse" => assert_eq!(
                serde_json::from_slice::<Changelog>(&output.stdout)
                    .unwrap()
                    .source,
                original
            ),
            "detect" => assert_eq!(output.stdout, b"keep-a-changelog\n"),
            _ => assert_eq!(output.stdout, Vec::<u8>::new()),
        }
    }
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["parse", "other.md"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Changelog>(&output.stdout)
            .unwrap()
            .format
            .as_str(),
        "conventional-changelog"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["detect", "other.md"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"conventional-changelog\n");
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["validate", "invalid.md"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn missing_default_file_reports_the_default_path_as_an_io_error() {
    let directory = tempfile::tempdir().unwrap();
    for command in ["parse", "detect", "validate"] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .current_dir(directory.path())
            .arg(command)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("cannot read CHANGELOG.md"));
        assert!(error.contains("Check the file path"));
    }
}
