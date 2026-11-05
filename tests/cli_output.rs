//! File output must leave the destination intact until the whole result is ready.

#![cfg(feature = "cli")]

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use logloom::Changelog;

#[test]
fn output_file_creates_and_replaces_files_without_stdout() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("release notes.json");
    for flag in ["--output-file", "-o"] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args([
                "parse",
                "tests/fixtures/standard.md",
                "--latest",
                "--input-format",
                "keep-a-changelog",
                "--output-format",
                "json-compact",
                "--category",
                "Fixed",
                flag,
            ])
            .arg(&destination)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(output.stderr, Vec::<u8>::new());
        let bytes = fs::read(&destination).unwrap();
        assert!(bytes.ends_with(b"\n"));
        let model: Changelog = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(model.releases.len(), 1);
        assert_eq!(model.releases[0].version.as_deref(), Some("2.1.0"));
        fs::write(&destination, "replace this").unwrap();
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn relative_output_path_without_parent_works() {
    let directory = tempfile::tempdir().unwrap();
    let input = fs::canonicalize("tests/fixtures/standard.md").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .arg("parse")
        .arg(input)
        .args(["-o", "changelog.json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let model: Changelog =
        serde_json::from_slice(&fs::read(directory.path().join("changelog.json")).unwrap())
            .unwrap();
    assert_eq!(model.releases.len(), 3);
}

#[test]
fn stdin_can_be_written_directly_to_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("stdin.json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["parse", "-", "-o"])
        .arg(&destination)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!("fixtures/only-unreleased.md"))
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    let model: Changelog = serde_json::from_slice(&fs::read(destination).unwrap()).unwrap();
    assert_eq!(model.releases[0].version, None);
}

#[test]
fn input_errors_and_usage_errors_leave_existing_output_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("existing.json");
    fs::write(&destination, "keep me").unwrap();
    for (input, extra, code) in [
        ("tests/fixtures/malformed-heading.md", vec![], 1),
        (
            "tests/fixtures/invalid-dates.md",
            vec!["--release", "1.6"],
            1,
        ),
        (
            "tests/fixtures/standard.md",
            vec!["--latest", "--unreleased"],
            2,
        ),
        (
            "tests/fixtures/standard.md",
            vec!["--output-format", "xml"],
            2,
        ),
        (
            "tests/fixtures/standard.md",
            vec![
                "--input-format",
                "keep-a-changelog",
                "--fragment-name",
                "1.bugfix.rst",
            ],
            2,
        ),
        ("does-not-exist.md", vec![], 3),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(["parse", input])
            .args(extra)
            .arg("-o")
            .arg(&destination)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "keep me");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
    let missing = directory.path().join("never-created.json");
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["parse", "tests/fixtures/invalid-dates.md", "-o"])
        .arg(&missing)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!missing.exists());
}

#[test]
fn output_io_errors_are_reported_and_temporary_files_are_cleaned_up() {
    let directory = tempfile::tempdir().unwrap();
    let existing_directory = directory.path().join("directory.json");
    fs::create_dir(&existing_directory).unwrap();
    for destination in [
        existing_directory.clone(),
        directory.path().join("missing/subdirectory.json"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args([
                "parse",
                "tests/fixtures/standard.md",
                "--color",
                "never",
                "-o",
            ])
            .arg(destination)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stdout, Vec::<u8>::new());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("error[output-io]"));
        assert!(error.contains("help:"));
        assert!(existing_directory.is_dir());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
fn the_input_cannot_be_overwritten_even_through_a_path_alias() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("changelog.md");
    let original = include_bytes!("fixtures/standard.md");
    fs::write(&input, original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .current_dir(directory.path())
        .args(["parse", "changelog.md", "-o"])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("error[output-path]")
    );
    assert_eq!(fs::read(&input).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn symlink_alias_of_the_input_cannot_be_overwritten() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("changelog.md");
    let destination = directory.path().join("alias.json");
    let original = include_bytes!("fixtures/standard.md");
    fs::write(&input, original).unwrap();
    std::os::unix::fs::symlink(&input, &destination).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&input)
        .arg("-o")
        .arg(&destination)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(&input).unwrap(), original);
    assert!(fs::symlink_metadata(&destination).unwrap().is_symlink());
}

#[test]
fn warnings_stay_on_stderr_and_never_color_the_output_file() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("warnings.json");
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args([
            "parse",
            "tests/fixtures/unconventional.md",
            "--color",
            "always",
            "-o",
        ])
        .arg(&destination)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(output.stderr.contains(&0x1b));
    let bytes = fs::read(destination).unwrap();
    assert!(!bytes.contains(&0x1b));
    assert!(serde_json::from_slice::<Changelog>(&bytes).is_ok());
}

// APFS cannot persist arbitrary non-UTF-8 filename bytes.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn output_paths_do_not_need_to_be_utf8() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let directory = tempfile::tempdir().unwrap();
    let destination = directory
        .path()
        .join(OsString::from_vec(b"release-\xff.json".to_vec()));
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["parse", "tests/fixtures/standard.md", "-o"])
        .arg(&destination)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(serde_json::from_slice::<Changelog>(&fs::read(destination).unwrap()).is_ok());
}

#[test]
fn unicode_input_and_output_paths_work_on_all_platforms() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("Änderungen-🦀.md");
    let output_file = directory.path().join("résumé-🦀.json");
    let original = include_bytes!("fixtures/only-unreleased.md");
    fs::write(&input, original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("parse")
        .arg(&input)
        .arg("-o")
        .arg(&output_file)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(output.stderr, Vec::<u8>::new());
    let model: Changelog = serde_json::from_slice(&fs::read(output_file).unwrap()).unwrap();
    assert_eq!(model.releases.len(), 1);
    assert_eq!(model.releases[0].version, None);
    assert_eq!(fs::read(input).unwrap(), original);
}
