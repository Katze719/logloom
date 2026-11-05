//! Opting out of the input cap must reach readers, detection, and named parsing.

use logloom::{DetectionError, ErrorKind, Format, MAX_INPUT_BYTES, ParseOptions, detect, parse};

#[test]
fn configurable_caps_count_bytes_and_accept_the_exact_boundary() {
    let source = "## [Unreleased]\n";
    let mut options = ParseOptions::default();
    options.max_input_bytes = Some(source.len());
    assert_eq!(options.detect(source).unwrap(), Format::KeepAChangelog);
    assert_eq!(
        options
            .parse(source, Format::KeepAChangelog)
            .unwrap()
            .source,
        source
    );
    options.max_input_bytes = Some(source.len().checked_sub(1).unwrap());
    assert_eq!(
        options
            .parse(source, Format::KeepAChangelog)
            .unwrap_err()
            .kind,
        ErrorKind::ResourceLimit
    );
    assert!(
        matches!(options.detect(source), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::ResourceLimit)
    );

    options.max_input_bytes = Some(8);
    assert_eq!(
        options
            .parse_named("🦀🦀", Format::Towncrier, "123.bugfix.rst")
            .unwrap()
            .source,
        "🦀🦀"
    );
    assert_eq!(
        options
            .parse_named("🦀🦀x", Format::Towncrier, "123.bugfix.rst")
            .unwrap_err()
            .kind,
        ErrorKind::ResourceLimit
    );
    assert!(
        matches!(options.detect_named("🦀🦀x", "123.bugfix.rst"), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::ResourceLimit)
    );
}

#[test]
fn explicit_opt_out_allows_large_inputs_and_changeset_detection() {
    let options = ParseOptions::without_input_limit();
    let source = format!(
        "---\n# {}\npkg: patch\n---\nFix Unicode paths.\n",
        "x".repeat(MAX_INPUT_BYTES)
    );
    assert_eq!(
        parse(&source, Format::Changesets).unwrap_err().kind,
        ErrorKind::ResourceLimit
    );
    assert!(
        matches!(detect(&source), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::ResourceLimit)
    );
    assert_eq!(options.detect(&source).unwrap(), Format::Changesets);
    let model = options.parse(&source, Format::Changesets).unwrap();
    assert_eq!(model.source, source);
    assert_eq!(model.releases[0].package(), Some("pkg"));
    assert!(model.releases[0].is_pending());
    assert_eq!(
        options.detect_named(&source, "notes.md").unwrap(),
        Format::Changesets
    );
}

#[test]
fn removing_the_input_cap_keeps_nesting_and_expansion_limits() {
    let options = ParseOptions::without_input_limit();
    let nested = format!("## [Unreleased]\n{}deep\n", "> ".repeat(2000));
    assert_eq!(
        options
            .parse(&nested, Format::KeepAChangelog)
            .unwrap_err()
            .kind,
        ErrorKind::ResourceLimit
    );
    assert!(
        matches!(options.detect(&nested), Err(DetectionError::InvalidInput(error)) if error.kind == ErrorKind::ResourceLimit)
    );
    let changeset = format!("---\na: minor\nb: patch\n---\n{}", "x".repeat(9_000_000));
    assert_eq!(
        options
            .parse(&changeset, Format::Changesets)
            .unwrap_err()
            .kind,
        ErrorKind::ResourceLimit
    );
}

#[cfg(feature = "cli")]
mod cli {
    use std::io::{self, Write};
    use std::process::{Command, Output, Stdio};

    use logloom::{Changelog, MAX_INPUT_BYTES};

    fn stdin(source: &str, args: &[&str]) -> io::Result<Output> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let result = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing stdin"))?
            .write_all(source.as_bytes());
        if let Err(error) = result
            && error.kind() != io::ErrorKind::BrokenPipe
        {
            return Err(error);
        }
        child.wait_with_output()
    }

    #[test]
    fn both_flag_names_work_for_files_stdin_and_every_input_command() {
        let source = format!(
            "# Changelog\n\n## [Unreleased]\n{}",
            " ".repeat(MAX_INPUT_BYTES)
        );
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();
        for command in ["parse", "validate", "detect"] {
            let rejected = Command::new(env!("CARGO_BIN_EXE_logloom"))
                .arg(command)
                .arg(file.path())
                .output()
                .unwrap();
            assert_eq!(rejected.status.code(), Some(1));
            assert_eq!(rejected.stdout, Vec::<u8>::new());
            assert!(String::from_utf8_lossy(&rejected.stderr).contains("--yes-i-know-i-am-stupid"));
            for flag in ["--no-size-limit", "--yes-i-know-i-am-stupid"] {
                let from_file = Command::new(env!("CARGO_BIN_EXE_logloom"))
                    .arg(flag)
                    .arg(command)
                    .arg(file.path())
                    .output()
                    .unwrap();
                let from_stdin = stdin(&source, &[command, "-", flag]).unwrap();
                for output in [from_file, from_stdin] {
                    assert!(
                        output.status.success(),
                        "{command} {flag}: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    assert_eq!(output.stderr, Vec::<u8>::new());
                    match command {
                        "parse" => {
                            let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
                            assert_eq!(model.releases.len(), 1);
                            assert_eq!(model.source, "");
                        }
                        "detect" => assert_eq!(output.stdout, b"keep-a-changelog\n"),
                        _ => assert_eq!(output.stdout, Vec::<u8>::new()),
                    }
                }
            }
        }
        let rejected = stdin(&source, &["parse", "-"]).unwrap();
        assert_eq!(rejected.status.code(), Some(1));
        assert_eq!(rejected.stdout, Vec::<u8>::new());
    }

    #[test]
    fn explicit_formats_named_fragments_and_validation_keep_their_rules() {
        let source = format!("Fix Unicode paths.\n{}", " ".repeat(MAX_INPUT_BYTES));
        let output = stdin(
            &source,
            &[
                "parse",
                "-",
                "--no-size-limit",
                "--input-format",
                "towncrier",
                "--fragment-name",
                "123.bugfix.rst",
            ],
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            model.releases[0]
                .attributes
                .get("issue")
                .map(String::as_str),
            Some("123")
        );

        for flag in ["--no-size-limit", "--yes-i-know-i-am-stupid"] {
            let bad_date = stdin(
                "# Changelog\n\n## [v1] - 2025-02-29\n",
                &["parse", "-", flag],
            )
            .unwrap();
            assert_eq!(bad_date.status.code(), Some(1));
            assert_eq!(bad_date.stdout, Vec::<u8>::new());
            assert!(String::from_utf8_lossy(&bad_date.stderr).contains("invalid-date"));
            let nested = format!("## [Unreleased]\n{}deep\n", "> ".repeat(2000));
            let output = stdin(&nested, &["parse", "-", flag]).unwrap();
            assert_eq!(output.status.code(), Some(1));
            assert_eq!(output.stdout, Vec::<u8>::new());
            assert!(String::from_utf8_lossy(&output.stderr).contains("128 containers"));
        }
    }
}
