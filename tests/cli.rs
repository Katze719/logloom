//! Black-box tests of the executable, output channels, and exit codes.

#![cfg(feature = "cli")]

use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

use logloom::{Changelog, MAX_INPUT_BYTES};

fn invoke(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(arguments)
        .output()
}

fn stdin(input: &str) -> io::Result<Output> {
    stdin_with_arguments(input, &["parse", "-"])
}

fn stdin_with_arguments(input: &str, arguments: &[&str]) -> io::Result<Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("child stdin was not piped"))?
        .write_all(input.as_bytes())?;
    child.wait_with_output()
}

#[test]
fn default_and_explicit_options_emit_valid_json() {
    for arguments in [
        vec!["parse", "tests/fixtures/standard.md"],
        vec![
            "parse",
            "tests/fixtures/standard.md",
            "--output-format",
            "json",
            "--input-format",
            "keep-a-changelog",
        ],
    ] {
        let output = invoke(&arguments).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stderr, Vec::<u8>::new());
        assert!(output.stdout.ends_with(b"\n"));
        let changelog: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(changelog.releases.len(), 3);
    }
}

#[test]
fn standard_input_works() {
    let output = stdin(include_str!("fixtures/only-unreleased.md")).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    let changelog: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(changelog.releases[0].version, None);
}

#[test]
fn help_version_and_subcommand_help_are_available() {
    for arguments in [
        vec!["--help"],
        vec!["--version"],
        vec!["parse", "--help"],
        vec!["parse", "--version"],
        vec!["validate", "--version"],
        vec!["detect", "--version"],
    ] {
        let output = invoke(&arguments).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stderr, Vec::<u8>::new());
        assert_ne!(output.stdout, Vec::<u8>::new());
    }
    let output = invoke(&["--version"]).unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        concat!("logloom ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn usage_errors_exit_two_and_use_stderr() {
    for arguments in [
        vec![],
        vec!["parse", "-", "--format", "keep-a-changelog"],
        vec!["parse", "-", "--output", "json"],
        vec!["parse", "-", "--input-format", "unknown"],
        vec!["parse", "-", "--output-format", "xml"],
        vec!["parse", "-", "--compact"],
    ] {
        let output = invoke(&arguments).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_ne!(output.stderr, Vec::<u8>::new());
    }
}

#[test]
fn malformed_input_exits_one_without_json() {
    let output = invoke(&["parse", "tests/fixtures/malformed-heading.md"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostic.contains("error[malformed-heading]"));
    assert!(diagnostic.contains("malformed-heading.md:3:1"));
    assert!(diagnostic.contains("3 | ## [1.0.0 - 2026-11-04"));
    assert!(diagnostic.contains("^^^^"));
    assert!(diagnostic.contains("help: For Keep a Changelog"));
    let empty = stdin("").unwrap();
    assert_eq!(empty.status.code(), Some(1));
    assert_eq!(empty.stdout, Vec::<u8>::new());
}

#[test]
fn warnings_preserve_json_and_success_exit_code() {
    let output = invoke(&["parse", "tests/fixtures/unconventional.md"]).unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("warning[missing-title]")
    );
    let changelog: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(changelog.releases.len(), 4);
}

#[test]
fn validation_errors_show_all_source_locations_and_emit_no_json() {
    let output = invoke(&["parse", "tests/fixtures/invalid-dates.md"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert_eq!(diagnostic.matches("error[invalid-date]").count(), 4);
    for line in [3, 5, 7, 9] {
        assert!(diagnostic.contains(&format!("invalid-dates.md:{line}:1")));
    }
    assert!(diagnostic.contains("## [2.0] - 2025-02-29"));
    assert!(diagnostic.contains("help: Use YYYY-MM-DD"));
}

#[test]
fn selected_format_controls_parse_error_help() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/malformed-heading.md",
        "--input-format",
        "keep-a-changelog",
        "--color",
        "never",
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostic.contains("help: For Keep a Changelog"));
    assert!(diagnostic.contains("## [version] - YYYY-MM-DD"));
    assert!(diagnostic.contains("### categories"));
}

