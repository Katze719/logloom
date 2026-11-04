//! Public API coverage for additional changelog and fragment formats.

use logloom::{
    Category, Changelog, DiagnosticCode, EntryKind, ErrorKind, Format, MAX_INPUT_BYTES, parse,
    parse_named, validate, validate_for_format,
};

const EXAMPLES: &[(Format, &str)] = &[
    (
        Format::ConventionalChangelog,
        include_str!("../examples/conventional-changelog.md"),
    ),
    (
        Format::GenericMarkdown,
        include_str!("../examples/generic-markdown.md"),
    ),
    (
        Format::GitHubReleaseNotes,
        include_str!("../examples/github-release-notes.md"),
    ),
    (
        Format::GitHubReleaseNotes,
        include_str!("../examples/github-release-notes.json"),
    ),
    (
        Format::Changesets,
        include_str!("../examples/changesets.md"),
    ),
    (
        Format::Changesets,
        include_str!("../examples/changesets-fragment.md"),
    ),
    (Format::Towncrier, include_str!("../examples/towncrier.rst")),
    (Format::Towncrier, include_str!("../examples/towncrier.md")),
    (
        Format::DebianChangelog,
        include_str!("../examples/debian.changelog"),
    ),
    (Format::GnuNews, include_str!("../examples/gnu-news.txt")),
    (
        Format::GnuChangeLog,
        include_str!("../examples/gnu-ChangeLog"),
    ),
];

fn check_spans(changelog: &Changelog) {
    for release in &changelog.releases {
        if let Some(span) = release.date_span {
            assert!(changelog.source.get(span.start..span.end).is_some());
        }
        assert!(
            changelog
                .source
                .get(release.span.start..release.span.end)
                .is_some()
        );
        for section in &release.sections {
            assert!(
                changelog
                    .source
                    .get(section.span.start..section.span.end)
                    .is_some()
            );
            for entry in &section.entries {
                assert!(
                    changelog
                        .source
                        .get(entry.span.start..entry.span.end)
                        .is_some()
                );
            }
        }
    }
    for link in &changelog.links {
        assert!(
            changelog
                .source
                .get(link.span.start..link.span.end)
                .is_some()
        );
    }
}

#[test]
fn examples_are_valid_and_round_trip_without_losing_source() {
    for &(format, input) in EXAMPLES {
        let changelog = parse(input, format).unwrap();
        assert_ne!(changelog.releases, Vec::new());
        assert_eq!(changelog.format, format);
        assert_eq!(validate(&changelog), Vec::new());
        assert_eq!(changelog.source, input);
        assert_eq!(
            validate_for_format(&changelog, format),
            Vec::new(),
            "{format:?}"
        );
        check_spans(&changelog);
        let serialized = serde_json::to_string(&changelog).unwrap();
        assert_eq!(
            serde_json::from_str::<Changelog>(&serialized).unwrap(),
            changelog
        );
    }
}

#[test]
fn conventional_histories_accept_major_minor_and_patch_heading_levels() {
    let changelog = parse(EXAMPLES[0].1, Format::ConventionalChangelog).unwrap();
    assert_eq!(
        changelog
            .releases
            .iter()
            .map(|release| release.version.as_deref())
            .collect::<Vec<_>>(),
        [Some("3.0.0"), Some("2.1.1"), Some("2.1.0")]
    );
    assert_eq!(
        changelog.releases[0].sections[0].category,
        Some(Category::Added)
    );
    assert_eq!(
        changelog.releases[1].sections[0].category,
        Some(Category::Fixed)
    );
    assert_eq!(
        changelog.releases[2].sections[0].category,
        Some(Category::Changed)
    );
    assert_eq!(
        changelog.releases[0].sections[1].category,
        Some(Category::Custom("⚠ BREAKING CHANGES".to_owned()))
    );
    assert!(
        changelog.releases[0].sections[0].entries[0]
            .markdown
            .contains("abc1234")
    );
    assert_eq!(changelog.releases[0].date.as_deref(), Some("2026-11-04"));
    assert!(
        changelog.releases[0]
            .link
            .as_deref()
            .unwrap()
            .contains("compare")
    );
}

