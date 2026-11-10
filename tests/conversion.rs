//! Experimental conversion contracts across native histories and CLI exports.
#![cfg(feature = "cli")]

use std::fs;
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

use logloom::{Changelog, EntryKind, Format, ParseOptions, parse};

const NATIVE: [(Format, &str); 9] = [
    (Format::KeepAChangelog, "tests/fixtures/output-rich.md"),
    (
        Format::ConventionalChangelog,
        "examples/conventional-changelog.md",
    ),
    (Format::GenericMarkdown, "examples/generic-markdown.md"),
    (
        Format::GitHubReleaseNotes,
        "examples/github-release-notes.json",
    ),
    (Format::Changesets, "examples/changesets.md"),
    (Format::Towncrier, "examples/towncrier.rst"),
    (Format::DebianChangelog, "examples/debian.changelog"),
    (Format::GnuNews, "examples/gnu-news.txt"),
    (Format::GnuChangeLog, "examples/gnu-ChangeLog"),
];

fn exports() -> Vec<&'static str> {
    let mut formats = vec!["json", "json-compact", "jsonl", "text", "markdown"];
    if cfg!(feature = "yaml") {
        formats.push("yaml");
    }
    if cfg!(feature = "toml") {
        formats.push("toml");
    }
    if cfg!(feature = "csv") {
        formats.push("csv");
    }
    formats
}

fn stdin(input: &str, args: &[&str]) -> io::Result<Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["convert", "-"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let written = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin not piped"))?
        .write_all(input.as_bytes());
    // Usage errors may close stdin before this helper writes even a small input.
    // Keep the child's actual diagnostic and status available to the caller.
    if let Err(error) = written
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error);
    }
    child.wait_with_output()
}

fn convert(input: &str, source: &str, target: &str) -> io::Result<Output> {
    stdin(
        input,
        &[
            "--input-format",
            source,
            "--output-format",
            target,
            "--color",
            "never",
        ],
    )
}

fn success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Experimental — best effort"));
    assert!(!output.stdout.contains(&0x1b));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Experimental — best effort"));
}

fn exported(file: &str, source: &str, target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let result = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args([
            "parse",
            file,
            "--input-format",
            source,
            "--output-format",
            target,
        ])
        .output()?;
    if !result.status.success() {
        return Err(format!("{file}: {}", String::from_utf8_lossy(&result.stderr)).into());
    }
    Ok(String::from_utf8(result.stdout)?)
}

