//! Selection is applied to the validated model without changing source offsets.

use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

use logloom::{Category, Changelog};

fn invoke(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_logloom"))
        .args(arguments)
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
fn exact_versions_are_repeatable_and_preserve_document_order() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/standard.md",
        "--version",
        "2.0.0",
        "--version",
        "2.1.0",
    ])
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        model
            .releases
            .iter()
            .filter_map(|release| release.version.as_deref())
            .collect::<Vec<_>>(),
        ["2.1.0", "2.0.0"]
    );
    assert!(model.releases[1].yanked);

    let output = stdin(
        "# Changelog\n\n## [release/alpha+build] - 2026-10-01\n\n### Added\n\n- Works.\n",
        &["--version", "release/alpha+build"],
    )
    .unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        model.releases[0].version.as_deref(),
        Some("release/alpha+build")
    );
}

#[test]
fn latest_skips_pending_and_is_applied_after_category_matching() {
    for (extra, expected) in [
        (vec!["--latest"], "2.1.0"),
        (vec!["--latest", "--category", "added"], "2.0.0"),
    ] {
        let mut args = vec!["parse", "tests/fixtures/standard.md"];
        args.extend(extra);
        let output = invoke(&args).unwrap();
        assert!(output.status.success());
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.releases.len(), 1);
        assert_eq!(model.releases[0].version.as_deref(), Some(expected));
    }
}

#[test]
fn latest_uses_source_order_instead_of_sorting_versions_or_dates() {
    let output = stdin("# Changelog\n\n## [older] - 2026-01-01\n\n### Added\n\n- First.\n\n## [newer] - 2026-02-01\n\n### Added\n\n- Second.\n", &["--latest"]).unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases.len(), 1);
    assert_eq!(model.releases[0].version.as_deref(), Some("older"));
}

#[test]
fn repeated_categories_use_normalized_names_and_keep_section_order() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/standard.md",
        "--version",
        "2.1.0",
        "--category",
        "SECURITY",
        "--category",
        "fixed",
    ])
    .unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases.len(), 1);
    assert_eq!(
        model.releases[0]
            .sections
            .iter()
            .filter_map(|section| section.category.clone())
            .collect::<Vec<_>>(),
        [Category::Fixed, Category::Security]
    );

    let output = invoke(&[
        "parse",
        "examples/conventional-changelog.md",
        "--input-format",
        "conventional-changelog",
        "--category",
        "Added",
    ])
    .unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases.len(), 1);
    assert_eq!(
        model.releases[0].sections[0].category,
        Some(Category::Added)
    );
    assert_eq!(
        model.releases[0].sections[0].heading.as_deref(),
        Some("### Features")
    );
}

#[test]
fn component_and_category_filters_intersect_and_keep_empty_categories() {
    for (component, category, count) in [
        ("firmware COMPONENTS", "fixed", 1),
        ("CLI 🦀", "Changed", 2),
        ("cli 🦀", "performance", 1),
    ] {
        let output = invoke(&[
            "parse",
            "tests/fixtures/components.md",
            "--component",
            component,
            "--category",
            category,
        ])
        .unwrap();
        assert!(output.status.success());
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.releases.len(), 1);
        assert_eq!(model.releases[0].sections.len(), count);
        if category == "fixed" {
            assert_eq!(model.releases[0].sections[0].entries.len(), 0);
        }
    }
}

#[test]
fn repeated_components_are_alternatives_without_global_sections() {
    let output = invoke(&[
        "parse",
        "tests/fixtures/components.md",
        "--component",
        "CLI 🦀",
        "--component",
        "Firmware components",
    ])
    .unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases.len(), 1);
    assert_eq!(model.releases[0].sections.len(), 6);
    assert!(
        model.releases[0]
            .sections
            .iter()
            .all(|section| section.component.is_some())
    );
}

#[test]
fn pending_package_selection_precedes_the_limit() {
    let output = invoke(&[
        "parse",
        "examples/changesets-fragment.md",
        "--input-format",
        "changesets",
        "--unreleased",
        "--package",
        "@lantern/cli",
        "--limit",
        "1",
    ])
    .unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases.len(), 1);
    assert_eq!(model.releases[0].version, None);
    assert_eq!(
        model.releases[0]
            .attributes
            .get("package")
            .map(String::as_str),
        Some("@lantern/cli")
    );

    let output = invoke(&[
        "parse",
        "examples/changesets-fragment.md",
        "--input-format",
        "changesets",
        "--package",
        "@lantern/cli",
        "--package",
        "@lantern/core",
    ])
    .unwrap();
    assert!(output.status.success());
    let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(model.releases.len(), 2);
    assert_eq!(
        model.releases[0]
            .attributes
            .get("package")
            .map(String::as_str),
        Some("@lantern/core")
    );
}