#[test]
fn conventional_parser_does_not_reinterpret_nested_headings() {
    let input = "# 1.0 (2026-11-04)\n### Features\n- Parent\n  - Child\n\n  ```md\n  # 9.0 (2026-11-04)\n  ```\n\n> # 8.0 (2026-11-04)\n";
    let changelog = parse(input, Format::ConventionalChangelog).unwrap();
    assert_eq!(changelog.releases.len(), 1);
    assert!(
        changelog.releases[0].sections[0].entries[0]
            .markdown
            .contains("Child")
    );
    assert!(
        changelog.releases[0].sections[0].entries[0]
            .markdown
            .contains("# 9.0")
    );
}

#[test]
fn conventional_malformed_headings_and_missing_releases_are_errors() {
    for heading in ["1.0 (2026-11-04", "[1.0", "1.0 - "] {
        let input = format!("## {heading}\n");
        assert_eq!(
            parse(&input, Format::ConventionalChangelog)
                .unwrap_err()
                .kind,
            ErrorKind::MalformedHeading
        );
    }
    assert_eq!(
        parse(
            "# Changelog\n### Features\n- Hello\n",
            Format::ConventionalChangelog
        )
        .unwrap_err()
        .kind,
        ErrorKind::NoReleases
    );
}

#[test]
fn generic_markdown_retains_named_releases_and_unversioned_bodies() {
    let changelog = parse(EXAMPLES[1].1, Format::GenericMarkdown).unwrap();
    assert_eq!(
        changelog.releases[0].version.as_deref(),
        Some("Winter preview")
    );
    assert_eq!(changelog.releases[0].date.as_deref(), Some("2026-11-04"));
    assert_eq!(
        changelog.releases[0].sections[1].category,
        Some(Category::Custom("Highlights".to_owned()))
    );
    assert!(
        changelog.releases[0].sections[1].entries[0]
            .markdown
            .contains("Optional components")
    );
    let unversioned = parse(
        "# My changes\n\nA summary without release headings.\n\n- A change.\n",
        Format::GenericMarkdown,
    )
    .unwrap();
    assert_eq!(unversioned.releases[0].version, None);
    assert_eq!(
        unversioned.releases[0]
            .attributes
            .get("version-status")
            .map(String::as_str),
        Some("unknown")
    );
    assert_eq!(unversioned.metadata.title.as_deref(), Some("My changes"));
    assert_eq!(unversioned.releases[0].sections[0].entries.len(), 2);
}

#[test]
fn generic_numeric_categories_are_not_releases() {
    let changelog = parse(
        "## 1.0\n### 2026 improvements\n- Faster.\n",
        Format::GenericMarkdown,
    )
    .unwrap();
    assert_eq!(changelog.releases.len(), 1);
    assert_eq!(
        changelog.releases[0].sections[0].category,
        Some(Category::Custom("2026 improvements".to_owned()))
    );
}

#[test]
fn github_markdown_keeps_contributors_and_full_changelog_link() {
    let changelog = parse(EXAMPLES[2].1, Format::GitHubReleaseNotes).unwrap();
    assert_eq!(changelog.releases[0].version, None);
    assert_eq!(changelog.releases[0].sections.len(), 2);
    assert!(
        changelog.releases[0].sections[1]
            .entries
            .iter()
            .any(|entry| entry.markdown.contains("Full Changelog"))
    );
    let reference = parse(
        "[Guide](https://example.com)\n\n## What's Changed\n- New API.\n",
        Format::GitHubReleaseNotes,
    )
    .unwrap();
    assert!(
        reference.releases[0].sections[0].entries[0]
            .markdown
            .contains("Guide")
    );
}