#[test]
fn color_always_styles_errors_even_when_redirected() {
    let output = invoke(&[
        "--color",
        "always",
        "parse",
        "tests/fixtures/invalid-dates.md",
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(output.stderr.contains(&0x1b));
}

#[test]
fn color_never_and_auto_keep_redirected_diagnostics_plain() {
    for color in ["never", "auto"] {
        let output = invoke(&[
            "parse",
            "tests/fixtures/malformed-heading.md",
            "--color",
            color,
        ])
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(!output.stderr.contains(&0x1b));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("error[malformed-heading]")
        );
    }
}

#[test]
fn color_overrides_also_apply_to_usage_errors() {
    for color in ["always", "never"] {
        let output = invoke(&["parse", "-", "--output-format", "xml", "--color", color]).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(output.stderr.contains(&0x1b), color == "always");
    }
}

#[test]
fn colored_warnings_leave_json_unstyled_and_valid() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/unconventional.md",
        "--color",
        "always",
    ])
    .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.contains(&0x1b));
    assert!(!output.stdout.contains(&0x1b));
    let changelog: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(changelog.releases.len(), 4);
}

#[test]
fn io_errors_use_the_same_colored_diagnostic_style() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/this-file-does-not-exist.md",
        "--color",
        "always",
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(output.stderr.contains(&0x1b));
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostic.contains("read-input"));
    assert!(diagnostic.contains("Check the file path"));
}

#[test]
fn stdin_errors_handle_unicode_and_crlf_source_locations() {
    let output = stdin("# Änderungen 🦀\r\n\r\n## [v🦀] - 2025-02-29\r\n").unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostic.contains("<stdin>:3:1"));
    assert!(diagnostic.contains("## [v🦀] - 2025-02-29"));
    assert!(diagnostic.contains("error[invalid-date]"));
    assert!(!diagnostic.contains("panicked"));
}

#[test]
fn duplicate_releases_with_warnings_still_emit_no_json() {
    let output = stdin_with_arguments(
        "## 1.0\n## 1.0\n",
        &["parse", "-", "--input-format", "keep-a-changelog"],
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let diagnostic = String::from_utf8(output.stderr).unwrap();
    assert!(diagnostic.contains("error[duplicate-release]"));
    assert!(diagnostic.contains("warning[missing-date]"));
    assert!(diagnostic.contains("<stdin>:2:1"));
    assert!(diagnostic.contains("unique identifier"));
}

#[test]
fn missing_file_exits_three() {
    let output = invoke(&["parse", "tests/fixtures/this-file-does-not-exist.md"]).unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("cannot read")
    );
}

#[test]
fn invalid_utf8_is_reported_without_panicking() {
    let path = std::env::temp_dir().join(format!("logloom-invalid-utf8-{}.md", std::process::id()));
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not UTF-8")
    );
}

#[test]
fn oversized_file_is_rejected_before_parsing() {
    let path = std::env::temp_dir().join(format!("logloom-oversized-{}.md", std::process::id()));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(
        u64::try_from(MAX_INPUT_BYTES)
            .unwrap()
            .checked_add(1)
            .ok_or("size overflow")
            .unwrap(),
    )
    .unwrap();
    drop(file);
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(String::from_utf8(output.stderr).unwrap().contains("16 MiB"));
}

#[test]
fn a_closed_output_pipe_is_handled_without_a_panic() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["parse", "tests/fixtures/standard.md"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
}

// APFS rejects non-UTF-8 names before the CLI can open them.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_file_names_are_supported() {
    use std::os::unix::ffi::OsStrExt;

    let path = std::env::temp_dir()
        .join(format!("logloom-path-{}", std::process::id()))
        .with_extension(std::ffi::OsStr::from_bytes(b"md\xff"));
    std::fs::write(&path, include_str!("fixtures/only-unreleased.md")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[cfg(unix)]
#[test]
fn nonexistent_non_utf8_input_paths_report_io_errors_without_panicking() {
    use std::os::unix::ffi::OsStrExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join(std::ffi::OsStr::from_bytes(b"missing-\xff.md"));
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("error[read-input]"));
    assert!(!error.contains("panicked"));
}