#[test]
fn unreleased_selects_pending_releases_and_limit_includes_them_by_default() {
    for options in [vec!["--unreleased"], vec!["--limit", "1"]] {
        let mut args = vec!["parse", "tests/fixtures/standard.md"];
        args.extend(options);
        let output = invoke(&args).unwrap();
        assert!(output.status.success());
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.releases.len(), 1);
        assert_eq!(model.releases[0].version, None);
    }
}

#[test]
fn github_drafts_are_pending_and_prereleases_can_be_latest() {
    let input = r#"[{"tag_name":"v3","draft":true,"body":"Draft"},{"tag_name":"v2-rc1","prerelease":true,"body":"Preview"},{"tag_name":"v1","body":"Stable"}]"#;
    for (selection, expected) in [("--latest", "v2-rc1"), ("--unreleased", "v3")] {
        let output = stdin(
            input,
            &["--input-format", "github-release-notes", selection],
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.releases.len(), 1);
        assert_eq!(model.releases[0].version.as_deref(), Some(expected));
    }
}

#[test]
fn unversioned_dated_records_are_published_but_unknown_notes_are_neither() {
    for (file, format, selection, count) in [
        ("examples/gnu-ChangeLog", "gnu-changelog", "--latest", 1),
        ("examples/gnu-ChangeLog", "gnu-changelog", "--unreleased", 0),
        (
            "examples/github-release-notes.md",
            "github-release-notes",
            "--latest",
            0,
        ),
        (
            "examples/github-release-notes.md",
            "github-release-notes",
            "--unreleased",
            0,
        ),
        (
            "examples/changesets-fragment.md",
            "changesets",
            "--latest",
            0,
        ),
    ] {
        let output = invoke(&["parse", file, "--input-format", format, selection]).unwrap();
        assert!(output.status.success());
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.releases.len(), count, "{file} {selection}");
    }
}

#[test]
fn filtering_preserves_original_source_links_and_spans_even_without_matches() {
    for version in ["2.0.0", "missing"] {
        let output =
            invoke(&["parse", "tests/fixtures/standard.md", "--version", version]).unwrap();
        assert!(output.status.success());
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.source, include_str!("fixtures/standard.md"));
        assert_ne!(model.links.len(), 0);
        if version == "missing" {
            assert_eq!(model.releases.len(), 0);
        } else {
            let release = &model.releases[0];
            assert!(
                model
                    .source
                    .get(release.span.start..release.span.end)
                    .unwrap()
                    .contains("2.0.0")
            );
        }
    }
    for options in [
        vec!["--category", "missing"],
        vec!["--component", "missing"],
        vec!["--package", "@LANTERN/core"],
    ] {
        let mut args = vec![
            "parse",
            "examples/changesets-fragment.md",
            "--input-format",
            "changesets",
        ];
        args.extend(options);
        let output = invoke(&args).unwrap();
        assert!(output.status.success());
        let model: Changelog = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(model.releases.len(), 0);
    }
}

#[test]
fn filtering_never_hides_invalid_releases_or_malformed_input() {
    for (file, options) in [
        ("tests/fixtures/invalid-dates.md", vec!["--version", "1.6"]),
        (
            "tests/fixtures/invalid-dates.md",
            vec!["--category", "missing"],
        ),
        ("tests/fixtures/malformed-heading.md", vec!["--latest"]),
    ] {
        let mut args = vec!["parse", file];
        args.extend(options);
        let output = invoke(&args).unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert!(String::from_utf8(output.stderr).unwrap().contains("error["));
    }
}

#[test]
fn conflicting_selections_and_invalid_limits_are_usage_errors() {
    for options in [
        vec!["--latest", "--version", "1"],
        vec!["--unreleased", "--version", "1"],
        vec!["--latest", "--unreleased"],
        vec!["--latest", "--limit", "1"],
        vec!["--limit", "0"],
        vec!["--limit", "-1"],
        vec!["--limit", "many"],
        vec!["--version"],
    ] {
        let mut args = vec!["parse", "tests/fixtures/standard.md"];
        args.extend(options);
        let output = invoke(&args).unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, Vec::<u8>::new());
        assert_ne!(output.stderr, Vec::<u8>::new());
    }
}

#[test]
fn compact_json_has_the_same_data_and_a_final_newline() {
    let pretty = invoke(&["parse", "tests/fixtures/standard.md", "--latest"]).unwrap();
    let compact = invoke(&[
        "parse",
        "tests/fixtures/standard.md",
        "--latest",
        "--output-format",
        "json-compact",
    ])
    .unwrap();
    assert!(pretty.status.success());
    assert!(compact.status.success());
    assert_eq!(
        serde_json::from_slice::<Changelog>(&pretty.stdout).unwrap(),
        serde_json::from_slice::<Changelog>(&compact.stdout).unwrap()
    );
    assert_eq!(
        String::from_utf8(compact.stdout.clone())
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(compact.stdout.ends_with(b"\n"));
    assert!(compact.stdout.len() < pretty.stdout.len());
}
