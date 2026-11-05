//! Release presence checks validate the whole document and stay quiet on stdout.

use std::fs;
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

fn invoke(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(arguments)
        .output()
}

fn stdin(source: &str, arguments: &[&str]) -> io::Result<Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["validate", "-"])
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin not piped"))?
        .write_all(source.as_bytes())?;
    child.wait_with_output()
}

#[test]
fn release_checks_accept_explicit_files_stdin_and_the_default_file() {
    let source = include_str!("fixtures/standard.md");
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("CHANGELOG.md"), source).unwrap();
    let default = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["validate", "--release", "2.1.0"])
        .output()
        .unwrap();
    let explicit = invoke(&[
        "validate",
        "tests/fixtures/standard.md",
        "--release",
        "2.1.0",
    ])
    .unwrap();
    let piped = stdin(source, &["--release", "2.1.0"]).unwrap();
    for output in [default, explicit, piped] {
        assert!(output.status.success());
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(output.stderr, Vec::<u8>::new());
    }
}

#[test]
fn missing_releases_fail_with_actionable_colored_diagnostics() {
    for (color, styled) in [("never", false), ("always", true)] {
        let output = invoke(&[
            "validate",
            "tests/fixtures/standard.md",
            "--release",
            "v2.1.0",
            "--color",
            color,
        ])
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("release-not-found"));
        assert!(error.contains("\"v2.1.0\""));
        assert!(error.contains("standard.md"));
        assert!(error.contains("exact release identifier"));
        assert!(error.contains("'v' prefix"));
        assert_eq!(error.contains('\x1b'), styled);
    }
}

#[test]
fn package_checks_require_the_version_and_package_on_the_same_release() {
    for (package, version, code) in [
        ("@lantern/core", "2.0.0", 0),
        ("@LANTERN/core", "2.0.0", 1),
        ("@lantern/other", "2.0.0", 1),
        ("@lantern/core", "9.0.0", 1),
    ] {
        let output = invoke(&[
            "validate",
            "examples/changesets.md",
            "--release",
            version,
            "--package",
            package,
        ])
        .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, Vec::<u8>::new());
        if code == 1 {
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(error.contains("release-not-found"));
            assert!(error.contains(package));
            assert!(error.contains(version));
        }
    }
    let source = include_str!("../examples/debian.changelog").replacen(
        "lantern (1:1.9.0-2)",
        "other (1:1.9.0-2)",
        1,
    );
    let output = stdin(
        &source,
        &[
            "--input-format",
            "debian-changelog",
            "--release",
            "1:2.0.0-1",
            "--package",
            "other",
        ],
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("release-not-found")
    );
}

#[test]
fn release_presence_uses_exact_opaque_identifiers_across_input_formats() {
    for (file, format, version) in [
        ("tests/fixtures/standard.md", "keep-a-changelog", "2.0.0"),
        (
            "examples/conventional-changelog.md",
            "conventional-changelog",
            "3.0.0",
        ),
        (
            "examples/generic-markdown.md",
            "generic-markdown",
            "Winter preview",
        ),
        (
            "examples/github-release-notes.json",
            "github-release-notes",
            "v1.1.0",
        ),
        ("examples/changesets.md", "changesets", "2.0.0"),
        ("examples/towncrier.md", "towncrier", "2.0.0"),
        ("examples/towncrier.rst", "towncrier", "2.0.0"),
        ("examples/debian.changelog", "debian-changelog", "1:2.0.0-1"),
        ("examples/gnu-news.txt", "gnu-news", "2.0"),
    ] {
        let output = invoke(&[
            "validate",
            file,
            "--input-format",
            format,
            "--release",
            version,
        ])
        .unwrap();
        assert!(
            output.status.success(),
            "{file}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
    for (version, code) in [
        ("Nightly+Build.7", 0),
        ("nightly+build.7", 1),
        ("Unreleased", 1),
    ] {
        let output = stdin(
            "# Changelog\n\n## [Unreleased]\n\n## [Nightly+Build.7] - 2026-11-04\n",
            &["--release", version],
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
    let output = invoke(&[
        "validate",
        "examples/gnu-ChangeLog",
        "--release",
        "2026-11-04",
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("release-not-found")
    );
    let output = stdin(
        r#"{"tag_name":"v2-rc1","draft":true,"prerelease":true,"body":"Preview."}"#,
        &["--release", "v2-rc1"],
    )
    .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn release_presence_does_not_hide_validation_errors_or_strict_warnings() {
    for version in ["1.6", "missing"] {
        let output = invoke(&[
            "validate",
            "tests/fixtures/invalid-dates.md",
            "--release",
            version,
        ])
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("error[invalid-date]"));
        assert!(!error.contains("release-not-found"));
    }
    for strict in [false, true] {
        let mut args = vec![
            "validate",
            "tests/fixtures/unconventional.md",
            "--input-format",
            "keep-a-changelog",
            "--release",
            "release_candidate_1",
        ];
        if strict {
            args.push("--strict");
        }
        let output = invoke(&args).unwrap();
        assert_eq!(output.status.code(), Some(i32::from(strict)));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("warning[missing-title]"));
        assert_eq!(error.contains("error[strict-validation]"), strict);
    }
}

#[test]
fn release_checks_keep_usage_and_io_failures_distinct() {
    for args in [
        vec!["validate", "--release", ""],
        vec!["validate", "--release"],
        vec!["validate", "--package", "@lantern/core"],
        vec!["validate", "--release", "1.0.0", "--package", ""],
    ] {
        let output = invoke(&args).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["validate", "--release", "1.0.0"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, Vec::<u8>::new());
}