#[test]
fn every_format_and_fragment_example_can_be_parsed_from_the_cli() {
    let cases = [
        (
            "conventional-changelog",
            "examples/conventional-changelog.md",
        ),
        ("generic-markdown", "examples/generic-markdown.md"),
        ("github-release-notes", "examples/github-release-notes.md"),
        ("github-release-notes", "examples/github-release-notes.json"),
        ("changesets", "examples/changesets.md"),
        ("changesets", "examples/changesets-fragment.md"),
        ("towncrier", "examples/towncrier.rst"),
        ("towncrier", "examples/towncrier.md"),
        ("towncrier", "examples/towncrier/123.bugfix.rst"),
        ("debian-changelog", "examples/debian.changelog"),
        ("gnu-news", "examples/gnu-news.txt"),
        ("gnu-changelog", "examples/gnu-ChangeLog"),
    ];
    for (format, file) in cases {
        let output =
            invoke(&["parse", file, "--input-format", format, "--color", "never"]).unwrap();
        assert!(
            output.status.success(),
            "{file}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stderr, Vec::<u8>::new());
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            json.get("format").and_then(serde_json::Value::as_str),
            Some(format)
        );
    }
}

#[test]
fn new_formats_use_their_own_error_help_and_never_emit_partial_json() {
    let cases = [
        (
            "conventional-changelog",
            "# Changelog\n",
            "Conventional Changelog",
        ),
        ("generic-markdown", "## [broken\n", "Generic Markdown"),
        ("github-release-notes", "{", "GitHub Release Notes"),
        (
            "changesets",
            "---\npkg: invalid\n---\nSummary",
            "Changesets",
        ),
        (
            "debian-changelog",
            "pkg (1.0) unstable; urgency=medium\n",
            "Debian Changelog",
        ),
        ("gnu-news", "No release headers.", "GNU NEWS"),
        ("gnu-changelog", "No date headers.", "GNU ChangeLog"),
    ];
    for (format, input, expected_help) in cases {
        let output = stdin_with_arguments(
            input,
            &["parse", "-", "--input-format", format, "--color", "never"],
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains(expected_help), "{format}: {diagnostic}");
        assert!(!diagnostic.contains("Keep a Changelog"));
        assert!(!diagnostic.contains('\u{1b}'));
    }
}

#[test]
fn towncrier_stdin_uses_explicit_fragment_names_and_allows_empty_misc() {
    let output = stdin_with_arguments(
        "Fix a bug.\n",
        &[
            "parse",
            "-",
            "--input-format",
            "towncrier",
            "--fragment-name",
            "123.bugfix.rst",
        ],
    )
    .unwrap();
    assert!(output.status.success());
    let changelog: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        changelog.releases[0]
            .attributes
            .get("issue")
            .map(String::as_str),
        Some("123")
    );
    let misc = stdin_with_arguments(
        "",
        &[
            "parse",
            "-",
            "--input-format",
            "towncrier",
            "--fragment-name",
            "123.misc",
        ],
    )
    .unwrap();
    assert!(misc.status.success());
    let invalid = stdin_with_arguments(
        "Fix a bug.\n",
        &[
            "parse",
            "-",
            "--input-format",
            "towncrier",
            "--fragment-name",
            "invalid",
        ],
    )
    .unwrap();
    assert_eq!(invalid.status.code(), Some(1));
    assert_eq!(invalid.stdout, Vec::<u8>::new());
    assert!(
        String::from_utf8(invalid.stderr)
            .unwrap()
            .contains("Towncrier")
    );
}

#[test]
fn new_format_validation_errors_also_suppress_json() {
    for (format, input) in [
        ("conventional-changelog", "## 1.0 (2025-02-29)\n"),
        ("changesets", "# pkg\n## 1.0\n## 1.0\n"),
        (
            "github-release-notes",
            r#"{"tag_name":"v1","published_at":"2025-02-29T12:00:00Z"}"#,
        ),
        (
            "gnu-news",
            "* Noteworthy changes in release 1.0 (2025-02-29)\n",
        ),
        (
            "gnu-changelog",
            "2025-02-29  Author <author@example.com>\n\t* file: change\n",
        ),
    ] {
        let output = stdin_with_arguments(
            input,
            &["parse", "-", "--input-format", format, "--color", "always"],
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(output.stderr.contains(&0x1b));
    }
}

#[test]
fn extended_keep_a_changelog_example_emits_component_categories_without_warnings() {
    let output = invoke(&["parse", "examples/extended-keep-a-changelog.md"]).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    let changelog: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        changelog.releases[3].sections[0].component.as_deref(),
        Some("Firmware components")
    );
    assert_eq!(changelog.releases[3].sections[0].category, None);
    assert_eq!(
        changelog.releases[3].sections[1].category,
        Some(logloom::Category::Added)
    );
    assert_eq!(
        changelog.releases[3].sections[2].category,
        Some(logloom::Category::Changed)
    );
}