#[test]
fn github_json_extracts_tags_dates_body_and_release_flags() {
    let changelog = parse(EXAMPLES[3].1, Format::GitHubReleaseNotes).unwrap();
    assert_eq!(changelog.releases.len(), 2);
    assert_eq!(changelog.releases[0].version.as_deref(), Some("v1.1.0"));
    assert_eq!(changelog.releases[0].date.as_deref(), Some("2026-11-04"));
    assert_eq!(changelog.releases[0].heading, "Lantern 1.1");
    assert!(
        changelog.releases[0].sections[0].entries[0]
            .markdown
            .contains("**streaming**")
    );
    assert_eq!(
        changelog.releases[0]
            .attributes
            .get("draft")
            .map(String::as_str),
        Some("false")
    );
    let draft = parse(r#"{"tagName":"v2-preview","name":"Preview","body":null,"publishedAt":null,"isDraft":true,"isPrerelease":true,"url":"https://example.com/release"}"#, Format::GitHubReleaseNotes).unwrap();
    assert_eq!(draft.releases[0].sections, Vec::new());
    assert_eq!(draft.releases[0].date, None);
    assert_eq!(
        draft.releases[0]
            .attributes
            .get("draft")
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        draft.releases[0].link.as_deref(),
        Some("https://example.com/release")
    );
}

#[test]
fn github_json_rejects_invalid_shapes_and_timestamps() {
    for input in [
        "{",
        "[]",
        "[42]",
        r#"{"body":"text"}"#,
        r#"{"tag_name":"","body":"text"}"#,
        r#"{"tag_name":"v1","body":42}"#,
        r#"{"tag_name":"v1","published_at":"2026-11-04TgarbageZ"}"#,
    ] {
        assert!(parse(input, Format::GitHubReleaseNotes).is_err(), "{input}");
    }
    let invalid_date = parse(
        r#"{"tag_name":"v1","published_at":"2025-02-29T10:00:00Z"}"#,
        Format::GitHubReleaseNotes,
    )
    .unwrap();
    assert_eq!(
        validate_for_format(&invalid_date, Format::GitHubReleaseNotes)[0].code,
        DiagnosticCode::InvalidDate
    );
}

#[test]
fn changesets_histories_preserve_package_and_bump_sections() {
    let changelog = parse(EXAMPLES[4].1, Format::Changesets).unwrap();
    assert_eq!(changelog.metadata.title.as_deref(), Some("@lantern/core"));
    assert_eq!(
        changelog.releases[0]
            .attributes
            .get("package")
            .map(String::as_str),
        Some("@lantern/core")
    );
    assert_eq!(changelog.releases[0].date, None);
    assert_eq!(
        changelog.releases[0].sections[0].category,
        Some(Category::Custom("Major Changes".to_owned()))
    );
    assert!(
        changelog.releases[0].sections[2].entries[0]
            .markdown
            .contains("@lantern/shared@1.2.0")
    );
}

#[test]
fn changeset_fragments_preserve_package_order_bumps_links_and_original_offsets() {
    let input = EXAMPLES[5].1;
    let changelog = parse(input, Format::Changesets).unwrap();
    assert_eq!(changelog.releases.len(), 2);
    assert_eq!(
        changelog.releases[0]
            .attributes
            .get("package")
            .map(String::as_str),
        Some("@lantern/core")
    );
    assert_eq!(
        changelog.releases[0]
            .attributes
            .get("bump")
            .map(String::as_str),
        Some("minor")
    );
    assert_eq!(
        changelog.releases[1]
            .attributes
            .get("package")
            .map(String::as_str),
        Some("@lantern/cli")
    );
    assert_eq!(
        changelog.releases[1]
            .attributes
            .get("bump")
            .map(String::as_str),
        Some("patch")
    );
    assert_eq!(changelog.releases[0].version, None);
    let entry = &changelog.releases[0].sections[0].entries[0];
    assert_eq!(
        entry.markdown,
        input.get(entry.span.start..entry.span.end).unwrap().trim()
    );
    assert_eq!(entry.span.line, 6);
    assert_eq!(changelog.links.len(), 1);
    assert_eq!(
        validate_for_format(&changelog, Format::Changesets),
        Vec::new()
    );
}

#[test]
fn changesets_reject_malformed_or_unsupported_frontmatter() {
    for input in [
        "---\npkg: minor\n",
        "---\npkg: invalid\n---\nsummary",
        "---\npkg: minor\npkg: patch\n---\nsummary",
        "---\npkg: [minor]\n---\nsummary",
        "---\npkg:\n  bump: minor\n---\nsummary",
    ] {
        assert!(parse(input, Format::Changesets).is_err());
    }
    let empty = parse(
        "---\n---\nA documentation-only change.\n",
        Format::Changesets,
    )
    .unwrap();
    assert_eq!(empty.releases.len(), 1);
    let commented = parse(
        "---\n# Comment\n'pkg': 'minor' # Bump\n---\nSummary.\n",
        Format::Changesets,
    )
    .unwrap();
    assert_eq!(
        commented.releases[0]
            .attributes
            .get("bump")
            .map(String::as_str),
        Some("minor")
    );
}

#[test]
fn towncrier_supports_rst_and_markdown_histories() {
    for input in [EXAMPLES[6].1, EXAMPLES[7].1] {
        let changelog = parse(input, Format::Towncrier).unwrap();
        assert_eq!(changelog.releases.len(), 2);
        assert_eq!(changelog.releases[0].version.as_deref(), Some("2.0.0"));
        assert_eq!(changelog.releases[0].date.as_deref(), Some("2026-11-04"));
        assert_eq!(
            changelog.releases[0].sections[0].category,
            Some(Category::Added)
        );
        assert_eq!(
            changelog.releases[0].sections[1].category,
            Some(Category::Fixed)
        );
        assert!(
            changelog.releases[0].sections[0].entries[0]
                .markdown
                .contains("#42")
        );
    }
}

#[test]
fn towncrier_fragments_use_filename_metadata_without_filesystem_access() {
    let input = include_str!("../examples/towncrier/123.bugfix.rst");
    let changelog = parse_named(input, Format::Towncrier, "123.bugfix.rst").unwrap();
    let release = &changelog.releases[0];
    assert_eq!(release.version, None);
    assert_eq!(
        release.attributes.get("issue").map(String::as_str),
        Some("123")
    );
    assert_eq!(
        release.attributes.get("type").map(String::as_str),
        Some("bugfix")
    );
    assert_eq!(release.sections[0].category, Some(Category::Fixed));
    assert_eq!(release.sections[0].entries[0].kind, EntryKind::Text);
    assert!(
        release.sections[0].entries[0]
            .markdown
            .contains("``read_file``")
    );
    let orphan = parse_named("A new feature.", Format::Towncrier, "+random.feature.md").unwrap();
    assert!(!orphan.releases[0].attributes.contains_key("issue"));
    assert_eq!(
        orphan.releases[0].sections[0].category,
        Some(Category::Added)
    );
    assert_eq!(
        parse(input, Format::Towncrier).unwrap().releases[0].sections[0].category,
        None
    );
    assert!(parse_named(input, Format::Towncrier, "invalid").is_err());
    assert!(parse_named("", Format::Towncrier, "123.bugfix").is_err());
    let misc = parse_named("", Format::Towncrier, "123.misc").unwrap();
    assert_eq!(
        misc.releases[0].attributes.get("type").map(String::as_str),
        Some("misc")
    );
}

#[test]
fn debian_records_preserve_epoch_distribution_urgency_maintainer_and_wrapped_entries() {
    let changelog = parse(EXAMPLES[8].1, Format::DebianChangelog).unwrap();
    let release = &changelog.releases[0];
    assert_eq!(release.version.as_deref(), Some("1:2.0.0-1"));
    assert_eq!(release.date.as_deref(), Some("2026-11-04"));
    assert_eq!(
        release.attributes.get("distributions").map(String::as_str),
        Some("unstable")
    );
    assert_eq!(
        release.attributes.get("urgency").map(String::as_str),
        Some("medium")
    );
    assert_eq!(
        release.attributes.get("maintainer").map(String::as_str),
        Some("Alice Example <alice@example.com>")
    );
    assert_eq!(
        release.sections[0].category,
        Some(Category::Custom("Alice Example".to_owned()))
    );
    assert_eq!(release.sections[0].entries.len(), 2);
    assert!(
        release.sections[0].entries[0]
            .markdown
            .contains("original order")
    );
    assert!(
        release.sections[0].entries[1]
            .markdown
            .contains("Closes: #123456")
    );
    assert_eq!(
        changelog.releases[1]
            .attributes
            .get("binary-only")
            .map(String::as_str),
        Some("yes")
    );
}

#[test]
fn debian_rejects_missing_trailers_bad_headers_and_bad_timestamp_syntax() {
    let input = EXAMPLES[8].1;
    for broken in [
        input.replace("urgency=medium", "missing=medium"),
        input.replace(
            "  Wed, 04 Nov 2026 12:30:00 +0000",
            " Wed, 04 Nov 2026 12:30:00 +0000",
        ),
        input.replace("12:30:00", "25:30:00"),
        "lantern (1.0) unstable; urgency=low\n\n  * Missing trailer.\n".to_owned(),
        "  * Orphan entry.\n".to_owned(),
    ] {
        assert_eq!(
            parse(&broken, Format::DebianChangelog).unwrap_err().kind,
            ErrorKind::MalformedInput
        );
    }
    let impossible = input.replace("04 Nov", "31 Nov");
    let changelog = parse(&impossible, Format::DebianChangelog).unwrap();
    assert_eq!(
        validate_for_format(&changelog, Format::DebianChangelog)[0].code,
        DiagnosticCode::InvalidDate
    );
}

#[test]
fn gnu_news_groups_paragraphs_categories_and_pending_placeholder() {
    let changelog = parse(EXAMPLES[9].1, Format::GnuNews).unwrap();
    assert_eq!(changelog.releases.len(), 3);
    assert_eq!(changelog.releases[0].version, None);
    assert_eq!(changelog.releases[0].date, None);
    assert_eq!(changelog.releases[1].version.as_deref(), Some("2.0"));
    assert_eq!(
        changelog.releases[1].sections[0].category,
        Some(Category::Fixed)
    );
    let entries = &changelog.releases[1].sections[0].entries;
    assert_eq!(entries.len(), 2);
    assert!(entries[0].markdown.contains("Continuation lines"));
    assert_eq!(entries[0].kind, EntryKind::Text);
}

#[test]
fn gnu_change_logs_group_by_date_and_author_without_inventing_versions() {
    let changelog = parse(EXAMPLES[10].1, Format::GnuChangeLog).unwrap();
    assert_eq!(changelog.releases.len(), 3);
    assert!(
        changelog
            .releases
            .iter()
            .all(|release| release.version.is_none())
    );
    assert_eq!(changelog.releases[0].date.as_deref(), Some("2026-11-04"));
    assert_eq!(changelog.releases[1].date, changelog.releases[0].date);
    assert!(
        changelog.releases[1]
            .attributes
            .get("author")
            .unwrap()
            .contains("Bob Example")
    );
    assert_eq!(changelog.releases[0].sections[0].entries.len(), 3);
    assert!(
        changelog.releases[0].sections[0].entries[1]
            .markdown
            .contains("Handle empty streams")
    );
    assert_eq!(
        validate_for_format(&changelog, Format::GnuChangeLog),
        Vec::new()
    );
}

#[test]
fn all_formats_preserve_crlf_and_valid_source_spans() {
    for &(format, input) in EXAMPLES {
        let input = input.replace('\n', "\r\n");
        let changelog = parse(&input, format).unwrap();
        assert_eq!(changelog.source, input);
        check_spans(&changelog);
        assert_eq!(validate_for_format(&changelog, format), Vec::new());
    }
}

#[test]
fn all_formats_bound_input_size_and_survive_adversarial_fragments() {
    let oversized = "x".repeat(MAX_INPUT_BYTES.saturating_add(1));
    let fragments = [
        "## [",
        "\0🦀",
        "```\n## 1.0\n```",
        "> ",
        "[",
        "{",
        "---\npkg: minor\n",
        "2026-11-04  Author\n\t* file: change",
    ];
    for &(format, _) in EXAMPLES {
        assert_eq!(
            parse(&oversized, format).unwrap_err().kind,
            ErrorKind::ResourceLimit
        );
        for first in fragments {
            for second in fragments {
                let input = format!("{first}\n{second}\n");
                assert!(std::panic::catch_unwind(|| parse(&input, format)).is_ok());
            }
        }
    }
}

#[test]
fn changesets_bound_summary_amplification_across_packages() {
    let input = format!("---\na: minor\nb: patch\n---\n{}", "x".repeat(9_000_000));
    assert_eq!(
        parse(&input, Format::Changesets).unwrap_err().kind,
        ErrorKind::ResourceLimit
    );
}

#[test]
fn deeply_nested_json_and_markdown_do_not_panic() {
    let json = format!("{}0{}", "[".repeat(2000), "]".repeat(2000));
    assert!(
        std::panic::catch_unwind(|| parse(&json, Format::GitHubReleaseNotes))
            .unwrap()
            .is_err()
    );
    for format in [
        Format::ConventionalChangelog,
        Format::GenericMarkdown,
        Format::Changesets,
        Format::Towncrier,
        Format::GitHubReleaseNotes,
    ] {
        let input = format!("## 1.0\n{}deep\n", "> ".repeat(2000));
        assert_eq!(
            std::panic::catch_unwind(|| parse(&input, format))
                .unwrap()
                .unwrap_err()
                .kind,
            ErrorKind::ResourceLimit
        );
    }
}

#[test]
fn named_towncrier_fragments_do_not_turn_body_headings_into_published_releases() {
    let changelog = parse_named(
        "## 2.0.0 (2026-11-04)\n\nAn upgrade guide for this fix.\n",
        Format::Towncrier,
        "123.bugfix.md",
    )
    .unwrap();
    assert_eq!(changelog.releases.len(), 1);
    assert_eq!(changelog.releases[0].version, None);
    assert_eq!(changelog.releases[0].date, None);
    assert_eq!(
        changelog.releases[0]
            .attributes
            .get("issue")
            .map(String::as_str),
        Some("123")
    );
    let untyped = parse_named("Unversioned notes.", Format::Towncrier, "NEWS.rst").unwrap();
    assert!(!untyped.releases[0].attributes.contains_key("issue"));
    assert!(!untyped.releases[0].attributes.contains_key("type"));
    assert!(parse_named("", Format::Towncrier, "NEWS.rst").is_err());
}

#[test]
fn legacy_json_without_source_format_still_deserializes_as_keep_a_changelog() {
    let original = parse(
        include_str!("../examples/keep-a-changelog.md"),
        Format::KeepAChangelog,
    )
    .unwrap();
    let mut encoded = serde_json::to_value(&original).unwrap();
    encoded.as_object_mut().unwrap().remove("format");
    let decoded: Changelog = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(validate(&decoded), Vec::new());
}

#[test]
fn external_date_locations_point_to_the_actual_timestamp() {
    let input = "{\n  \"tag_name\":\"v1\",\n  \"body\":\"Notes.\",\n  \"published_at\":\"2025-02-29T10:00:00Z\"\n}\n";
    let github = parse(input, Format::GitHubReleaseNotes).unwrap();
    assert_eq!(validate(&github)[0].line, Some(4));
    let date_span = github.releases[0].date_span.unwrap();
    assert!(
        input
            .get(date_span.start..date_span.end)
            .unwrap()
            .contains("2025-02-29")
    );
    let input = include_str!("../examples/debian.changelog").replace("04 Nov", "31 Nov");
    let debian = parse(&input, Format::DebianChangelog).unwrap();
    assert_eq!(validate(&debian)[0].line, Some(8));
}

#[test]
fn debian_timestamp_spacing_is_strict_but_maintainer_names_can_contain_double_spaces() {
    let source = include_str!("../examples/debian.changelog");
    let name = source.replace("Alice Example <", "Alice  Example <");
    let parsed = parse(&name, Format::DebianChangelog).unwrap();
    assert!(
        parsed.releases[0]
            .attributes
            .get("maintainer")
            .unwrap()
            .contains("Alice  Example")
    );
    for invalid in [
        source.replace("12:30:00", "1:30:00"),
        source.replace("  Wed,", "   Wed,"),
        source.replace("Nov 2026", "Nov +123"),
        source.replace("+0000", "++000"),
    ] {
        assert_eq!(
            parse(&invalid, Format::DebianChangelog).unwrap_err().kind,
            ErrorKind::MalformedInput
        );
    }
}
