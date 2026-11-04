//! Integration coverage of the public library and Markdown boundaries.

use logloom::{
    Category, Changelog, DiagnosticCode, EntryKind, ErrorKind, Format, MAX_INPUT_BYTES, Severity,
    parse, validate,
};

const STANDARD: &str = include_str!("fixtures/standard.md");
const RICH: &str = include_str!("fixtures/rich.md");

#[test]
fn standard_document_preserves_releases_categories_entries_and_links() {
    let changelog = parse(STANDARD, Format::KeepAChangelog).unwrap();
    assert_eq!(changelog.metadata.title.as_deref(), Some("Changelog"));
    assert!(changelog.metadata.description.contains("License: MIT"));
    assert_eq!(changelog.source, STANDARD);
    assert_eq!(changelog.releases.len(), 3);
    assert_eq!(changelog.releases[0].version, None);
    assert_eq!(changelog.releases[1].version.as_deref(), Some("2.1.0"));
    assert_eq!(changelog.releases[1].date.as_deref(), Some("2026-10-14"));
    assert!(changelog.releases[2].yanked);
    assert_eq!(
        changelog.releases[0].sections[0].category,
        Some(Category::Added)
    );
    let entries = &changelog.releases[0].sections[0].entries;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].markdown, "- A streaming export API.");
    assert_eq!(entries[1].markdown, "- A command-line preview.");
    assert_eq!(
        changelog.releases[1]
            .sections
            .iter()
            .map(|section| section.category.clone())
            .collect::<Vec<_>>(),
        vec![
            Some(Category::Changed),
            Some(Category::Deprecated),
            Some(Category::Removed),
            Some(Category::Fixed),
            Some(Category::Security)
        ]
    );
    assert_eq!(changelog.links.len(), 3);
    assert_eq!(
        changelog.releases[0].link.as_deref(),
        Some("https://example.com/compare/v2.1.0...HEAD")
    );
    assert_eq!(validate(&changelog), Vec::new());
}

#[test]
fn empty_and_non_changelog_documents_return_typed_errors() {
    for input in ["", " \r\n\t"] {
        assert_eq!(
            parse(input, Format::KeepAChangelog).unwrap_err().kind,
            ErrorKind::EmptyInput
        );
    }
    for input in [
        "# Changelog\n",
        "A description.",
        "```\n## [Unreleased]\n```",
        "<!--\n## [Unreleased]\n-->",
    ] {
        assert_eq!(
            parse(input, Format::KeepAChangelog).unwrap_err().kind,
            ErrorKind::NoReleases
        );
    }
}

#[test]
fn only_unreleased_and_empty_releases_are_valid() {
    let changelog = parse(
        include_str!("fixtures/only-unreleased.md"),
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(changelog.releases.len(), 1);
    assert_eq!(changelog.releases[0].sections, Vec::new());
    assert_eq!(validate(&changelog), Vec::new());
    let empty_release = parse(
        "# Changelog\n## 1.0 - 2026-11-04\n### Added\n### Fixed\n",
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(empty_release.releases[0].sections.len(), 2);
    assert!(
        empty_release.releases[0]
            .sections
            .iter()
            .all(|section| section.entries.is_empty())
    );
    assert_eq!(validate(&empty_release), Vec::new());
}

#[test]
fn missing_dates_custom_categories_and_non_semver_are_preserved() {
    let changelog = parse(
        include_str!("fixtures/unconventional.md"),
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(changelog.metadata.title, None);
    assert_eq!(
        changelog.releases[0].version.as_deref(),
        Some("nightly-2026.11")
    );
    assert_eq!(changelog.releases[0].date, None);
    assert_eq!(changelog.releases[0].sections[0].category, None);
    assert_eq!(
        changelog.releases[0].sections[1].category,
        Some(Category::Custom("Performance".to_owned()))
    );
    assert_eq!(changelog.releases[0].sections[2].entries, Vec::new());
    assert_eq!(changelog.releases[1].version, None);
    assert_eq!(changelog.releases[2].sections, Vec::new());
    let diagnostics = validate(&changelog);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Warning)
    );
    for code in [
        DiagnosticCode::MissingTitle,
        DiagnosticCode::MissingDate,
        DiagnosticCode::CustomCategory,
        DiagnosticCode::UncategorizedContent,
        DiagnosticCode::PendingReleaseOrder,
    ] {
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code == code));
    }
}

