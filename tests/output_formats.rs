//! Export contracts, escaping, source fidelity, and output failure behavior.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

use logloom::{Changelog, EntryKind, Format, Release, parse};

const FORMATS: [&str; 8] = [
    "json",
    "json-compact",
    "yaml",
    "toml",
    "jsonl",
    "csv",
    "text",
    "markdown",
];

fn invoke(file: &str, input: &str, output: &str, extra: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args([
            "parse",
            file,
            "--input-format",
            input,
            "--output-format",
            output,
        ])
        .args(extra)
        .output()
}

fn stdin(input: &str, arguments: &[&str]) -> io::Result<Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(["parse", "-"])
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
fn document_formats_round_trip_all_input_families_without_a_source_copy() {
    for (file, input) in [
        ("tests/fixtures/output-rich.md", "keep-a-changelog"),
        ("tests/fixtures/only-unreleased.md", "keep-a-changelog"),
        (
            "examples/conventional-changelog.md",
            "conventional-changelog",
        ),
        ("examples/generic-markdown.md", "generic-markdown"),
        ("examples/github-release-notes.json", "github-release-notes"),
        ("examples/github-release-notes.md", "github-release-notes"),
        ("examples/changesets.md", "changesets"),
        ("examples/changesets-fragment.md", "changesets"),
        ("examples/towncrier.rst", "towncrier"),
        ("examples/towncrier/123.bugfix.rst", "towncrier"),
        ("examples/debian.changelog", "debian-changelog"),
        ("examples/gnu-news.txt", "gnu-news"),
        ("examples/gnu-ChangeLog", "gnu-changelog"),
    ] {
        let baseline = invoke(file, input, "json", &[]).unwrap();
        assert!(
            baseline.status.success(),
            "{file}: {}",
            String::from_utf8_lossy(&baseline.stderr)
        );
        let expected: Changelog = serde_json::from_slice(&baseline.stdout).unwrap();
        assert_eq!(expected.source, "");
        for format in ["json-compact", "yaml", "toml"] {
            let output = invoke(file, input, format, &[]).unwrap();
            assert!(
                output.status.success(),
                "{file} / {format}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stdout.ends_with(b"\n"));
            let actual: Changelog = match format {
                "yaml" => yaml_serde::from_slice(&output.stdout).unwrap(),
                "toml" => toml::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap(),
                _ => serde_json::from_slice(&output.stdout).unwrap(),
            };
            assert_eq!(actual, expected, "{file} / {format}");
        }
    }
}

#[test]
fn structured_outputs_preserve_crlf_and_quote_ambiguous_yaml_strings() {
    let source = "# Changelog\r\n\r\n## [null] - 2026-10-01\r\n\r\n### Added\r\n\r\n- A Unicode 🦀 entry.\r\n\r\n## [true] - 2026-09-01\r\n";
    let mut expected = parse(source, Format::KeepAChangelog).unwrap();
    expected.source.clear();
    for format in ["json", "json-compact", "yaml", "toml"] {
        let output = stdin(source, &["--output-format", format]).unwrap();
        assert!(output.status.success());
        let actual: Changelog = match format {
            "yaml" => yaml_serde::from_slice(&output.stdout).unwrap(),
            "toml" => toml::from_str(std::str::from_utf8(&output.stdout).unwrap()).unwrap(),
            _ => serde_json::from_slice(&output.stdout).unwrap(),
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn exports_omit_source_entirely_including_filtered_and_file_output() {
    let source = "# Changelog\r\n\r\n## [2.0.0] - 2026-10-01\r\n\r\n### Fixed\r\n- Export this fix.\r\n\r\n## [1.0.0] - 2026-09-01\r\n\r\n### Added\r\n- UNSELECTED-SOURCE-ENTRY.\r\n\r\n<!-- PRIVATE-SOURCE-COMMENT -->\r\n";
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result");
    for format in FORMATS {
        let output = stdin(source, &["--release", "2.0.0", "--output-format", format]).unwrap();
        assert!(output.status.success(), "{format}");
        let text = std::str::from_utf8(&output.stdout).unwrap();
        assert!(!text.contains("PRIVATE-SOURCE-COMMENT"), "{format}");
        assert!(!text.contains("UNSELECTED-SOURCE-ENTRY"), "{format}");
        let document: Option<serde_json::Value> = match format {
            "json" | "json-compact" => Some(serde_json::from_str(text).unwrap()),
            "yaml" => Some(yaml_serde::from_str(text).unwrap()),
            "toml" => {
                Some(serde_json::to_value(toml::from_str::<toml::Value>(text).unwrap()).unwrap())
            }
            _ => None,
        };
        if let Some(document) = document {
            assert!(document.get("source").is_none(), "{format}");
            let model: Changelog = serde_json::from_value(document).unwrap();
            assert_eq!(model.source, "");
            assert_eq!(model.releases.len(), 1);
            assert_eq!(model.releases[0].version.as_deref(), Some("2.0.0"));
        }
        let output_file = stdin(
            source,
            &[
                "--release",
                "2.0.0",
                "--output-format",
                format,
                "--output-file",
                path.to_str().unwrap(),
            ],
        )
        .unwrap();
        assert!(output_file.status.success(), "{format}");
        assert_eq!(output_file.stdout, Vec::<u8>::new());
        assert_eq!(fs::read(&path).unwrap(), output.stdout, "{format}");
    }
}

#[test]
fn every_output_can_render_every_input_family() {
    for (file, input) in [
        ("tests/fixtures/output-rich.md", "keep-a-changelog"),
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
        for format in FORMATS {
            let output = invoke(file, input, format, &[]).unwrap();
            assert!(
                output.status.success(),
                "{file} / {format}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_ne!(output.stdout, Vec::<u8>::new());
            assert!(!output.stdout.contains(&0x1b));
        }
    }
}

#[test]
fn jsonl_has_one_complete_release_per_line_including_empty_releases() {
    let baseline = invoke(
        "tests/fixtures/output-rich.md",
        "keep-a-changelog",
        "json",
        &[],
    )
    .unwrap();
    let expected: Changelog = serde_json::from_slice(&baseline.stdout).unwrap();
    let output = invoke(
        "tests/fixtures/output-rich.md",
        "keep-a-changelog",
        "jsonl",
        &[],
    )
    .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let releases: Vec<Release> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(releases, expected.releases);
    assert_eq!(releases[1].sections.len(), 0);
    assert!(text.ends_with('\n'));
    assert!(!text.contains("\"source\""));
}

#[test]
fn csv_preserves_quoted_multiline_entries_components_and_empty_groups() {
    let output = invoke(
        "tests/fixtures/output-rich.md",
        "keep-a-changelog",
        "csv",
        &[],
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut reader = csv::Reader::from_reader(output.stdout.as_slice());
    let rows: Vec<BTreeMap<String, String>> =
        reader.deserialize().collect::<Result<_, _>>().unwrap();
    assert_eq!(rows.len(), 5);
    let entry = &rows[0];
    assert_eq!(entry["format"], "keep-a-changelog");
    assert_eq!(entry["row_type"], "entry");
    assert_eq!(entry["release_index"], "0");
    assert_eq!(entry["version"], "");
    assert_eq!(entry["component"], "Firmware [core]");
    assert_eq!(entry["category"], "Added");
    assert_eq!(entry["kind"], "list_item");
    assert!(entry["content"].contains("**bold**, \"quoted\""));
    assert!(entry["content"].contains("\n  - A nested child."));
    assert!(entry["content"].contains("A second paragraph"));
    assert_eq!(rows[1]["row_type"], "section");
    assert_eq!(rows[1]["category"], "Fixed");
    assert_eq!(rows[1]["content"], "");
    assert_eq!(rows[2]["component"], "");
    assert_eq!(rows[3]["row_type"], "release");
    assert_eq!(rows[3]["version"], "opaque+build");
    assert_eq!(rows[3]["yanked"], "true");
    assert_eq!(rows[4]["category"], "Performance");
}

#[test]
fn csv_preserves_package_attributes_as_compact_json() {
    let output = invoke(
        "examples/changesets-fragment.md",
        "changesets",
        "csv",
        &["--package", "@lantern/cli"],
    )
    .unwrap();
    assert!(output.status.success());
    let mut reader = csv::Reader::from_reader(output.stdout.as_slice());
    for row in reader.deserialize::<BTreeMap<String, String>>() {
        let row = row.unwrap();
        let attributes: BTreeMap<String, String> =
            serde_json::from_str(&row["attributes"]).unwrap();
        assert_eq!(attributes["package"], "@lantern/cli");
        assert_eq!(attributes["bump"], "patch");
    }
}

#[test]
fn markdown_keeps_components_empty_categories_links_and_yanked_releases() {
    let source = include_str!("fixtures/output-rich.md");
    let expected = parse(source, Format::KeepAChangelog).unwrap();
    let output = invoke(
        "tests/fixtures/output-rich.md",
        "keep-a-changelog",
        "markdown",
        &[],
    )
    .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("### Firmware \\[core\\]"));
    assert!(text.contains("#### Added\n"));
    assert!(text.contains("#### Fixed\n"));
    assert!(text.contains("## opaque+build - 2026-10-01 [YANKED]"));
    let actual = parse(&text, Format::KeepAChangelog).unwrap();
    assert_eq!(actual.releases.len(), expected.releases.len());
    for (actual, expected) in actual.releases.iter().zip(&expected.releases) {
        assert_eq!(actual.version, expected.version);
        assert_eq!(actual.yanked, expected.yanked);
        assert_eq!(actual.sections.len(), expected.sections.len());
        for (actual, expected) in actual.sections.iter().zip(&expected.sections) {
            assert_eq!(actual.category, expected.category);
            assert_eq!(actual.component, expected.component);
            assert_eq!(
                actual
                    .entries
                    .iter()
                    .map(|entry| &entry.markdown)
                    .collect::<Vec<_>>(),
                expected
                    .entries
                    .iter()
                    .map(|entry| &entry.markdown)
                    .collect::<Vec<_>>()
            );
        }
    }
    assert_eq!(actual.links[0].destination, expected.links[0].destination);
    assert_eq!(actual.links[0].title, expected.links[0].title);
}

#[test]
fn text_removes_markdown_styling_but_retains_nested_content_and_reference_urls() {
    let output = invoke(
        "tests/fixtures/output-rich.md",
        "keep-a-changelog",
        "text",
        &[],
    )
    .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("Changes 🦀\n"));
    assert!(text.contains("Firmware [core] / Added"));
    assert!(
        text.contains(
            "A bold, \"quoted\" fix with code and a manual <https://example.com/überblick>"
        )
    );
    assert!(text.contains("  - A nested child."));
    assert!(text.contains("A second paragraph"));
    assert!(text.contains("opaque+build - 2026-10-01 [YANKED]"));
    assert!(!text.contains("**bold**"));
    assert!(!text.contains("[GUIDE]"));
}

#[test]
fn text_reference_resolution_uses_commonmark_unicode_case_folding() {
    let source = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- [Read this][STRASSE] and ![an image][picture guide].\n\n[straße]: https://example.com/read\n[picture   guide]: https://example.com/image.png\n";
    let output = stdin(source, &["--output-format", "text"]).unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Read this <https://example.com/read>"));
    assert!(text.contains("an image <https://example.com/image.png>"));
}

#[test]
fn readable_unversioned_labels_keep_parser_classification_out_of_the_content() {
    for format in ["text", "markdown"] {
        let notes = invoke(
            "examples/github-release-notes.md",
            "github-release-notes",
            format,
            &[],
        )
        .unwrap();
        assert!(notes.status.success());
        let text = String::from_utf8(notes.stdout).unwrap();
        assert!(text.contains("Unversioned"));
        assert!(!text.contains("version-status"));
        let records = invoke("examples/gnu-ChangeLog", "gnu-changelog", format, &[]).unwrap();
        assert!(records.status.success());
        let text = String::from_utf8(records.stdout).unwrap();
        assert!(text.contains("Changes - 2026-11-04"));
        assert!(!text.contains("record-kind"));
        assert!(text.contains("author"));
    }
}

#[test]
fn raw_text_is_preserved_and_markdown_fences_cannot_be_closed_by_the_content() {
    let source = "Raw **RST**\n\n```\n## [fake-release] - 2026-10-01\n`````\n";
    for format in ["markdown", "text"] {
        let output = stdin(
            source,
            &[
                "--input-format",
                "towncrier",
                "--fragment-name",
                "123.bugfix.rst",
                "--output-format",
                format,
            ],
        )
        .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains(source.trim_end()));
        if format == "markdown" {
            assert!(text.contains("``````text\n"));
            let parsed = parse(&text, Format::KeepAChangelog).unwrap();
            assert_eq!(parsed.releases.len(), 1);
            let code: Vec<_> = parsed.releases[0]
                .sections
                .iter()
                .flat_map(|section| &section.entries)
                .filter(|entry| entry.markdown.contains("fake-release"))
                .collect();
            assert_eq!(code.len(), 1);
            assert_eq!(code[0].kind, EntryKind::Markdown);
        }
    }
}

#[test]
fn no_match_output_is_well_formed_in_every_format() {
    for format in FORMATS {
        let output = invoke(
            "tests/fixtures/output-rich.md",
            "keep-a-changelog",
            format,
            &["--release", "missing"],
        )
        .unwrap();
        assert!(output.status.success());
        match format {
            "jsonl" => assert_eq!(output.stdout, Vec::<u8>::new()),
            "csv" => {
                let mut reader = csv::Reader::from_reader(output.stdout.as_slice());
                assert_eq!(reader.headers().unwrap().len(), 14);
                assert_eq!(reader.records().count(), 0);
            }
            "yaml" => assert_eq!(
                yaml_serde::from_slice::<Changelog>(&output.stdout)
                    .unwrap()
                    .releases
                    .len(),
                0
            ),
            "toml" => assert_eq!(
                toml::from_str::<Changelog>(std::str::from_utf8(&output.stdout).unwrap())
                    .unwrap()
                    .releases
                    .len(),
                0
            ),
            "json" | "json-compact" => assert_eq!(
                serde_json::from_slice::<Changelog>(&output.stdout)
                    .unwrap()
                    .releases
                    .len(),
                0
            ),
            _ => assert!(
                !String::from_utf8(output.stdout)
                    .unwrap()
                    .contains("Less waiting.")
            ),
        }
    }
}

#[test]
fn every_format_supports_filtered_file_output_and_full_validation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result");
    for format in FORMATS {
        let expected = invoke(
            "tests/fixtures/standard.md",
            "keep-a-changelog",
            format,
            &["--latest", "--category", "Fixed"],
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args([
                "parse",
                "tests/fixtures/standard.md",
                "--latest",
                "--category",
                "Fixed",
                "--output-format",
                format,
                "-o",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success(), "{format}");
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(fs::read(&path).unwrap(), expected.stdout);
        fs::write(&path, "keep the existing output").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_logloom"))
            .args([
                "parse",
                "tests/fixtures/invalid-dates.md",
                "--release",
                "1.6",
                "--output-format",
                format,
                "-o",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "keep the existing output"
        );
        let output = invoke(
            "tests/fixtures/malformed-heading.md",
            "keep-a-changelog",
            format,
            &[],
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
    }
}