#[test]
fn every_source_converts_to_every_available_destination() {
    let mut sources: Vec<(String, String)> = NATIVE
        .iter()
        .map(|(format, file)| (format.as_str().into(), fs::read_to_string(file).unwrap()))
        .collect();
    for format in exports() {
        sources.push((
            format.into(),
            exported("tests/fixtures/output-rich.md", "keep-a-changelog", format).unwrap(),
        ));
    }
    // Distinct input variants also feed every destination.
    for (format, file) in [
        ("changesets", "examples/changesets-fragment.md"),
        ("github-release-notes", "examples/github-release-notes.md"),
        ("towncrier", "examples/towncrier.md"),
    ] {
        sources.push((format.into(), fs::read_to_string(file).unwrap()));
    }
    for (source, input) in sources {
        let baseline = convert(&input, &source, "json").unwrap();
        success(&baseline, &source);
        let expected: Changelog = serde_json::from_slice(&baseline.stdout).unwrap();
        for target in exports()
            .into_iter()
            .chain(NATIVE.iter().map(|(format, _)| format.as_str()))
        {
            let output = convert(&input, &source, target).unwrap();
            let context = format!("{source} -> {target}");
            success(&output, &context);
            if let Some((format, _)) = NATIVE.iter().find(|(format, _)| format.as_str() == target) {
                let text = std::str::from_utf8(&output.stdout).unwrap();
                let actual = parse(text, *format)
                    .unwrap_or_else(|error| panic!("{context}: {error}\n{text}"));
                assert_eq!(actual.releases.len(), expected.releases.len(), "{context}");
                for (actual, expected) in actual.releases.iter().zip(&expected.releases) {
                    if expected
                        .sections
                        .iter()
                        .any(|section| !section.entries.is_empty())
                    {
                        assert!(
                            actual
                                .sections
                                .iter()
                                .any(|section| !section.entries.is_empty()),
                            "{context}: release body lost"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn structured_imports_preserve_the_existing_document_contract() {
    let baseline = exported("tests/fixtures/output-rich.md", "keep-a-changelog", "json").unwrap();
    let expected: Changelog = serde_json::from_str(&baseline).unwrap();
    for format in exports()
        .into_iter()
        .filter(|format| matches!(*format, "json" | "json-compact" | "yaml" | "toml"))
    {
        let input = exported("tests/fixtures/output-rich.md", "keep-a-changelog", format).unwrap();
        for source in [format, "auto"] {
            let output = convert(&input, source, "json").unwrap();
            success(&output, source);
            assert_eq!(
                serde_json::from_slice::<Changelog>(&output.stdout).unwrap(),
                expected,
                "{format}/{source}"
            );
        }
    }
}

#[test]
fn placeholders_are_explicit_and_stable_without_changing_the_source_model() {
    let input = "# Changelog\n\n## [Unreleased]\n\n### Added\n- Hello.\n";
    let output = convert(input, "keep-a-changelog", "debian-changelog").unwrap();
    success(&output, "placeholders");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("logloom-placeholder (0.0.0-logloom-placeholder-1) UNRELEASED; urgency=low")
    );
    assert!(text.contains("Unknown <unknown@example.invalid>  Thu, 01 Jan 1970 00:00:00 +0000"));
    let stderr = String::from_utf8(output.stderr).unwrap();
    for field in [
        "package",
        "distributions",
        "urgency",
        "version",
        "maintainer",
        "date",
        "timestamp",
    ] {
        assert!(stderr.contains(field), "{field}");
    }
    let original = convert(input, "keep-a-changelog", "json").unwrap();
    assert_eq!(
        serde_json::from_slice::<Changelog>(&original.stdout)
            .unwrap()
            .releases[0]
            .version,
        None
    );
}

#[test]
fn strict_rejects_conversion_warnings_but_not_the_experimental_note() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result");
    fs::write(&path, "keep me").unwrap();
    let input = "# Changelog\n\n## [Unreleased]\n\n### Added\n- Hello.\n";
    let output = stdin(
        input,
        &[
            "--input-format",
            "keep-a-changelog",
            "--output-format",
            "debian-changelog",
            "--strict",
            "-o",
            path.to_str().unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep me");
    let output = stdin(
        input,
        &[
            "--input-format",
            "keep-a-changelog",
            "--output-format",
            "json",
            "--strict",
        ],
    )
    .unwrap();
    success(&output, "strict note");
}

#[test]
fn invalid_input_and_usage_preserve_destinations_and_channels() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result");
    fs::write(&path, "keep me").unwrap();
    for (input, format) in [
        ("{broken", "json"),
        ("{}\n", "jsonl"),
        ("## [1] - 2026-02-30\n", "keep-a-changelog"),
        ("# C\n\n## [1]\n\n## [1]\n", "keep-a-changelog"),
    ] {
        let output = stdin(
            input,
            &[
                "--input-format",
                format,
                "--output-format",
                "json",
                "-o",
                path.to_str().unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(1), "{format}");
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep me");
    }
    for args in [
        vec!["convert"],
        vec!["convert", "--output-format", "auto"],
        vec!["convert", "--output-format", "unknown"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
}

#[test]
fn ambiguous_auto_detection_requires_a_source_format() {
    let output = convert(
        "# Changelog\n\n## 1.2.0\n\n### Fixed\n- A fix.\n",
        "auto",
        "json",
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--input-format"));
    let output = convert("Just some arbitrary text.\n", "auto", "json").unwrap();
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn text_fallback_preserves_raw_content_and_recognizes_exported_structure() {
    let text = "Unclear **raw** text\r\n\r\n  with indentation\r\n";
    let output = convert(text, "text", "json").unwrap();
    success(&output, "text fallback");
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases[0].sections[0].entries[0].markdown, text);
    assert_eq!(
        model.releases[0].sections[0].entries[0].kind,
        EntryKind::Text
    );
    let text = exported("tests/fixtures/standard.md", "keep-a-changelog", "text").unwrap();
    let output = convert(&text, "text", "json").unwrap();
    success(&output, "text reconstruction");
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert!(model.releases.len() > 1);
    assert!(
        model
            .releases
            .iter()
            .flat_map(|release| &release.sections)
            .any(|section| section.category.is_some())
    );
}

#[test]
fn local_reference_targets_survive_combining_release_bodies() {
    let input = r####"[{"tag_name":"1.0","body":"### Fixed\n- [Guide][x]\n\n[x]: https://one.example/"},{"tag_name":"2.0","body":"### Fixed\n- [Guide][x]\n\n[x]: https://two.example/"}]"####;
    let output = convert(input, "github-release-notes", "keep-a-changelog").unwrap();
    success(&output, "local references");
    let text = String::from_utf8(output.stdout).unwrap();
    let model = parse(&text, Format::KeepAChangelog).unwrap();
    for (release, url) in model
        .releases
        .iter()
        .zip(["https://one.example/", "https://two.example/"])
    {
        assert!(
            release
                .sections
                .iter()
                .flat_map(|section| &section.entries)
                .any(|entry| entry.markdown.contains(url))
        );
    }
}

#[test]
fn files_filters_fragment_names_colors_and_io_errors_follow_cli_contracts() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("result");
    let result = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args([
            "convert",
            "tests/fixtures/standard.md",
            "--input-format",
            "keep-a-changelog",
            "--output-format",
            "json",
            "--first-published",
            "-o",
        ])
        .arg(&destination)
        .output()
        .unwrap();
    success(&result, "file output");
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert_eq!(
        serde_json::from_slice::<Changelog>(&fs::read(&destination).unwrap())
            .unwrap()
            .releases
            .len(),
        1
    );
    let output = stdin(
        "Fix paths.\n",
        &[
            "--input-format",
            "towncrier",
            "--fragment-name",
            "123.bugfix.rst",
            "--output-format",
            "json",
            "--color",
            "always",
        ],
    )
    .unwrap();
    success(&output, "fragment");
    assert!(output.stderr.contains(&0x1b));
    assert_eq!(
        serde_json::from_slice::<Changelog>(&output.stdout)
            .unwrap()
            .releases[0]
            .attributes["issue"],
        "123"
    );
    let output = stdin(
        "## [Unreleased]\n",
        &[
            "--input-format",
            "keep-a-changelog",
            "--fragment-name",
            "123.bugfix.rst",
            "--output-format",
            "json",
        ],
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["convert", "does-not-exist", "--output-format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let input = directory.path().join("input.md");
    fs::write(&input, "# Changelog\n\n## [Unreleased]\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("convert")
        .arg(&input)
        .args([
            "--input-format",
            "keep-a-changelog",
            "--output-format",
            "json",
            "-o",
        ])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        fs::read_to_string(&input)
            .unwrap()
            .starts_with("# Changelog")
    );
}

#[test]
fn jsonl_reconstruction_warns_and_never_uses_old_span_lines_as_json_locations() {
    let input = exported("tests/fixtures/standard.md", "keep-a-changelog", "jsonl").unwrap();
    let output = convert(&input, "auto", "json").unwrap();
    success(&output, "JSONL auto");
    assert!(String::from_utf8_lossy(&output.stderr).contains("no document metadata"));
    let mut model: Changelog = serde_json::from_str(
        &exported("tests/fixtures/standard.md", "keep-a-changelog", "json").unwrap(),
    )
    .unwrap();
    model.releases[0].date = Some("2026-02-30".into());
    model.releases[0].span.line = 2;
    let output = convert(
        &serde_json::to_string_pretty(&model).unwrap(),
        "json",
        "json",
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("<stdin>:2:"));
}

#[cfg(feature = "csv")]
#[test]
fn csv_keeps_multiline_content_empty_groups_and_warns_about_duplicate_boundaries() {
    let source = "# Changelog\n\n## [Unreleased]\n\n### Firmware\n\n#### Added\n- A **bold**, \"quoted\" entry.\n\n  More content.\n\n#### Added\n- Another entry.\n\n#### Fixed\n\n#### Fixed\n";
    let output = convert(source, "keep-a-changelog", "csv").unwrap();
    success(&output, "CSV export");
    let csv = std::str::from_utf8(&output.stdout).unwrap();
    let imported = convert(csv, "auto", "json").unwrap();
    success(&imported, "CSV import");
    let model: Changelog = serde_json::from_slice(&imported.stdout).unwrap();
    assert_eq!(model.releases[0].sections.len(), 3);
    assert_eq!(model.releases[0].sections[0].entries.len(), 2);
    assert!(
        model.releases[0].sections[0].entries[0]
            .markdown
            .contains('\n')
    );
    assert_eq!(
        model.releases[0].sections[0].component.as_deref(),
        Some("Firmware")
    );
    assert!(String::from_utf8_lossy(&imported.stderr).contains("boundary cannot be recovered"));
}

#[test]
fn imported_markdown_nesting_and_input_size_stay_bounded() {
    let mut model = ParseOptions::default()
        .parse(
            "# Changelog\n\n## [Unreleased]\n\n### Added\n- A change.\n",
            Format::KeepAChangelog,
        )
        .unwrap();
    model.releases[0].sections[0].entries[0].markdown = format!("{}deep", "> ".repeat(129));
    let output = convert(&serde_json::to_string(&model).unwrap(), "json", "text").unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("large.txt");
    fs::write(
        &input,
        "x".repeat(logloom::MAX_INPUT_BYTES.saturating_add(1)),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .arg("convert")
        .arg(input)
        .args(["--input-format", "text", "--output-format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn opaque_versions_components_and_untrusted_headings_survive_best_effort_output() {
    let input = "# Changelog\n\n## [spring-cleaning] - 2026-11-04\n\n### Component\n\n#### Fixed\n- A repair.\n\n##### Extra details\n\nMore details.\n";
    let output = convert(input, "keep-a-changelog", "keep-a-changelog").unwrap();
    success(&output, "opaque version");
    let text = std::str::from_utf8(&output.stdout).unwrap();
    let actual = parse(text, Format::KeepAChangelog).unwrap();
    assert_eq!(
        actual.releases[0].version.as_deref(),
        Some("spring-cleaning")
    );
    assert_eq!(
        actual.releases[0].sections[0].component.as_deref(),
        Some("Component")
    );
    assert!(text.contains("Extra details"));
    let mut model = parse(input, Format::KeepAChangelog).unwrap();
    model.releases[0].sections[0].entries[0].markdown = "## 999.0\n\nInjected release?".into();
    let output = convert(
        &serde_json::to_string(&model).unwrap(),
        "json",
        "conventional-changelog",
    )
    .unwrap();
    success(&output, "structural entry");
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(
        parse(text, Format::ConventionalChangelog)
            .unwrap()
            .releases
            .len(),
        1
    );
    assert!(text.contains("spring-cleaning"));
    assert!(text.contains("Injected release?"));
    let source = "Version 999\n\nMore raw text.\n";
    let output = stdin(
        source,
        &[
            "--input-format",
            "towncrier",
            "--fragment-name",
            "123.bugfix.txt",
            "--output-format",
            "gnu-news",
        ],
    )
    .unwrap();
    success(&output, "plain heading");
    assert_eq!(
        parse(
            std::str::from_utf8(&output.stdout).unwrap(),
            Format::GnuNews
        )
        .unwrap()
        .releases
        .len(),
        1
    );
}

#[test]
fn feature_gates_and_experimental_help_apply_to_both_directions() {
    let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["convert", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, Vec::<u8>::new());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("Experimental — best effort"));
    for (format, enabled) in [
        ("yaml", cfg!(feature = "yaml")),
        ("toml", cfg!(feature = "toml")),
        ("csv", cfg!(feature = "csv")),
    ] {
        let output = convert(
            "# Changelog\n\n## [Unreleased]\n",
            "keep-a-changelog",
            format,
        )
        .unwrap();
        if enabled {
            success(&output, format);
            let input = std::str::from_utf8(&output.stdout).unwrap();
            success(&convert(input, format, "json").unwrap(), format);
            assert!(help.contains(format));
        } else {
            assert_eq!(output.status.code(), Some(2));
            let source = convert("unused", format, "json").unwrap();
            assert_eq!(source.status.code(), Some(2));
        }
    }
}

#[test]
fn empty_exports_and_legacy_documents_have_documented_defaults() {
    let output = convert("", "jsonl", "json").unwrap();
    success(&output, "empty JSONL");
    assert_eq!(
        serde_json::from_slice::<Changelog>(&output.stdout)
            .unwrap()
            .releases
            .len(),
        0
    );
    let output = stdin(
        "# Changelog\n\n## [Unreleased]\n",
        &[
            "--input-format",
            "keep-a-changelog",
            "--output-format",
            "gnu-news",
            "--release",
            "missing",
        ],
    )
    .unwrap();
    success(&output, "empty native");
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no selected releases"));
    let legacy = include_str!("fixtures/contracts/legacy-export.json");
    let output = convert(legacy, "json", "json").unwrap();
    success(&output, "legacy JSON");
    let expected: Changelog = serde_json::from_str(legacy).unwrap();
    assert_eq!(
        serde_json::from_slice::<Changelog>(&output.stdout).unwrap(),
        expected
    );
}

#[cfg(feature = "yaml")]
#[test]
fn yaml_aliases_are_rejected_without_misreading_literal_text_as_aliases() {
    for input in [
        "metadata: &x {}\nreleases: *x\n",
        "metadata: !!map &x {}\nreleases: *x\n",
        "? &x metadata\n: {}\n",
    ] {
        let output = convert(input, "yaml", "json").unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8_lossy(&output.stderr).contains("YAML"));
    }
    let input = "# Changelog\n\n## [Unreleased]\n\n### Added\n- A **bold** entry with &literal.\n\n```text\n&a\n*alias\n```\n";
    let export = convert(input, "keep-a-changelog", "yaml").unwrap();
    success(&export, "YAML literals");
    success(
        &convert(std::str::from_utf8(&export.stdout).unwrap(), "yaml", "json").unwrap(),
        "YAML literal import",
    );
}

#[test]
fn decoder_errors_have_current_locations_and_native_timestamps_use_real_dates() {
    let output = convert("{\n  \"metadata\": !\n}", "json", "json").unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("<stdin>:2:"));
    let mut model = parse("# C\n\n## [1] - 2026-11-04\n", Format::KeepAChangelog).unwrap();
    model.releases[0]
        .attributes
        .insert("timestamp".into(), "not a date".into());
    let output = convert(
        &serde_json::to_string(&model).unwrap(),
        "json",
        "debian-changelog",
    )
    .unwrap();
    success(&output, "timestamp fallback");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Wed, 04 Nov 2026 00:00:00 +0000"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("inconsistent timestamp"));
}

#[test]
fn pending_withdrawal_and_category_labels_never_create_extra_release_boundaries() {
    let input = "# Changelog\n\n## [Unreleased] [YANKED]\n\n### Roadmap (2026-11-04)\n- Plan.\n\n### Version 999\n- A label, not a release.\n";
    for (format, _) in NATIVE {
        let output = convert(input, "keep-a-changelog", format.as_str()).unwrap();
        success(&output, format.as_str());
        let model = parse(std::str::from_utf8(&output.stdout).unwrap(), format).unwrap();
        assert_eq!(model.releases.len(), 1, "{}", format.as_str());
    }
}

#[cfg(feature = "yaml")]
#[test]
fn yaml_quoted_apostrophes_and_comments_are_not_anchor_syntax() {
    let input = "# Changelog\n\nDon't: &break things.\n\n## [Unreleased]\n\n### Added\n- Don't: &break quoted strings.\n";
    let output = convert(input, "keep-a-changelog", "yaml").unwrap();
    success(&output, "YAML apostrophes");
    let mut yaml = String::from_utf8(output.stdout).unwrap();
    yaml.insert_str(0, "# comment: &not-an-anchor\n");
    yaml = yaml.replace(
        "format: keep-a-changelog",
        "format: keep-a-changelog # comment: &not-an-anchor",
    );
    let actual = convert(&yaml, "yaml", "json").unwrap();
    success(&actual, "YAML apostrophe import");
    assert_eq!(
        serde_json::from_slice::<Changelog>(&actual.stdout)
            .unwrap()
            .metadata
            .description,
        "Don't: &break things."
    );
}

#[test]
fn markdown_reading_views_recover_components_without_adopting_keep_a_changelog_validation() {
    let input = exported(
        "tests/fixtures/output-rich.md",
        "keep-a-changelog",
        "markdown",
    )
    .unwrap();
    let output = convert(&input, "markdown", "json").unwrap();
    success(&output, "Markdown components");
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.format, Format::GenericMarkdown);
    assert_eq!(
        model.releases[0].sections[0].component.as_deref(),
        Some("Firmware [core]")
    );
    let notes = exported(
        "examples/github-release-notes.md",
        "github-release-notes",
        "markdown",
    )
    .unwrap();
    let output = convert(&notes, "markdown", "json").unwrap();
    success(&output, "Markdown unversioned notes");
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases[0].version, None);
    assert_eq!(
        model.releases[0].status(),
        logloom::ReleaseStatus::Unversioned
    );
}

fn without_provenance(mut document: Changelog) -> Changelog {
    document.source.clear();
    for link in &mut document.links {
        link.span = logloom::Span::default();
    }
    for release in &mut document.releases {
        release.span = logloom::Span::default();
        release.date_span = None;
        for link in &mut release.links {
            link.span = logloom::Span::default();
        }
        for section in &mut release.sections {
            section.span = logloom::Span::default();
            for entry in &mut section.entries {
                entry.span = logloom::Span::default();
            }
        }
    }
    document
}

#[test]
fn keep_a_changelog_json_round_trip_preserves_the_model_including_raw_markdown_and_links() {
    for file in [
        "examples/keep-a-changelog.md",
        "examples/extended-keep-a-changelog.md",
        "tests/fixtures/components.md",
        "tests/fixtures/rich.md",
        "tests/fixtures/reference-links.md",
        "tests/fixtures/setext.md",
        "tests/fixtures/unconventional.md",
        "tests/fixtures/output-rich.md",
    ] {
        let source = fs::read_to_string(file).unwrap();
        for source in [source.clone(), source.replace('\n', "\r\n")] {
            let json = convert(&source, "keep-a-changelog", "json").unwrap();
            success(&json, file);
            let original: Changelog = serde_json::from_slice(&json.stdout).unwrap();
            assert_eq!(original.source, "");
            let output = convert(
                std::str::from_utf8(&json.stdout).unwrap(),
                "json",
                "keep-a-changelog",
            )
            .unwrap();
            success(&output, file);
            assert!(
                !String::from_utf8_lossy(&output.stderr).contains("conversion-loss"),
                "{file}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let actual = parse(
                std::str::from_utf8(&output.stdout).unwrap(),
                Format::KeepAChangelog,
            )
            .unwrap();
            assert_eq!(
                without_provenance(actual),
                without_provenance(original),
                "{file}"
            );
        }
    }
}

#[test]
fn extended_example_returns_reference_headings_and_compact_lists_without_loss_warnings() {
    let json = exported(
        "examples/extended-keep-a-changelog.md",
        "keep-a-changelog",
        "json",
    )
    .unwrap();
    let output = stdin(
        &json,
        &[
            "--input-format",
            "json",
            "--output-format",
            "keep-a-changelog",
            "--strict",
        ],
    )
    .unwrap();
    success(&output, "strict Keep a Changelog round trip");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("## [2.1.0] - 2026-10-14"));
    assert!(text.contains("[Unreleased]: <https://example.com/compare/v2.1.0...HEAD>"));
    assert!(text.contains("[documentation][docs]"));
    assert!(text.contains("- A streaming export API.\n- A command-line preview"));
    assert!(text.contains("### Firmware components\n\n#### Added"));
    assert!(!text.contains("Release link:"));
}

#[test]
fn keep_a_changelog_round_trip_respects_edited_json_fields_instead_of_replaying_source() {
    let source = "# Changelog\n\n## [1.0] - 2026-10-01\n\n### Added\n- Original entry.\n";
    let mut document = parse(source, Format::KeepAChangelog).unwrap();
    document.metadata.title = Some("Edited changelog".into());
    document.releases[0].version = Some("2.0".into());
    document.releases[0].date = Some("2026-11-04".into());
    document.releases[0].link = Some("https://example.com/edited".into());
    document.releases[0].sections[0].category = Some(logloom::Category::Fixed);
    document.releases[0].sections[0].entries[0].markdown = "- Edited entry.".into();
    let json = serde_json::to_string(&document).unwrap();
    let output = stdin(
        &json,
        &[
            "--input-format",
            "json",
            "--output-format",
            "keep-a-changelog",
            "--strict",
        ],
    )
    .unwrap();
    success(&output, "edited model");
    let actual = parse(
        std::str::from_utf8(&output.stdout).unwrap(),
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(actual.metadata.title, document.metadata.title);
    assert_eq!(actual.releases[0].version, document.releases[0].version);
    assert_eq!(actual.releases[0].date, document.releases[0].date);
    assert_eq!(actual.releases[0].link, document.releases[0].link);
    assert_eq!(
        actual.releases[0].sections[0].category,
        Some(logloom::Category::Fixed)
    );
    assert_eq!(
        actual.releases[0].sections[0].entries[0].markdown,
        "- Edited entry."
    );
    assert_eq!(document.source, source);
}

#[test]
fn keep_a_changelog_round_trip_keeps_component_introductions_empty_groups_and_deep_headings() {
    let source = "# Changelog\n\n## [Unreleased]\n\n### Firmware\n\nA component introduction.\n\n#### Added\n\n##### Further details\n\n###### Deep details\n\n### Fixed\n\n#### Ordinary subheading\n\nA paragraph.\n\n### CLI\n\n#### Fixed\n\n#### Fixed\n";
    let json = convert(source, "keep-a-changelog", "json").unwrap();
    success(&json, "component source");
    let output = convert(
        std::str::from_utf8(&json.stdout).unwrap(),
        "json",
        "keep-a-changelog",
    )
    .unwrap();
    success(&output, "component round trip");
    let actual = parse(
        std::str::from_utf8(&output.stdout).unwrap(),
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(
        without_provenance(actual),
        without_provenance(parse(source, Format::KeepAChangelog).unwrap())
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Uncategorized"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("```text"));
}