#[test]
fn reference_definitions_resolve_and_keep_original_labels_and_source() {
    let source = include_str!("fixtures/reference-links.md");
    let changelog = parse(source, Format::KeepAChangelog).unwrap();
    assert_eq!(changelog.releases[0].version.as_deref(), Some("1.0"));
    assert_eq!(
        changelog.releases[0].link.as_deref(),
        Some("https://example.com/compare/0.9...1.0")
    );
    assert_eq!(changelog.links.len(), 3);
    assert_eq!(changelog.links[0].label, "release");
    assert_eq!(
        changelog.links[1].title.as_deref(),
        Some("Documentation title")
    );
    assert_eq!(changelog.releases[0].sections[0].entries.len(), 1);
    assert!(
        changelog.releases[0].sections[0].entries[0]
            .markdown
            .contains("[manual][docs]")
    );
    assert_eq!(changelog.source, source);
}

#[test]
fn rich_and_nested_markdown_stays_inside_its_entry() {
    let changelog = parse(RICH, Format::KeepAChangelog).unwrap();
    assert_eq!(changelog.metadata.title.as_deref(), Some("Lantern changes"));
    assert!(changelog.metadata.description.contains("<!--"));
    assert_eq!(changelog.releases.len(), 1);
    assert_eq!(changelog.releases[0].sections.len(), 3);
    let entries = &changelog.releases[0].sections[1].entries;
    assert_eq!(entries.len(), 10);
    assert!(entries[0].markdown.contains("A second paragraph"));
    assert!(entries[0].markdown.contains("- A deeper child."));
    assert!(entries[0].markdown.contains("## This is not a release"));
    assert_eq!(entries[0].kind, EntryKind::ListItem);
    assert!(entries[1].markdown.starts_with("- [x]"));
    assert!(entries[2].markdown.starts_with("1."));
    assert!(entries[3].markdown.starts_with("2."));
    assert!(entries[4].markdown.starts_with('>'));
    assert!(entries[5].markdown.starts_with('|'));
    assert!(entries[6].markdown.starts_with("####"));
    assert!(entries[8].markdown.starts_with("```rust"));
    assert!(entries[9].markdown.contains("<details>"));
}

#[test]
fn crlf_preserves_meaning_and_source_offsets() {
    let crlf = STANDARD.replace('\n', "\r\n");
    let unix = parse(STANDARD, Format::KeepAChangelog).unwrap();
    let windows = parse(&crlf, Format::KeepAChangelog).unwrap();
    assert_eq!(windows.source, crlf);
    for (left, right) in unix.releases.iter().zip(&windows.releases) {
        assert_eq!(left.version, right.version);
        assert_eq!(left.date, right.date);
        assert_eq!(left.span.line, right.span.line);
        assert_eq!(
            right.heading,
            crlf.get(right.span.start..right.span.end).unwrap().trim()
        );
    }
}

#[test]
fn malformed_headings_have_source_locations() {
    let error = parse(
        include_str!("fixtures/malformed-heading.md"),
        Format::KeepAChangelog,
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::MalformedHeading);
    assert_eq!(error.line, Some(3));
    assert!(error.to_string().contains("line 3"));
    for heading in [
        "[1.0",
        "1.0]",
        "[]",
        "",
        "1.0 -",
        "1.0 2026-11-04",
        "1.0 – 2026-11-04",
    ] {
        let input = format!("# Changelog\n## {heading}\n");
        assert_eq!(
            parse(&input, Format::KeepAChangelog).unwrap_err().kind,
            ErrorKind::MalformedHeading
        );
    }
}

