//! Strict validation rejects warnings before filtering, rendering, or file output.

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
fn warnings_are_fatal_only_when_strict_is_requested() {
    for command in ["parse", "validate"] {
        let normal = invoke(&[command, "tests/fixtures/unconventional.md"]).unwrap();
        assert!(normal.status.success());
        if command == "parse" {
            assert!(serde_json::from_slice::<Changelog>(&normal.stdout).is_ok());
        }
        let strict = invoke(&[command, "--strict", "tests/fixtures/unconventional.md"]).unwrap();
        assert_eq!(strict.status.code(), Some(1));
        assert_eq!(strict.stdout, Vec::<u8>::new());
        let error = String::from_utf8(strict.stderr).unwrap();
        assert!(error.contains("warning[missing-title]"));
        assert!(error.contains("error[strict-validation]"));
        assert!(error.contains("omit --strict"));
    }
}

#[test]
fn strict_accepts_clean_inputs_and_uses_each_formats_own_conventions() {
    for file in [
        "tests/fixtures/standard.md",
        "examples/conventional-changelog.md",
        "examples/changesets.md",
        "examples/changesets-fragment.md",
        "examples/towncrier.rst",
        "examples/debian.changelog",
        "examples/gnu-news.txt",
        "examples/gnu-ChangeLog",
        "examples/github-release-notes.json",
        "examples/generic-markdown.md",
    ] {
        for command in ["parse", "validate"] {
            let output = invoke(&[command, file, "--strict"]).unwrap();
            assert!(
                output.status.success(),
                "{command} {file}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stderr, Vec::<u8>::new());
            if command == "parse" {
                assert!(serde_json::from_slice::<Changelog>(&output.stdout).is_ok());
            } else {
                assert_eq!(output.stdout, Vec::<u8>::new());
            }
        }
    }
}

#[test]
fn warnings_outside_the_filters_block_every_output_and_preserve_destinations() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("result");
    for format in [
        "json",
        "json-compact",
        "yaml",
        "toml",
        "jsonl",
        "csv",
        "text",
        "markdown",
    ] {
        fs::write(&destination, "keep the existing result").unwrap();
        for existing in [true, false] {
            let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
                .args([
                    "parse",
                    "tests/fixtures/unconventional.md",
                    "--release",
                    "release_candidate_1",
                    "--strict",
                    "--output-format",
                    format,
                    "-o",
                ])
                .arg(&destination)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(1), "{format}");
            assert_eq!(output.stdout, Vec::<u8>::new());
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(error.contains("error[strict-validation]"));
            if existing {
                assert_eq!(
                    fs::read_to_string(&destination).unwrap(),
                    "keep the existing result"
                );
                fs::remove_file(&destination).unwrap();
            } else {
                assert!(!destination.exists());
            }
        }
    }
}

#[test]
fn strict_summary_counts_all_warnings_including_grouped_ones() {
    let input = "# Changelog\n\n## [Unreleased]\n\n### Custom A\n\n### Custom B\n\n### Custom C\n";
    let output = stdin(input, &["validate", "-", "--strict", "--color", "always"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(output.stderr.contains(&0x1b));
    let error = String::from_utf8(output.stderr).unwrap();
    assert_eq!(error.matches("warning[custom-category]").count(), 1);
    assert!(error.contains("2 more occurrences"));
    assert!(error.contains("3 validation warnings"));
    let output = stdin(
        "# Changelog\n\n## [Unreleased]\n\n### Custom\n",
        &["validate", "-", "--strict"],
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("1 validation warning is not allowed")
    );
}

#[test]
fn notes_and_existing_error_exit_codes_keep_their_meaning_in_strict_mode() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/standard.md",
        "--strict",
        "--category",
        "missing",
        "--output-format",
        "jsonl",
    ])
    .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let note = String::from_utf8(output.stderr).unwrap();
    assert!(note.contains("note[no-matches]"));
    assert!(!note.contains("strict-validation"));
    for (file, code) in [
        ("tests/fixtures/invalid-dates.md", 1),
        ("tests/fixtures/malformed-heading.md", 1),
        ("does-not-exist.md", 3),
    ] {
        let output = invoke(&["parse", file, "--strict"]).unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!error.contains("strict-validation"));
        if file.ends_with("invalid-dates.md") {
            assert_eq!(error.matches("error[invalid-date]").count(), 4);
        }
    }
}

#[test]
fn strict_works_with_the_default_file_and_is_not_a_detection_option() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("CHANGELOG.md");
    fs::write(&path, include_str!("fixtures/only-unreleased.md")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["validate", "--strict"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    fs::write(&path, "## [Unreleased]\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["validate", "--strict"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let output = invoke(&["detect", "--strict"]).unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
}