#[test]
fn dates_are_preserved_during_parsing_and_checked_separately() {
    let changelog = parse(
        include_str!("fixtures/invalid-dates.md"),
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(changelog.releases[2].date.as_deref(), Some("04/03/2024"));
    assert_eq!(
        validate(&changelog)
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::InvalidDate)
            .count(),
        4
    );
    for date in ["2000-02-29", "2024-02-29", "0001-01-01", "9999-12-31"] {
        let input = format!("# Changelog\n## v1 - {date}\n");
        assert_eq!(
            validate(&parse(&input, Format::KeepAChangelog).unwrap()),
            Vec::new()
        );
    }
    for date in [
        "1900-02-29",
        "2024-00-01",
        "2024-01-00",
        "2024-04-31",
        "2024-1-01",
        "0000-01-01",
        "2024-01-01T12:00:00Z",
        "２０２４-01-01",
    ] {
        let input = format!("# Changelog\n## v1 - {date}\n");
        assert_eq!(
            validate(&parse(&input, Format::KeepAChangelog).unwrap())[0].code,
            DiagnosticCode::InvalidDate
        );
    }
}

#[test]
fn setext_headings_and_case_insensitive_categories_work() {
    let changelog = parse(include_str!("fixtures/setext.md"), Format::KeepAChangelog).unwrap();
    assert_eq!(changelog.metadata.title.as_deref(), Some("Changelog"));
    assert_eq!(changelog.releases[0].version.as_deref(), Some("1.0.0"));
    assert_eq!(
        changelog.releases[0].sections[0].category,
        Some(Category::Fixed)
    );
    let changelog = parse(
        "## unreleased\n### aDdEd\n* An entry.",
        Format::KeepAChangelog,
    )
    .unwrap();
    assert_eq!(changelog.releases[0].version, None);
    assert_eq!(
        changelog.releases[0].sections[0].category,
        Some(Category::Added)
    );
}

#[test]
fn unsupported_document_structure_is_distinct_from_malformed_input() {
    for input in [
        "### Added\n## Unreleased",
        "# First\n# Second\n## Unreleased",
        "## Unreleased\n# Another document",
    ] {
        assert_eq!(
            parse(input, Format::KeepAChangelog).unwrap_err().kind,
            ErrorKind::UnsupportedStructure
        );
    }
}

#[test]
fn validation_reports_duplicates_and_order_without_reordering() {
    let input = "# Changelog\n## 1 - 2024-01-01\n## 2 - 2025-01-01\n## 1\n## Unreleased - 2026-11-04\n## Unreleased\n";
    let changelog = parse(input, Format::KeepAChangelog).unwrap();
    let diagnostics = validate(&changelog);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::DuplicateRelease)
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::ReleaseDateOrder)
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::DatedPendingRelease)
    );
    assert_eq!(changelog.releases[0].version.as_deref(), Some("1"));
}

#[test]
fn model_round_trips_through_json() {
    let changelog = parse(STANDARD, Format::KeepAChangelog).unwrap();
    let encoded = serde_json::to_string(&changelog).unwrap();
    let decoded: Changelog = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, changelog);
}

#[test]
fn large_documents_work_and_oversized_inputs_return_errors() {
    let input = format!(
        "# Changelog\n## Unreleased\n### Added\n{}",
        "- A change with Unicode 🦀.\n".repeat(50_000)
    );
    assert_eq!(
        parse(&input, Format::KeepAChangelog).unwrap().releases[0].sections[0]
            .entries
            .len(),
        50_000
    );
    let oversized = format!("{}\n", "x".repeat(MAX_INPUT_BYTES));
    assert_eq!(
        parse(&oversized, Format::KeepAChangelog).unwrap_err().kind,
        ErrorKind::ResourceLimit
    );
}

#[test]
fn deeply_nested_input_is_bounded_without_panicking() {
    let input = format!(
        "# Changelog\n## Unreleased\n### Added\n{}nested\n",
        "> ".repeat(2000)
    );
    let result = std::panic::catch_unwind(|| parse(&input, Format::KeepAChangelog));
    assert_eq!(result.unwrap().unwrap_err().kind, ErrorKind::ResourceLimit);
}

#[test]
fn adversarial_markdown_fragments_never_panic() {
    let fragments = [
        "\0",
        "## [",
        "\n",
        "🦀é",
        "```",
        "<!--",
        "[x]: /target",
        "> ## heading",
        "- [ ] **text**",
        "## Unreleased",
    ];
    for first in fragments {
        for second in fragments {
            for third in fragments {
                let input =
                    format!("# Changelog\n## Unreleased\n### Added\n{first}\n{second}\n{third}\n");
                assert!(std::panic::catch_unwind(|| parse(&input, Format::KeepAChangelog)).is_ok());
            }
        }
    }
}
